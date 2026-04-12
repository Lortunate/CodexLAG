use std::time::{SystemTime, UNIX_EPOCH};

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
    choose_endpoint_at(mode, endpoints, wall_clock_now_ms())
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
    refresh_endpoint_health(endpoint, now_ms);

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
            // Ignored errors do not participate in failure streaks.
            endpoint.health.consecutive_timeouts = 0;
            endpoint.health.consecutive_server_errors = 0;
            endpoint.health.last_failure = Some(FailureClass::Ignored);
        }
    }
    endpoint.health.state
}

pub fn mark_success(endpoint: &mut CandidateEndpoint) {
    endpoint.health = EndpointHealth::default();
}

pub fn endpoint_health_state(endpoint: &CandidateEndpoint, now_ms: u64) -> EndpointHealthState {
    let _ = now_ms;
    endpoint.health.state
}

fn open_circuit(endpoint: &mut CandidateEndpoint, now_ms: u64, rules: &FailureRules) {
    endpoint.health.state = EndpointHealthState::Open;
    endpoint.health.open_until_ms = Some(now_ms.saturating_add(rules.cooldown_ms));
}

fn refresh_endpoint_health(endpoint: &mut CandidateEndpoint, now_ms: u64) {
    if endpoint.health.state != EndpointHealthState::Open {
        return;
    }

    let is_expired = endpoint
        .health
        .open_until_ms
        .is_some_and(|until_ms| now_ms >= until_ms);
    if is_expired {
        endpoint.health.state = EndpointHealthState::Degraded;
        endpoint.health.open_until_ms = None;
    }
}

fn choose_from_pool(
    endpoints: &[CandidateEndpoint],
    pool: PoolKind,
    now_ms: u64,
) -> Option<CandidateEndpoint> {
    let mut candidates: Vec<_> = endpoints.to_vec();
    for candidate in &mut candidates {
        refresh_endpoint_health(candidate, now_ms);
    }

    let mut candidates: Vec<_> = candidates
        .into_iter()
        .filter(|item| item.available && item.pool == pool && item.health.state != EndpointHealthState::Open)
        .collect();

    candidates.sort_by(|left, right| {
        health_rank(left.health.state)
            .cmp(&health_rank(right.health.state))
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
    let mut endpoint = endpoint.clone();
    refresh_endpoint_health(&mut endpoint, now_ms);

    if !endpoint.available {
        return Some("unavailable");
    }

    if endpoint.health.state == EndpointHealthState::Open {
        return Some("circuit_open");
    }

    None
}

pub fn endpoint_downgrade_reason(endpoint: &CandidateEndpoint, now_ms: u64) -> Option<&'static str> {
    let mut endpoint = endpoint.clone();
    refresh_endpoint_health(&mut endpoint, now_ms);

    if endpoint.health.state == EndpointHealthState::Degraded {
        return Some("degraded_health");
    }

    None
}

pub fn wall_clock_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
