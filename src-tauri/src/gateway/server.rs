use std::sync::{Arc, RwLock};

use axum::Router;

use crate::{
    gateway::{
        auth::GatewayState,
        routes::build_routes,
        runtime_routing::{default_data_plane_executor, DataPlaneExecutor},
    },
    logging::usage::UsageRecord,
    routing::engine::{choose_endpoint, CandidateEndpoint},
    state::AppState,
};

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
        Self::new_with_runtime(
            app_state,
            usage_records,
            default_candidates(),
            default_data_plane_executor(),
        )
    }

    pub fn new_with_runtime(
        app_state: Arc<RwLock<AppState>>,
        usage_records: Arc<RwLock<Vec<UsageRecord>>>,
        candidates: Vec<CandidateEndpoint>,
        executor: DataPlaneExecutor,
    ) -> Self {
        let state = GatewayState::new_with_runtime(app_state, usage_records, candidates, executor);
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
        default_data_plane_executor(),
    )
    .router()
}

pub fn default_candidates() -> Vec<CandidateEndpoint> {
    vec![
        CandidateEndpoint::official("official-default", 10, true),
        CandidateEndpoint::relay("relay-default", 20, true),
    ]
}
