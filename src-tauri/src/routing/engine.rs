use crate::routing::policy::RoutingMode;
pub use crate::models::{EndpointFailure, EndpointHealthState, FailureRules};
use crate::models::{EndpointHealth, FailureClass};

#[derive(Debug, Clone, PartialEq, Eq)]
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

pub fn choose_endpoint(mode: &str, endpoints: &[CandidateEndpoint]) -> Result<CandidateEndpoint, RoutingError> {
    choose_endpoint_at(mode, endpoints, 0)
}

pub fn choose_endpoint_at(
    mode: &str,
    endpoints: &[CandidateEndpoint],
    now_ms: u64,
) -> Result<CandidateEndpoint, RoutingError> {
    match RoutingMode::parse(mode).ok_or(RoutingError::InvalidMode)? {
        RoutingMode::AccountOnly => choose_from_pool(endpoints, PoolKind::Official, now_ms),
        RoutingMode::RelayOnly => choose_from_pool(endpoints, PoolKind::Relay, now_ms),
        RoutingMode::Hybrid => choose_from_pool(endpoints, PoolKind::Official, now_ms)
            .or_else(|| choose_from_pool(endpoints, PoolKind::Relay, now_ms)),
    }
    .ok_or(RoutingError::NoAvailableEndpoint)
}

pub fn record_failure(
    endpoint: &mut CandidateEndpoint,
    failure: EndpointFailure,
    now_ms: u64,
    rules: &FailureRules,
) -> EndpointHealthState {
    match rules.classify_failure(&failure) {
        FailureClass::Timeout => {
            endpoint.health.consecutive_timeouts += 1;
            endpoint.health.last_failure = Some(FailureClass::Timeout);
            if endpoint.health.consecutive_timeouts >= rules.timeout_open_after {
                open_circuit(endpoint, now_ms, rules);
            } else {
                endpoint.health.state = EndpointHealthState::Degraded;
            }
        }
        FailureClass::RateLimited => {
            endpoint.health.last_failure = Some(FailureClass::RateLimited);
            open_circuit(endpoint, now_ms, rules);
        }
        FailureClass::ServerError => {
            endpoint.health.consecutive_server_errors += 1;
            endpoint.health.last_failure = Some(FailureClass::ServerError);
            if endpoint.health.consecutive_server_errors >= rules.server_error_open_after {
                open_circuit(endpoint, now_ms, rules);
            } else {
                endpoint.health.state = EndpointHealthState::Degraded;
            }
        }
        FailureClass::Ignored => {
            endpoint.health.last_failure = Some(FailureClass::Ignored);
        }
    }
    endpoint_health_state(endpoint, now_ms)
}

pub fn mark_success(endpoint: &mut CandidateEndpoint) {
    endpoint.health = EndpointHealth::default();
}

pub fn endpoint_health_state(endpoint: &CandidateEndpoint, now_ms: u64) -> EndpointHealthState {
    if endpoint.health.state != EndpointHealthState::Open {
        return endpoint.health.state;
    }

    match endpoint.health.open_until_ms {
        Some(until_ms) if now_ms >= until_ms => EndpointHealthState::Degraded,
        Some(_) => EndpointHealthState::Open,
        None => EndpointHealthState::Open,
    }
}

fn open_circuit(endpoint: &mut CandidateEndpoint, now_ms: u64, rules: &FailureRules) {
    endpoint.health.state = EndpointHealthState::Open;
    endpoint.health.open_until_ms = Some(now_ms.saturating_add(rules.cooldown_ms));
}

fn choose_from_pool(
    endpoints: &[CandidateEndpoint],
    pool: PoolKind,
    now_ms: u64,
) -> Option<CandidateEndpoint> {
    let mut candidates: Vec<_> = endpoints
        .iter()
        .filter(|item| {
            item.available
                && item.pool == pool
                && endpoint_health_state(item, now_ms) != EndpointHealthState::Open
        })
        .cloned()
        .collect();

    candidates.sort_by(|left, right| {
        health_rank(endpoint_health_state(left, now_ms))
            .cmp(&health_rank(endpoint_health_state(right, now_ms)))
            .then_with(|| left.priority.cmp(&right.priority))
            .then_with(|| left.id.cmp(&right.id))
    });
    candidates.into_iter().next()
}

fn health_rank(state: EndpointHealthState) -> i32 {
    match state {
        EndpointHealthState::Healthy => 0,
        EndpointHealthState::Degraded => 1,
        EndpointHealthState::Open => 2,
    }
}

pub fn endpoint_rejection_reason(endpoint: &CandidateEndpoint, now_ms: u64) -> Option<&'static str> {
    if !endpoint.available {
        return Some("unavailable");
    }

    if endpoint_health_state(endpoint, now_ms) == EndpointHealthState::Open {
        return Some("circuit_open");
    }

    None
}

pub fn endpoint_downgrade_reason(endpoint: &CandidateEndpoint, now_ms: u64) -> Option<&'static str> {
    if endpoint_health_state(endpoint, now_ms) == EndpointHealthState::Degraded {
        return Some("degraded_health");
    }

    None
}
