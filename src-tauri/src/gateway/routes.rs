use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;

use crate::gateway::auth::{AuthenticatedPlatformKey, GatewayState};
use crate::gateway::server::default_candidates;
use crate::logging::{log_route_downgrade, log_route_rejection};
use crate::routing::engine::{RoutingError, choose_endpoint_at, wall_clock_now_ms};

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
    auth: AuthenticatedPlatformKey,
) -> Result<Json<CodexRequestSummary>, (StatusCode, Json<RoutingErrorResponse>)> {
    let platform_key = auth.platform_key();
    let policy = gateway_state
        .policy_for_platform_key(platform_key)
        .ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(RoutingErrorResponse {
                error: "policy_missing",
                mode: platform_key.allowed_mode.clone(),
            }),
        ))?;
    let now_ms = wall_clock_now_ms();
    let mode = platform_key.allowed_mode.as_str();
    let candidates = default_candidates();
    let selected = choose_endpoint_at(mode, &candidates, now_ms).map_err(|error| {
        log_route_rejection(mode, &error, &candidates, now_ms);
        map_routing_error(mode, error)
    })?;

    if selected.pool == crate::routing::engine::PoolKind::Relay {
        log_route_downgrade(mode, &selected, &candidates, now_ms);
    }

    Ok(Json(CodexRequestSummary {
        platform_key: platform_key.name.clone(),
        policy: policy.name,
        allowed_mode: platform_key.allowed_mode.clone(),
        endpoint_id: selected.id,
    }))
}

fn map_routing_error(
    mode: &str,
    error: RoutingError,
) -> (StatusCode, Json<RoutingErrorResponse>) {
    let (status, code) = match error {
        RoutingError::InvalidMode => (StatusCode::BAD_REQUEST, "invalid_mode"),
        RoutingError::NoAvailableEndpoint => (StatusCode::SERVICE_UNAVAILABLE, "no_available_endpoint"),
    };

    (
        status,
        Json(RoutingErrorResponse {
            error: code,
            mode: mode.to_string(),
        }),
    )
}
