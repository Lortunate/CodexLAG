use crate::routing::policy::RoutingMode;

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
}

impl CandidateEndpoint {
    pub fn official(id: &str, priority: i32, available: bool) -> Self {
        Self {
            id: id.into(),
            priority,
            available,
            pool: PoolKind::Official,
        }
    }

    pub fn relay(id: &str, priority: i32, available: bool) -> Self {
        Self {
            id: id.into(),
            priority,
            available,
            pool: PoolKind::Relay,
        }
    }
}

pub fn choose_endpoint(mode: &str, endpoints: &[CandidateEndpoint]) -> Result<CandidateEndpoint, RoutingError> {
    match RoutingMode::parse(mode).ok_or(RoutingError::InvalidMode)? {
        RoutingMode::AccountOnly => choose_from_pool(endpoints, PoolKind::Official),
        RoutingMode::RelayOnly => choose_from_pool(endpoints, PoolKind::Relay),
        RoutingMode::Hybrid => choose_from_pool(endpoints, PoolKind::Official)
            .or_else(|| choose_from_pool(endpoints, PoolKind::Relay)),
    }
    .ok_or(RoutingError::NoAvailableEndpoint)
}

fn choose_from_pool(endpoints: &[CandidateEndpoint], pool: PoolKind) -> Option<CandidateEndpoint> {
    let mut candidates: Vec<_> = endpoints
        .iter()
        .filter(|item| item.available && item.pool == pool)
        .cloned()
        .collect();

    candidates.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.id.cmp(&right.id))
    });
    candidates.into_iter().next()
}
