use crate::routing::policy::{ACCOUNT_ONLY, RELAY_ONLY};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoolKind {
    Official,
    Relay,
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

pub fn choose_endpoint(mode: &str, endpoints: &[CandidateEndpoint]) -> Option<CandidateEndpoint> {
    match mode {
        ACCOUNT_ONLY => choose_from_pool(endpoints, PoolKind::Official),
        RELAY_ONLY => choose_from_pool(endpoints, PoolKind::Relay),
        _ => choose_from_pool(endpoints, PoolKind::Official)
            .or_else(|| choose_from_pool(endpoints, PoolKind::Relay)),
    }
}

fn choose_from_pool(endpoints: &[CandidateEndpoint], pool: PoolKind) -> Option<CandidateEndpoint> {
    let mut candidates: Vec<_> = endpoints
        .iter()
        .filter(|item| item.available && item.pool == pool)
        .cloned()
        .collect();

    candidates.sort_by_key(|item| item.priority);
    candidates.into_iter().next()
}
