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
