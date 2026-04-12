use std::collections::HashSet;

use crate::{
    models::EndpointFailure,
    logging::runtime::build_attempt_id,
    routing::engine::{
        choose_endpoint_at, mark_success_for_endpoint, record_failure_for_endpoint, wall_clock_now_ms,
        CandidateEndpoint, FailureRules, RoutingError,
    },
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
}

#[derive(Debug, Clone)]
pub struct RouteSelection {
    pub endpoint: CandidateEndpoint,
    pub attempt_count: usize,
}

pub struct RuntimeRoutingState {
    candidates: Vec<CandidateEndpoint>,
    rules: FailureRules,
    last_debug: Option<RouteDebugSnapshot>,
}

impl RuntimeRoutingState {
    pub fn new(candidates: Vec<CandidateEndpoint>, rules: FailureRules) -> Self {
        Self {
            candidates,
            rules,
            last_debug: None,
        }
    }

    pub fn candidates_snapshot(&self) -> Vec<CandidateEndpoint> {
        self.candidates.clone()
    }

    pub fn last_debug(&self) -> Option<&RouteDebugSnapshot> {
        self.last_debug.as_ref()
    }

    pub fn choose_with_failover<F>(
        &mut self,
        request_id: &str,
        mode: &str,
        mut invoke: F,
    ) -> Result<RouteSelection, RouteSelectionError>
    where
        F: FnMut(&CandidateEndpoint, &RoutingAttemptContext) -> Result<(), EndpointFailure>,
    {
        let max_attempts = self.candidates.len().max(1);
        let mut attempt_count = 0usize;
        let mut attempted_endpoint_ids = HashSet::<String>::new();
        let mut last_selected_endpoint: Option<String> = None;

        while attempt_count < max_attempts {
            let now_ms = wall_clock_now_ms();
            let selected = match choose_endpoint_at(mode, &self.candidates, now_ms) {
                Ok(candidate) => candidate,
                Err(error) => {
                    return Err(RouteSelectionError {
                        error,
                        attempt_count,
                    });
                }
            };

            if attempted_endpoint_ids.contains(selected.id.as_str()) {
                self.last_debug = last_selected_endpoint.map(|endpoint_id| RouteDebugSnapshot {
                    request_id: request_id.to_string(),
                    selected_endpoint_id: endpoint_id,
                    attempt_count,
                });
                return Err(RouteSelectionError {
                    error: RoutingError::NoAvailableEndpoint,
                    attempt_count,
                });
            }

            attempt_count = attempt_count.saturating_add(1);
            attempted_endpoint_ids.insert(selected.id.clone());
            last_selected_endpoint = Some(selected.id.clone());
            let attempt_index = attempt_count.saturating_sub(1);
            let context = RoutingAttemptContext {
                request_id: request_id.to_string(),
                attempt_id: build_attempt_id(request_id, attempt_index),
                attempt_index,
                mode: mode.to_string(),
            };

            match invoke(&selected, &context) {
                Ok(()) => {
                    let _ = mark_success_for_endpoint(&mut self.candidates, selected.id.as_str());
                    self.last_debug = Some(RouteDebugSnapshot {
                        request_id: request_id.to_string(),
                        selected_endpoint_id: selected.id.clone(),
                        attempt_count,
                    });
                    return Ok(RouteSelection {
                        endpoint: selected,
                        attempt_count,
                    });
                }
                Err(failure) => {
                    let _ = record_failure_for_endpoint(
                        &mut self.candidates,
                        selected.id.as_str(),
                        failure,
                        now_ms,
                        &self.rules,
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
        })
    }
}
