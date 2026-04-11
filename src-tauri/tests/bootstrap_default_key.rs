use std::time::{SystemTime, UNIX_EPOCH};

use codexlag_lib::{
    bootstrap::{bootstrap_state_for_test, bootstrap_state_for_test_at},
    routing::policy::RoutingMode,
    secret_store::SecretKey,
};
use codexlag_lib::routing::policy::HYBRID;
use rusqlite::Connection;

const DEFAULT_PLATFORM_KEY_SECRET_PREFIX: &str = "ck_local_";
const LEGACY_DEFAULT_PLATFORM_KEY_SECRET_SEED: &str = "ck_local_default_seed";

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
    assert_ne!(secret, LEGACY_DEFAULT_PLATFORM_KEY_SECRET_SEED);
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

    let mut first_state = bootstrap_state_for_test_at(&database_path)
        .await
        .expect("first bootstrap");
    let first_key = first_state
        .get_platform_key_by_name("default")
        .expect("first default key");

    assert_eq!(first_state.iter_policies().count(), 1);
    assert_eq!(first_state.iter_platform_keys().count(), 1);
    assert_eq!(first_key.allowed_mode.as_str(), HYBRID);

    first_state
        .set_default_key_allowed_mode(RoutingMode::RelayOnly)
        .expect("persist default key mode change");

    drop(first_state);

    assert!(database_path.exists(), "bootstrap should create sqlite file");

    let connection = Connection::open(&database_path).expect("open sqlite database");
    let policy_rows: i64 = connection
        .query_row("SELECT COUNT(*) FROM routing_policies", [], |row| row.get(0))
        .expect("count routing policies");
    let key_rows: i64 = connection
        .query_row("SELECT COUNT(*) FROM platform_keys", [], |row| row.get(0))
        .expect("count platform keys");
    let persisted_mode: String = connection
        .query_row(
            "SELECT allowed_mode FROM platform_keys WHERE name = 'default'",
            [],
            |row| row.get(0),
        )
        .expect("select default key mode");

    assert_eq!(policy_rows, 1);
    assert_eq!(key_rows, 1);
    assert_eq!(persisted_mode, RoutingMode::RelayOnly.as_str());

    let second_state = bootstrap_state_for_test_at(&database_path)
        .await
        .expect("second bootstrap");
    let second_key = second_state
        .get_platform_key_by_name("default")
        .expect("second default key");

    assert_eq!(second_key.allowed_mode.as_str(), RoutingMode::RelayOnly.as_str());
    assert_eq!(second_state.iter_policies().count(), 1);
    assert_eq!(second_state.iter_platform_keys().count(), 1);

    drop(second_state);
    let _ = std::fs::remove_file(&database_path);
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
