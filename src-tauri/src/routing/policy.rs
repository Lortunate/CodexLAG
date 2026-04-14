pub const ACCOUNT_ONLY: &str = "account_only";
pub const RELAY_ONLY: &str = "relay_only";
pub const HYBRID: &str = "hybrid";

use std::collections::HashMap;

use crate::routing::engine::CandidateEndpoint;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingMode {
    AccountOnly,
    RelayOnly,
    Hybrid,
}

impl RoutingMode {
    pub const ALL: [Self; 3] = [Self::AccountOnly, Self::RelayOnly, Self::Hybrid];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AccountOnly => ACCOUNT_ONLY,
            Self::RelayOnly => RELAY_ONLY,
            Self::Hybrid => HYBRID,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            ACCOUNT_ONLY => Some(Self::AccountOnly),
            RELAY_ONLY => Some(Self::RelayOnly),
            HYBRID => Some(Self::Hybrid),
            _ => None,
        }
    }
}

pub fn apply_selection_order(
    candidates: &[CandidateEndpoint],
    selection_order: &[String],
) -> Vec<CandidateEndpoint> {
    let mut by_id = candidates
        .iter()
        .cloned()
        .map(|candidate| (candidate.id.clone(), candidate))
        .collect::<HashMap<_, _>>();

    let mut ordered = Vec::new();
    for endpoint_id in selection_order {
        if let Some(candidate) = by_id.remove(endpoint_id) {
            ordered.push(candidate);
        }
    }
    for candidate in candidates {
        if let Some(remaining) = by_id.remove(candidate.id.as_str()) {
            ordered.push(remaining);
        }
    }
    ordered
}
