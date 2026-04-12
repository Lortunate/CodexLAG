pub mod runtime;
pub mod usage;

use crate::routing::engine::{
    CandidateEndpoint, RoutingError, endpoint_downgrade_reason, endpoint_rejection_reason,
};
use crate::logging::runtime::format_event_fields;

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

    let reasons_joined = reasons.join(",");
    let line = format_event_fields(&[
        ("event", "routing.endpoint.downgraded"),
        ("mode", mode),
        ("selected", selected.id.as_str()),
        ("reasons", reasons_joined.as_str()),
    ]);
    log::warn!("{line}");
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
    let error_repr = format!("{error:?}");
    let line = format_event_fields(&[
        ("event", "routing.endpoint.rejected"),
        ("mode", mode),
        ("error", error_repr.as_str()),
        ("reasons", detail.as_str()),
    ]);
    log::warn!("{line}");
}
