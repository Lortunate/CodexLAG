use codexlag_lib::logging::usage::{
    append_usage_record, query_usage_ledger, record_request, request_detail, request_history,
    UsageLedgerQuery, UsageProvenance, UsageRecordInput, USAGE_RECORD_RETENTION_CAP,
};
use serde_json::{from_str, to_string};

#[test]
fn record_request_captures_input_output_cache_and_estimated_cost() {
    let record = record_request(UsageRecordInput {
        request_id: "req-1".into(),
        endpoint_id: "official-1".into(),
        model: None,
        input_tokens: 120,
        output_tokens: 30,
        cache_read_tokens: 10,
        cache_write_tokens: 0,
        reasoning_tokens: 0,
        estimated_cost: "0.0123".into(),
        cost_provenance: UsageProvenance::Unknown,
        cost_is_estimated: false,
        pricing_profile_id: None,
        declared_capability_requirements: None,
        effective_capability_result: None,
        final_upstream_status: None,
        final_upstream_error_code: None,
        final_upstream_error_reason: None,
    });

    assert_eq!(record.input_tokens, 120);
    assert_eq!(record.output_tokens, 30);
    assert_eq!(record.cache_read_tokens, 10);
    assert_eq!(record.cache_write_tokens, 0);
    assert_eq!(record.total_tokens, 160);
    assert_eq!(record.estimated_cost, "0.0123");
}

#[test]
fn usage_records_support_serde_round_trip() {
    let input = UsageRecordInput {
        request_id: "req-2".into(),
        endpoint_id: "relay-1".into(),
        model: None,
        input_tokens: 40,
        output_tokens: 15,
        cache_read_tokens: 5,
        cache_write_tokens: 2,
        reasoning_tokens: 0,
        estimated_cost: "0.0042".into(),
        cost_provenance: UsageProvenance::Unknown,
        cost_is_estimated: false,
        pricing_profile_id: None,
        declared_capability_requirements: None,
        effective_capability_result: None,
        final_upstream_status: None,
        final_upstream_error_code: None,
        final_upstream_error_reason: None,
    };

    let input_json = to_string(&input).expect("serialize usage input");
    let decoded_input: UsageRecordInput = from_str(&input_json).expect("deserialize usage input");
    assert_eq!(decoded_input.request_id, "req-2");
    assert_eq!(decoded_input.cache_write_tokens, 2);

    let record = record_request(input);
    let record_json = to_string(&record).expect("serialize usage record");
    let decoded_record = from_str::<codexlag_lib::logging::usage::UsageRecord>(&record_json)
        .expect("deserialize usage record");

    assert_eq!(decoded_record.request_id, "req-2");
    assert_eq!(decoded_record.endpoint_id, "relay-1");
    assert_eq!(decoded_record.input_tokens, 40);
    assert_eq!(decoded_record.output_tokens, 15);
    assert_eq!(decoded_record.cache_read_tokens, 5);
    assert_eq!(decoded_record.cache_write_tokens, 2);
    assert_eq!(decoded_record.total_tokens, 62);
    assert_eq!(decoded_record.estimated_cost, "0.0042");
}

#[test]
fn usage_record_deserialization_rejects_invalid_total_tokens() {
    let error = from_str::<codexlag_lib::logging::usage::UsageRecord>(
        r#"{
            "request_id":"req-3",
            "endpoint_id":"relay-2",
            "input_tokens":10,
            "output_tokens":20,
            "cache_read_tokens":3,
            "cache_write_tokens":4,
            "total_tokens":999,
            "estimated_cost":"0.0010"
        }"#,
    )
    .expect_err("mismatched total_tokens should fail");

    assert!(
        error
            .to_string()
            .contains("total_tokens must equal the sum of component token fields"),
        "unexpected error: {error}"
    );
}

#[test]
fn usage_query_helpers_expose_estimated_and_unknown_provenance() {
    let records = vec![
        record_request(UsageRecordInput {
            request_id: "req-1".into(),
            endpoint_id: "official-1".into(),
            model: None,
            input_tokens: 120,
            output_tokens: 30,
            cache_read_tokens: 10,
            cache_write_tokens: 0,
            reasoning_tokens: 0,
            estimated_cost: "0.0123".into(),
            cost_provenance: UsageProvenance::Unknown,
            cost_is_estimated: false,
            pricing_profile_id: None,
            declared_capability_requirements: None,
            effective_capability_result: None,
            final_upstream_status: None,
            final_upstream_error_code: None,
            final_upstream_error_reason: None,
        }),
        record_request(UsageRecordInput {
            request_id: "req-2".into(),
            endpoint_id: "relay-1".into(),
            model: None,
            input_tokens: 40,
            output_tokens: 15,
            cache_read_tokens: 5,
            cache_write_tokens: 2,
            reasoning_tokens: 0,
            estimated_cost: "".into(),
            cost_provenance: UsageProvenance::Unknown,
            cost_is_estimated: false,
            pricing_profile_id: None,
            declared_capability_requirements: None,
            effective_capability_result: None,
            final_upstream_status: None,
            final_upstream_error_code: None,
            final_upstream_error_reason: None,
        }),
    ];

    let detail = request_detail(&records, "req-1").expect("request detail");
    assert_eq!(detail.cost.provenance, UsageProvenance::Estimated);
    assert_eq!(detail.cost.amount.as_deref(), Some("0.0123"));
    assert!(request_detail(&records, "missing").is_none());

    let history = request_history(&records, Some(1));
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].request_id, "req-2");

    let filtered = query_usage_ledger(
        &records,
        UsageLedgerQuery {
            endpoint_id: Some("relay-1".into()),
            request_id_prefix: Some("req-".into()),
            limit: Some(5),
        },
    );
    assert_eq!(filtered.entries.len(), 1);
    assert_eq!(filtered.total_tokens, 62);
    assert_eq!(filtered.total_cost.provenance, UsageProvenance::Unknown);
    assert_eq!(filtered.total_cost.amount, None);
}

#[test]
fn append_usage_record_caps_history_and_drops_oldest_entries() {
    let overflow = 3;
    let total = USAGE_RECORD_RETENTION_CAP + overflow;
    let mut records = Vec::new();

    for idx in 0..total {
        append_usage_record(
            &mut records,
            UsageRecordInput {
                request_id: format!("req-{idx}"),
                endpoint_id: "official-default".into(),
                model: None,
                input_tokens: 1,
                output_tokens: 0,
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
                final_upstream_error_code: None,
                final_upstream_error_reason: None,
            },
        );
    }

    assert_eq!(records.len(), USAGE_RECORD_RETENTION_CAP);
    assert_eq!(
        records.first().map(|record| record.request_id.as_str()),
        Some("req-3")
    );
    let expected_last = format!("req-{}", total - 1);
    assert_eq!(
        records.last().map(|record| record.request_id.as_str()),
        Some(expected_last.as_str())
    );
}
