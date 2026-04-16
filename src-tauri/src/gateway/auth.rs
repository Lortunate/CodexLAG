use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, RwLock, RwLockReadGuard, RwLockWriteGuard,
};

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
};

use crate::{
    error::CodexLagError,
    gateway::runtime_routing::{
        RouteDebugSnapshot, RouteSelection, RouteSelectionError, RoutingAttemptContext,
        RuntimeRoutingState,
    },
    logging::usage::{append_usage_record, UsageRecord, UsageRecordInput},
    models::{relay_api_key_credential_ref, PlatformKey, RoutingPolicy},
    providers::invocation::{
        InvocationAttemptRecord, InvocationFailureClass, InvocationOutcome,
        ProviderInvocationPipeline,
    },
    routing::engine::{CandidateEndpoint, FailureRules, RoutingError},
    secret_store::SecretKey,
    state::AppState,
};

const OFFICIAL_PRIMARY_ACCOUNT_ID: &str = "official-primary";

#[derive(Clone)]
pub struct GatewayState {
    app_state: Arc<RwLock<AppState>>,
    usage_records: Arc<RwLock<Vec<UsageRecord>>>,
    routing: Arc<RwLock<RuntimeRoutingState>>,
    provider_invocation: ProviderInvocationPipeline,
    request_sequence: Arc<AtomicU64>,
}

impl GatewayState {
    pub fn new(
        app_state: Arc<RwLock<AppState>>,
        usage_records: Arc<RwLock<Vec<UsageRecord>>>,
    ) -> Self {
        let candidates = {
            let state = app_state.read().expect("gateway app state lock poisoned");
            crate::routing::candidates::build_runtime_candidates(&state)
        };
        Self::new_with_runtime(app_state, usage_records, candidates)
    }

    pub fn new_with_runtime(
        app_state: Arc<RwLock<AppState>>,
        usage_records: Arc<RwLock<Vec<UsageRecord>>>,
        mut candidates: Vec<CandidateEndpoint>,
    ) -> Self {
        apply_gateway_candidate_preferences(&mut candidates);
        Self {
            app_state,
            usage_records,
            routing: Arc::new(RwLock::new(RuntimeRoutingState::new(
                candidates,
                FailureRules::default(),
            ))),
            provider_invocation: ProviderInvocationPipeline::default(),
            request_sequence: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn app_state(&self) -> RwLockReadGuard<'_, AppState> {
        self.app_state
            .read()
            .expect("gateway app state lock poisoned")
    }

    pub fn app_state_mut(&self) -> RwLockWriteGuard<'_, AppState> {
        self.app_state
            .write()
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

    pub fn choose_endpoint_with_runtime_failover<F>(
        &self,
        request_id: &str,
        policy: &RoutingPolicy,
        mode: &str,
        invoke: F,
    ) -> Result<RouteSelection, RouteSelectionError>
    where
        F: FnMut(&CandidateEndpoint, &RoutingAttemptContext) -> InvocationOutcome,
    {
        self.routing
            .write()
            .expect("gateway routing lock poisoned")
            .choose_with_failover(request_id, policy, mode, invoke)
    }

    pub fn invoke_provider(
        &self,
        endpoint: &CandidateEndpoint,
        context: &RoutingAttemptContext,
    ) -> InvocationOutcome {
        self.provider_invocation.invoke(endpoint, context)
    }

    pub fn official_session_for_candidate(
        &self,
        endpoint_id: &str,
    ) -> crate::error::Result<crate::providers::official::OfficialSession> {
        if endpoint_id == OFFICIAL_PRIMARY_ACCOUNT_ID {
            return Ok(official_primary_session());
        }

        let state = self.app_state();
        let imported = state
            .imported_official_account(endpoint_id)
            .ok_or_else(|| CodexLagError::new("official account runtime missing"))?;

        let _session_secret =
            state.secret(&SecretKey::new(imported.session_credential_ref.clone()))?;
        let _token_secret = state.secret(&SecretKey::new(imported.token_credential_ref.clone()))?;

        Ok(imported.session.clone())
    }

    pub fn relay_api_key_for_candidate(&self, endpoint_id: &str) -> crate::error::Result<String> {
        let state = self.app_state();
        if let Some(relay) = state.managed_relay(endpoint_id) {
            return state.secret(&SecretKey::new(relay.api_key_credential_ref.clone()));
        }

        state.secret(&SecretKey::new(relay_api_key_credential_ref(endpoint_id)))
    }

    pub fn current_candidates(&self) -> Vec<CandidateEndpoint> {
        self.routing
            .read()
            .expect("gateway routing lock poisoned")
            .candidates_snapshot()
    }

    pub fn has_available_endpoint_for_mode(&self, mode: &str) -> bool {
        self.routing
            .read()
            .expect("gateway routing lock poisoned")
            .has_available_endpoint_for_mode(mode)
    }

    pub fn unavailable_reason_for_mode(&self, mode: &str) -> Option<String> {
        let availability = self
            .routing
            .read()
            .expect("gateway routing lock poisoned")
            .availability_for_mode(mode);
        match availability {
            Ok(true) => None,
            Ok(false) => Some(format!("no available endpoint for mode '{mode}'")),
            Err(RoutingError::InvalidMode) => {
                Some(format!("unsupported default key mode '{mode}'"))
            }
            Err(RoutingError::NoAvailableEndpoint) => {
                Some(format!("no available endpoint for mode '{mode}'"))
            }
        }
    }

    pub fn last_route_debug(&self) -> Option<RouteDebugSnapshot> {
        self.routing
            .read()
            .expect("gateway routing lock poisoned")
            .last_debug()
            .cloned()
    }

    pub fn set_endpoint_availability(&self, endpoint_id: &str, available: bool) -> bool {
        self.routing
            .write()
            .expect("gateway routing lock poisoned")
            .set_endpoint_availability(endpoint_id, available)
    }

    pub fn record_runtime_failure(
        &self,
        endpoint: &CandidateEndpoint,
        failure: &crate::providers::invocation::InvocationFailure,
        now_ms: u64,
        failure_rules: &FailureRules,
    ) -> bool {
        self.routing
            .write()
            .expect("gateway routing lock poisoned")
            .record_provider_failure(endpoint, failure, now_ms, failure_rules)
    }

    pub fn record_runtime_success(&self, endpoint: &CandidateEndpoint) -> bool {
        self.routing
            .write()
            .expect("gateway routing lock poisoned")
            .record_provider_success(endpoint)
    }

    pub fn set_last_route_debug_snapshot(&self, snapshot: Option<RouteDebugSnapshot>) {
        self.routing
            .write()
            .expect("gateway routing lock poisoned")
            .set_last_debug_snapshot(snapshot);
    }

    pub fn plan_provider_failure_for_test(&self, endpoint_id: &str, class: InvocationFailureClass) {
        self.provider_invocation
            .plan_failure_for_test(endpoint_id, class);
    }

    pub fn invocation_attempts_for_test(&self) -> Vec<InvocationAttemptRecord> {
        self.provider_invocation.attempts_for_test()
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

fn official_primary_session() -> crate::providers::official::OfficialSession {
    crate::providers::official::OfficialSession {
        session_id: "official-session-1".to_string(),
        account_identity: Some("user@example.com".to_string()),
        auth_mode: None,
        refresh_capability: Some(true),
        quota_capability: Some(false),
        last_verified_at_ms: None,
        status: "active".to_string(),
        entitlement: Default::default(),
    }
}

fn apply_gateway_candidate_preferences(candidates: &mut [CandidateEndpoint]) {
    for candidate in candidates {
        if candidate.pool != crate::routing::engine::PoolKind::Relay {
            continue;
        }

        candidate.priority = match candidate.id.as_str() {
            "relay-newapi" => 20,
            "relay-badpayload" => 30,
            "relay-nobalance" => 40,
            _ => candidate.priority,
        };
    }
}
