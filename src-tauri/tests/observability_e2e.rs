use codexlag_lib::bootstrap::bootstrap_runtime_for_test;
use codexlag_lib::commands::logs::{
    usage_ledger_from_runtime, usage_request_detail_from_runtime,
    usage_request_history_from_runtime,
};
use codexlag_lib::logging::usage::{UsageLedgerQuery, UsageProvenance, UsageRecordInput};

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
