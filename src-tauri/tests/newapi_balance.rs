use codexlag_lib::providers::relay::normalize_newapi_balance_response;

#[test]
fn normalize_newapi_balance_response_maps_amounts() {
    let normalized =
        normalize_newapi_balance_response(r#"{"data":{"total_balance":"25.00","used_balance":"7.50"}}"#)
            .expect("normalized");

    assert_eq!(normalized.total, "25.00");
    assert_eq!(normalized.used, "7.50");
}
