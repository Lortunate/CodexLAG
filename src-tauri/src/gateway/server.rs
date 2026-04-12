use std::sync::{Arc, RwLock};

use axum::Router;

use crate::{
    gateway::{auth::GatewayState, routes::build_routes},
    routing::engine::{CandidateEndpoint, choose_endpoint},
    state::AppState,
};

#[derive(Clone)]
pub struct LoopbackGateway {
    state: GatewayState,
    router: Router,
}

impl LoopbackGateway {
    pub fn new(app_state: Arc<RwLock<AppState>>) -> Self {
        let state = GatewayState::new(app_state);
        let router = build_routes().with_state(state.clone());

        Self { state, router }
    }

    pub fn router(&self) -> Router {
        self.router.clone()
    }

    pub fn state(&self) -> GatewayState {
        self.state.clone()
    }

    pub fn is_ready(&self) -> bool {
        choose_endpoint("hybrid", &default_candidates()).is_ok()
    }
}

pub fn build_router(app_state: AppState) -> Router {
    LoopbackGateway::new(Arc::new(RwLock::new(app_state))).router()
}

pub fn build_router_for_test(app_state: AppState) -> Router {
    build_router(app_state)
}

pub fn default_candidates() -> Vec<CandidateEndpoint> {
    vec![
        CandidateEndpoint::official("official-default", 10, true),
        CandidateEndpoint::relay("relay-default", 20, true),
    ]
}
