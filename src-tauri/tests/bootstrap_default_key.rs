use codexlag_lib::{bootstrap::bootstrap_state_for_test, secret_store::SecretKey};
use codexlag_lib::routing::policy::HYBRID;

const DEFAULT_PLATFORM_KEY_SECRET_PREFIX: &str = "ck_local_";

#[tokio::test]
async fn bootstrap_creates_default_policy_and_default_key() {
    let state = bootstrap_state_for_test().await.expect("bootstrap");

    let policy = state.get_policy_by_name("default").expect("default policy");
    let key = state
        .get_platform_key_by_name("default")
        .expect("default key");

    assert_eq!(policy.name, "default");
    assert_eq!(key.name, "default");
    assert_eq!(key.allowed_mode.as_str(), HYBRID);
}

#[tokio::test]
async fn bootstrap_persists_default_key_secret_in_secret_store() {
    let state = bootstrap_state_for_test().await.expect("bootstrap");
    let key = state
        .get_platform_key_by_name("default")
        .expect("default key");

    let secret = state
        .secret(&SecretKey::platform_key(&key.id))
        .expect("default key secret");

    assert!(secret.starts_with(DEFAULT_PLATFORM_KEY_SECRET_PREFIX));
    assert!(secret.len() > DEFAULT_PLATFORM_KEY_SECRET_PREFIX.len());
}

#[tokio::test]
async fn tray_model_contains_default_key_mode_actions() {
    use codexlag_lib::{
        routing::policy::RoutingMode,
        tray::{build_tray_model_for_state, TrayItemId, TrayItemKind},
    };

    let state = bootstrap_state_for_test().await.expect("bootstrap");
    let model = build_tray_model_for_state(&state);

    assert!(model.items.iter().any(|item| {
        item.kind == TrayItemKind::Mode && item.id == TrayItemId::Mode(RoutingMode::AccountOnly)
    }));
    assert!(model.items.iter().any(|item| {
        item.kind == TrayItemKind::Mode && item.id == TrayItemId::Mode(RoutingMode::RelayOnly)
    }));
    assert!(model.items.iter().any(|item| {
        item.kind == TrayItemKind::Mode && item.id == TrayItemId::Mode(RoutingMode::Hybrid)
    }));
    assert_eq!(model.current_mode(), Some(RoutingMode::Hybrid));
}
