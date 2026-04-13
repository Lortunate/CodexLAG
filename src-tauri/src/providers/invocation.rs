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
    pub model: Option<String>,
    pub upstream_status: u16,
    pub usage_dimensions: Option<InvocationUsageDimensions>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvocationUsageDimensions {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_write_tokens: u32,
    pub reasoning_tokens: u32,
}

impl InvocationUsageDimensions {
    pub fn has_non_zero_dimensions(&self) -> bool {
        self.input_tokens > 0
            || self.output_tokens > 0
            || self.cache_read_tokens > 0
            || self.cache_write_tokens > 0
            || self.reasoning_tokens > 0
    }
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
    pub pool: PoolKind,
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
    attempts: Arc<RwLock<VecDeque<InvocationAttemptRecord>>>,
}

pub const INVOCATION_ATTEMPT_RETENTION_CAP: usize = 256;

impl ProviderInvocationPipeline {
    pub fn invoke(
        &self,
        endpoint: &CandidateEndpoint,
        context: &RoutingAttemptContext,
    ) -> InvocationOutcome {
        let mut attempts = self
            .attempts
            .write()
            .expect("provider invocation attempts lock poisoned");
        if attempts.len() >= INVOCATION_ATTEMPT_RETENTION_CAP {
            let _ = attempts.pop_front();
        }
        attempts.push_back(InvocationAttemptRecord {
            request_id: context.request_id.clone(),
            attempt_id: context.attempt_id.clone(),
            endpoint_id: endpoint.id.clone(),
        });

        if let Some(planned_failure) = self.pop_planned_failure(endpoint.id.as_str()) {
            return Err(InvocationFailure {
                request_id: context.request_id.clone(),
                attempt_id: context.attempt_id.clone(),
                endpoint_id: endpoint.id.clone(),
                pool: endpoint.pool.clone(),
                class: planned_failure.class,
                upstream_status: planned_failure.upstream_status,
            });
        }

        Ok(InvocationSuccessMetadata {
            request_id: context.request_id.clone(),
            attempt_id: context.attempt_id.clone(),
            endpoint_id: endpoint.id.clone(),
            model: success_model_for_endpoint(endpoint).map(str::to_string),
            upstream_status: 200,
            usage_dimensions: success_usage_dimensions_for_endpoint(endpoint),
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
            .iter()
            .cloned()
            .collect()
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

fn success_model_for_endpoint(endpoint: &CandidateEndpoint) -> Option<&'static str> {
    models_for_endpoint(endpoint).last().copied()
}

fn success_usage_dimensions_for_endpoint(
    endpoint: &CandidateEndpoint,
) -> Option<InvocationUsageDimensions> {
    let model = success_model_for_endpoint(endpoint)?;
    Some(match model {
        "claude-3-7-sonnet" => InvocationUsageDimensions {
            input_tokens: 1_024,
            output_tokens: 256,
            cache_read_tokens: 128,
            cache_write_tokens: 0,
            reasoning_tokens: 64,
        },
        "gpt-4o-mini" => InvocationUsageDimensions {
            input_tokens: 640,
            output_tokens: 128,
            cache_read_tokens: 256,
            cache_write_tokens: 0,
            reasoning_tokens: 32,
        },
        _ => InvocationUsageDimensions {
            input_tokens: 512,
            output_tokens: 96,
            cache_read_tokens: 64,
            cache_write_tokens: 0,
            reasoning_tokens: 16,
        },
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::routing::engine::CandidateEndpoint;

    #[test]
    fn invocation_attempts_are_bounded_to_retention_limit() {
        let pipeline = ProviderInvocationPipeline::default();
        let endpoint = CandidateEndpoint::official("official-default", 1, true);

        for idx in 0..(INVOCATION_ATTEMPT_RETENTION_CAP + 5) {
            let request_id = format!("req-{idx}");
            let context = RoutingAttemptContext {
                request_id: request_id.clone(),
                attempt_id: format!("{request_id}:0"),
                attempt_index: 0,
                mode: "hybrid".to_string(),
            };
            let _ = pipeline.invoke(&endpoint, &context);
        }

        let attempts = pipeline.attempts_for_test();
        assert_eq!(attempts.len(), INVOCATION_ATTEMPT_RETENTION_CAP);
        assert_eq!(attempts.first().expect("first attempt").request_id, "req-5");
        assert_eq!(
            attempts.last().expect("last attempt").request_id,
            format!("req-{}", INVOCATION_ATTEMPT_RETENTION_CAP + 4)
        );
    }
}
