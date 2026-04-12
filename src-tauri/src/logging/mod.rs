pub mod usage;

use crate::routing::engine::{
    CandidateEndpoint, RoutingError, endpoint_downgrade_reason, endpoint_rejection_reason,
};

pub fn log_route_downgrade(mode: &str, selected: &CandidateEndpoint, candidates: &[CandidateEndpoint], now_ms: u64) {
    let mut reasons = Vec::new();
    for candidate in candidates {
        if candidate.id == selected.id {
            continue;
        }
        if let Some(reason) = endpoint_rejection_reason(candidate, now_ms) {
            reasons.push(format!("{}:{}", candidate.id, reason));
        } else if let Some(reason) = endpoint_downgrade_reason(candidate, now_ms) {
            reasons.push(format!("{}:{}", candidate.id, reason));
        }
    }

    if reasons.is_empty() {
        return;
    }

    eprintln!(
        "gateway.routing outcome=downgraded mode={} selected={} reasons={}",
        mode,
        selected.id,
        reasons.join(",")
    );
}

pub fn log_route_rejection(mode: &str, error: &RoutingError, candidates: &[CandidateEndpoint], now_ms: u64) {
    let mut reasons = Vec::new();
    for candidate in candidates {
        if let Some(reason) = endpoint_rejection_reason(candidate, now_ms) {
            reasons.push(format!("{}:{}", candidate.id, reason));
        }
    }

    let detail = if reasons.is_empty() {
        "none".to_string()
    } else {
        reasons.join(",")
    };
    eprintln!(
        "gateway.routing outcome=rejected mode={} error={:?} reasons={}",
        mode, error, detail
    );
}
