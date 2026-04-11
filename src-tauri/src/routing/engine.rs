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
    let mut candidates: Vec<_> = endpoints
        .iter()
        .filter(|item| item.available)
        .filter(|item| match mode {
            ACCOUNT_ONLY => item.pool == PoolKind::Official,
            RELAY_ONLY => item.pool == PoolKind::Relay,
            _ => true,
        })
        .cloned()
        .collect();

    candidates.sort_by_key(|item| item.priority);
    candidates.into_iter().next()
}
