pub mod redaction;
pub mod runtime;
pub mod usage;

use crate::logging::runtime::{build_attempt_id, format_runtime_event_fields};
use crate::routing::engine::{
    endpoint_downgrade_reason, endpoint_rejection_reason, CandidateEndpoint, RoutingError,
};

fn routing_error_code(error: &RoutingError) -> &'static str {
    match error {
        RoutingError::InvalidMode => "invalid_mode",
        RoutingError::NoAvailableEndpoint => "no_available_endpoint",
    }
}

pub fn log_route_downgrade(
    request_id: &str,
    attempt_id: &str,
    mode: &str,
    selected: &CandidateEndpoint,
    candidates: &[CandidateEndpoint],
    now_ms: u64,
) {
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
    let line = format_runtime_event_fields(
        "routing",
        "routing.endpoint.downgraded",
        request_id,
        Some(attempt_id),
        Some(selected.id.as_str()),
        None,
        None,
        &[
            ("mode", mode),
            ("selected", selected.id.as_str()),
            ("reasons", reasons_joined.as_str()),
        ],
    );
    log::warn!("{line}");
}

pub fn log_route_rejection(
    request_id: &str,
    attempt_count: usize,
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
    let attempt_count_value = attempt_count.to_string();
    let attempt_id = if attempt_count == 0 {
        None
    } else {
        Some(build_attempt_id(
            request_id,
            attempt_count.saturating_sub(1),
        ))
    };
    let line = format_runtime_event_fields(
        "routing",
        "routing.endpoint.rejected",
        request_id,
        attempt_id.as_deref(),
        None,
        None,
        Some(error_code),
        &[
            ("attempt_count", attempt_count_value.as_str()),
            ("mode", mode),
            ("reasons", detail.as_str()),
        ],
    );
    log::warn!("{line}");
}
