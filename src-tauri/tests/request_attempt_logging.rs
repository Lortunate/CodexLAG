use std::path::PathBuf;

use codexlag_lib::db::repositories::Repositories;
use codexlag_lib::models::{PricingProfile, RequestAttemptLog, RequestLog};
use rand::{rngs::OsRng, RngCore};
use rusqlite::{params, Connection};

#[test]
fn appending_request_and_attempt_rows_is_transactional() {
    let database_path = temp_database_path("codexlag-request-attempts");
    let repositories = Repositories::open(&database_path).expect("open repositories");

    let request = RequestLog {
        request_id: "req-001".into(),
        platform_key_id: "key-default".into(),
        request_type: "chat.completions".into(),
        model: "gpt-5".into(),
        selected_endpoint_id: Some("endpoint-official-1".into()),
        attempt_count: 2,
        final_status: "success".into(),
        http_status: Some(200),
        started_at_ms: 1_710_000_000_000,
        finished_at_ms: Some(1_710_000_000_450),
        latency_ms: Some(450),
        error_code: None,
        error_reason: None,
        requested_context_window: Some(200_000),
        requested_context_compression: Some("auto".into()),
        effective_context_window: Some(200_000),
        effective_context_compression: Some("auto".into()),
    };
    let attempts = vec![
        RequestAttemptLog {
            attempt_id: "attempt-001".into(),
            request_id: "req-001".into(),
            attempt_index: 0,
            endpoint_id: "endpoint-official-1".into(),
            pool_type: "official_pool".into(),
            trigger_reason: "initial".into(),
            upstream_status: Some(429),
            timeout_ms: Some(30_000),
            latency_ms: Some(110),
            token_usage_snapshot: Some("{\"input\":20,\"output\":0}".into()),
            estimated_cost_snapshot: Some("0.0002".into()),
            balance_snapshot_id: None,
            feature_resolution_snapshot: Some("{\"compression\":\"none\"}".into()),
        },
        RequestAttemptLog {
            attempt_id: "attempt-002".into(),
            request_id: "req-001".into(),
            attempt_index: 1,
            endpoint_id: "endpoint-relay-1".into(),
            pool_type: "relay_pool".into(),
            trigger_reason: "fallback_after_429".into(),
            upstream_status: Some(200),
            timeout_ms: Some(30_000),
            latency_ms: Some(340),
            token_usage_snapshot: Some("{\"input\":20,\"output\":40}".into()),
            estimated_cost_snapshot: Some("0.0011".into()),
            balance_snapshot_id: Some("balance-1".into()),
            feature_resolution_snapshot: Some("{\"compression\":\"auto\"}".into()),
        },
    ];

    repositories
        .append_request_with_attempts(&request, &attempts)
        .expect("append request + attempts transactionally");
    drop(repositories);

    {
        let sqlite = Connection::open(&database_path).expect("open sqlite file");
        let request_rows: i64 = sqlite
            .query_row("SELECT COUNT(*) FROM request_logs", [], |row| row.get(0))
            .expect("count request rows");
        let attempt_rows: i64 = sqlite
            .query_row("SELECT COUNT(*) FROM request_attempt_logs", [], |row| {
                row.get(0)
            })
            .expect("count attempt rows");
        assert_eq!(request_rows, 1);
        assert_eq!(attempt_rows, 2);

        let mut statement = sqlite
            .prepare("PRAGMA table_info(request_attempt_logs)")
            .expect("prepare table info query");
        let columns: Vec<String> = statement
            .query_map([], |row| row.get(1))
            .expect("read table info rows")
            .map(|row| row.expect("decode column name"))
            .collect();

        assert!(columns.iter().any(|column| column == "attempt_index"));
        assert!(columns.iter().any(|column| column == "trigger_reason"));
        assert!(columns.iter().any(|column| column == "upstream_status"));
        assert!(columns
            .iter()
            .any(|column| column == "feature_resolution_snapshot"));
    }

    let _ = std::fs::remove_file(&database_path);
}

#[test]
fn active_pricing_profile_lookup_is_model_and_time_scoped() {
    let database_path = temp_database_path("codexlag-pricing-lookup");
    let repositories = Repositories::open(&database_path).expect("open repositories");

    repositories
        .upsert_pricing_profile(&PricingProfile {
            id: "price-old".into(),
            model: "gpt-5".into(),
            input_price_per_1k_micros: 900,
            output_price_per_1k_micros: 2_500,
            cache_read_price_per_1k_micros: 100,
            currency: "USD".into(),
            effective_from_ms: 1_700_000_000_000,
            effective_to_ms: Some(1_710_000_000_000),
            active: true,
        })
        .expect("insert old pricing profile");

    repositories
        .upsert_pricing_profile(&PricingProfile {
            id: "price-active".into(),
            model: "gpt-5".into(),
            input_price_per_1k_micros: 1_000,
            output_price_per_1k_micros: 3_000,
            cache_read_price_per_1k_micros: 120,
            currency: "USD".into(),
            effective_from_ms: 1_710_000_000_000,
            effective_to_ms: None,
            active: true,
        })
        .expect("insert active pricing profile");

    let active_profile = repositories
        .active_pricing_profile_by_model("gpt-5", 1_715_000_000_000)
        .expect("query active profile")
        .expect("active profile exists");

    assert_eq!(active_profile.id, "price-active");
    assert_eq!(active_profile.model, "gpt-5");
    assert_eq!(active_profile.currency, "USD");

    let sqlite = Connection::open(&database_path).expect("open sqlite file");
    let pricing_rows: i64 = sqlite
        .query_row("SELECT COUNT(*) FROM pricing_profiles", [], |row| {
            row.get(0)
        })
        .expect("count pricing rows");
    let active_rows: i64 = sqlite
        .query_row(
            "SELECT COUNT(*) FROM pricing_profiles WHERE model = ?1 AND active = 1",
            params!["gpt-5"],
            |row| row.get(0),
        )
        .expect("count active pricing rows");
    assert_eq!(pricing_rows, 2);
    assert_eq!(active_rows, 2);

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
