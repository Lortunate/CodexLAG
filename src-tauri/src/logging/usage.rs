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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UsageProvenance {
    Estimated,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageCost {
    pub amount: Option<String>,
    pub provenance: UsageProvenance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageRequestDetail {
    pub request_id: String,
    pub endpoint_id: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_write_tokens: u32,
    pub total_tokens: u32,
    pub cost: UsageCost,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageLedgerQuery {
    pub endpoint_id: Option<String>,
    pub request_id_prefix: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageLedger {
    pub entries: Vec<UsageRequestDetail>,
    pub total_tokens: u32,
    pub total_cost: UsageCost,
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

pub fn request_detail(records: &[UsageRecord], request_id: &str) -> Option<UsageRequestDetail> {
    records
        .iter()
        .find(|record| record.request_id == request_id)
        .map(usage_request_detail_from_record)
}

pub fn request_history(records: &[UsageRecord], limit: Option<usize>) -> Vec<UsageRequestDetail> {
    let iter = records.iter().rev().map(usage_request_detail_from_record);
    match limit {
        Some(value) => iter.take(value).collect(),
        None => iter.collect(),
    }
}

pub fn query_usage_ledger(records: &[UsageRecord], query: UsageLedgerQuery) -> UsageLedger {
    let mut entries = request_history(records, None)
        .into_iter()
        .filter(|entry| {
            query
                .endpoint_id
                .as_ref()
                .map(|endpoint_id| entry.endpoint_id == *endpoint_id)
                .unwrap_or(true)
        })
        .filter(|entry| {
            query
                .request_id_prefix
                .as_ref()
                .map(|prefix| entry.request_id.starts_with(prefix))
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();

    if let Some(limit) = query.limit {
        entries.truncate(limit);
    }

    UsageLedger {
        total_tokens: entries.iter().map(|entry| entry.total_tokens).sum(),
        total_cost: aggregate_total_cost(&entries),
        entries,
    }
}

fn usage_request_detail_from_record(record: &UsageRecord) -> UsageRequestDetail {
    UsageRequestDetail {
        request_id: record.request_id.clone(),
        endpoint_id: record.endpoint_id.clone(),
        input_tokens: record.input_tokens,
        output_tokens: record.output_tokens,
        cache_read_tokens: record.cache_read_tokens,
        cache_write_tokens: record.cache_write_tokens,
        total_tokens: record.total_tokens,
        cost: usage_cost_from_estimate(record.estimated_cost.as_str()),
    }
}

fn usage_cost_from_estimate(estimated_cost: &str) -> UsageCost {
    let amount = normalize_cost_string(estimated_cost);
    let provenance = if amount.is_some() {
        UsageProvenance::Estimated
    } else {
        UsageProvenance::Unknown
    };

    UsageCost { amount, provenance }
}

fn normalize_cost_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn aggregate_total_cost(entries: &[UsageRequestDetail]) -> UsageCost {
    let mut sum = 0.0_f64;
    for entry in entries {
        let Some(amount) = entry.cost.amount.as_ref() else {
            return UsageCost {
                amount: None,
                provenance: UsageProvenance::Unknown,
            };
        };

        let Ok(parsed) = amount.parse::<f64>() else {
            return UsageCost {
                amount: None,
                provenance: UsageProvenance::Unknown,
            };
        };
        sum += parsed;
    }

    UsageCost {
        amount: Some(format!("{sum:.4}")),
        provenance: UsageProvenance::Estimated,
    }
}
