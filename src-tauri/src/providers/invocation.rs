use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};

use crate::{
    gateway::runtime_routing::RoutingAttemptContext,
    models::EndpointFailure,
    routing::engine::{CandidateEndpoint, PoolKind},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationSuccessMetadata {
    pub request_id: String,
    pub attempt_id: String,
    pub endpoint_id: String,
    pub upstream_status: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvocationFailureClass {
    Timeout,
    Http429,
    Http5xx,
    Auth,
    Config,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationFailure {
    pub request_id: String,
    pub attempt_id: String,
    pub endpoint_id: String,
    pub class: InvocationFailureClass,
    pub upstream_status: Option<u16>,
}

impl InvocationFailure {
    pub fn to_endpoint_failure(&self) -> EndpointFailure {
        match self.class {
            InvocationFailureClass::Timeout => EndpointFailure::Timeout,
            InvocationFailureClass::Http429 => EndpointFailure::HttpStatus(429),
            InvocationFailureClass::Http5xx => {
                EndpointFailure::HttpStatus(self.upstream_status.unwrap_or(503))
            }
            InvocationFailureClass::Auth => EndpointFailure::HttpStatus(401),
            InvocationFailureClass::Config => EndpointFailure::HttpStatus(400),
        }
    }
}

pub type InvocationOutcome = Result<InvocationSuccessMetadata, InvocationFailure>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationAttemptRecord {
    pub request_id: String,
    pub attempt_id: String,
    pub endpoint_id: String,
}

#[derive(Debug, Clone)]
struct PlannedInvocationFailure {
    class: InvocationFailureClass,
    upstream_status: Option<u16>,
}

#[derive(Clone, Default)]
pub struct ProviderInvocationPipeline {
    planned_failures: Arc<RwLock<HashMap<String, VecDeque<PlannedInvocationFailure>>>>,
    attempts: Arc<RwLock<Vec<InvocationAttemptRecord>>>,
}

impl ProviderInvocationPipeline {
    pub fn invoke(
        &self,
        endpoint: &CandidateEndpoint,
        context: &RoutingAttemptContext,
    ) -> InvocationOutcome {
        self.attempts
            .write()
            .expect("provider invocation attempts lock poisoned")
            .push(InvocationAttemptRecord {
                request_id: context.request_id.clone(),
                attempt_id: context.attempt_id.clone(),
                endpoint_id: endpoint.id.clone(),
            });

        if let Some(planned_failure) = self.pop_planned_failure(endpoint.id.as_str()) {
            return Err(InvocationFailure {
                request_id: context.request_id.clone(),
                attempt_id: context.attempt_id.clone(),
                endpoint_id: endpoint.id.clone(),
                class: planned_failure.class,
                upstream_status: planned_failure.upstream_status,
            });
        }

        Ok(InvocationSuccessMetadata {
            request_id: context.request_id.clone(),
            attempt_id: context.attempt_id.clone(),
            endpoint_id: endpoint.id.clone(),
            upstream_status: 200,
        })
    }

    pub fn plan_failure_for_test(&self, endpoint_id: &str, class: InvocationFailureClass) {
        let upstream_status = match class {
            InvocationFailureClass::Http429 => Some(429),
            InvocationFailureClass::Http5xx => Some(503),
            InvocationFailureClass::Auth => Some(401),
            InvocationFailureClass::Config => Some(400),
            InvocationFailureClass::Timeout => None,
        };
        let planned_failure = PlannedInvocationFailure {
            class,
            upstream_status,
        };
        let mut plans = self
            .planned_failures
            .write()
            .expect("provider invocation plan lock poisoned");
        plans
            .entry(endpoint_id.to_string())
            .or_default()
            .push_back(planned_failure);
    }

    pub fn attempts_for_test(&self) -> Vec<InvocationAttemptRecord> {
        self.attempts
            .read()
            .expect("provider invocation attempts lock poisoned")
            .clone()
    }

    fn pop_planned_failure(&self, endpoint_id: &str) -> Option<PlannedInvocationFailure> {
        let mut plans = self
            .planned_failures
            .write()
            .expect("provider invocation plan lock poisoned");
        plans
            .get_mut(endpoint_id)
            .and_then(|queue| queue.pop_front())
    }
}

const OFFICIAL_MODEL_MATRIX: &[&str] = &["claude-3-5-sonnet", "claude-3-7-sonnet"];
const RELAY_MODEL_MATRIX: &[&str] = &["gpt-4.1-mini", "gpt-4o-mini"];

pub fn models_for_endpoint(endpoint: &CandidateEndpoint) -> &'static [&'static str] {
    match endpoint.id.as_str() {
        "official-default" => OFFICIAL_MODEL_MATRIX,
        "relay-default" => RELAY_MODEL_MATRIX,
        _ => match endpoint.pool {
            PoolKind::Official => OFFICIAL_MODEL_MATRIX,
            PoolKind::Relay => RELAY_MODEL_MATRIX,
        },
    }
}
