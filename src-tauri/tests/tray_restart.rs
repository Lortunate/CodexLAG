use codexlag_lib::{
    bootstrap::bootstrap_runtime_for_test,
    commands::accounts::refresh_account_balance_from_runtime,
    routing::{engine::PoolKind, policy::RoutingMode},
    tray::{apply_tray_action_for_runtime, build_tray_model_for_runtime, TrayItemId, TrayModel},
};

fn tray_label(model: &TrayModel, id: TrayItemId) -> String {
    model
        .items
        .iter()
        .find(|item| item.id == id)
        .map(|item| item.label.text().to_string())
        .unwrap_or_else(|| panic!("missing tray item {:?}", id))
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
        "Available endpoints | official: 1, relay: 1"
    );
    assert_eq!(
        tray_label(&model, TrayItemId::LastBalanceRefresh),
        "Last balance refresh | none"
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
        "Available endpoints | official: 1, relay: 0"
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
        "Available endpoints | official: 1, relay: 1"
    );
    assert_eq!(
        tray_label(&restarted, TrayItemId::LastBalanceRefresh),
        format!(
            "Last balance refresh | account:official-primary @ {} (non_queryable)",
            refresh.refreshed_at
        )
    );
}
