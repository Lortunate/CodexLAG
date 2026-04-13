use serde::{de::Error as DeError, Deserialize, Deserializer, Serialize};

pub const USAGE_RECORD_RETENTION_CAP: usize = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecordInput {
    pub request_id: String,
    pub endpoint_id: String,
    #[serde(default)]
    pub model: Option<String>,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_write_tokens: u32,
    #[serde(default)]
    pub reasoning_tokens: u32,
    pub estimated_cost: String,
    #[serde(default)]
    pub cost_provenance: UsageProvenance,
    #[serde(default)]
    pub cost_is_estimated: bool,
    #[serde(default)]
    pub pricing_profile_id: Option<String>,
    #[serde(default)]
    pub declared_capability_requirements: Option<String>,
    #[serde(default)]
    pub effective_capability_result: Option<String>,
    #[serde(default)]
    pub final_upstream_status: Option<u16>,
    #[serde(default)]
    pub final_upstream_error_code: Option<String>,
    #[serde(default)]
    pub final_upstream_error_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsageRecord {
    pub request_id: String,
    pub endpoint_id: String,
    pub model: Option<String>,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_write_tokens: u32,
    pub reasoning_tokens: u32,
    pub total_tokens: u32,
    pub estimated_cost: String,
    pub cost_provenance: UsageProvenance,
    pub cost_is_estimated: bool,
    pub pricing_profile_id: Option<String>,
    pub declared_capability_requirements: Option<String>,
    pub effective_capability_result: Option<String>,
    pub final_upstream_status: Option<u16>,
    pub final_upstream_error_code: Option<String>,
    pub final_upstream_error_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UsageProvenance {
    Actual,
    Estimated,
    Unknown,
}

impl Default for UsageProvenance {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageCost {
    pub amount: Option<String>,
    pub provenance: UsageProvenance,
    #[serde(default)]
    pub is_estimated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageRequestDetail {
    pub request_id: String,
    pub endpoint_id: String,
    pub model: Option<String>,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_write_tokens: u32,
    pub reasoning_tokens: u32,
    pub total_tokens: u32,
    pub cost: UsageCost,
    pub pricing_profile_id: Option<String>,
    pub declared_capability_requirements: Option<String>,
    pub effective_capability_result: Option<String>,
    pub final_upstream_status: Option<u16>,
    pub final_upstream_error_code: Option<String>,
    pub final_upstream_error_reason: Option<String>,
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
    #[serde(default)]
    model: Option<String>,
    input_tokens: u32,
    output_tokens: u32,
    cache_read_tokens: u32,
    cache_write_tokens: u32,
    #[serde(default)]
    reasoning_tokens: u32,
    total_tokens: u32,
    estimated_cost: String,
    #[serde(default)]
    cost_provenance: UsageProvenance,
    #[serde(default)]
    cost_is_estimated: bool,
    #[serde(default)]
    pricing_profile_id: Option<String>,
    #[serde(default)]
    declared_capability_requirements: Option<String>,
    #[serde(default)]
    effective_capability_result: Option<String>,
    #[serde(default)]
    final_upstream_status: Option<u16>,
    #[serde(default)]
    final_upstream_error_code: Option<String>,
    #[serde(default)]
    final_upstream_error_reason: Option<String>,
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
            + wire.cache_write_tokens
            + wire.reasoning_tokens;

        if wire.total_tokens != computed_total {
            return Err(D::Error::custom(
                "total_tokens must equal the sum of component token fields",
            ));
        }

        Ok(Self {
            request_id: wire.request_id,
            endpoint_id: wire.endpoint_id,
            model: wire.model,
            input_tokens: wire.input_tokens,
            output_tokens: wire.output_tokens,
            cache_read_tokens: wire.cache_read_tokens,
            cache_write_tokens: wire.cache_write_tokens,
            reasoning_tokens: wire.reasoning_tokens,
            total_tokens: wire.total_tokens,
            estimated_cost: wire.estimated_cost,
            cost_provenance: wire.cost_provenance,
            cost_is_estimated: wire.cost_is_estimated,
            pricing_profile_id: wire.pricing_profile_id,
            declared_capability_requirements: wire.declared_capability_requirements,
            effective_capability_result: wire.effective_capability_result,
            final_upstream_status: wire.final_upstream_status,
            final_upstream_error_code: wire.final_upstream_error_code,
            final_upstream_error_reason: wire.final_upstream_error_reason,
        })
    }
}

pub fn record_request(input: UsageRecordInput) -> UsageRecord {
    UsageRecord {
        total_tokens: input.input_tokens
            + input.output_tokens
            + input.cache_read_tokens
            + input.cache_write_tokens
            + input.reasoning_tokens,
        request_id: input.request_id,
        endpoint_id: input.endpoint_id,
        model: input.model,
        input_tokens: input.input_tokens,
        output_tokens: input.output_tokens,
        cache_read_tokens: input.cache_read_tokens,
        cache_write_tokens: input.cache_write_tokens,
        reasoning_tokens: input.reasoning_tokens,
        estimated_cost: input.estimated_cost,
        cost_provenance: input.cost_provenance,
        cost_is_estimated: input.cost_is_estimated,
        pricing_profile_id: input.pricing_profile_id,
        declared_capability_requirements: input.declared_capability_requirements,
        effective_capability_result: input.effective_capability_result,
        final_upstream_status: input.final_upstream_status,
        final_upstream_error_code: input.final_upstream_error_code,
        final_upstream_error_reason: input.final_upstream_error_reason,
    }
}

pub fn append_usage_record(records: &mut Vec<UsageRecord>, input: UsageRecordInput) {
    records.push(record_request(input));
    enforce_usage_record_retention(records);
}

fn enforce_usage_record_retention(records: &mut Vec<UsageRecord>) {
    if records.len() <= USAGE_RECORD_RETENTION_CAP {
        return;
    }

    let overflow = records.len() - USAGE_RECORD_RETENTION_CAP;
    records.drain(0..overflow);
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
        model: record.model.clone(),
        input_tokens: record.input_tokens,
        output_tokens: record.output_tokens,
        cache_read_tokens: record.cache_read_tokens,
        cache_write_tokens: record.cache_write_tokens,
        reasoning_tokens: record.reasoning_tokens,
        total_tokens: record.total_tokens,
        cost: usage_cost_from_estimate(
            record.estimated_cost.as_str(),
            &record.cost_provenance,
            record.cost_is_estimated,
        ),
        pricing_profile_id: record.pricing_profile_id.clone(),
        declared_capability_requirements: record.declared_capability_requirements.clone(),
        effective_capability_result: record.effective_capability_result.clone(),
        final_upstream_status: record.final_upstream_status,
        final_upstream_error_code: record.final_upstream_error_code.clone(),
        final_upstream_error_reason: record.final_upstream_error_reason.clone(),
    }
}

fn usage_cost_from_estimate(
    estimated_cost: &str,
    persisted_provenance: &UsageProvenance,
    persisted_is_estimated: bool,
) -> UsageCost {
    let amount = normalize_cost_string(estimated_cost);
    let provenance = if amount.is_none() {
        UsageProvenance::Unknown
    } else if *persisted_provenance == UsageProvenance::Unknown && !persisted_is_estimated {
        // Backward compatibility: historical records only carried an estimated_cost string.
        UsageProvenance::Estimated
    } else {
        persisted_provenance.clone()
    };
    let is_estimated = persisted_is_estimated || provenance == UsageProvenance::Estimated;

    UsageCost {
        amount,
        provenance,
        is_estimated,
    }
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
    if entries.is_empty() {
        return UsageCost {
            amount: None,
            provenance: UsageProvenance::Unknown,
            is_estimated: false,
        };
    }

    let mut sum = 0.0_f64;
    let mut has_unknown = false;
    let mut has_estimated = false;

    for entry in entries {
        if entry.cost.provenance == UsageProvenance::Unknown {
            has_unknown = true;
        }
        if entry.cost.provenance == UsageProvenance::Estimated || entry.cost.is_estimated {
            has_estimated = true;
        }

        let Some(amount) = entry.cost.amount.as_ref() else {
            return UsageCost {
                amount: None,
                provenance: UsageProvenance::Unknown,
                is_estimated: false,
            };
        };

        let Ok(parsed) = amount.parse::<f64>() else {
            return UsageCost {
                amount: None,
                provenance: UsageProvenance::Unknown,
                is_estimated: false,
            };
        };
        sum += parsed;
    }

    if has_unknown {
        return UsageCost {
            amount: None,
            provenance: UsageProvenance::Unknown,
            is_estimated: false,
        };
    }

    let provenance = if has_estimated {
        UsageProvenance::Estimated
    } else {
        UsageProvenance::Actual
    };

    UsageCost {
        amount: Some(format!("{sum:.4}")),
        is_estimated: provenance == UsageProvenance::Estimated,
        provenance,
    }
}
