use codexlag_lib::{
    bootstrap::{bootstrap_runtime_for_test, bootstrap_state_for_test_at},
    commands::accounts::refresh_account_balance_from_runtime,
    gateway::runtime_routing::RouteDebugSnapshot,
    models::{relay_api_key_credential_ref, ManagedRelay, RelayBalanceAdapter},
    routing::{
        engine::{endpoint_rejection_reason, wall_clock_now_ms, PoolKind},
        policy::RoutingMode,
    },
    secret_store::SecretKey,
    state::{RuntimeLogConfig, RuntimeState},
    tray::{apply_tray_action_for_runtime, build_tray_model_for_runtime, TrayItemId, TrayModel},
};
use rand::{rngs::OsRng, RngCore};

fn tray_label(model: &TrayModel, id: TrayItemId) -> String {
    model
        .items
        .iter()
        .find(|item| item.id == id)
        .map(|item| item.label.text().to_string())
        .unwrap_or_else(|| panic!("missing tray item {:?}", id))
}

fn available_endpoints_label(runtime: &codexlag_lib::state::RuntimeState) -> String {
    let mut available_official = 0usize;
    let mut available_relay = 0usize;
    let now_ms = wall_clock_now_ms();
    for candidate in runtime.loopback_gateway().state().current_candidates() {
        if endpoint_rejection_reason(&candidate, now_ms).is_some() {
            continue;
        }
        match candidate.pool {
            PoolKind::Official => available_official += 1,
            PoolKind::Relay => available_relay += 1,
        }
    }

    format!("Available endpoints | official: {available_official}, relay: {available_relay}")
}

#[tokio::test]
async fn tray_model_exposes_operational_summary_lines() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let model = build_tray_model_for_runtime(&runtime);

    assert_eq!(
        tray_label(&model, TrayItemId::CurrentMode),
        "Default key state | Current mode: hybrid"
    );
    assert_eq!(
        tray_label(&model, TrayItemId::GatewayStatus),
        "Gateway status | ready"
    );
    assert_eq!(
        tray_label(&model, TrayItemId::ListenAddress),
        "Listen address | http://127.0.0.1:8787"
    );
    assert_eq!(
        tray_label(&model, TrayItemId::AvailableEndpoints),
        available_endpoints_label(&runtime)
    );
    assert_eq!(
        tray_label(&model, TrayItemId::LastBalanceRefresh),
        "Last balance refresh | none"
    );
}

#[tokio::test]
async fn tray_summary_counts_inventory_derived_official_and_relay_candidates() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let model = runtime.tray_model();
    let labels = model
        .items
        .iter()
        .map(|item| item.label.text().to_string())
        .collect::<Vec<_>>();

    assert!(labels.iter().any(|label| label.contains("official:")));
    assert!(labels.iter().any(|label| label.contains("relay:")));
}

#[test]
fn tray_summary_appends_last_route_debug_when_available() {
    let database_path = temp_database_path("codexlag-tray-diagnostics");
    let log_dir = database_path
        .parent()
        .expect("test database path should have a parent")
        .join("logs");
    let mut app_state = tokio::runtime::Runtime::new()
        .expect("create tokio runtime")
        .block_on(bootstrap_state_for_test_at(&database_path))
        .expect("bootstrap isolated app state");
    app_state
        .save_managed_relay(ManagedRelay {
            relay_id: "relay-newapi".into(),
            name: "Relay Alpha".into(),
            endpoint: "https://relay.example.test".into(),
            adapter: RelayBalanceAdapter::NewApi,
            api_key_credential_ref: relay_api_key_credential_ref("relay-newapi"),
        })
        .expect("save relay");
    app_state
        .store_secret(
            &SecretKey::new(relay_api_key_credential_ref("relay-newapi")),
            "relay-key".into(),
        )
        .expect("store relay api key");
    let runtime =
        RuntimeState::start(app_state, RuntimeLogConfig { log_dir }).expect("start runtime");
    runtime
        .loopback_gateway()
        .state()
        .set_last_route_debug_snapshot(Some(RouteDebugSnapshot {
            request_id: "req-tray-1".into(),
            selected_endpoint_id: "official-primary".into(),
            attempt_count: 2,
        }));

    let model = build_tray_model_for_runtime(&runtime);

    assert_eq!(
        tray_label(&model, TrayItemId::GatewayStatus),
        "Gateway status | ready | last route: official-primary (attempts 2)"
    );
}

#[tokio::test]
async fn restart_tray_action_restarts_the_real_gateway_host() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    runtime
        .set_current_mode(RoutingMode::RelayOnly)
        .expect("switch to relay-only");
    let before = runtime.gateway_host_status();

    for candidate in runtime.loopback_gateway().state().current_candidates() {
        if candidate.pool == PoolKind::Relay {
            let updated = runtime
                .loopback_gateway()
                .state()
                .set_endpoint_availability(candidate.id.as_str(), false);
            assert!(updated, "relay candidate availability should be mutable");
        }
    }

    let refresh = refresh_account_balance_from_runtime(&runtime, "official-primary".to_string())
        .expect("account refresh should succeed");
    let degraded = build_tray_model_for_runtime(&runtime);
    assert_eq!(
        tray_label(&degraded, TrayItemId::GatewayStatus),
        "Gateway status | unavailable (no available endpoint for mode 'relay_only')"
    );
    assert_eq!(
        tray_label(&degraded, TrayItemId::AvailableEndpoints),
        available_endpoints_label(&runtime)
    );

    let restarted = apply_tray_action_for_runtime(&runtime, TrayItemId::RestartGateway)
        .expect("restart tray action should succeed");
    let after = runtime.gateway_host_status();

    assert!(before.is_running);
    assert!(after.is_running);
    assert_eq!(after.listen_addr.ip().to_string(), "127.0.0.1");
    assert_eq!(after.listen_addr.port(), 8787);
    assert_eq!(restarted.current_mode(), Some(RoutingMode::RelayOnly));
    assert_eq!(
        tray_label(&restarted, TrayItemId::GatewayStatus),
        "Gateway status | ready | last restart: ok"
    );
    assert_eq!(
        tray_label(&restarted, TrayItemId::AvailableEndpoints),
        available_endpoints_label(&runtime)
    );
    assert_eq!(
        tray_label(&restarted, TrayItemId::LastBalanceRefresh),
        format!(
            "Last balance refresh | account:official-primary @ {} (non_queryable)",
            refresh.refreshed_at
        )
    );
}

fn temp_database_path(prefix: &str) -> std::path::PathBuf {
    std::env::temp_dir()
        .join("codexlag-tests")
        .join(random_suffix())
        .join(format!("{prefix}.sqlite3"))
}

fn random_suffix() -> String {
    let mut bytes = [0_u8; 16];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
