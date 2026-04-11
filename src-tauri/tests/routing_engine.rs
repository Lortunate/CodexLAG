use codexlag_lib::routing::engine::{CandidateEndpoint, choose_endpoint};

#[test]
fn hybrid_mode_prefers_official_then_relay() {
    let endpoints = vec![
        CandidateEndpoint::official("official-1", 50, true),
        CandidateEndpoint::relay("relay-1", 10, true),
    ];

    let selected = choose_endpoint("hybrid", &endpoints).expect("selected endpoint");
    assert_eq!(selected.id, "official-1");
}

#[test]
fn relay_only_skips_official_candidates() {
    let endpoints = vec![
        CandidateEndpoint::official("official-1", 10, true),
        CandidateEndpoint::relay("relay-1", 20, true),
    ];

    let selected = choose_endpoint("relay_only", &endpoints).expect("selected endpoint");
    assert_eq!(selected.id, "relay-1");
}

#[test]
fn invalid_mode_is_rejected_instead_of_falling_back_to_hybrid() {
    let endpoints = vec![
        CandidateEndpoint::official("official-1", 10, true),
        CandidateEndpoint::relay("relay-1", 20, true),
    ];

    let selected = choose_endpoint("not-a-real-mode", &endpoints);
    assert!(selected.is_none(), "unexpected endpoint selected for invalid mode");
}

#[test]
fn equal_priority_candidates_use_stable_secondary_ordering() {
    let endpoints = vec![
        CandidateEndpoint::relay("relay-b", 10, true),
        CandidateEndpoint::relay("relay-a", 10, true),
    ];

    let selected = choose_endpoint("relay_only", &endpoints).expect("selected endpoint");
    assert_eq!(selected.id, "relay-a");
}
