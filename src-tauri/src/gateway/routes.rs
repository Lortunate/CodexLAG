use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use serde_json::json;
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

use crate::error::{CodexLagError, ConfigErrorKind, ErrorCategory, RoutingErrorKind};
use crate::gateway::auth::{AuthenticatedPlatformKey, GatewayState};
use crate::logging::runtime::{build_attempt_id, format_runtime_event_fields};
use crate::logging::usage::{UsageProvenance, UsageRecordInput};
use crate::logging::{log_route_downgrade, log_route_rejection};
use crate::providers::invocation::{
    models_for_endpoint, InvocationFailure, InvocationFailureClass,
};
use crate::providers::official::map_official_invocation_failure;
use crate::providers::relay::map_relay_invocation_failure;
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
    error: String,
    category: ErrorCategory,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    internal_context: Option<String>,
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

    let accepted_line = format_runtime_event_fields(
        "gateway",
        "gateway.request.accepted",
        request_id.as_str(),
        None,
        None,
        None,
        None,
        &[("platform_key", platform_key.name.as_str()), ("mode", mode)],
    );
    log::info!("{accepted_line}");

    let policy = match gateway_state.policy_for_platform_key(platform_key) {
        Some(policy) => policy,
        None => {
            let line = format_runtime_event_fields(
                "routing",
                "routing.endpoint.rejected",
                request_id.as_str(),
                None,
                None,
                None,
                Some("policy_missing"),
                &[
                    ("attempt_count", "0"),
                    ("mode", mode),
                    ("reasons", "policy_missing"),
                ],
            );
            log::warn!("{line}");
            let (status, error_payload) = map_gateway_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                request_id.as_str(),
                0,
                mode,
                CodexLagError::config(
                    ConfigErrorKind::PolicyMissing,
                    "Selected platform key policy is missing.",
                ),
            );
            let error_code = error_payload.0.error.clone();
            let error_reason = error_payload.0.message.clone();
            gateway_state.record_usage_request(UsageRecordInput {
                request_id: request_id.clone(),
                endpoint_id: "unrouted".to_string(),
                model: None,
                input_tokens: 0,
                output_tokens: 0,
                cache_read_tokens: 0,
                cache_write_tokens: 0,
                reasoning_tokens: 0,
                estimated_cost: String::new(),
                cost_provenance: UsageProvenance::Unknown,
                cost_is_estimated: false,
                pricing_profile_id: None,
                declared_capability_requirements: Some(capability_requirements_snapshot(
                    mode,
                    platform_key.policy_id.as_str(),
                    None,
                )),
                effective_capability_result: Some(capability_result_snapshot(
                    mode,
                    0,
                    None,
                    None,
                    "error",
                    None,
                    Some(error_code.as_str()),
                    Some(error_reason.as_str()),
                )),
                final_upstream_status: None,
                final_upstream_error_code: Some(error_code),
                final_upstream_error_reason: Some(error_reason),
            });
            return Err((status, error_payload));
        }
    };
    let policy_name = policy.name.clone();
    let declared_capability_requirements = Some(capability_requirements_snapshot(
        mode,
        policy.id.as_str(),
        Some(policy_name.as_str()),
    ));
    let selection = match gateway_state.choose_endpoint_with_runtime_failover(
        request_id.as_str(),
        mode,
        |endpoint, context| match endpoint.pool {
            PoolKind::Official => {
                if let Err(failure) = gateway_state.invoke_provider(endpoint, context) {
                    return Err(failure);
                }
                let session =
                    match gateway_state.official_session_for_candidate(endpoint.id.as_str()) {
                        Ok(session) => session,
                        Err(_) => {
                            return Err(InvocationFailure {
                                request_id: context.request_id.clone(),
                                attempt_id: context.attempt_id.clone(),
                                endpoint_id: endpoint.id.clone(),
                                pool: endpoint.pool.clone(),
                                class: InvocationFailureClass::Config,
                                upstream_status: None,
                            });
                        }
                    };
                crate::providers::official::invoke_official_session(
                    &session,
                    context.request_id.as_str(),
                    context.attempt_id.as_str(),
                    endpoint.id.as_str(),
                )
            }
            PoolKind::Relay => gateway_state.invoke_provider(endpoint, context),
        },
    ) {
        Ok(selection) => selection,
        Err(route_error) => {
            let candidates = gateway_state.current_candidates();
            log_route_rejection(
                request_id.as_str(),
                route_error.attempt_count,
                mode,
                &route_error.error,
                &candidates,
                now_ms,
            );
            let crate::gateway::runtime_routing::RouteSelectionError {
                error,
                attempt_count,
                last_invocation_failure,
            } = route_error;
            let failure_endpoint_id = last_invocation_failure
                .as_ref()
                .map(|failure| failure.endpoint_id.as_str());
            let model = failure_endpoint_id
                .and_then(|endpoint_id| model_for_endpoint_id(&candidates, endpoint_id));
            let (status, error_payload) = map_routing_error(
                request_id.as_str(),
                attempt_count,
                mode,
                error,
                last_invocation_failure.as_ref(),
            );
            let error_code = error_payload.0.error.clone();
            let error_reason = error_payload.0.message.clone();
            gateway_state.record_usage_request(UsageRecordInput {
                request_id: request_id.clone(),
                endpoint_id: failure_endpoint_id.unwrap_or("unrouted").to_string(),
                model,
                input_tokens: 0,
                output_tokens: 0,
                cache_read_tokens: 0,
                cache_write_tokens: 0,
                reasoning_tokens: 0,
                estimated_cost: String::new(),
                cost_provenance: UsageProvenance::Unknown,
                cost_is_estimated: false,
                pricing_profile_id: None,
                declared_capability_requirements: declared_capability_requirements.clone(),
                effective_capability_result: Some(capability_result_snapshot(
                    mode,
                    attempt_count,
                    failure_endpoint_id,
                    last_invocation_failure
                        .as_ref()
                        .map(|failure| &failure.pool),
                    "error",
                    None,
                    Some(error_code.as_str()),
                    Some(error_reason.as_str()),
                )),
                final_upstream_status: last_invocation_failure
                    .as_ref()
                    .and_then(|failure| failure.upstream_status),
                final_upstream_error_code: Some(error_code),
                final_upstream_error_reason: Some(error_reason),
            });
            return Err((status, error_payload));
        }
    };
    let selected = selection.endpoint;
    let success_metadata = selection.success_metadata;
    let attempt_count = selection.attempt_count;
    let candidates = gateway_state.current_candidates();
    let attempt_index = attempt_count.saturating_sub(1);
    let attempt_id = build_attempt_id(request_id.as_str(), attempt_index);

    let selected_line = format_runtime_event_fields(
        "routing",
        "routing.endpoint.selected",
        request_id.as_str(),
        Some(attempt_id.as_str()),
        Some(selected.id.as_str()),
        None,
        None,
        &[("mode", mode)],
    );
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
    let usage_dimensions = success_metadata.usage_dimensions;
    let has_usage_dimensions = usage_dimensions
        .as_ref()
        .is_some_and(|dimensions| dimensions.has_non_zero_dimensions());
    let input_tokens = usage_dimensions.map_or(0, |dimensions| dimensions.input_tokens);
    let output_tokens = usage_dimensions.map_or(0, |dimensions| dimensions.output_tokens);
    let cache_read_tokens = usage_dimensions.map_or(0, |dimensions| dimensions.cache_read_tokens);
    let cache_write_tokens = usage_dimensions.map_or(0, |dimensions| dimensions.cache_write_tokens);
    let reasoning_tokens = usage_dimensions.map_or(0, |dimensions| dimensions.reasoning_tokens);
    let model = success_metadata.model.clone();
    let mut estimated_cost = String::new();
    let mut cost_provenance = UsageProvenance::Unknown;
    let mut cost_is_estimated = false;
    let mut pricing_profile_id = None;
    let mut pricing_estimation_status = "not_attempted";

    if let Some(model_name) = model.as_deref() {
        let at_ms = i64::try_from(now_ms).unwrap_or(i64::MAX);
        let app_state = gateway_state.app_state();
        if has_usage_dimensions {
            match app_state.estimate_usage_cost_for_model_at(
                model_name,
                at_ms,
                input_tokens,
                output_tokens,
                cache_read_tokens,
                cache_write_tokens,
                reasoning_tokens,
            ) {
                Ok(Some(estimate)) => {
                    estimated_cost = estimate.amount;
                    cost_provenance = estimate.provenance;
                    cost_is_estimated = estimate.estimated;
                    pricing_profile_id = Some(estimate.pricing_profile_id);
                    pricing_estimation_status = "estimated";
                }
                Ok(None) => {
                    pricing_estimation_status = "pricing_profile_missing";
                }
                Err(error) => {
                    pricing_estimation_status = "lookup_failed";
                    let cause = error.to_string();
                    log::warn!(
                        "{}",
                        format_runtime_event_fields(
                            "gateway",
                            "gateway.usage.cost_profile_lookup_failed",
                            request_id.as_str(),
                            Some(attempt_id.as_str()),
                            Some(endpoint_id.as_str()),
                            None,
                            None,
                            &[("cause", cause.as_str())]
                        )
                    );
                }
            }
        } else {
            match app_state.active_pricing_profile_id_for_model_at(model_name, at_ms) {
                Ok(profile_id) => {
                    pricing_profile_id = profile_id;
                    pricing_estimation_status = if pricing_profile_id.is_some() {
                        "deferred_missing_dimensions"
                    } else {
                        "pricing_profile_missing"
                    };
                }
                Err(error) => {
                    pricing_estimation_status = "lookup_failed";
                    let cause = error.to_string();
                    log::warn!(
                        "{}",
                        format_runtime_event_fields(
                            "gateway",
                            "gateway.usage.cost_profile_lookup_failed",
                            request_id.as_str(),
                            Some(attempt_id.as_str()),
                            Some(endpoint_id.as_str()),
                            None,
                            None,
                            &[("cause", cause.as_str())]
                        )
                    );
                }
            }
        }
    }

    gateway_state.record_usage_request(UsageRecordInput {
        request_id: request_id.clone(),
        endpoint_id: endpoint_id.clone(),
        model,
        input_tokens,
        output_tokens,
        cache_read_tokens,
        cache_write_tokens,
        reasoning_tokens,
        estimated_cost,
        cost_provenance,
        cost_is_estimated,
        pricing_profile_id,
        declared_capability_requirements: declared_capability_requirements.clone(),
        effective_capability_result: Some(capability_result_snapshot(
            mode,
            attempt_count,
            Some(endpoint_id.as_str()),
            Some(&selected.pool),
            "success",
            Some(pricing_estimation_status),
            None,
            None,
        )),
        final_upstream_status: Some(success_metadata.upstream_status),
        final_upstream_error_code: None,
        final_upstream_error_reason: None,
    });

    Ok(Json(CodexRequestSummary {
        platform_key: platform_key.name.clone(),
        policy: policy_name,
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
            return Err(map_gateway_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                request_id.as_str(),
                0,
                mode,
                CodexLagError::config(
                    ConfigErrorKind::PolicyMissing,
                    "Selected platform key policy is missing.",
                ),
            ));
        }
    };
    let candidates = gateway_state.current_candidates();
    let models = allowed_models_for_mode(&candidates, mode, now_ms)
        .map_err(|error| map_routing_error(request_id.as_str(), 0, mode, error, None))?;

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

fn capability_requirements_snapshot(
    mode: &str,
    policy_id: &str,
    policy_name: Option<&str>,
) -> String {
    json!({
        "routing_mode": mode,
        "policy_id": policy_id,
        "policy_name": policy_name,
    })
    .to_string()
}

#[allow(clippy::too_many_arguments)]
fn capability_result_snapshot(
    mode: &str,
    attempt_count: usize,
    endpoint_id: Option<&str>,
    pool: Option<&PoolKind>,
    outcome: &str,
    pricing_estimation: Option<&str>,
    error_code: Option<&str>,
    error_reason: Option<&str>,
) -> String {
    json!({
        "routing_mode": mode,
        "attempt_count": attempt_count,
        "selected_endpoint_id": endpoint_id,
        "selected_pool": pool.map(pool_kind_label),
        "outcome": outcome,
        "pricing_estimation": pricing_estimation,
        "error_code": error_code,
        "error_reason": error_reason,
    })
    .to_string()
}

fn pool_kind_label(pool: &PoolKind) -> &'static str {
    match pool {
        PoolKind::Official => "official",
        PoolKind::Relay => "relay",
    }
}

fn model_for_endpoint_id(
    candidates: &[crate::routing::engine::CandidateEndpoint],
    endpoint_id: &str,
) -> Option<String> {
    candidates
        .iter()
        .find(|candidate| candidate.id == endpoint_id)
        .and_then(|candidate| models_for_endpoint(candidate).first())
        .map(|model| (*model).to_string())
}

fn map_routing_error(
    request_id: &str,
    attempt_count: usize,
    mode: &str,
    error: RoutingError,
    last_invocation_failure: Option<&InvocationFailure>,
) -> (StatusCode, Json<RoutingErrorResponse>) {
    if let Some(failure) = last_invocation_failure {
        let status = map_provider_failure_status(&failure.class);
        let mapped = map_provider_failure(failure);
        return map_gateway_error(status, request_id, attempt_count, mode, mapped);
    }

    let (status, mapped) = match error {
        RoutingError::InvalidMode => (
            StatusCode::BAD_REQUEST,
            CodexLagError::routing(
                RoutingErrorKind::InvalidMode,
                "The platform key mode is not supported by routing policy.",
            ),
        ),
        RoutingError::NoAvailableEndpoint => (
            StatusCode::SERVICE_UNAVAILABLE,
            CodexLagError::routing(
                RoutingErrorKind::NoAvailableEndpoint,
                "No available endpoint matched the current routing constraints.",
            ),
        ),
    };

    map_gateway_error(status, request_id, attempt_count, mode, mapped)
}

fn map_provider_failure(failure: &InvocationFailure) -> CodexLagError {
    match failure.pool {
        PoolKind::Official => map_official_invocation_failure(failure),
        PoolKind::Relay => map_relay_invocation_failure(failure),
    }
}

fn map_provider_failure_status(class: &InvocationFailureClass) -> StatusCode {
    match class {
        InvocationFailureClass::Auth => StatusCode::UNAUTHORIZED,
        InvocationFailureClass::Http429 => StatusCode::TOO_MANY_REQUESTS,
        InvocationFailureClass::Http5xx | InvocationFailureClass::Timeout => {
            StatusCode::BAD_GATEWAY
        }
        InvocationFailureClass::Config => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn map_gateway_error(
    status: StatusCode,
    request_id: &str,
    attempt_count: usize,
    mode: &str,
    error: CodexLagError,
) -> (StatusCode, Json<RoutingErrorResponse>) {
    let public_request_id = public_request_id(request_id);
    let route_context =
        format!("mode={mode};request_id={public_request_id};attempt_count={attempt_count}");
    let internal_context = match error.internal_context() {
        Some(provider_context) => Some(format!("{route_context};{provider_context}")),
        None => Some(route_context),
    };

    (
        status,
        Json(RoutingErrorResponse {
            error: error.code().to_string(),
            category: error.category(),
            message: error.message().to_string(),
            internal_context,
            mode: mode.to_string(),
            request_id: public_request_id,
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
