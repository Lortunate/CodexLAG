use codexlag_lib::routing::engine::{
    choose_endpoint, choose_endpoint_at, mark_success, record_failure, CandidateEndpoint,
    EndpointFailure, EndpointHealthState, FailureRules, RoutingError,
};

#[test]
fn hybrid_mode_prefers_lower_priority_candidate_when_health_is_equal() {
    let endpoints = vec![
        CandidateEndpoint::official("official-1", 50, true),
        CandidateEndpoint::relay("relay-1", 10, true),
    ];

    let selected = choose_endpoint("hybrid", &endpoints).expect("selected endpoint");
    assert_eq!(selected.id, "relay-1");
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
fn invalid_mode_returns_distinct_error() {
    let endpoints = vec![
        CandidateEndpoint::official("official-1", 10, true),
        CandidateEndpoint::relay("relay-1", 20, true),
    ];

    let selected = choose_endpoint("not-a-real-mode", &endpoints);
    assert!(matches!(selected, Err(RoutingError::InvalidMode)));
}

#[test]
fn no_available_endpoint_returns_distinct_error() {
    let endpoints = vec![
        CandidateEndpoint::official("official-1", 10, true),
        CandidateEndpoint::relay("relay-1", 20, false),
    ];

    let selected = choose_endpoint("relay_only", &endpoints);
    assert!(matches!(selected, Err(RoutingError::NoAvailableEndpoint)));
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

#[test]
fn rate_limited_endpoint_is_opened_and_demoted() {
    let rules = FailureRules::default();
    let now_ms = 1_000;
    let mut official = CandidateEndpoint::official("official-1", 10, true);
    let relay = CandidateEndpoint::relay("relay-1", 20, true);

    let state = record_failure(
        &mut official,
        EndpointFailure::HttpStatus(429),
        now_ms,
        &rules,
    );
    assert_eq!(state, EndpointHealthState::OpenCircuit);

    let selected = choose_endpoint_at("hybrid", &[official.clone(), relay.clone()], now_ms + 1)
        .expect("relay should be selected when official is open");
    assert_eq!(selected.id, "relay-1");
}

#[test]
fn repeated_5xx_opens_circuit_until_cooldown_elapsed() {
    let rules = FailureRules {
        server_error_open_after: 2,
        cooldown_ms: 500,
        ..FailureRules::default()
    };
    let now_ms = 5_000;
    let mut endpoint = CandidateEndpoint::relay("relay-1", 10, true);

    let first = record_failure(
        &mut endpoint,
        EndpointFailure::HttpStatus(500),
        now_ms,
        &rules,
    );
    assert_eq!(first, EndpointHealthState::Degraded);

    let second = record_failure(
        &mut endpoint,
        EndpointFailure::HttpStatus(503),
        now_ms + 1,
        &rules,
    );
    assert_eq!(second, EndpointHealthState::OpenCircuit);

    let before_recovery = choose_endpoint_at("relay_only", &[endpoint.clone()], now_ms + 200);
    assert!(matches!(
        before_recovery,
        Err(RoutingError::NoAvailableEndpoint)
    ));

    let after_recovery = choose_endpoint_at("relay_only", &[endpoint], now_ms + 600);
    assert!(
        after_recovery.is_ok(),
        "endpoint should recover after cooldown"
    );
}

#[test]
fn timeout_threshold_opens_and_success_resets_health() {
    let rules = FailureRules {
        timeout_open_after: 2,
        cooldown_ms: 50,
        ..FailureRules::default()
    };
    let mut endpoint = CandidateEndpoint::relay("relay-1", 10, true);
    let now_ms = 10_000;

    assert_eq!(
        record_failure(&mut endpoint, EndpointFailure::Timeout, now_ms, &rules),
        EndpointHealthState::Degraded
    );
    assert_eq!(
        record_failure(&mut endpoint, EndpointFailure::Timeout, now_ms + 1, &rules),
        EndpointHealthState::OpenCircuit
    );

    assert!(choose_endpoint_at("relay_only", &[endpoint.clone()], now_ms + 2).is_err());

    mark_success(&mut endpoint);
    let selected = choose_endpoint_at("relay_only", &[endpoint], now_ms + 3).expect("recovered");
    assert_eq!(selected.id, "relay-1");
}

#[test]
fn timeout_and_server_error_streaks_reset_each_other() {
    let rules = FailureRules {
        timeout_open_after: 2,
        server_error_open_after: 2,
        ..FailureRules::default()
    };
    let mut endpoint = CandidateEndpoint::relay("relay-1", 10, true);
    let now_ms = 11_000;

    assert_eq!(
        record_failure(&mut endpoint, EndpointFailure::Timeout, now_ms, &rules),
        EndpointHealthState::Degraded
    );
    assert_eq!(
        record_failure(
            &mut endpoint,
            EndpointFailure::HttpStatus(500),
            now_ms + 1,
            &rules
        ),
        EndpointHealthState::Degraded
    );
    assert_eq!(
        record_failure(&mut endpoint, EndpointFailure::Timeout, now_ms + 2, &rules),
        EndpointHealthState::Degraded
    );
    assert_eq!(
        record_failure(&mut endpoint, EndpointFailure::Timeout, now_ms + 3, &rules),
        EndpointHealthState::OpenCircuit
    );
}

#[test]
fn cooldown_recovery_updates_health_state_before_selection() {
    let rules = FailureRules {
        cooldown_ms: 50,
        ..FailureRules::default()
    };
    let now_ms = 12_000;
    let mut endpoint = CandidateEndpoint::relay("relay-1", 10, true);

    assert_eq!(
        record_failure(
            &mut endpoint,
            EndpointFailure::HttpStatus(429),
            now_ms,
            &rules
        ),
        EndpointHealthState::OpenCircuit
    );
    let selected = choose_endpoint_at("relay_only", &[endpoint], now_ms + 60).expect("recovered");
    assert_eq!(selected.health.state, EndpointHealthState::HalfOpen);
}

#[test]
fn cooldown_expiry_moves_open_circuit_endpoint_to_half_open() {
    let rules = FailureRules {
        cooldown_ms: 30_000,
        timeout_open_after: 1,
        server_error_open_after: 1,
    };
    let mut endpoint = CandidateEndpoint::official("official-primary", 10, true);

    let opened = record_failure(&mut endpoint, EndpointFailure::Timeout, 1_000, &rules);
    assert_eq!(opened, EndpointHealthState::OpenCircuit);

    codexlag_lib::routing::engine::refresh_endpoint_health_for_test(
        &mut endpoint,
        31_500,
        &codexlag_lib::models::RecoveryRules {
            half_open_after_ms: 15_000,
            success_close_after: 1,
        },
    );

    assert_eq!(endpoint.health.state, EndpointHealthState::HalfOpen);
}

#[test]
fn ignored_failures_do_not_reset_consecutive_failure_streaks() {
    let rules = FailureRules {
        timeout_open_after: 2,
        ..FailureRules::default()
    };
    let mut endpoint = CandidateEndpoint::relay("relay-1", 10, true);
    let now_ms = 13_000;

    assert_eq!(
        record_failure(&mut endpoint, EndpointFailure::Timeout, now_ms, &rules),
        EndpointHealthState::Degraded
    );
    assert_eq!(
        record_failure(
            &mut endpoint,
            EndpointFailure::HttpStatus(400),
            now_ms + 1,
            &rules
        ),
        EndpointHealthState::Degraded
    );
    assert_eq!(
        record_failure(&mut endpoint, EndpointFailure::Timeout, now_ms + 2, &rules),
        EndpointHealthState::OpenCircuit
    );
}

#[test]
fn hybrid_prefers_healthy_relay_over_degraded_official_even_with_worse_priority() {
    let rules = FailureRules::default();
    let now_ms = 14_000;
    let mut official = CandidateEndpoint::official("official-1", 5, true);
    let relay = CandidateEndpoint::relay("relay-1", 50, true);

    assert_eq!(
        record_failure(&mut official, EndpointFailure::Timeout, now_ms, &rules),
        EndpointHealthState::Degraded
    );
    let selected = choose_endpoint_at("hybrid", &[official, relay], now_ms + 1).expect("selected");
    assert_eq!(selected.id, "relay-1");
}
