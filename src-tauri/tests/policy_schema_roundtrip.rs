use std::path::PathBuf;

use codexlag_lib::db::repositories::Repositories;
use codexlag_lib::models::{ExpandedRoutingPolicy, FailureRules, RecoveryRules};
use rand::{rngs::OsRng, RngCore};
use rusqlite::Connection;

#[test]
fn expanded_policy_roundtrips_through_sqlite_repository() {
    let database_path = temp_database_path("codexlag-policy-roundtrip");

    let mut repositories = Repositories::open(&database_path).expect("open repositories");
    let policy = ExpandedRoutingPolicy {
        id: "policy-v1".into(),
        name: "spec-default".into(),
        selection_order: vec![
            "official_pool".into(),
            "relay_pool".into(),
            "fallback_pool".into(),
        ],
        cross_pool_fallback: true,
        retry_budget: 2,
        failure_rules: FailureRules {
            cooldown_ms: 45_000,
            timeout_open_after: 2,
            server_error_open_after: 4,
        },
        recovery_rules: RecoveryRules {
            half_open_after_ms: 20_000,
            success_close_after: 2,
        },
    };

    repositories
        .save_policy(policy.clone())
        .expect("save expanded policy");
    drop(repositories);

    {
        let sqlite = Connection::open(&database_path).expect("open sqlite file");
        let mut statement = sqlite
            .prepare("PRAGMA table_info(routing_policies)")
            .expect("prepare table info query");
        let columns: Vec<String> = statement
            .query_map([], |row| row.get(1))
            .expect("read table info rows")
            .map(|row| row.expect("decode column name"))
            .collect();

        assert!(columns.iter().any(|column| column == "selection_order"));
        assert!(columns.iter().any(|column| column == "cross_pool_fallback"));
        assert!(columns.iter().any(|column| column == "retry_budget"));
        assert!(columns.iter().any(|column| column == "failure_rules"));
        assert!(columns.iter().any(|column| column == "recovery_rules"));
    }

    let repositories = Repositories::open(&database_path).expect("re-open repositories");
    let persisted = repositories
        .expanded_policy("spec-default")
        .cloned()
        .expect("persisted policy");
    assert_eq!(persisted, policy);

    let _ = std::fs::remove_file(&database_path);
}

fn temp_database_path(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{prefix}-{}.sqlite3", random_suffix()))
}

fn random_suffix() -> String {
    let mut bytes = [0_u8; 16];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
