use axum::Router;

use crate::{
    gateway::{auth::GatewayState, routes::build_routes},
    state::AppState,
};

pub fn build_router(app_state: AppState) -> Router {
    build_routes().with_state(GatewayState::new(app_state))
}

pub fn build_router_for_test(app_state: AppState) -> Router {
    build_router(app_state)
}
