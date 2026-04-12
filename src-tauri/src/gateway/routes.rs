use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

use crate::gateway::auth::{AuthenticatedPlatformKey, GatewayState};
use crate::logging::runtime::{build_attempt_id, format_event_fields};
use crate::logging::usage::UsageRecordInput;
use crate::logging::{log_route_downgrade, log_route_rejection};
use crate::providers::invocation::models_for_endpoint;
use crate::routing::engine::{
    endpoint_downgrade_reason, endpoint_rejection_reason, wall_clock_now_ms, PoolKind, RoutingError,
};
use crate::routing::policy::RoutingMode;

#[derive(Debug, Serialize)]
struct CodexRequestSummary {
    platform_key: String,
    policy: String,
    allowed_mode: String,
    endpoint_id: String,
}

#[derive(Debug, Serialize)]
struct RoutingErrorResponse {
    error: &'static str,
    mode: String,
    request_id: String,
    attempt_count: usize,
}

#[derive(Debug, Serialize)]
struct ModelsResponse {
    platform_key: String,
    policy: String,
    allowed_mode: String,
    models: Vec<String>,
}

pub fn build_routes() -> Router<GatewayState> {
    Router::new()
        .route("/health", get(health))
        .route("/models", get(models))
        .route("/codex/request", post(codex_request))
}

async fn health() -> &'static str {
    "ok"
}

async fn codex_request(
    State(gateway_state): State<GatewayState>,
    auth: AuthenticatedPlatformKey,
) -> Result<Json<CodexRequestSummary>, (StatusCode, Json<RoutingErrorResponse>)> {
    let platform_key = auth.platform_key();
    let now_ms = wall_clock_now_ms();
    let mode_value = platform_key.allowed_mode.clone();
    let mode = mode_value.as_str();
    let request_id = gateway_state.next_request_id(&platform_key.name, now_ms, "unrouted");

    let accepted_line = format_event_fields(&[
        ("event", "gateway.request.accepted"),
        ("request_id", request_id.as_str()),
        ("platform_key", platform_key.name.as_str()),
        ("mode", mode),
    ]);
    log::info!("{accepted_line}");

    let policy = match gateway_state.policy_for_platform_key(platform_key) {
        Some(policy) => policy,
        None => {
            let line = format_event_fields(&[
                ("event", "routing.endpoint.rejected"),
                ("request_id", request_id.as_str()),
                ("attempt_count", "0"),
                ("mode", mode),
                ("error", "policy_missing"),
                ("reasons", "policy_missing"),
            ]);
            log::warn!("{line}");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(RoutingErrorResponse {
                    error: "policy_missing",
                    mode: platform_key.allowed_mode.clone(),
                    request_id: public_request_id(request_id.as_str()),
                    attempt_count: 0,
                }),
            ));
        }
    };
    let selection = gateway_state
        .choose_endpoint_with_runtime_failover(request_id.as_str(), mode, |endpoint, context| {
            gateway_state.invoke_provider(endpoint, context)
        })
        .map_err(|route_error| {
            let candidates = gateway_state.current_candidates();
            log_route_rejection(
                request_id.as_str(),
                route_error.attempt_count,
                mode,
                &route_error.error,
                &candidates,
                now_ms,
            );
            map_routing_error(
                request_id.as_str(),
                route_error.attempt_count,
                mode,
                route_error.error,
            )
        })?;
    let selected = selection.endpoint;
    let candidates = gateway_state.current_candidates();
    let attempt_index = selection.attempt_count.saturating_sub(1);
    let attempt_id = build_attempt_id(request_id.as_str(), attempt_index);

    let selected_line = format_event_fields(&[
        ("event", "routing.endpoint.selected"),
        ("request_id", request_id.as_str()),
        ("attempt_id", attempt_id.as_str()),
        ("endpoint_id", selected.id.as_str()),
    ]);
    log::info!("{selected_line}");

    if should_log_downgrade(mode, &selected, &candidates, now_ms) {
        log_route_downgrade(
            request_id.as_str(),
            attempt_id.as_str(),
            mode,
            &selected,
            &candidates,
            now_ms,
        );
    }

    let endpoint_id = selected.id.clone();
    gateway_state.record_usage_request(UsageRecordInput {
        request_id: request_id.clone(),
        endpoint_id: endpoint_id.clone(),
        input_tokens: 0,
        output_tokens: 0,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        estimated_cost: String::new(),
    });

    Ok(Json(CodexRequestSummary {
        platform_key: platform_key.name.clone(),
        policy: policy.name,
        allowed_mode: platform_key.allowed_mode.clone(),
        endpoint_id,
    }))
}

async fn models(
    State(gateway_state): State<GatewayState>,
    auth: AuthenticatedPlatformKey,
) -> Result<Json<ModelsResponse>, (StatusCode, Json<RoutingErrorResponse>)> {
    let platform_key = auth.platform_key();
    let now_ms = wall_clock_now_ms();
    let mode_value = platform_key.allowed_mode.clone();
    let mode = mode_value.as_str();
    let request_id = gateway_state.next_request_id(&platform_key.name, now_ms, "models");
    let policy = match gateway_state.policy_for_platform_key(platform_key) {
        Some(policy) => policy,
        None => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(RoutingErrorResponse {
                    error: "policy_missing",
                    mode: platform_key.allowed_mode.clone(),
                    request_id: public_request_id(request_id.as_str()),
                    attempt_count: 0,
                }),
            ));
        }
    };
    let candidates = gateway_state.current_candidates();
    let models = allowed_models_for_mode(&candidates, mode, now_ms)
        .map_err(|error| map_routing_error(request_id.as_str(), 0, mode, error))?;

    Ok(Json(ModelsResponse {
        platform_key: platform_key.name.clone(),
        policy: policy.name,
        allowed_mode: platform_key.allowed_mode.clone(),
        models,
    }))
}

fn should_log_downgrade(
    mode: &str,
    selected: &crate::routing::engine::CandidateEndpoint,
    candidates: &[crate::routing::engine::CandidateEndpoint],
    now_ms: u64,
) -> bool {
    if mode != "hybrid" || selected.pool != PoolKind::Relay {
        return false;
    }

    candidates.iter().any(|candidate| {
        candidate.pool == PoolKind::Official
            && candidate.id != selected.id
            && (endpoint_rejection_reason(candidate, now_ms).is_some()
                || endpoint_downgrade_reason(candidate, now_ms).is_some())
    })
}

fn map_routing_error(
    request_id: &str,
    attempt_count: usize,
    mode: &str,
    error: RoutingError,
) -> (StatusCode, Json<RoutingErrorResponse>) {
    let (status, code) = match error {
        RoutingError::InvalidMode => (StatusCode::BAD_REQUEST, "invalid_mode"),
        RoutingError::NoAvailableEndpoint => {
            (StatusCode::SERVICE_UNAVAILABLE, "no_available_endpoint")
        }
    };

    (
        status,
        Json(RoutingErrorResponse {
            error: code,
            mode: mode.to_string(),
            request_id: public_request_id(request_id),
            attempt_count,
        }),
    )
}

fn allowed_models_for_mode(
    candidates: &[crate::routing::engine::CandidateEndpoint],
    mode: &str,
    now_ms: u64,
) -> Result<Vec<String>, RoutingError> {
    let parsed_mode = RoutingMode::parse(mode).ok_or(RoutingError::InvalidMode)?;
    let mut models = BTreeSet::<String>::new();

    for candidate in candidates {
        let pool_allowed = match parsed_mode {
            RoutingMode::AccountOnly => candidate.pool == PoolKind::Official,
            RoutingMode::RelayOnly => candidate.pool == PoolKind::Relay,
            RoutingMode::Hybrid => true,
        };
        if !pool_allowed {
            continue;
        }
        if endpoint_rejection_reason(candidate, now_ms).is_some() {
            continue;
        }
        for model in models_for_endpoint(candidate) {
            models.insert((*model).to_string());
        }
    }

    if models.is_empty() {
        return Err(RoutingError::NoAvailableEndpoint);
    }

    Ok(models.into_iter().collect())
}

fn public_request_id(request_id: &str) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    request_id.hash(&mut hasher);
    format!("req_{:016x}", hasher.finish())
}
