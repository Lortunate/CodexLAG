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
use codexlag_lib::logging::usage::{UsageLedgerQuery, UsageProvenance, UsageRecordInput};
use codexlag_lib::models::PricingProfile;
use codexlag_lib::providers::invocation::InvocationFailureClass;
use codexlag_lib::routing::policy::RoutingMode;
use codexlag_lib::secret_store::SecretKey;
use serde_json::Value;
use tower::ServiceExt;

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
    assert_eq!(detail.endpoint_id, "relay-default");
    assert_eq!(detail.model.as_deref(), Some("gpt-4o-mini"));
    assert_eq!(detail.input_tokens, 640);
    assert_eq!(detail.output_tokens, 128);
    assert_eq!(detail.cache_read_tokens, 256);
    assert_eq!(detail.cache_write_tokens, 0);
    assert_eq!(detail.reasoning_tokens, 32);
    assert_eq!(detail.total_tokens, 1_056);
    assert_eq!(
        detail.pricing_profile_id.as_deref(),
        Some("price-relay-gpt-4o-mini")
    );
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
    assert_eq!(effective_result["selected_endpoint_id"], "relay-default");
    assert_eq!(effective_result["pricing_estimation"], "estimated");
}

#[tokio::test]
async fn usage_request_detail_exposes_dimensions_capabilities_and_final_upstream_context() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    runtime.record_usage_request(UsageRecordInput {
        request_id: "req-actual".to_string(),
        endpoint_id: "official-default".to_string(),
        model: Some("gpt-5".to_string()),
        input_tokens: 1_200,
        output_tokens: 300,
        cache_read_tokens: 100,
        cache_write_tokens: 0,
        reasoning_tokens: 50,
        estimated_cost: "0.0100".to_string(),
        cost_provenance: UsageProvenance::Actual,
        cost_is_estimated: false,
        pricing_profile_id: Some("pricing-gpt5-202604".to_string()),
        declared_capability_requirements: Some(
            "{\"reasoning_effort\":\"high\",\"tool_choice\":\"required\"}".to_string(),
        ),
        effective_capability_result: Some(
            "{\"reasoning_effort\":\"medium\",\"tool_choice\":\"auto\"}".to_string(),
        ),
        final_upstream_status: Some(200),
        final_upstream_error_code: None,
        final_upstream_error_reason: None,
    });

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
    assert_eq!(detail.cost.provenance, UsageProvenance::Actual);
    assert_eq!(detail.cost.amount.as_deref(), Some("0.0100"));
    assert!(!detail.cost.is_estimated);
    assert_eq!(
        detail.declared_capability_requirements.as_deref(),
        Some("{\"reasoning_effort\":\"high\",\"tool_choice\":\"required\"}")
    );
    assert_eq!(
        detail.effective_capability_result.as_deref(),
        Some("{\"reasoning_effort\":\"medium\",\"tool_choice\":\"auto\"}")
    );
    assert_eq!(detail.final_upstream_status, Some(200));
    assert_eq!(detail.final_upstream_error_code, None);
    assert_eq!(detail.final_upstream_error_reason, None);
}

#[tokio::test]
async fn usage_ledger_tracks_actual_estimated_and_unknown_provenance() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    runtime.record_usage_request(UsageRecordInput {
        request_id: "req-actual".to_string(),
        endpoint_id: "official-default".to_string(),
        model: Some("gpt-5".to_string()),
        input_tokens: 500,
        output_tokens: 200,
        cache_read_tokens: 50,
        cache_write_tokens: 0,
        reasoning_tokens: 20,
        estimated_cost: "0.0050".to_string(),
        cost_provenance: UsageProvenance::Actual,
        cost_is_estimated: false,
        pricing_profile_id: Some("price-actual".to_string()),
        declared_capability_requirements: None,
        effective_capability_result: None,
        final_upstream_status: Some(200),
        final_upstream_error_code: None,
        final_upstream_error_reason: None,
    });
    runtime.record_usage_request(UsageRecordInput {
        request_id: "req-estimated".to_string(),
        endpoint_id: "relay-default".to_string(),
        model: Some("gpt-5".to_string()),
        input_tokens: 400,
        output_tokens: 150,
        cache_read_tokens: 25,
        cache_write_tokens: 0,
        reasoning_tokens: 25,
        estimated_cost: "0.0035".to_string(),
        cost_provenance: UsageProvenance::Estimated,
        cost_is_estimated: true,
        pricing_profile_id: Some("price-estimated".to_string()),
        declared_capability_requirements: None,
        effective_capability_result: None,
        final_upstream_status: Some(502),
        final_upstream_error_code: Some("upstream.provider_http_failure".to_string()),
        final_upstream_error_reason: Some("relay upstream 502".to_string()),
    });
    runtime.record_usage_request(UsageRecordInput {
        request_id: "req-unknown".to_string(),
        endpoint_id: "relay-default".to_string(),
        model: Some("gpt-5".to_string()),
        input_tokens: 10,
        output_tokens: 10,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        reasoning_tokens: 0,
        estimated_cost: String::new(),
        cost_provenance: UsageProvenance::Unknown,
        cost_is_estimated: false,
        pricing_profile_id: None,
        declared_capability_requirements: None,
        effective_capability_result: None,
        final_upstream_status: None,
        final_upstream_error_code: Some("upstream.provider_timeout".to_string()),
        final_upstream_error_reason: Some("timed out".to_string()),
    });

    let history = usage_request_history_from_runtime(&runtime, None);
    assert_eq!(history.len(), 3);

    let actual_only = usage_ledger_from_runtime(
        &runtime,
        Some(UsageLedgerQuery {
            endpoint_id: Some("official-default".to_string()),
            request_id_prefix: None,
            limit: None,
        }),
    );
    assert_eq!(actual_only.entries.len(), 1);
    assert_eq!(actual_only.total_cost.provenance, UsageProvenance::Actual);
    assert_eq!(actual_only.total_cost.amount.as_deref(), Some("0.0050"));

    let estimated_only = usage_ledger_from_runtime(
        &runtime,
        Some(UsageLedgerQuery {
            endpoint_id: Some("relay-default".to_string()),
            request_id_prefix: Some("req-estimated".to_string()),
            limit: None,
        }),
    );
    assert_eq!(estimated_only.entries.len(), 1);
    assert_eq!(
        estimated_only.total_cost.provenance,
        UsageProvenance::Estimated
    );
    assert_eq!(estimated_only.total_cost.amount.as_deref(), Some("0.0035"));
    assert!(estimated_only.total_cost.is_estimated);

    let all_entries = usage_ledger_from_runtime(&runtime, None);
    assert_eq!(all_entries.entries.len(), 3);
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
        .plan_provider_failure_for_test("official-default", InvocationFailureClass::Http5xx);

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
    assert_eq!(attempts[0].endpoint_id, "official-default");
    assert_eq!(attempts[1].endpoint_id, "relay-default");

    let history = usage_request_history_from_runtime(&runtime, Some(1));
    assert_eq!(history.len(), 1, "expected one persisted request record");
    assert_eq!(history[0].request_id, request_id);

    let detail = usage_request_detail_from_runtime(&runtime, request_id.as_str())
        .expect("usage request detail");
    assert_eq!(detail.request_id, request_id);
    assert_eq!(detail.endpoint_id, "relay-default");
    assert_eq!(detail.final_upstream_status, Some(200));
}
