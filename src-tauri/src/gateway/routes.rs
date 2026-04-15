use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use serde_json::json;
use std::collections::{BTreeSet, HashSet};
use std::hash::{Hash, Hasher};

use crate::error::{CodexLagError, ConfigErrorKind, ErrorCategory, RoutingErrorKind};
use crate::gateway::auth::{AuthenticatedPlatformKey, GatewayState};
use crate::logging::runtime::{build_attempt_id, format_runtime_event_fields};
use crate::logging::usage::{UsageProvenance, UsageRecordInput};
use crate::logging::{log_route_downgrade, log_route_rejection};
use crate::models::{RequestAttemptLog, RequestLog};
use crate::providers::invocation::{
    models_for_endpoint, InvocationFailure, InvocationFailureClass,
};
use crate::providers::official::map_official_invocation_failure;
use crate::providers::relay::map_relay_invocation_failure;
use crate::routing::engine::{
    choose_endpoint_at_with_recovery, endpoint_downgrade_reason, endpoint_rejection_reason,
    wall_clock_now_ms, PoolKind, RoutingError,
};
use crate::routing::policy::apply_selection_order;
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

#[derive(Debug, Clone)]
struct AttemptLifecycleSnapshot {
    attempt_id: String,
    request_id: String,
    attempt_index: usize,
    endpoint_id: String,
    pool: PoolKind,
    trigger_reason: String,
    upstream_status: Option<u16>,
    latency_ms: i64,
    token_usage_snapshot: Option<String>,
    estimated_cost_snapshot: Option<String>,
    feature_resolution_snapshot: Option<String>,
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
    let request_started_at_ms = wall_clock_now_ms();
    let mode_value = platform_key.allowed_mode.clone();
    let mode = mode_value.as_str();
    let request_id =
        gateway_state.next_request_id(&platform_key.name, request_started_at_ms, "unrouted");

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
            let capability_result = capability_result_snapshot(
                mode,
                0,
                None,
                None,
                "error",
                None,
                Some(error_code.as_str()),
                Some(error_reason.as_str()),
            );
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
                effective_capability_result: Some(capability_result.clone()),
                final_upstream_status: None,
                final_upstream_error_code: Some(error_code.clone()),
                final_upstream_error_reason: Some(error_reason.clone()),
            });
            let request_finished_at_ms = wall_clock_now_ms();
            persist_request_lifecycle_snapshot(
                &gateway_state,
                request_id.as_str(),
                platform_key.id.as_str(),
                None,
                None,
                "error",
                Some(StatusCode::INTERNAL_SERVER_ERROR.as_u16()),
                request_started_at_ms,
                request_finished_at_ms,
                Some(error_code.as_str()),
                Some(error_reason.as_str()),
                vec![].as_slice(),
            );
            return Err((status, error_payload));
        }
    };
    let policy_name = policy.name.clone();
    let declared_capability_requirements = Some(capability_requirements_snapshot(
        mode,
        policy.id.as_str(),
        Some(policy_name.as_str()),
    ));
    let max_attempts = {
        let candidate_count = gateway_state.current_candidates().len().max(1);
        if policy.retry_budget == 0 {
            candidate_count
        } else {
            usize::min(policy.retry_budget as usize, candidate_count).max(1)
        }
    };
    let mut attempt_count = 0usize;
    let mut attempted_endpoint_keys = HashSet::<(PoolKind, String)>::new();
    let mut last_selected_endpoint: Option<String> = None;
    let mut last_invocation_failure: Option<InvocationFailure> = None;
    let mut primary_pool: Option<PoolKind> = None;
    let mut attempt_snapshots = Vec::<AttemptLifecycleSnapshot>::new();
    let mut selected_endpoint: Option<crate::routing::engine::CandidateEndpoint> = None;
    let mut success_metadata: Option<crate::providers::invocation::InvocationSuccessMetadata> =
        None;
    let mut terminal_routing_error = RoutingError::NoAvailableEndpoint;

    while attempt_count < max_attempts {
        let now_ms = wall_clock_now_ms();
        let mut ordered =
            apply_selection_order(&gateway_state.current_candidates(), &policy.selection_order);
        for candidate in &mut ordered {
            if let Some(position) = policy
                .selection_order
                .iter()
                .position(|endpoint_id| endpoint_id == &candidate.id)
            {
                candidate.priority = i32::try_from(position).unwrap_or(i32::MAX);
            }
        }
        let selected = match choose_endpoint_at_with_recovery(
            mode,
            &ordered,
            now_ms,
            &policy.recovery_rules,
        ) {
            Ok(candidate) => candidate,
            Err(error) => {
                terminal_routing_error = error;
                break;
            }
        };
        if let Some(pool) = primary_pool.as_ref() {
            if !policy.cross_pool_fallback && selected.pool != *pool {
                terminal_routing_error = RoutingError::NoAvailableEndpoint;
                break;
            }
        } else {
            primary_pool = Some(selected.pool.clone());
        }

        let selected_key = (selected.pool.clone(), selected.id.clone());
        if attempted_endpoint_keys.contains(&selected_key) {
            terminal_routing_error = RoutingError::NoAvailableEndpoint;
            break;
        }

        attempt_count = attempt_count.saturating_add(1);
        attempted_endpoint_keys.insert(selected_key);
        last_selected_endpoint = Some(selected.id.clone());
        let attempt_index = attempt_count.saturating_sub(1);
        let context = crate::gateway::runtime_routing::RoutingAttemptContext {
            request_id: request_id.clone(),
            attempt_id: build_attempt_id(request_id.as_str(), attempt_index),
            attempt_index,
            mode: mode.to_string(),
        };
        let attempt_started_at_ms = wall_clock_now_ms();
        let invocation = match gateway_state.invoke_provider(&selected, &context) {
            Err(failure) => Err(failure),
            Ok(_) => match selected.pool {
                PoolKind::Official => {
                    let imported = {
                        let state = gateway_state.app_state();
                        state
                            .imported_official_account(selected.id.as_str())
                            .cloned()
                    };

                    match imported {
                        Some(imported) => {
                            if matches!(
                                imported.provider.as_str(),
                                crate::providers::generic_openai::GENERIC_OPENAI_PROVIDER_ID
                                    | "generic_openai"
                            ) {
                                let token_secret = {
                                    let state = gateway_state.app_state();
                                    state.secret(&crate::secret_store::SecretKey::new(
                                        imported.token_credential_ref.clone(),
                                    ))
                                };
                                match token_secret {
                                    Ok(token_secret) => {
                                        crate::providers::generic_openai::invoke_generic_openai(
                                            token_secret.as_str(),
                                            context.request_id.as_str(),
                                            context.attempt_id.as_str(),
                                            selected.id.as_str(),
                                        )
                                        .await
                                    }
                                    Err(_) => Err(InvocationFailure {
                                        request_id: context.request_id.clone(),
                                        attempt_id: context.attempt_id.clone(),
                                        endpoint_id: selected.id.clone(),
                                        pool: selected.pool.clone(),
                                        class: InvocationFailureClass::Config,
                                        upstream_status: None,
                                    }),
                                }
                            } else {
                                let secrets: Result<(String, String), InvocationFailure> = {
                                    let state = gateway_state.app_state();
                                    match (
                                        state.secret(&crate::secret_store::SecretKey::new(
                                            imported.session_credential_ref.clone(),
                                        )),
                                        state.secret(&crate::secret_store::SecretKey::new(
                                            imported.token_credential_ref.clone(),
                                        )),
                                    ) {
                                        (Ok(session_secret), Ok(token_secret)) => {
                                            Ok((session_secret, token_secret))
                                        }
                                        _ => Err(InvocationFailure {
                                            request_id: context.request_id.clone(),
                                            attempt_id: context.attempt_id.clone(),
                                            endpoint_id: selected.id.clone(),
                                            pool: selected.pool.clone(),
                                            class: InvocationFailureClass::Config,
                                            upstream_status: None,
                                        }),
                                    }
                                };
                                match secrets {
                                    Ok((session_secret, token_secret)) => {
                                        crate::providers::official::invoke_official_session(
                                            &imported.session,
                                            session_secret.as_str(),
                                            token_secret.as_str(),
                                            context.request_id.as_str(),
                                            context.attempt_id.as_str(),
                                            selected.id.as_str(),
                                        )
                                        .await
                                    }
                                    Err(failure) => Err(failure),
                                }
                            }
                        }
                        None => Err(InvocationFailure {
                            request_id: context.request_id.clone(),
                            attempt_id: context.attempt_id.clone(),
                            endpoint_id: selected.id.clone(),
                            pool: selected.pool.clone(),
                            class: InvocationFailureClass::Config,
                            upstream_status: None,
                        }),
                    }
                }
                PoolKind::Relay => {
                    let relay = {
                        let state = gateway_state.app_state();
                        state.managed_relay(selected.id.as_str()).cloned()
                    };

                    match relay {
                        Some(relay) => {
                            let api_key = {
                                let state = gateway_state.app_state();
                                match state.secret(&crate::secret_store::SecretKey::new(
                                    relay.api_key_credential_ref.clone(),
                                )) {
                                    Ok(api_key) => Ok(api_key),
                                    Err(_) => Err(InvocationFailure {
                                        request_id: context.request_id.clone(),
                                        attempt_id: context.attempt_id.clone(),
                                        endpoint_id: selected.id.clone(),
                                        pool: selected.pool.clone(),
                                        class: InvocationFailureClass::Config,
                                        upstream_status: None,
                                    }),
                                }
                            };
                            match api_key {
                                Ok(api_key) => {
                                    crate::providers::relay::invoke_newapi_relay(
                                        relay.endpoint.as_str(),
                                        api_key.as_str(),
                                        context.request_id.as_str(),
                                        context.attempt_id.as_str(),
                                        selected.id.as_str(),
                                    )
                                    .await
                                }
                                Err(failure) => Err(failure),
                            }
                        }
                        None => Err(InvocationFailure {
                            request_id: context.request_id.clone(),
                            attempt_id: context.attempt_id.clone(),
                            endpoint_id: selected.id.clone(),
                            pool: selected.pool.clone(),
                            class: InvocationFailureClass::Config,
                            upstream_status: None,
                        }),
                    }
                }
            },
        };
        let attempt_finished_at_ms = wall_clock_now_ms();
        let attempt_latency_ms = non_zero_latency_ms(attempt_started_at_ms, attempt_finished_at_ms);
        let trigger_reason = if attempt_index == 0 {
            "primary".to_string()
        } else {
            "fallback".to_string()
        };

        match invocation {
            Ok(metadata) => {
                let _ = gateway_state.record_runtime_success(&selected);
                gateway_state.set_last_route_debug_snapshot(Some(
                    crate::gateway::runtime_routing::RouteDebugSnapshot {
                        request_id: request_id.clone(),
                        selected_endpoint_id: selected.id.clone(),
                        attempt_count,
                    },
                ));
                attempt_snapshots.push(AttemptLifecycleSnapshot {
                    attempt_id: context.attempt_id,
                    request_id: context.request_id,
                    attempt_index,
                    endpoint_id: selected.id.clone(),
                    pool: selected.pool.clone(),
                    trigger_reason,
                    upstream_status: Some(metadata.upstream_status),
                    latency_ms: attempt_latency_ms,
                    token_usage_snapshot: metadata.usage_dimensions.as_ref().map(|dimensions| {
                        json!({
                            "input_tokens": dimensions.input_tokens,
                            "output_tokens": dimensions.output_tokens,
                            "cache_read_tokens": dimensions.cache_read_tokens,
                            "cache_write_tokens": dimensions.cache_write_tokens,
                            "reasoning_tokens": dimensions.reasoning_tokens,
                        })
                        .to_string()
                    }),
                    estimated_cost_snapshot: None,
                    feature_resolution_snapshot: None,
                });
                selected_endpoint = Some(selected);
                success_metadata = Some(metadata);
                break;
            }
            Err(failure) => {
                last_invocation_failure = Some(failure.clone());
                let _ = gateway_state.record_runtime_failure(
                    &selected,
                    &failure,
                    now_ms,
                    &policy.failure_rules,
                );
                let mapped_error = map_provider_failure(&failure);
                attempt_snapshots.push(AttemptLifecycleSnapshot {
                    attempt_id: context.attempt_id,
                    request_id: context.request_id,
                    attempt_index,
                    endpoint_id: selected.id.clone(),
                    pool: selected.pool.clone(),
                    trigger_reason,
                    upstream_status: failure.upstream_status,
                    latency_ms: attempt_latency_ms,
                    token_usage_snapshot: None,
                    estimated_cost_snapshot: None,
                    feature_resolution_snapshot: Some(capability_result_snapshot(
                        mode,
                        attempt_count,
                        Some(selected.id.as_str()),
                        Some(&selected.pool),
                        "error",
                        None,
                        Some(mapped_error.code()),
                        Some(mapped_error.message()),
                    )),
                });
            }
        }
    }

    let selected = match selected_endpoint {
        Some(selected) => selected,
        None => {
            if let Some(endpoint_id) = last_selected_endpoint {
                gateway_state.set_last_route_debug_snapshot(Some(
                    crate::gateway::runtime_routing::RouteDebugSnapshot {
                        request_id: request_id.clone(),
                        selected_endpoint_id: endpoint_id,
                        attempt_count,
                    },
                ));
            }
            let candidates = gateway_state.current_candidates();
            log_route_rejection(
                request_id.as_str(),
                attempt_count,
                mode,
                &terminal_routing_error,
                &candidates,
                wall_clock_now_ms(),
            );
            let failure_endpoint_id = last_invocation_failure
                .as_ref()
                .map(|failure| failure.endpoint_id.as_str());
            let model = failure_endpoint_id
                .and_then(|endpoint_id| model_for_endpoint_id(&candidates, endpoint_id));
            let (status, error_payload) = map_routing_error(
                request_id.as_str(),
                attempt_count,
                mode,
                terminal_routing_error,
                last_invocation_failure.as_ref(),
            );
            let error_code = error_payload.0.error.clone();
            let error_reason = error_payload.0.message.clone();
            gateway_state.record_usage_request(UsageRecordInput {
                request_id: request_id.clone(),
                endpoint_id: failure_endpoint_id.unwrap_or("unrouted").to_string(),
                model: model.clone(),
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
                final_upstream_error_code: Some(error_code.clone()),
                final_upstream_error_reason: Some(error_reason.clone()),
            });
            let request_finished_at_ms = wall_clock_now_ms();
            persist_request_lifecycle_snapshot(
                &gateway_state,
                request_id.as_str(),
                platform_key.id.as_str(),
                model.as_deref(),
                failure_endpoint_id,
                "error",
                Some(status.as_u16()),
                request_started_at_ms,
                request_finished_at_ms,
                Some(error_code.as_str()),
                Some(error_reason.as_str()),
                attempt_snapshots.as_slice(),
            );
            return Err((status, error_payload));
        }
    };
    let success_metadata =
        success_metadata.expect("success metadata must exist for selected endpoint");
    let candidates = gateway_state.current_candidates();
    let attempt_index = attempt_count.saturating_sub(1);
    let attempt_id = build_attempt_id(request_id.as_str(), attempt_index);
    let attempt_count_value = attempt_count.to_string();
    let now_ms = wall_clock_now_ms();

    let selected_line = format_runtime_event_fields(
        "routing",
        "routing.endpoint.selected",
        request_id.as_str(),
        Some(attempt_id.as_str()),
        Some(selected.id.as_str()),
        None,
        None,
        &[
            ("mode", mode),
            ("policy_id", policy.id.as_str()),
            ("attempt_count", attempt_count_value.as_str()),
        ],
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
        model: model.clone(),
        input_tokens,
        output_tokens,
        cache_read_tokens,
        cache_write_tokens,
        reasoning_tokens,
        estimated_cost: estimated_cost.clone(),
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
    let capability_result = capability_result_snapshot(
        mode,
        attempt_count,
        Some(endpoint_id.as_str()),
        Some(&selected.pool),
        "success",
        Some(pricing_estimation_status),
        None,
        None,
    );
    if let Some(final_attempt) = attempt_snapshots.last_mut() {
        final_attempt.estimated_cost_snapshot = Some(json!({"amount": estimated_cost}).to_string());
        final_attempt.feature_resolution_snapshot = Some(capability_result.clone());
    }
    let request_finished_at_ms = wall_clock_now_ms();
    persist_request_lifecycle_snapshot(
        &gateway_state,
        request_id.as_str(),
        platform_key.id.as_str(),
        model.as_deref(),
        Some(endpoint_id.as_str()),
        "success",
        Some(StatusCode::OK.as_u16()),
        request_started_at_ms,
        request_finished_at_ms,
        None,
        None,
        attempt_snapshots.as_slice(),
    );

    Ok(Json(CodexRequestSummary {
        platform_key: platform_key.name.clone(),
        policy: policy_name,
        allowed_mode: platform_key.allowed_mode.clone(),
        endpoint_id,
    }))
}

fn non_zero_latency_ms(started_at_ms: u64, finished_at_ms: u64) -> i64 {
    let elapsed = finished_at_ms.saturating_sub(started_at_ms).max(1);
    i64::try_from(elapsed).unwrap_or(i64::MAX)
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

#[allow(clippy::too_many_arguments)]
fn persist_request_lifecycle_snapshot(
    gateway_state: &GatewayState,
    request_id: &str,
    platform_key_id: &str,
    model: Option<&str>,
    selected_endpoint_id: Option<&str>,
    final_status: &str,
    http_status: Option<u16>,
    started_at_ms: u64,
    finished_at_ms: u64,
    error_code: Option<&str>,
    error_reason: Option<&str>,
    attempts: &[AttemptLifecycleSnapshot],
) {
    let started_at_ms = i64::try_from(started_at_ms).unwrap_or(i64::MAX);
    let finished_at_ms = i64::try_from(finished_at_ms).unwrap_or(i64::MAX);
    let latency_ms = Some(non_zero_latency_ms(
        u64::try_from(started_at_ms).unwrap_or_default(),
        u64::try_from(finished_at_ms).unwrap_or_default(),
    ));

    let attempts = attempts
        .iter()
        .map(|attempt| RequestAttemptLog {
            attempt_id: attempt.attempt_id.clone(),
            request_id: attempt.request_id.clone(),
            attempt_index: u32::try_from(attempt.attempt_index).unwrap_or(u32::MAX),
            endpoint_id: attempt.endpoint_id.clone(),
            pool_type: pool_kind_label(&attempt.pool).to_string(),
            trigger_reason: attempt.trigger_reason.clone(),
            upstream_status: attempt.upstream_status.map(i64::from),
            timeout_ms: None,
            latency_ms: Some(attempt.latency_ms),
            token_usage_snapshot: attempt.token_usage_snapshot.clone(),
            estimated_cost_snapshot: attempt.estimated_cost_snapshot.clone(),
            balance_snapshot_id: None,
            feature_resolution_snapshot: attempt.feature_resolution_snapshot.clone(),
        })
        .collect::<Vec<_>>();

    let request = RequestLog {
        request_id: request_id.to_string(),
        platform_key_id: platform_key_id.to_string(),
        request_type: "codex".to_string(),
        model: model.unwrap_or_default().to_string(),
        selected_endpoint_id: selected_endpoint_id.map(str::to_string),
        attempt_count: u32::try_from(attempts.len()).unwrap_or(u32::MAX),
        final_status: final_status.to_string(),
        http_status: http_status.map(i64::from),
        started_at_ms,
        finished_at_ms: Some(finished_at_ms),
        latency_ms,
        error_code: error_code.map(str::to_string),
        error_reason: error_reason.map(str::to_string),
        requested_context_window: None,
        requested_context_compression: None,
        effective_context_window: None,
        effective_context_compression: None,
    };

    if let Err(error) = gateway_state
        .app_state()
        .repositories()
        .append_runtime_request_lifecycle(&request, &attempts)
    {
        let cause = error.to_string();
        log::error!(
            "{}",
            format_runtime_event_fields(
                "gateway",
                "gateway.request.persistence_failed",
                request_id,
                None,
                selected_endpoint_id,
                None,
                Some("request_lifecycle_write_failed"),
                &[("cause", cause.as_str())],
            )
        );
    }
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
