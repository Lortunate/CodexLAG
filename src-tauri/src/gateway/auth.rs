use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, RwLock, RwLockReadGuard,
};

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
};

use crate::{
    gateway::{
        runtime_routing::{
            data_plane_executor_from_outcomes, default_data_plane_executor, DataPlaneExecutor,
            RouteDebugSnapshot, RuntimeRoutingState,
        },
        server::default_candidates,
    },
    logging::usage::{append_usage_record, UsageRecord, UsageRecordInput},
    models::{PlatformKey, RoutingPolicy},
    routing::engine::{CandidateEndpoint, FailureRules, RoutingError},
    state::AppState,
};

#[derive(Clone)]
pub struct GatewayState {
    app_state: Arc<RwLock<AppState>>,
    usage_records: Arc<RwLock<Vec<UsageRecord>>>,
    routing: Arc<RwLock<RuntimeRoutingState>>,
    request_sequence: Arc<AtomicU64>,
}

impl GatewayState {
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
        Self {
            app_state,
            usage_records,
            routing: Arc::new(RwLock::new(RuntimeRoutingState::new(
                candidates,
                FailureRules::default(),
                executor,
            ))),
            request_sequence: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn app_state(&self) -> RwLockReadGuard<'_, AppState> {
        self.app_state
            .read()
            .expect("gateway app state lock poisoned")
    }

    pub fn policy_for_platform_key(&self, platform_key: &PlatformKey) -> Option<RoutingPolicy> {
        self.app_state()
            .get_policy_by_id(&platform_key.policy_id)
            .cloned()
    }

    fn authenticate_platform_key(&self, provided_secret: &str) -> Option<PlatformKey> {
        self.app_state().authenticate_platform_key(provided_secret)
    }

    pub fn usage_records(&self) -> Vec<UsageRecord> {
        self.usage_records
            .read()
            .expect("gateway usage records lock poisoned")
            .clone()
    }

    pub fn record_usage_request(&self, input: UsageRecordInput) {
        let mut records = self
            .usage_records
            .write()
            .expect("gateway usage records lock poisoned");
        append_usage_record(&mut records, input);
    }

    pub fn next_request_id(
        &self,
        platform_key_name: &str,
        now_ms: u64,
        endpoint_id: &str,
    ) -> String {
        let sequence = self.request_sequence.fetch_add(1, Ordering::Relaxed);
        format!("{platform_key_name}:{now_ms}:{endpoint_id}:{sequence}")
    }

    pub fn choose_endpoint_with_runtime_failover(
        &self,
        request_id: &str,
        mode: &str,
    ) -> Result<CandidateEndpoint, RoutingError> {
        self.routing
            .write()
            .expect("gateway routing lock poisoned")
            .choose_with_failover(request_id, mode)
    }

    pub fn current_candidates(&self) -> Vec<CandidateEndpoint> {
        self.routing
            .read()
            .expect("gateway routing lock poisoned")
            .candidates_snapshot()
    }

    pub fn set_test_outcomes(&self, outcomes: Vec<(String, Option<u16>)>) {
        self.routing
            .write()
            .expect("gateway routing lock poisoned")
            .set_data_plane_executor(data_plane_executor_from_outcomes(outcomes));
    }

    pub fn last_route_debug(&self) -> Option<RouteDebugSnapshot> {
        self.routing
            .read()
            .expect("gateway routing lock poisoned")
            .last_debug()
            .cloned()
    }
}

pub struct AuthenticatedPlatformKey {
    platform_key: PlatformKey,
}

impl AuthenticatedPlatformKey {
    pub fn platform_key(&self) -> &PlatformKey {
        &self.platform_key
    }
}

impl FromRequestParts<GatewayState> for AuthenticatedPlatformKey {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &GatewayState,
    ) -> std::result::Result<Self, Self::Rejection> {
        let authorization = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let bearer_token = parse_bearer_token(authorization).ok_or(StatusCode::UNAUTHORIZED)?;
        let platform_key = state
            .authenticate_platform_key(bearer_token)
            .ok_or(StatusCode::UNAUTHORIZED)?;

        Ok(Self { platform_key })
    }
}

fn parse_bearer_token(authorization: &str) -> Option<&str> {
    let mut parts = authorization.split_whitespace();
    let scheme = parts.next()?;
    let token = parts.next()?;

    if !scheme.eq_ignore_ascii_case("bearer") || parts.next().is_some() {
        return None;
    }

    Some(token)
}
