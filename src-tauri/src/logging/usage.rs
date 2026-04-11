#[derive(Debug, Clone)]
pub struct UsageRecordInput {
    pub request_id: String,
    pub endpoint_id: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_write_tokens: u32,
    pub estimated_cost: String,
}

#[derive(Debug, Clone)]
pub struct UsageRecord {
    pub request_id: String,
    pub endpoint_id: String,
    pub total_tokens: u32,
    pub estimated_cost: String,
}

pub fn record_request(input: UsageRecordInput) -> UsageRecord {
    UsageRecord {
        request_id: input.request_id,
        endpoint_id: input.endpoint_id,
        total_tokens: input.input_tokens
            + input.output_tokens
            + input.cache_read_tokens
            + input.cache_write_tokens,
        estimated_cost: input.estimated_cost,
    }
}
