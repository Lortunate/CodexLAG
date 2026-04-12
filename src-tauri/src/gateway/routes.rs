use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use std::collections::HashMap;

use crate::gateway::auth::{AuthenticatedPlatformKey, GatewayState};
use crate::logging::runtime::{build_attempt_id, format_event_fields};
use crate::logging::usage::UsageRecordInput;
use crate::logging::{log_route_downgrade, log_route_rejection};
use crate::routing::engine::{
    endpoint_downgrade_reason, endpoint_rejection_reason, wall_clock_now_ms, PoolKind,
    RoutingError,
};

#[derive(Debug, Serialize)]
struct CodexRequestSummary {
    platform_key: String,
    policy: String,
    allowed_mode: String,
    endpoint_id: String,
}

#[derive(Debug, Serialize)]
struct RoutingErrorResponse {
    error: &'static str,
    mode: String,
}

pub fn build_routes() -> Router<GatewayState> {
    Router::new()
        .route("/health", get(health))
        .route("/codex/request", post(codex_request))
}

async fn health() -> &'static str {
    "ok"
}

async fn codex_request(
    State(gateway_state): State<GatewayState>,
    headers: HeaderMap,
    auth: AuthenticatedPlatformKey,
) -> Result<Json<CodexRequestSummary>, (StatusCode, Json<RoutingErrorResponse>)> {
    let platform_key = auth.platform_key();
    let now_ms = wall_clock_now_ms();
    let mode_value = platform_key.allowed_mode.clone();
    let mode = mode_value.as_str();
    let request_id = gateway_state.next_request_id(&platform_key.name, now_ms, "unrouted");
    let endpoint_status_plan = if gateway_state.test_route_headers_enabled() {
        parse_endpoint_status_plan(&headers)
    } else {
        HashMap::new()
    };

    let accepted_line = format_event_fields(&[
        ("event", "gateway.request.accepted"),
        ("request_id", request_id.as_str()),
        ("platform_key", platform_key.name.as_str()),
        ("mode", mode),
    ]);
    log::info!("{accepted_line}");

    let policy = gateway_state.policy_for_platform_key(platform_key).ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(RoutingErrorResponse {
            error: "policy_missing",
            mode: platform_key.allowed_mode.clone(),
        }),
    ))?;
    let selection = gateway_state
        .choose_endpoint_with_runtime_failover(request_id.as_str(), mode, |endpoint, _context| {
            invoke_endpoint_with_plan(endpoint.id.as_str(), &endpoint_status_plan)
        })
        .map_err(|error| {
            let candidates = gateway_state.current_candidates();
            log_route_rejection(request_id.as_str(), mode, &error, &candidates, now_ms);
            map_routing_error(mode, error)
        })?;
    let selected = selection.endpoint;
    let candidates = gateway_state.current_candidates();
    let attempt_index = selection.attempt_count.saturating_sub(1);
    let attempt_id = build_attempt_id(request_id.as_str(), attempt_index);

    let selected_line = format_event_fields(&[
        ("event", "routing.endpoint.selected"),
        ("request_id", request_id.as_str()),
        ("attempt_id", attempt_id.as_str()),
        ("endpoint_id", selected.id.as_str()),
    ]);
    log::info!("{selected_line}");

    if should_log_downgrade(mode, &selected, &candidates, now_ms) {
        log_route_downgrade(
            request_id.as_str(),
            attempt_id.as_str(),
            mode,
            &selected,
            &candidates,
            now_ms,
        );
    }

    let endpoint_id = selected.id.clone();
    gateway_state.record_usage_request(UsageRecordInput {
        request_id: request_id.clone(),
        endpoint_id: endpoint_id.clone(),
        input_tokens: 0,
        output_tokens: 0,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        estimated_cost: String::new(),
    });

    Ok(Json(CodexRequestSummary {
        platform_key: platform_key.name.clone(),
        policy: policy.name,
        allowed_mode: platform_key.allowed_mode.clone(),
        endpoint_id,
    }))
}

fn parse_endpoint_status_plan(headers: &HeaderMap) -> HashMap<String, u16> {
    headers
        .get("x-codexlag-endpoint-status")
        .and_then(|value| value.to_str().ok())
        .map(|raw| {
            raw.split(',')
                .filter_map(|segment| {
                    let (endpoint_id, status) = segment.trim().split_once(':')?;
                    let status = status.trim().parse::<u16>().ok()?;
                    Some((endpoint_id.trim().to_string(), status))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn invoke_endpoint_with_plan(
    endpoint_id: &str,
    status_plan: &HashMap<String, u16>,
) -> Result<(), crate::models::EndpointFailure> {
    match status_plan.get(endpoint_id) {
        Some(status) if *status >= 400 => Err(crate::models::EndpointFailure::HttpStatus(*status)),
        _ => Ok(()),
    }
}

fn should_log_downgrade(
    mode: &str,
    selected: &crate::routing::engine::CandidateEndpoint,
    candidates: &[crate::routing::engine::CandidateEndpoint],
    now_ms: u64,
) -> bool {
    if mode != "hybrid" || selected.pool != PoolKind::Relay {
        return false;
    }

    candidates.iter().any(|candidate| {
        candidate.pool == PoolKind::Official
            && candidate.id != selected.id
            && (endpoint_rejection_reason(candidate, now_ms).is_some()
                || endpoint_downgrade_reason(candidate, now_ms).is_some())
    })
}

fn map_routing_error(mode: &str, error: RoutingError) -> (StatusCode, Json<RoutingErrorResponse>) {
    let (status, code) = match error {
        RoutingError::InvalidMode => (StatusCode::BAD_REQUEST, "invalid_mode"),
        RoutingError::NoAvailableEndpoint => {
            (StatusCode::SERVICE_UNAVAILABLE, "no_available_endpoint")
        }
    };

    (
        status,
        Json(RoutingErrorResponse {
            error: code,
            mode: mode.to_string(),
        }),
    )
}
