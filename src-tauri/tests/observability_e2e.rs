use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use codexlag_lib::bootstrap::bootstrap_runtime_for_test;
use codexlag_lib::commands::logs::{
    usage_ledger_from_runtime, usage_request_detail_from_runtime,
    usage_request_history_from_runtime,
};
use codexlag_lib::db::repositories::Repositories;
use codexlag_lib::logging::usage::{UsageLedgerQuery, UsageProvenance};
use codexlag_lib::models::{PricingProfile, RequestAttemptLog, RequestLog};
use codexlag_lib::providers::invocation::InvocationFailureClass;
use codexlag_lib::routing::policy::RoutingMode;
use codexlag_lib::secret_store::SecretKey;
use rusqlite::Connection;
use serde_json::Value;
use tower::ServiceExt;

fn append_persisted_request_bundle(
    runtime: &codexlag_lib::state::RuntimeState,
    request: RequestLog,
    attempts: Vec<RequestAttemptLog>,
) {
    runtime
        .app_state()
        .repositories()
        .append_request_with_attempts(&request, &attempts)
        .expect("append persisted request lifecycle bundle");
}

#[tokio::test]
async fn codex_request_records_usage_detail_with_dimensions_model_and_pricing_metadata() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    runtime
        .set_current_mode(RoutingMode::RelayOnly)
        .expect("switch mode");

    let database_path = runtime
        .runtime_log()
        .log_dir
        .parent()
        .expect("runtime log parent")
        .join("codexlag.sqlite3");
    let repositories = Repositories::open(&database_path).expect("open repositories");
    repositories
        .upsert_pricing_profile(&PricingProfile {
            id: "price-relay-gpt-4o-mini".to_string(),
            model: "gpt-4o-mini".to_string(),
            input_price_per_1k_micros: 900,
            output_price_per_1k_micros: 2_500,
            cache_read_price_per_1k_micros: 250,
            currency: "usd".to_string(),
            effective_from_ms: 0,
            effective_to_ms: None,
            active: true,
        })
        .expect("seed pricing profile");

    let secret = runtime
        .app_state()
        .secret(&SecretKey::default_platform_key())
        .expect("platform key secret");
    let response = runtime
        .loopback_gateway()
        .router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/codex/request")
                .header("authorization", format!("bearer {secret}"))
                .body(Body::empty())
                .expect("gateway request"),
        )
        .await
        .expect("gateway response");

    assert_eq!(response.status(), StatusCode::OK);

    let history = usage_request_history_from_runtime(&runtime, Some(1));
    assert_eq!(
        history.len(),
        1,
        "gateway request should create one usage record"
    );

    let detail = usage_request_detail_from_runtime(&runtime, history[0].request_id.as_str())
        .expect("usage request detail");
    assert_eq!(detail.endpoint_id, "relay-newapi");
    assert_eq!(detail.model.as_deref(), Some("gpt-4o-mini"));
    assert_eq!(detail.input_tokens, 640);
    assert_eq!(detail.output_tokens, 128);
    assert_eq!(detail.cache_read_tokens, 256);
    assert_eq!(detail.cache_write_tokens, 0);
    assert_eq!(detail.reasoning_tokens, 32);
    assert_eq!(detail.total_tokens, 1_056);
    assert_eq!(detail.pricing_profile_id, None);
    assert_eq!(detail.cost.provenance, UsageProvenance::Estimated);
    assert_eq!(detail.cost.amount.as_deref(), Some("0.0010"));
    assert!(detail.cost.is_estimated);
    assert_eq!(detail.final_upstream_status, Some(200));

    let effective_result = serde_json::from_str::<Value>(
        detail
            .effective_capability_result
            .as_deref()
            .expect("effective capability result"),
    )
    .expect("effective capability result json");
    assert_eq!(effective_result["outcome"], "success");
    assert_eq!(effective_result["selected_endpoint_id"], "relay-newapi");
    assert_eq!(effective_result["pricing_estimation"], "estimated");
}

#[tokio::test]
async fn usage_request_detail_exposes_dimensions_capabilities_and_final_upstream_context() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    append_persisted_request_bundle(
        &runtime,
        RequestLog {
            request_id: "req-actual".to_string(),
            platform_key_id: "key-default".to_string(),
            request_type: "codex".to_string(),
            model: "gpt-5".to_string(),
            selected_endpoint_id: Some("official-primary".to_string()),
            attempt_count: 1,
            final_status: "success".to_string(),
            http_status: Some(200),
            started_at_ms: 1_000,
            finished_at_ms: Some(1_050),
            latency_ms: Some(50),
            error_code: None,
            error_reason: None,
            requested_context_window: None,
            requested_context_compression: None,
            effective_context_window: None,
            effective_context_compression: None,
        },
        vec![RequestAttemptLog {
            attempt_id: "req-actual:0".to_string(),
            request_id: "req-actual".to_string(),
            attempt_index: 0,
            endpoint_id: "official-primary".to_string(),
            pool_type: "official".to_string(),
            trigger_reason: "primary".to_string(),
            upstream_status: Some(200),
            timeout_ms: None,
            latency_ms: Some(50),
            token_usage_snapshot: Some(
                "{\"input_tokens\":1200,\"output_tokens\":300,\"cache_read_tokens\":100,\"cache_write_tokens\":0,\"reasoning_tokens\":50}"
                    .to_string(),
            ),
            estimated_cost_snapshot: Some("{\"amount\":\"0.0100\"}".to_string()),
            balance_snapshot_id: None,
            feature_resolution_snapshot: Some(
                "{\"reasoning_effort\":\"medium\",\"tool_choice\":\"auto\"}".to_string(),
            ),
        }],
    );

    let detail =
        usage_request_detail_from_runtime(&runtime, "req-actual").expect("usage request detail");
    assert_eq!(detail.request_id, "req-actual");
    assert_eq!(detail.model.as_deref(), Some("gpt-5"));
    assert_eq!(detail.input_tokens, 1_200);
    assert_eq!(detail.output_tokens, 300);
    assert_eq!(detail.cache_read_tokens, 100);
    assert_eq!(detail.cache_write_tokens, 0);
    assert_eq!(detail.reasoning_tokens, 50);
    assert_eq!(detail.total_tokens, 1_650);
    assert_eq!(detail.cost.provenance, UsageProvenance::Estimated);
    assert_eq!(detail.cost.amount.as_deref(), Some("0.0100"));
    assert!(detail.cost.is_estimated);
    assert_eq!(detail.declared_capability_requirements, None);
    assert_eq!(
        detail.effective_capability_result.as_deref(),
        Some("{\"reasoning_effort\":\"medium\",\"tool_choice\":\"auto\"}")
    );
    assert_eq!(detail.pricing_profile_id, None);
    assert_eq!(detail.final_upstream_status, Some(200));
    assert_eq!(detail.final_upstream_error_code, None);
    assert_eq!(detail.final_upstream_error_reason, None);
}

#[tokio::test]
async fn usage_ledger_tracks_actual_estimated_and_unknown_provenance() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    append_persisted_request_bundle(
        &runtime,
        RequestLog {
            request_id: "req-estimated".to_string(),
            platform_key_id: "key-default".to_string(),
            request_type: "codex".to_string(),
            model: "gpt-5".to_string(),
            selected_endpoint_id: Some("official-primary".to_string()),
            attempt_count: 1,
            final_status: "success".to_string(),
            http_status: Some(200),
            started_at_ms: 1_000,
            finished_at_ms: Some(1_010),
            latency_ms: Some(10),
            error_code: None,
            error_reason: None,
            requested_context_window: None,
            requested_context_compression: None,
            effective_context_window: None,
            effective_context_compression: None,
        },
        vec![RequestAttemptLog {
            attempt_id: "req-estimated:0".to_string(),
            request_id: "req-estimated".to_string(),
            attempt_index: 0,
            endpoint_id: "official-primary".to_string(),
            pool_type: "official".to_string(),
            trigger_reason: "primary".to_string(),
            upstream_status: Some(200),
            timeout_ms: None,
            latency_ms: Some(10),
            token_usage_snapshot: Some(
                "{\"input_tokens\":500,\"output_tokens\":200,\"cache_read_tokens\":50,\"cache_write_tokens\":0,\"reasoning_tokens\":20}"
                    .to_string(),
            ),
            estimated_cost_snapshot: Some("{\"amount\":\"0.0050\"}".to_string()),
            balance_snapshot_id: None,
            feature_resolution_snapshot: None,
        }],
    );
    append_persisted_request_bundle(
        &runtime,
        RequestLog {
            request_id: "req-unknown".to_string(),
            platform_key_id: "key-default".to_string(),
            request_type: "codex".to_string(),
            model: "gpt-5".to_string(),
            selected_endpoint_id: Some("relay-newapi".to_string()),
            attempt_count: 1,
            final_status: "error".to_string(),
            http_status: Some(502),
            started_at_ms: 2_000,
            finished_at_ms: Some(2_030),
            latency_ms: Some(30),
            error_code: Some("upstream.provider_timeout".to_string()),
            error_reason: Some("timed out".to_string()),
            requested_context_window: None,
            requested_context_compression: None,
            effective_context_window: None,
            effective_context_compression: None,
        },
        vec![RequestAttemptLog {
            attempt_id: "req-unknown:0".to_string(),
            request_id: "req-unknown".to_string(),
            attempt_index: 0,
            endpoint_id: "relay-newapi".to_string(),
            pool_type: "relay".to_string(),
            trigger_reason: "primary".to_string(),
            upstream_status: None,
            timeout_ms: None,
            latency_ms: Some(30),
            token_usage_snapshot: Some(
                "{\"input_tokens\":10,\"output_tokens\":10,\"cache_read_tokens\":0,\"cache_write_tokens\":0,\"reasoning_tokens\":0}"
                    .to_string(),
            ),
            estimated_cost_snapshot: None,
            balance_snapshot_id: None,
            feature_resolution_snapshot: None,
        }],
    );

    let history = usage_request_history_from_runtime(&runtime, None);
    assert_eq!(history.len(), 2);

    let estimated_only = usage_ledger_from_runtime(
        &runtime,
        Some(UsageLedgerQuery {
            endpoint_id: Some("official-primary".to_string()),
            request_id_prefix: None,
            limit: None,
        }),
    );
    assert_eq!(estimated_only.entries.len(), 1);
    assert_eq!(
        estimated_only.total_cost.provenance,
        UsageProvenance::Estimated
    );
    assert_eq!(estimated_only.total_cost.amount.as_deref(), Some("0.0050"));
    assert!(estimated_only.total_cost.is_estimated);

    let all_entries = usage_ledger_from_runtime(&runtime, None);
    assert_eq!(all_entries.entries.len(), 2);
    assert_eq!(all_entries.total_cost.provenance, UsageProvenance::Unknown);
    assert_eq!(all_entries.total_cost.amount, None);
}

#[tokio::test]
async fn codex_request_preserves_request_and_attempt_id_lineage_across_observability_surfaces() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    let gateway_state = runtime.loopback_gateway().state();
    gateway_state
        .plan_provider_failure_for_test("official-primary", InvocationFailureClass::Http5xx);

    let secret = runtime
        .app_state()
        .secret(&SecretKey::default_platform_key())
        .expect("platform key secret");
    let response = runtime
        .loopback_gateway()
        .router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/codex/request")
                .header("authorization", format!("bearer {secret}"))
                .body(Body::empty())
                .expect("gateway request"),
        )
        .await
        .expect("gateway response");
    assert_eq!(response.status(), StatusCode::OK);

    let attempts = gateway_state.invocation_attempts_for_test();
    assert_eq!(attempts.len(), 2);
    let request_id = attempts[0].request_id.clone();
    assert_eq!(attempts[1].request_id, request_id);
    assert_eq!(
        attempts[0].attempt_id,
        format!("{}:0", attempts[0].request_id)
    );
    assert_eq!(
        attempts[1].attempt_id,
        format!("{}:1", attempts[1].request_id)
    );
    assert_eq!(attempts[0].endpoint_id, "official-primary");
    assert_eq!(attempts[1].endpoint_id, "relay-newapi");

    let history = usage_request_history_from_runtime(&runtime, Some(1));
    assert_eq!(history.len(), 1, "expected one persisted request record");
    assert_eq!(history[0].request_id, request_id);

    let detail = usage_request_detail_from_runtime(&runtime, request_id.as_str())
        .expect("usage request detail");
    assert_eq!(detail.request_id, request_id);
    assert_eq!(detail.endpoint_id, "relay-newapi");
    assert_eq!(detail.final_upstream_status, Some(200));

    let database_path = runtime
        .runtime_log()
        .log_dir
        .parent()
        .expect("runtime log parent")
        .join("codexlag.sqlite3");
    let sqlite = Connection::open(database_path).expect("open sqlite");
    let attempt_rows: Vec<(String, i64, String)> = {
        let mut statement = sqlite
            .prepare(
                "
                SELECT attempt_id, attempt_index, endpoint_id
                FROM request_attempt_logs
                WHERE request_id = ?1
                ORDER BY attempt_index ASC
                ",
            )
            .expect("prepare attempt query");
        statement
            .query_map([request_id.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .expect("query attempt rows")
            .map(|row| row.expect("decode attempt row"))
            .collect()
    };
    assert_eq!(attempt_rows.len(), 2);
    assert_eq!(attempt_rows[0], (format!("{request_id}:0"), 0, "official-primary".to_string()));
    assert_eq!(attempt_rows[1], (format!("{request_id}:1"), 1, "relay-newapi".to_string()));
}
