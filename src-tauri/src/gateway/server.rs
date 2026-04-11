use axum::Router;

use crate::gateway::{auth::GatewayAuthState, routes::build_routes};

pub fn build_router_for_test(secret: impl Into<String>) -> Router {
    build_routes().with_state(GatewayAuthState::new(secret))
}
