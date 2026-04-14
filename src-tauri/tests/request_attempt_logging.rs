use std::path::PathBuf;

use codexlag_lib::db::repositories::Repositories;
use codexlag_lib::models::{PricingProfile, RequestAttemptLog, RequestLog, UsageProvenance};
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
fn appending_request_with_attempt_count_mismatch_is_rejected_without_partial_inserts() {
    let database_path = temp_database_path("codexlag-request-attempts-mismatch");
    let repositories = Repositories::open(&database_path).expect("open repositories");

    let request = RequestLog {
        request_id: "req-mismatch-001".into(),
        platform_key_id: "key-default".into(),
        request_type: "chat.completions".into(),
        model: "gpt-5".into(),
        selected_endpoint_id: Some("endpoint-official-1".into()),
        attempt_count: 2,
        final_status: "success".into(),
        http_status: Some(200),
        started_at_ms: 1_710_100_000_000,
        finished_at_ms: Some(1_710_100_000_250),
        latency_ms: Some(250),
        error_code: None,
        error_reason: None,
        requested_context_window: Some(200_000),
        requested_context_compression: Some("auto".into()),
        effective_context_window: Some(200_000),
        effective_context_compression: Some("auto".into()),
    };
    let attempts = vec![RequestAttemptLog {
        attempt_id: "attempt-mismatch-001".into(),
        request_id: "req-mismatch-001".into(),
        attempt_index: 0,
        endpoint_id: "endpoint-official-1".into(),
        pool_type: "official_pool".into(),
        trigger_reason: "initial".into(),
        upstream_status: Some(200),
        timeout_ms: Some(30_000),
        latency_ms: Some(250),
        token_usage_snapshot: Some("{\"input\":12,\"output\":30}".into()),
        estimated_cost_snapshot: Some("0.0009".into()),
        balance_snapshot_id: None,
        feature_resolution_snapshot: Some("{\"compression\":\"auto\"}".into()),
    }];

    let error = repositories
        .append_request_with_attempts(&request, &attempts)
        .expect_err("attempt_count mismatch should be rejected");
    assert!(
        error.to_string().contains("attempt_count"),
        "expected attempt_count validation error, got: {error}"
    );
    drop(repositories);

    let sqlite = Connection::open(&database_path).expect("open sqlite file");
    let request_rows: i64 = sqlite
        .query_row("SELECT COUNT(*) FROM request_logs", [], |row| row.get(0))
        .expect("count request rows");
    let attempt_rows: i64 = sqlite
        .query_row("SELECT COUNT(*) FROM request_attempt_logs", [], |row| {
            row.get(0)
        })
        .expect("count attempt rows");
    assert_eq!(request_rows, 0);
    assert_eq!(attempt_rows, 0);

    let _ = std::fs::remove_file(&database_path);
}

#[test]
fn appending_request_rolls_back_transaction_when_attempt_insert_fails() {
    let database_path = temp_database_path("codexlag-request-attempts-rollback");
    let repositories = Repositories::open(&database_path).expect("open repositories");

    let request = RequestLog {
        request_id: "req-rollback-001".into(),
        platform_key_id: "key-default".into(),
        request_type: "chat.completions".into(),
        model: "gpt-5".into(),
        selected_endpoint_id: Some("endpoint-official-1".into()),
        attempt_count: 2,
        final_status: "success".into(),
        http_status: Some(200),
        started_at_ms: 1_710_200_000_000,
        finished_at_ms: Some(1_710_200_000_250),
        latency_ms: Some(250),
        error_code: None,
        error_reason: None,
        requested_context_window: Some(200_000),
        requested_context_compression: Some("auto".into()),
        effective_context_window: Some(200_000),
        effective_context_compression: Some("auto".into()),
    };
    let attempts = vec![
        RequestAttemptLog {
            attempt_id: "attempt-duplicate".into(),
            request_id: "req-rollback-001".into(),
            attempt_index: 0,
            endpoint_id: "endpoint-official-1".into(),
            pool_type: "official_pool".into(),
            trigger_reason: "initial".into(),
            upstream_status: Some(429),
            timeout_ms: Some(30_000),
            latency_ms: Some(100),
            token_usage_snapshot: Some("{\"input\":10,\"output\":0}".into()),
            estimated_cost_snapshot: Some("0.0001".into()),
            balance_snapshot_id: None,
            feature_resolution_snapshot: Some("{\"compression\":\"none\"}".into()),
        },
        RequestAttemptLog {
            attempt_id: "attempt-duplicate".into(),
            request_id: "req-rollback-001".into(),
            attempt_index: 1,
            endpoint_id: "endpoint-relay-1".into(),
            pool_type: "relay_pool".into(),
            trigger_reason: "fallback_after_429".into(),
            upstream_status: Some(200),
            timeout_ms: Some(30_000),
            latency_ms: Some(150),
            token_usage_snapshot: Some("{\"input\":10,\"output\":20}".into()),
            estimated_cost_snapshot: Some("0.0007".into()),
            balance_snapshot_id: Some("balance-rollback".into()),
            feature_resolution_snapshot: Some("{\"compression\":\"auto\"}".into()),
        },
    ];

    let error = repositories
        .append_request_with_attempts(&request, &attempts)
        .expect_err("duplicate attempt ids should fail insertion");
    assert!(
        error
            .to_string()
            .contains("failed to insert request attempt"),
        "expected attempt insert error, got: {error}"
    );
    drop(repositories);

    let sqlite = Connection::open(&database_path).expect("open sqlite file");
    let request_rows: i64 = sqlite
        .query_row("SELECT COUNT(*) FROM request_logs", [], |row| row.get(0))
        .expect("count request rows");
    let attempt_rows: i64 = sqlite
        .query_row("SELECT COUNT(*) FROM request_attempt_logs", [], |row| {
            row.get(0)
        })
        .expect("count attempt rows");
    assert_eq!(request_rows, 0);
    assert_eq!(attempt_rows, 0);

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

#[test]
fn request_detail_can_be_derived_from_persisted_request_and_attempt_rows() {
    let request = RequestLog {
        request_id: "req-1".into(),
        platform_key_id: "key-default".into(),
        request_type: "codex".into(),
        model: "gpt-4o-mini".into(),
        selected_endpoint_id: Some("relay-newapi".into()),
        attempt_count: 1,
        final_status: "success".into(),
        http_status: Some(200),
        started_at_ms: 1_000,
        finished_at_ms: Some(1_120),
        latency_ms: Some(120),
        error_code: None,
        error_reason: None,
        requested_context_window: None,
        requested_context_compression: None,
        effective_context_window: None,
        effective_context_compression: None,
    };

    let attempts = vec![RequestAttemptLog {
        attempt_id: "req-1:0".into(),
        request_id: "req-1".into(),
        attempt_index: 0,
        endpoint_id: "relay-newapi".into(),
        pool_type: "relay".into(),
        trigger_reason: "primary".into(),
        upstream_status: Some(200),
        timeout_ms: None,
        latency_ms: Some(120),
        token_usage_snapshot: Some(
            "{\"input_tokens\":640,\"output_tokens\":128,\"cache_read_tokens\":256,\"cache_write_tokens\":0,\"reasoning_tokens\":32}"
                .into(),
        ),
        estimated_cost_snapshot: Some("{\"amount\":\"0.0010\"}".into()),
        balance_snapshot_id: None,
        feature_resolution_snapshot: Some("{\"outcome\":\"success\"}".into()),
    }];

    let detail =
        codexlag_lib::logging::usage::usage_request_detail_from_persisted_rows(&request, &attempts);
    assert_eq!(detail.request_id, "req-1");
    assert_eq!(detail.endpoint_id, "relay-newapi");
    assert_eq!(detail.input_tokens, 640);
    assert_eq!(detail.total_tokens, 1_056);
    assert_eq!(detail.cost.provenance, UsageProvenance::Estimated);
    assert_eq!(detail.cost.amount.as_deref(), Some("0.0010"));
    assert_eq!(
        detail.effective_capability_result.as_deref(),
        Some("{\"outcome\":\"success\"}")
    );
    assert_eq!(detail.final_upstream_status, Some(200));
}

#[test]
fn pricing_profile_cost_estimation_is_scoped_by_model_and_time_and_marks_estimated() {
    let database_path = temp_database_path("codexlag-pricing-estimate");
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

    let estimate = repositories
        .estimate_usage_cost_for_model_at("gpt-5", 1_715_000_000_000, 1_000, 1_000, 1_000, 0, 1_000)
        .expect("estimate usage cost for active profile")
        .expect("active profile estimate");

    assert_eq!(estimate.pricing_profile_id, "price-active");
    assert_eq!(estimate.provenance, UsageProvenance::Estimated);
    assert!(estimate.estimated);
    assert_eq!(estimate.amount, "0.0071");

    let before_effective = repositories
        .estimate_usage_cost_for_model_at("gpt-5", 1_600_000_000_000, 1_000, 1_000, 1_000, 0, 1_000)
        .expect("estimate before pricing effective window");
    assert!(before_effective.is_none());

    let other_model = repositories
        .estimate_usage_cost_for_model_at(
            "gpt-4.1-mini",
            1_715_000_000_000,
            1_000,
            1_000,
            1_000,
            0,
            1_000,
        )
        .expect("estimate for different model");
    assert!(other_model.is_none());

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
