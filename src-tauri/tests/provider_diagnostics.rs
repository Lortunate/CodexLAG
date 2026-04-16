use std::path::PathBuf;

use codexlag_lib::{
    bootstrap::bootstrap_state_for_test_at,
    commands::logs::provider_diagnostics_from_runtime,
    gateway::runtime_routing::RouteDebugSnapshot,
    models::{
        relay_api_key_credential_ref, ImportedOfficialAccount, ManagedRelay, OfficialAuthMode,
        OfficialSession, ProviderSessionSummary, RelayBalanceAdapter,
    },
    secret_store::SecretKey,
    state::{RuntimeLogConfig, RuntimeState},
};
use rand::{rngs::OsRng, RngCore};

#[test]
fn diagnostics_surface_includes_auth_provider_capability_and_routing_sections() {
    let database_path = temp_database_path("codexlag-provider-diagnostics");
    let log_dir = database_path
        .parent()
        .expect("test database path should have a parent")
        .join("logs");
    let mut app_state = tokio::runtime::Runtime::new()
        .expect("create tokio runtime")
        .block_on(bootstrap_state_for_test_at(&database_path))
        .expect("bootstrap isolated app state");

    app_state
        .save_imported_official_account(ImportedOfficialAccount {
            account_id: "official-primary".into(),
            name: "Primary Publisher".into(),
            provider: "openai".into(),
            session: OfficialSession {
                session_id: "session:official-primary".into(),
                account_identity: Some("primary@example.test".into()),
                auth_mode: Some(OfficialAuthMode::ApiKey),
                refresh_capability: Some(true),
                quota_capability: Some(false),
                last_verified_at_ms: None,
                status: "active".into(),
                entitlement: Default::default(),
            },
            session_credential_ref: "credential://official/session/official-primary".into(),
            token_credential_ref: "credential://official/token/official-primary".into(),
        })
        .expect("save official account");
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
            &SecretKey::new("credential://official/session/official-primary"),
            "session-cookie".into(),
        )
        .expect("store official session secret");
    app_state
        .store_secret(
            &SecretKey::new("credential://official/token/official-primary"),
            serde_json::json!({ "api_key": "official-key" }).to_string(),
        )
        .expect("store official token secret");
    app_state
        .store_secret(
            &SecretKey::new(relay_api_key_credential_ref("relay-newapi")),
            "relay-key".into(),
        )
        .expect("store relay api key");

    let runtime =
        RuntimeState::start(app_state, RuntimeLogConfig { log_dir }).expect("start runtime");
    runtime
        .openai_auth_mut()
        .store_session(
            ProviderSessionSummary {
                provider_id: "openai_official".into(),
                account_id: "openai-primary".into(),
                display_name: "OpenAI Primary".into(),
                auth_state: "active".into(),
                expires_at_ms: Some(1_731_111_222_000),
                last_refresh_at_ms: Some(1_731_111_111_000),
                last_refresh_error: None,
            },
            "session-cookie".into(),
            serde_json::json!({
                "access_token": "fresh-access-token",
                "refresh_token": "refresh-token",
            })
            .to_string(),
        )
        .expect("store provider session");
    runtime
        .openai_auth_mut()
        .store_session(
            ProviderSessionSummary {
                provider_id: "openai_official".into(),
                account_id: "openai-stale".into(),
                display_name: "OpenAI Stale".into(),
                auth_state: "expired".into(),
                expires_at_ms: Some(1_731_000_000_000),
                last_refresh_at_ms: Some(1_731_111_100_000),
                last_refresh_error: Some("refresh rejected by provider".into()),
            },
            "expired-session-cookie".into(),
            serde_json::json!({
                "access_token": "expired-access-token",
                "refresh_token": "stale-refresh-token",
            })
            .to_string(),
        )
        .expect("store degraded provider session");
    runtime.record_balance_refresh_summary(
        "relay:relay-newapi @ 2026-04-15T08:00:00Z (queryable)".into(),
    );
    assert!(
        runtime
            .loopback_gateway()
            .state()
            .set_endpoint_availability("official-primary", false),
        "official endpoint should exist in runtime candidates"
    );
    runtime
        .loopback_gateway()
        .state()
        .set_last_route_debug_snapshot(Some(RouteDebugSnapshot {
            request_id: "req-diagnostics".into(),
            selected_endpoint_id: "relay-newapi".into(),
            attempt_count: 2,
        }));

    let diagnostics = provider_diagnostics_from_runtime(&runtime).expect("provider diagnostics");
    let section_ids = diagnostics
        .sections
        .iter()
        .map(|section| section.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        section_ids,
        vec![
            "auth_health",
            "provider_health",
            "capability_probe",
            "routing_visibility",
        ]
    );

    let auth_section = diagnostics
        .sections
        .iter()
        .find(|section| section.id == "auth_health")
        .expect("auth health section");
    assert!(
        auth_section
            .rows
            .iter()
            .any(|row| row.key == "openai-primary" && row.status == "healthy"),
        "stored OpenAI sessions should surface as healthy auth rows"
    );
    let healthy_auth_row = find_row(auth_section, "openai-primary");
    assert_eq!(
        detail_value(healthy_auth_row, "refresh_outcome"),
        Some("succeeded"),
        "healthy sessions should report a successful refresh outcome"
    );
    let degraded_auth_row = find_row(auth_section, "openai-stale");
    assert_eq!(degraded_auth_row.status, "error");
    assert_eq!(
        detail_value(degraded_auth_row, "refresh_outcome"),
        Some("failed"),
        "expired sessions with refresh failures should expose a failed outcome"
    );
    assert_eq!(
        detail_value(degraded_auth_row, "last_refresh_error"),
        Some("refresh rejected by provider"),
        "refresh diagnostics should surface the last refresh failure reason"
    );

    let provider_section = diagnostics
        .sections
        .iter()
        .find(|section| section.id == "provider_health")
        .expect("provider health section");
    assert!(
        provider_section
            .rows
            .iter()
            .any(|row| row.key == "official-primary"),
        "persisted official inventory should project into provider health diagnostics"
    );
    assert!(
        provider_section
            .rows
            .iter()
            .any(|row| row.key == "relay-newapi"),
        "persisted relay inventory should project into provider health diagnostics"
    );
    let rejected_provider_row = find_row(provider_section, "official-primary");
    assert_eq!(rejected_provider_row.status, "error");
    assert_eq!(
        detail_value(rejected_provider_row, "rejection_reason"),
        Some("unavailable"),
        "provider diagnostics should expose candidate rejection reasons"
    );
    assert_eq!(
        detail_value(rejected_provider_row, "routing_decision"),
        Some("rejected"),
        "provider diagnostics should structure rejected candidate decisions"
    );
    let selected_provider_row = find_row(provider_section, "relay-newapi");
    assert_eq!(
        detail_value(selected_provider_row, "routing_decision"),
        Some("selected"),
        "selected candidates should be marked explicitly in provider diagnostics"
    );
    assert_eq!(
        detail_value(selected_provider_row, "selected_rationale"),
        Some("selected_for_last_route"),
        "selected candidates should expose a rationale when a last-route snapshot exists"
    );

    let capability_section = diagnostics
        .sections
        .iter()
        .find(|section| section.id == "capability_probe")
        .expect("capability probe section");
    assert!(
        capability_section
            .rows
            .iter()
            .any(|row| row.key == "official-primary"),
        "official capability metadata should be included in diagnostics"
    );

    let routing_section = diagnostics
        .sections
        .iter()
        .find(|section| section.id == "routing_visibility")
        .expect("routing visibility section");
    assert!(
        routing_section
            .rows
            .iter()
            .any(|row| row.key == "current-mode"),
        "routing visibility should summarize the active routing mode"
    );
    let last_route_row = find_row(routing_section, "last-route");
    assert_eq!(last_route_row.value, "relay-newapi");
    assert_eq!(
        detail_value(last_route_row, "selected_pool"),
        Some("relay"),
        "routing visibility should expose the selected provider pool"
    );
    assert_eq!(
        detail_value(last_route_row, "selected_rationale"),
        Some("selected_for_last_route"),
        "routing visibility should expose the selected-candidate rationale"
    );
    let balance_row = find_row(routing_section, "last-balance-refresh");
    assert_eq!(
        detail_value(balance_row, "refresh_outcome"),
        Some("queryable"),
        "routing visibility should preserve the parsed balance refresh outcome"
    );
}

fn find_row<'a>(
    section: &'a codexlag_lib::logging::diagnostics::DiagnosticsSection,
    key: &str,
) -> &'a codexlag_lib::logging::diagnostics::DiagnosticsRow {
    section
        .rows
        .iter()
        .find(|row| row.key == key)
        .unwrap_or_else(|| panic!("expected diagnostics row '{key}'"))
}

fn detail_value<'a>(
    row: &'a codexlag_lib::logging::diagnostics::DiagnosticsRow,
    label: &str,
) -> Option<&'a str> {
    row.details
        .iter()
        .find(|detail| detail.label == label)
        .map(|detail| detail.value.as_str())
}

fn temp_database_path(prefix: &str) -> PathBuf {
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
