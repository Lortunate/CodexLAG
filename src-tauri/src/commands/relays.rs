use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

use crate::providers::relay::{
    normalize_relay_balance_response, relay_balance_capability, NormalizedBalance,
    RelayBalanceAdapter, RelayBalanceCapability,
};

#[derive(Debug, Clone, Serialize)]
pub struct RelaySummary {
    pub relay_id: String,
    pub name: String,
    pub endpoint: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RelayBalanceSnapshot {
    pub relay_id: String,
    pub endpoint: String,
    pub balance: RelayBalanceAvailability,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RelayBalanceAvailability {
    Queryable {
        adapter: String,
        balance: NormalizedBalance,
    },
    Unsupported {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RelayCapabilityDetail {
    pub relay_id: String,
    pub endpoint: String,
    pub balance_capability: RelayBalanceCapability,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RelayUpsertInput {
    pub relay_id: String,
    pub name: String,
    pub endpoint: String,
    pub adapter: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RelayConnectionTestResult {
    pub relay_id: String,
    pub endpoint: String,
    pub status: String,
    pub latency_ms: u64,
}

#[derive(Debug, Clone)]
struct ManagedRelay {
    summary: RelaySummary,
    adapter: RelayBalanceAdapter,
}

static MANAGED_RELAYS: OnceLock<Mutex<HashMap<String, ManagedRelay>>> = OnceLock::new();

fn managed_relays() -> &'static Mutex<HashMap<String, ManagedRelay>> {
    MANAGED_RELAYS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[tauri::command]
pub fn list_relays() -> Vec<RelaySummary> {
    let mut relays = default_relays();
    relays.extend(
        managed_relays()
            .lock()
            .expect("managed relays store lock poisoned")
            .values()
            .map(|entry| entry.summary.clone()),
    );
    relays.sort_by(|left, right| left.relay_id.cmp(&right.relay_id));
    relays
}

fn default_relays() -> Vec<RelaySummary> {
    vec![
        RelaySummary {
            relay_id: "relay-newapi".into(),
            name: "Local Gateway".into(),
            endpoint: "http://127.0.0.1:8787".into(),
        },
        RelaySummary {
            relay_id: "relay-nobalance".into(),
            name: "Upstream Proxy".into(),
            endpoint: "https://relay.example.test".into(),
        },
        RelaySummary {
            relay_id: "relay-badpayload".into(),
            name: "Broken Upstream".into(),
            endpoint: "https://badpayload.example.test".into(),
        },
    ]
}

#[tauri::command]
pub fn refresh_relay_balance(relay_id: String) -> Result<RelayBalanceSnapshot, String> {
    let summary = relay_summary_by_id(relay_id.as_str())?;
    let adapter = relay_adapter_for(summary.relay_id.as_str())?;
    let payload = relay_balance_fixture_payload(summary.relay_id.as_str());
    let balance = normalize_relay_balance_response(adapter, payload)
        .map_err(|error| format!("relay balance payload parse error: {error:?}"))?;
    let balance = match balance {
        Some(value) => RelayBalanceAvailability::Queryable {
            adapter: adapter_name(adapter),
            balance: value,
        },
        None => RelayBalanceAvailability::Unsupported {
            reason: "relay does not provide a balance endpoint".into(),
        },
    };

    Ok(RelayBalanceSnapshot {
        relay_id: summary.relay_id,
        endpoint: summary.endpoint,
        balance,
    })
}

#[tauri::command]
pub fn get_relay_capability_detail(relay_id: String) -> Result<RelayCapabilityDetail, String> {
    let summary = relay_summary_by_id(relay_id.as_str())?;
    let adapter = relay_adapter_for(summary.relay_id.as_str())?;

    Ok(RelayCapabilityDetail {
        relay_id: summary.relay_id,
        endpoint: summary.endpoint,
        balance_capability: relay_balance_capability(adapter),
    })
}

#[tauri::command]
pub fn add_relay(input: RelayUpsertInput) -> Result<RelaySummary, String> {
    let relay_id = validate_identifier(input.relay_id.clone(), "relay_id")?;
    if list_relays().iter().any(|relay| relay.relay_id == relay_id) {
        return Err(format!("relay id already exists: {relay_id}"));
    }

    let created = build_managed_relay(relay_id, input)?;
    let summary = created.summary.clone();
    managed_relays()
        .lock()
        .expect("managed relays store lock poisoned")
        .insert(summary.relay_id.clone(), created);
    Ok(summary)
}

#[tauri::command]
pub fn update_relay(input: RelayUpsertInput) -> Result<RelaySummary, String> {
    let relay_id = validate_identifier(input.relay_id.clone(), "relay_id")?;
    let mut store = managed_relays()
        .lock()
        .expect("managed relays store lock poisoned");
    if !store.contains_key(relay_id.as_str()) {
        return Err(format!("unknown relay id: {relay_id}"));
    }
    let updated = build_managed_relay(relay_id, input)?;
    let summary = updated.summary.clone();
    store.insert(summary.relay_id.clone(), updated);
    Ok(summary)
}

#[tauri::command]
pub fn delete_relay(relay_id: String) -> Result<(), String> {
    let relay_id = validate_identifier(relay_id, "relay_id")?;
    let removed = managed_relays()
        .lock()
        .expect("managed relays store lock poisoned")
        .remove(relay_id.as_str());
    if removed.is_some() {
        Ok(())
    } else {
        Err(format!("unknown relay id: {relay_id}"))
    }
}

#[tauri::command]
pub fn test_relay_connection(relay_id: String) -> Result<RelayConnectionTestResult, String> {
    let summary = relay_summary_by_id(relay_id.as_str())?;
    if !is_http_endpoint(summary.endpoint.as_str()) {
        return Err("relay endpoint must start with 'http://' or 'https://'".to_string());
    }

    let status = if summary.endpoint.contains("badpayload") || summary.endpoint.contains("offline")
    {
        "failed"
    } else {
        "ok"
    };
    let latency_ms = if status == "ok" { 18 } else { 250 };

    Ok(RelayConnectionTestResult {
        relay_id: summary.relay_id,
        endpoint: summary.endpoint,
        status: status.to_string(),
        latency_ms,
    })
}

fn relay_adapter_for(relay_id: &str) -> Result<RelayBalanceAdapter, String> {
    if let Some(adapter) = managed_relays()
        .lock()
        .expect("managed relays store lock poisoned")
        .get(relay_id)
        .map(|entry| entry.adapter)
    {
        return Ok(adapter);
    }

    match relay_id {
        "relay-newapi" | "relay-badpayload" => Ok(RelayBalanceAdapter::NewApi),
        "relay-nobalance" => Ok(RelayBalanceAdapter::NoBalance),
        _ => Err(format!("unknown relay id: {relay_id}")),
    }
}

fn relay_balance_fixture_payload(relay_id: &str) -> &'static str {
    if managed_relays()
        .lock()
        .expect("managed relays store lock poisoned")
        .contains_key(relay_id)
    {
        return r#"{"data":{"total_balance":"50.00","used_balance":"10.00"}}"#;
    }

    match relay_id {
        "relay-newapi" => r#"{"data":{"total_balance":"25.00","used_balance":"7.50"}}"#,
        "relay-badpayload" => r#"{"data":{"total_balance":"25.00"}}"#,
        "relay-nobalance" => r#"{"ignored":true}"#,
        _ => r#"{"ignored":true}"#,
    }
}

fn adapter_name(adapter: RelayBalanceAdapter) -> String {
    match adapter {
        RelayBalanceAdapter::NewApi => "newapi".into(),
        RelayBalanceAdapter::NoBalance => "none".into(),
    }
}

fn relay_summary_by_id(relay_id: &str) -> Result<RelaySummary, String> {
    list_relays()
        .into_iter()
        .find(|candidate| candidate.relay_id == relay_id)
        .ok_or_else(|| format!("unknown relay id: {relay_id}"))
}

fn build_managed_relay(relay_id: String, input: RelayUpsertInput) -> Result<ManagedRelay, String> {
    let name = validate_non_empty(input.name, "name")?;
    let endpoint = validate_non_empty(input.endpoint, "endpoint")?;
    if !is_http_endpoint(endpoint.as_str()) {
        return Err("relay endpoint must start with 'http://' or 'https://'".to_string());
    }
    let adapter = parse_adapter(input.adapter.as_deref())?;

    Ok(ManagedRelay {
        summary: RelaySummary {
            relay_id,
            name,
            endpoint,
        },
        adapter,
    })
}

fn validate_identifier(raw: String, field_name: &str) -> Result<String, String> {
    let value = validate_non_empty(raw, field_name)?;
    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
    {
        Ok(value)
    } else {
        Err(format!(
            "{field_name} must use only ascii letters, numbers, '-' or '_'"
        ))
    }
}

fn validate_non_empty(raw: String, field_name: &str) -> Result<String, String> {
    let value = raw.trim().to_string();
    if value.is_empty() {
        Err(format!("{field_name} must not be empty"))
    } else {
        Ok(value)
    }
}

fn is_http_endpoint(endpoint: &str) -> bool {
    endpoint.starts_with("http://") || endpoint.starts_with("https://")
}

fn parse_adapter(raw: Option<&str>) -> Result<RelayBalanceAdapter, String> {
    let Some(value) = raw.map(str::trim) else {
        return Ok(RelayBalanceAdapter::NewApi);
    };
    if value.is_empty() {
        return Ok(RelayBalanceAdapter::NewApi);
    }
    match value {
        "newapi" => Ok(RelayBalanceAdapter::NewApi),
        "none" | "nobalance" => Ok(RelayBalanceAdapter::NoBalance),
        _ => Err("adapter must be one of: newapi, none".to_string()),
    }
}
