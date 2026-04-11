use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;

use crate::gateway::auth::{AuthenticatedPlatformKey, GatewayState};

#[derive(Debug, Serialize)]
struct CodexRequestSummary {
    platform_key: String,
    policy: String,
    allowed_mode: String,
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
) -> Result<Json<CodexRequestSummary>, StatusCode> {
    let platform_key = auth.platform_key();
    let policy = gateway_state
        .policy_for_platform_key(platform_key)
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(CodexRequestSummary {
        platform_key: platform_key.name.clone(),
        policy: policy.name,
        allowed_mode: platform_key.allowed_mode.clone(),
    }))
}
