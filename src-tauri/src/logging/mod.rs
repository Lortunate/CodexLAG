pub mod runtime;
pub mod usage;

use crate::routing::engine::{
    CandidateEndpoint, RoutingError, endpoint_downgrade_reason, endpoint_rejection_reason,
};
use crate::logging::runtime::format_event_fields;

fn routing_error_code(error: &RoutingError) -> &'static str {
    match error {
        RoutingError::InvalidMode => "invalid_mode",
        RoutingError::NoAvailableEndpoint => "no_available_endpoint",
    }
}

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

pub fn log_route_rejection(
    request_id: &str,
    mode: &str,
    error: &RoutingError,
    candidates: &[CandidateEndpoint],
    now_ms: u64,
) {
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
    let error_code = routing_error_code(error);
    let line = format_event_fields(&[
        ("event", "routing.endpoint.rejected"),
        ("request_id", request_id),
        ("mode", mode),
        ("error", error_code),
        ("reasons", detail.as_str()),
    ]);
    log::warn!("{line}");
}
