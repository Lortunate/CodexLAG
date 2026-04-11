use axum::{
    routing::{get, post},
    Router,
};

use crate::gateway::auth::{AuthenticatedPlatformKey, GatewayState};

pub fn build_routes() -> Router<GatewayState> {
    Router::new()
        .route("/health", get(health))
        .route("/codex/request", post(codex_request))
}

async fn health() -> &'static str {
    "ok"
}

async fn codex_request(auth: AuthenticatedPlatformKey) -> String {
    auth.platform_key().name.clone()
}
