use std::collections::HashSet;

use crate::{
    logging::runtime::build_attempt_id,
    models::RoutingPolicy,
    providers::invocation::{InvocationFailure, InvocationOutcome, InvocationSuccessMetadata},
    routing::engine::{
        choose_endpoint_at, choose_endpoint_at_with_recovery, mark_success_for_endpoint,
        record_failure_for_endpoint, wall_clock_now_ms, CandidateEndpoint, FailureRules,
        PoolKind, RoutingError,
    },
    routing::policy::apply_selection_order,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingAttemptContext {
    pub request_id: String,
    pub attempt_id: String,
    pub attempt_index: usize,
    pub mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteDebugSnapshot {
    pub request_id: String,
    pub selected_endpoint_id: String,
    pub attempt_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteSelectionError {
    pub error: RoutingError,
    pub attempt_count: usize,
    pub last_invocation_failure: Option<InvocationFailure>,
}

#[derive(Debug, Clone)]
pub struct RouteSelection {
    pub endpoint: CandidateEndpoint,
    pub attempt_count: usize,
    pub success_metadata: InvocationSuccessMetadata,
}

pub struct RuntimeRoutingState {
    candidates: Vec<CandidateEndpoint>,
    last_debug: Option<RouteDebugSnapshot>,
}

impl RuntimeRoutingState {
    pub fn new(candidates: Vec<CandidateEndpoint>, _rules: FailureRules) -> Self {
        Self {
            candidates,
            last_debug: None,
        }
    }

    pub fn candidates_snapshot(&self) -> Vec<CandidateEndpoint> {
        self.candidates.clone()
    }

    pub fn last_debug(&self) -> Option<&RouteDebugSnapshot> {
        self.last_debug.as_ref()
    }

    pub fn has_available_endpoint_for_mode(&self, mode: &str) -> bool {
        choose_endpoint_at(mode, &self.candidates, wall_clock_now_ms()).is_ok()
    }

    pub fn availability_for_mode(&self, mode: &str) -> Result<bool, RoutingError> {
        match choose_endpoint_at(mode, &self.candidates, wall_clock_now_ms()) {
            Ok(_) => Ok(true),
            Err(RoutingError::NoAvailableEndpoint) => Ok(false),
            Err(error) => Err(error),
        }
    }

    pub fn set_endpoint_availability(&mut self, endpoint_id: &str, available: bool) -> bool {
        if let Some(candidate) = self
            .candidates
            .iter_mut()
            .find(|candidate| candidate.id == endpoint_id)
        {
            candidate.available = available;
            return true;
        }

        false
    }

    pub fn choose_with_failover<F>(
        &mut self,
        request_id: &str,
        policy: &RoutingPolicy,
        mode: &str,
        mut invoke: F,
    ) -> Result<RouteSelection, RouteSelectionError>
    where
        F: FnMut(&CandidateEndpoint, &RoutingAttemptContext) -> InvocationOutcome,
    {
        let max_attempts = if policy.retry_budget == 0 {
            self.candidates.len().max(1)
        } else {
            usize::min(policy.retry_budget as usize, self.candidates.len()).max(1)
        };
        let mut attempt_count = 0usize;
        let mut attempted_endpoint_keys = HashSet::<(PoolKind, String)>::new();
        let mut last_selected_endpoint: Option<String> = None;
        let mut last_invocation_failure: Option<InvocationFailure> = None;
        let mut primary_pool: Option<PoolKind> = None;

        while attempt_count < max_attempts {
            let now_ms = wall_clock_now_ms();
            let mut ordered = apply_selection_order(&self.candidates, &policy.selection_order);
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
                    return Err(RouteSelectionError {
                        error,
                        attempt_count,
                        last_invocation_failure,
                    });
                }
            };
            if let Some(pool) = primary_pool.as_ref() {
                if !policy.cross_pool_fallback && selected.pool != *pool {
                    return Err(RouteSelectionError {
                        error: RoutingError::NoAvailableEndpoint,
                        attempt_count,
                        last_invocation_failure,
                    });
                }
            } else {
                primary_pool = Some(selected.pool.clone());
            }

            let selected_key = (selected.pool.clone(), selected.id.clone());
            if attempted_endpoint_keys.contains(&selected_key) {
                self.last_debug = last_selected_endpoint.map(|endpoint_id| RouteDebugSnapshot {
                    request_id: request_id.to_string(),
                    selected_endpoint_id: endpoint_id,
                    attempt_count,
                });
                return Err(RouteSelectionError {
                    error: RoutingError::NoAvailableEndpoint,
                    attempt_count,
                    last_invocation_failure,
                });
            }

            attempt_count = attempt_count.saturating_add(1);
            attempted_endpoint_keys.insert(selected_key);
            last_selected_endpoint = Some(selected.id.clone());
            let attempt_index = attempt_count.saturating_sub(1);
            let context = RoutingAttemptContext {
                request_id: request_id.to_string(),
                attempt_id: build_attempt_id(request_id, attempt_index),
                attempt_index,
                mode: mode.to_string(),
            };

            match invoke(&selected, &context) {
                Ok(success_metadata) => {
                    let _ = mark_success_for_endpoint(
                        &mut self.candidates,
                        selected.id.as_str(),
                        &selected.pool,
                    );
                    self.last_debug = Some(RouteDebugSnapshot {
                        request_id: request_id.to_string(),
                        selected_endpoint_id: selected.id.clone(),
                        attempt_count,
                    });
                    return Ok(RouteSelection {
                        endpoint: selected,
                        attempt_count,
                        success_metadata,
                    });
                }
                Err(failure) => {
                    last_invocation_failure = Some(failure.clone());
                    let _ = record_failure_for_endpoint(
                        &mut self.candidates,
                        selected.id.as_str(),
                        &selected.pool,
                        failure.to_endpoint_failure(),
                        now_ms,
                        &policy.failure_rules,
                    );
                }
            }
        }

        if let Some(endpoint_id) = last_selected_endpoint {
            self.last_debug = Some(RouteDebugSnapshot {
                request_id: request_id.to_string(),
                selected_endpoint_id: endpoint_id,
                attempt_count,
            });
        }

        Err(RouteSelectionError {
            error: RoutingError::NoAvailableEndpoint,
            attempt_count,
            last_invocation_failure,
        })
    }
}
