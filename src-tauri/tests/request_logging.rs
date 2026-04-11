use codexlag_lib::logging::usage::{record_request, UsageRecordInput};
use serde_json::{from_str, to_string};

#[test]
fn record_request_captures_input_output_cache_and_estimated_cost() {
    let record = record_request(UsageRecordInput {
        request_id: "req-1".into(),
        endpoint_id: "official-1".into(),
        input_tokens: 120,
        output_tokens: 30,
        cache_read_tokens: 10,
        cache_write_tokens: 0,
        estimated_cost: "0.0123".into(),
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
        input_tokens: 40,
        output_tokens: 15,
        cache_read_tokens: 5,
        cache_write_tokens: 2,
        estimated_cost: "0.0042".into(),
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
