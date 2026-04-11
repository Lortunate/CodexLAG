use axum::{
    http::StatusCode,
    routing::{get, post},
    Router,
};

use crate::gateway::auth::{GatewayAuthState, PlatformKeyAuth};

pub fn build_routes() -> Router<GatewayAuthState> {
    Router::new()
        .route("/health", get(health))
        .route("/codex/request", post(codex_request))
}

async fn health() -> &'static str {
    "ok"
}

async fn codex_request(_auth: PlatformKeyAuth) -> StatusCode {
    StatusCode::OK
}
