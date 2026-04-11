use serde::{de::Error as DeError, Deserialize, Deserializer, Serialize};

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

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Deserialize)]
struct UsageRecordWire {
    request_id: String,
    endpoint_id: String,
    input_tokens: u32,
    output_tokens: u32,
    cache_read_tokens: u32,
    cache_write_tokens: u32,
    total_tokens: u32,
    estimated_cost: String,
}

impl<'de> Deserialize<'de> for UsageRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = UsageRecordWire::deserialize(deserializer)?;
        let computed_total = wire.input_tokens
            + wire.output_tokens
            + wire.cache_read_tokens
            + wire.cache_write_tokens;

        if wire.total_tokens != computed_total {
            return Err(D::Error::custom(
                "total_tokens must equal the sum of component token fields",
            ));
        }

        Ok(Self {
            request_id: wire.request_id,
            endpoint_id: wire.endpoint_id,
            input_tokens: wire.input_tokens,
            output_tokens: wire.output_tokens,
            cache_read_tokens: wire.cache_read_tokens,
            cache_write_tokens: wire.cache_write_tokens,
            total_tokens: wire.total_tokens,
            estimated_cost: wire.estimated_cost,
        })
    }
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
