use std::collections::{HashMap, VecDeque};

use crate::{
    models::EndpointFailure,
    routing::engine::{
        choose_endpoint_at, mark_success_for_endpoint, record_failure_for_endpoint, wall_clock_now_ms,
        CandidateEndpoint, FailureRules, RoutingError,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteDebugSnapshot {
    pub request_id: String,
    pub selected_endpoint_id: String,
    pub attempt_count: usize,
}

pub struct RuntimeRoutingState {
    candidates: Vec<CandidateEndpoint>,
    rules: FailureRules,
    test_outcomes: HashMap<String, VecDeque<Option<u16>>>,
    last_debug: Option<RouteDebugSnapshot>,
}

impl RuntimeRoutingState {
    pub fn new(candidates: Vec<CandidateEndpoint>, rules: FailureRules) -> Self {
        Self {
            candidates,
            rules,
            test_outcomes: HashMap::new(),
            last_debug: None,
        }
    }

    pub fn candidates_snapshot(&self) -> Vec<CandidateEndpoint> {
        self.candidates.clone()
    }

    pub fn last_debug(&self) -> Option<&RouteDebugSnapshot> {
        self.last_debug.as_ref()
    }

    pub fn set_test_outcomes(&mut self, outcomes: Vec<(String, Option<u16>)>) {
        let mut grouped = HashMap::<String, VecDeque<Option<u16>>>::new();
        for (endpoint_id, outcome) in outcomes {
            grouped.entry(endpoint_id).or_default().push_back(outcome);
        }
        self.test_outcomes = grouped;
    }

    pub fn choose_with_failover(
        &mut self,
        request_id: &str,
        mode: &str,
    ) -> Result<CandidateEndpoint, RoutingError> {
        let max_attempts = self.candidates.len().max(1);
        let mut attempt_count = 0usize;
        let mut last_selected_endpoint: Option<String> = None;

        while attempt_count < max_attempts {
            let now_ms = wall_clock_now_ms();
            let selected = choose_endpoint_at(mode, &self.candidates, now_ms)?;
            attempt_count += 1;
            last_selected_endpoint = Some(selected.id.clone());

            match self.invoke_for_endpoint(&selected) {
                Ok(()) => {
                    let _ = mark_success_for_endpoint(&mut self.candidates, selected.id.as_str());
                    self.last_debug = Some(RouteDebugSnapshot {
                        request_id: request_id.to_string(),
                        selected_endpoint_id: selected.id.clone(),
                        attempt_count,
                    });
                    return Ok(selected);
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

        Err(RoutingError::NoAvailableEndpoint)
    }

    fn invoke_for_endpoint(&mut self, endpoint: &CandidateEndpoint) -> Result<(), EndpointFailure> {
        let outcome = self
            .test_outcomes
            .get_mut(endpoint.id.as_str())
            .and_then(|queue| queue.pop_front());

        match outcome {
            Some(Some(status)) => Err(EndpointFailure::HttpStatus(status)),
            Some(None) | None => Ok(()),
        }
    }
}
