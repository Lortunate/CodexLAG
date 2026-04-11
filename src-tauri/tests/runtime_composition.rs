use codexlag_lib::{
    bootstrap::bootstrap_runtime_for_test,
    commands::{
        keys::default_key_summary_from_state,
        logs::log_summary_from_runtime,
        policies::policy_summaries_from_state,
    },
    routing::policy::{RoutingMode, HYBRID},
};

#[tokio::test]
async fn bootstrapped_runtime_feeds_commands_and_tray_from_shared_state() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let key_summary = default_key_summary_from_state(runtime.app_state()).expect("key summary");
    let policy_summaries = policy_summaries_from_state(runtime.app_state());
    let log_summary = log_summary_from_runtime(&runtime);

    assert_eq!(key_summary.name, "default");
    assert_eq!(key_summary.allowed_mode, HYBRID);
    assert_eq!(policy_summaries.len(), 1);
    assert_eq!(policy_summaries[0].name, "default");
    assert_eq!(policy_summaries[0].status, "active");
    assert_eq!(runtime.tray_model().current_mode(), Some(RoutingMode::Hybrid));
    assert!(runtime.loopback_gateway().is_ready());
    assert_eq!(log_summary.level, "info");
    assert!(log_summary.last_event.contains("default"));
    assert!(log_summary.last_event.contains(HYBRID));
}
