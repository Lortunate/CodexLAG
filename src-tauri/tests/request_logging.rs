use codexlag_lib::logging::usage::{record_request, UsageRecordInput};

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
