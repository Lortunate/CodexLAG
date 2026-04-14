use std::time::{SystemTime, UNIX_EPOCH};

pub use crate::models::{EndpointFailure, EndpointHealthState, FailureRules};
use crate::models::{EndpointHealth, FailureClass, RecoveryRules};
use crate::routing::policy::RoutingMode;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PoolKind {
    Official,
    Relay,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingError {
    InvalidMode,
    NoAvailableEndpoint,
}

#[derive(Debug, Clone)]
pub struct CandidateEndpoint {
    pub id: String,
    pub priority: i32,
    pub available: bool,
    pub pool: PoolKind,
    pub health: EndpointHealth,
}

impl CandidateEndpoint {
    pub fn official(id: &str, priority: i32, available: bool) -> Self {
        Self {
            id: id.into(),
            priority,
            available,
            pool: PoolKind::Official,
            health: EndpointHealth::default(),
        }
    }

    pub fn relay(id: &str, priority: i32, available: bool) -> Self {
        Self {
            id: id.into(),
            priority,
            available,
            pool: PoolKind::Relay,
            health: EndpointHealth::default(),
        }
    }
}

pub fn choose_endpoint(
    mode: &str,
    endpoints: &[CandidateEndpoint],
) -> Result<CandidateEndpoint, RoutingError> {
    choose_endpoint_at_with_recovery(mode, endpoints, wall_clock_now_ms(), &RecoveryRules::default())
}

pub fn choose_endpoint_at(
    mode: &str,
    endpoints: &[CandidateEndpoint],
    now_ms: u64,
) -> Result<CandidateEndpoint, RoutingError> {
    choose_endpoint_at_with_recovery(mode, endpoints, now_ms, &RecoveryRules::default())
}

pub fn choose_endpoint_at_with_recovery(
    mode: &str,
    endpoints: &[CandidateEndpoint],
    now_ms: u64,
    recovery_rules: &RecoveryRules,
) -> Result<CandidateEndpoint, RoutingError> {
    match RoutingMode::parse(mode).ok_or(RoutingError::InvalidMode)? {
        RoutingMode::AccountOnly => {
            choose_from_pool(endpoints, PoolKind::Official, now_ms, recovery_rules)
        }
        RoutingMode::RelayOnly => choose_from_pool(endpoints, PoolKind::Relay, now_ms, recovery_rules),
        RoutingMode::Hybrid => choose_hybrid(endpoints, now_ms, recovery_rules),
    }
    .ok_or(RoutingError::NoAvailableEndpoint)
}

pub fn record_failure(
    endpoint: &mut CandidateEndpoint,
    failure: EndpointFailure,
    now_ms: u64,
    rules: &FailureRules,
) -> EndpointHealthState {
    refresh_endpoint_health(endpoint, now_ms, &RecoveryRules::default());

    match rules.classify_failure(&failure) {
        FailureClass::Timeout => {
            endpoint.health.consecutive_timeouts += 1;
            endpoint.health.consecutive_server_errors = 0;
            endpoint.health.last_failure = Some(FailureClass::Timeout);
            if endpoint.health.consecutive_timeouts >= rules.timeout_open_after {
                open_circuit(endpoint, now_ms, rules);
            } else {
                endpoint.health.state = EndpointHealthState::Degraded;
                endpoint.health.open_until_ms = None;
            }
        }
        FailureClass::RateLimited => {
            endpoint.health.consecutive_timeouts = 0;
            endpoint.health.consecutive_server_errors = 0;
            endpoint.health.last_failure = Some(FailureClass::RateLimited);
            open_circuit(endpoint, now_ms, rules);
        }
        FailureClass::ServerError => {
            endpoint.health.consecutive_server_errors += 1;
            endpoint.health.consecutive_timeouts = 0;
            endpoint.health.last_failure = Some(FailureClass::ServerError);
            if endpoint.health.consecutive_server_errors >= rules.server_error_open_after {
                open_circuit(endpoint, now_ms, rules);
            } else {
                endpoint.health.state = EndpointHealthState::Degraded;
                endpoint.health.open_until_ms = None;
            }
        }
        FailureClass::Ignored => {
            // Ignored failures preserve existing streak counters.
            endpoint.health.last_failure = Some(FailureClass::Ignored);
        }
    }
    endpoint.health.state
}

pub fn mark_success(endpoint: &mut CandidateEndpoint) {
    endpoint.health = EndpointHealth::default();
}

pub fn record_failure_for_endpoint(
    endpoints: &mut [CandidateEndpoint],
    endpoint_id: &str,
    pool: &PoolKind,
    failure: EndpointFailure,
    now_ms: u64,
    rules: &FailureRules,
) -> Option<EndpointHealthState> {
    let endpoint = endpoints
        .iter_mut()
        .find(|candidate| candidate.id == endpoint_id && &candidate.pool == pool)?;
    Some(record_failure(endpoint, failure, now_ms, rules))
}

pub fn mark_success_for_endpoint(
    endpoints: &mut [CandidateEndpoint],
    endpoint_id: &str,
    pool: &PoolKind,
) -> bool {
    if let Some(endpoint) = endpoints
        .iter_mut()
        .find(|candidate| candidate.id == endpoint_id && &candidate.pool == pool)
    {
        mark_success(endpoint);
        return true;
    }

    false
}

pub fn endpoint_health_state(endpoint: &CandidateEndpoint) -> EndpointHealthState {
    endpoint.health.state
}

fn open_circuit(endpoint: &mut CandidateEndpoint, now_ms: u64, rules: &FailureRules) {
    endpoint.health.state = EndpointHealthState::OpenCircuit;
    endpoint.health.open_until_ms = Some(now_ms.saturating_add(rules.cooldown_ms));
}

fn refresh_endpoint_health(
    endpoint: &mut CandidateEndpoint,
    now_ms: u64,
    _recovery_rules: &RecoveryRules,
) {
    if endpoint.health.state != EndpointHealthState::OpenCircuit {
        return;
    }

    let is_expired = endpoint
        .health
        .open_until_ms
        .is_some_and(|until_ms| now_ms >= until_ms);
    if is_expired {
        endpoint.health.state = EndpointHealthState::HalfOpen;
        endpoint.health.open_until_ms = None;
    }
}

pub fn refresh_endpoint_health_for_test(
    endpoint: &mut CandidateEndpoint,
    now_ms: u64,
    recovery_rules: &RecoveryRules,
) {
    refresh_endpoint_health(endpoint, now_ms, recovery_rules);
}

fn choose_from_pool(
    endpoints: &[CandidateEndpoint],
    pool: PoolKind,
    now_ms: u64,
    recovery_rules: &RecoveryRules,
) -> Option<CandidateEndpoint> {
    let mut candidates: Vec<_> = endpoints.to_vec();
    for candidate in &mut candidates {
        refresh_endpoint_health(candidate, now_ms, recovery_rules);
    }

    let mut candidates: Vec<_> = candidates
        .into_iter()
        .filter(|item| {
            item.available
                && item.pool == pool
                && item.health.state != EndpointHealthState::OpenCircuit
                && item.health.state != EndpointHealthState::Disabled
        })
        .collect();

    candidates.sort_by(|left, right| {
        health_rank(left.health.state)
            .cmp(&health_rank(right.health.state))
            .then_with(|| left.priority.cmp(&right.priority))
            .then_with(|| left.id.cmp(&right.id))
    });
    candidates.into_iter().next()
}

fn choose_hybrid(
    endpoints: &[CandidateEndpoint],
    now_ms: u64,
    recovery_rules: &RecoveryRules,
) -> Option<CandidateEndpoint> {
    let official = choose_from_pool(endpoints, PoolKind::Official, now_ms, recovery_rules);
    let relay = choose_from_pool(endpoints, PoolKind::Relay, now_ms, recovery_rules);

    match (official, relay) {
        (Some(official), Some(relay)) => {
            let official_rank = health_rank(official.health.state);
            let relay_rank = health_rank(relay.health.state);
            if official_rank < relay_rank {
                Some(official)
            } else if relay_rank < official_rank {
                Some(relay)
            } else if official.priority <= relay.priority {
                Some(official)
            } else {
                Some(relay)
            }
        }
        (Some(official), None) => Some(official),
        (None, Some(relay)) => Some(relay),
        (None, None) => None,
    }
}

fn health_rank(state: EndpointHealthState) -> i32 {
    match state {
        EndpointHealthState::Healthy => 0,
        EndpointHealthState::Degraded => 1,
        EndpointHealthState::HalfOpen => 2,
        EndpointHealthState::OpenCircuit => 3,
        EndpointHealthState::Disabled => 4,
    }
}

pub fn endpoint_rejection_reason(
    endpoint: &CandidateEndpoint,
    now_ms: u64,
) -> Option<&'static str> {
    let mut endpoint = endpoint.clone();
    refresh_endpoint_health(&mut endpoint, now_ms, &RecoveryRules::default());

    if !endpoint.available {
        return Some("unavailable");
    }

    if endpoint.health.state == EndpointHealthState::OpenCircuit {
        return Some("circuit_open");
    }
    if endpoint.health.state == EndpointHealthState::Disabled {
        return Some("disabled");
    }

    None
}

pub fn endpoint_downgrade_reason(
    endpoint: &CandidateEndpoint,
    now_ms: u64,
) -> Option<&'static str> {
    let mut endpoint = endpoint.clone();
    refresh_endpoint_health(&mut endpoint, now_ms, &RecoveryRules::default());

    if endpoint.health.state == EndpointHealthState::Degraded {
        return Some("degraded_health");
    }
    if endpoint.health.state == EndpointHealthState::HalfOpen {
        return Some("half_open_probe");
    }

    None
}

pub fn wall_clock_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
