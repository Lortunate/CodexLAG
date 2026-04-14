use std::sync::{Arc, RwLock};

use axum::Router;

use crate::{
    gateway::{auth::GatewayState, routes::build_routes},
    logging::usage::UsageRecord,
    routing::engine::CandidateEndpoint,
    state::AppState,
};

pub const LOOPBACK_GATEWAY_LISTEN_ADDRESS: &str = "http://127.0.0.1:8787";

#[derive(Clone)]
pub struct LoopbackGateway {
    state: GatewayState,
    router: Router,
}

impl LoopbackGateway {
    pub fn new(
        app_state: Arc<RwLock<AppState>>,
        usage_records: Arc<RwLock<Vec<UsageRecord>>>,
    ) -> Self {
        let candidates = {
            let state = app_state.read().expect("gateway app state lock poisoned");
            build_candidates_from_state(&state)
        };
        Self::new_with_runtime(app_state, usage_records, candidates)
    }

    pub fn new_with_runtime(
        app_state: Arc<RwLock<AppState>>,
        usage_records: Arc<RwLock<Vec<UsageRecord>>>,
        candidates: Vec<CandidateEndpoint>,
    ) -> Self {
        let state = GatewayState::new_with_runtime(app_state, usage_records, candidates);
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
        self.is_ready_for_mode("hybrid")
    }

    pub fn is_ready_for_mode(&self, mode: &str) -> bool {
        self.state.has_available_endpoint_for_mode(mode)
    }
}

pub fn build_router(app_state: AppState) -> Router {
    LoopbackGateway::new(
        Arc::new(RwLock::new(app_state)),
        Arc::new(RwLock::new(Vec::new())),
    )
    .router()
}

pub fn build_router_for_test(app_state: AppState) -> Router {
    build_router(app_state)
}

pub fn build_router_for_test_with_runtime(
    app_state: AppState,
    candidates: Vec<CandidateEndpoint>,
) -> Router {
    LoopbackGateway::new_with_runtime(
        Arc::new(RwLock::new(app_state)),
        Arc::new(RwLock::new(Vec::new())),
        candidates,
    )
    .router()
}

pub fn build_candidates_from_state(app_state: &crate::state::AppState) -> Vec<CandidateEndpoint> {
    crate::routing::candidates::build_runtime_candidates(app_state)
}
