use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecordInput {
    pub request_id: String,
    pub endpoint_id: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_write_tokens: u32,
    pub estimated_cost: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub request_id: String,
    pub endpoint_id: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_write_tokens: u32,
    pub total_tokens: u32,
    pub estimated_cost: String,
}

pub fn record_request(input: UsageRecordInput) -> UsageRecord {
    UsageRecord {
        total_tokens: input.input_tokens
            + input.output_tokens
            + input.cache_read_tokens
            + input.cache_write_tokens,
        request_id: input.request_id,
        endpoint_id: input.endpoint_id,
        input_tokens: input.input_tokens,
        output_tokens: input.output_tokens,
        cache_read_tokens: input.cache_read_tokens,
        cache_write_tokens: input.cache_write_tokens,
        estimated_cost: input.estimated_cost,
    }
}
