use std::time::{SystemTime, UNIX_EPOCH};

use codexlag_lib::{
    bootstrap::{bootstrap_state_for_test, bootstrap_state_for_test_at},
    secret_store::SecretKey,
};
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
async fn bootstrap_persists_default_state_across_restarts() {
    let database_path = std::env::temp_dir().join(format!(
        "codexlag-bootstrap-persistence-{}.sqlite3",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos()
    ));

    let first_state = bootstrap_state_for_test_at(&database_path)
        .await
        .expect("first bootstrap");
    let first_policy = first_state
        .get_policy_by_name("default")
        .expect("first default policy");
    let first_key = first_state
        .get_platform_key_by_name("default")
        .expect("first default key");
    let first_policy_id = first_policy.id.clone();
    let first_key_id = first_key.id.clone();

    assert_eq!(first_state.iter_policies().count(), 1);
    assert_eq!(first_state.iter_platform_keys().count(), 1);

    drop(first_state);

    let second_state = bootstrap_state_for_test_at(&database_path)
        .await
        .expect("second bootstrap");
    let second_policy = second_state
        .get_policy_by_name("default")
        .expect("second default policy");
    let second_key = second_state
        .get_platform_key_by_name("default")
        .expect("second default key");

    assert_eq!(second_policy.id, first_policy_id);
    assert_eq!(second_key.id, first_key_id);
    assert_eq!(second_state.iter_policies().count(), 1);
    assert_eq!(second_state.iter_platform_keys().count(), 1);
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
    assert!(
        model
            .items
            .iter()
            .any(|item| item.id.menu_id().as_ref() == "mode:account_only")
    );
    assert!(
        model
            .items
            .iter()
            .any(|item| item.id.menu_id().as_ref() == "mode:relay_only")
    );
    assert!(
        model
            .items
            .iter()
            .any(|item| item.id.menu_id().as_ref() == "mode:hybrid")
    );
    assert_eq!(model.current_mode(), Some(RoutingMode::Hybrid));
}
