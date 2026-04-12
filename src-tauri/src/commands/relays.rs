use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::models::ManagedRelay;
use crate::providers::relay::{
    normalize_relay_balance_response, relay_balance_capability, NormalizedBalance,
    RelayBalanceAdapter, RelayBalanceCapability,
};
use crate::state::{AppState, RuntimeState};

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

pub fn list_relays_from_runtime(runtime: &RuntimeState) -> Vec<RelaySummary> {
    list_relays_from_state(&runtime.app_state())
}

#[tauri::command]
pub fn list_relays(state: State<'_, RuntimeState>) -> Vec<RelaySummary> {
    list_relays_from_runtime(&state)
}

pub fn refresh_relay_balance_from_runtime(
    runtime: &RuntimeState,
    relay_id: String,
) -> Result<RelayBalanceSnapshot, String> {
    let state = runtime.app_state();
    let summary = relay_summary_by_id_from_state(&state, relay_id.as_str())?;
    let adapter = relay_adapter_for_state(&state, summary.relay_id.as_str())?;
    let payload = relay_balance_fixture_payload(&state, summary.relay_id.as_str());
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
pub fn refresh_relay_balance(
    relay_id: String,
    state: State<'_, RuntimeState>,
) -> Result<RelayBalanceSnapshot, String> {
    refresh_relay_balance_from_runtime(&state, relay_id)
}

pub fn get_relay_capability_detail_from_runtime(
    runtime: &RuntimeState,
    relay_id: String,
) -> Result<RelayCapabilityDetail, String> {
    let state = runtime.app_state();
    let summary = relay_summary_by_id_from_state(&state, relay_id.as_str())?;
    let adapter = relay_adapter_for_state(&state, summary.relay_id.as_str())?;

    Ok(RelayCapabilityDetail {
        relay_id: summary.relay_id,
        endpoint: summary.endpoint,
        balance_capability: relay_balance_capability(adapter),
    })
}

#[tauri::command]
pub fn get_relay_capability_detail(
    relay_id: String,
    state: State<'_, RuntimeState>,
) -> Result<RelayCapabilityDetail, String> {
    get_relay_capability_detail_from_runtime(&state, relay_id)
}

pub fn add_relay_from_runtime(
    runtime: &RuntimeState,
    input: RelayUpsertInput,
) -> Result<RelaySummary, String> {
    let relay_id = validate_identifier(input.relay_id.clone(), "relay_id")?;
    if list_relays_from_runtime(runtime)
        .iter()
        .any(|relay| relay.relay_id == relay_id)
    {
        return Err(format!("relay id already exists: {relay_id}"));
    }

    let created = build_managed_relay(relay_id, input)?;
    runtime
        .app_state_mut()
        .save_managed_relay(created.clone())
        .map_err(|error| error.to_string())?;
    Ok(relay_summary(&created))
}

#[tauri::command]
pub fn add_relay(
    input: RelayUpsertInput,
    state: State<'_, RuntimeState>,
) -> Result<RelaySummary, String> {
    add_relay_from_runtime(&state, input)
}

pub fn update_relay_from_runtime(
    runtime: &RuntimeState,
    input: RelayUpsertInput,
) -> Result<RelaySummary, String> {
    let relay_id = validate_identifier(input.relay_id.clone(), "relay_id")?;
    if runtime
        .app_state()
        .managed_relay(relay_id.as_str())
        .is_none()
    {
        return Err(format!("unknown relay id: {relay_id}"));
    }

    let updated = build_managed_relay(relay_id, input)?;
    runtime
        .app_state_mut()
        .save_managed_relay(updated.clone())
        .map_err(|error| error.to_string())?;
    Ok(relay_summary(&updated))
}

#[tauri::command]
pub fn update_relay(
    input: RelayUpsertInput,
    state: State<'_, RuntimeState>,
) -> Result<RelaySummary, String> {
    update_relay_from_runtime(&state, input)
}

pub fn delete_relay_from_runtime(runtime: &RuntimeState, relay_id: String) -> Result<(), String> {
    let relay_id = validate_identifier(relay_id, "relay_id")?;
    let removed = runtime
        .app_state_mut()
        .delete_managed_relay(relay_id.as_str())
        .map_err(|error| error.to_string())?;
    if removed {
        Ok(())
    } else {
        Err(format!("unknown relay id: {relay_id}"))
    }
}

#[tauri::command]
pub fn delete_relay(relay_id: String, state: State<'_, RuntimeState>) -> Result<(), String> {
    delete_relay_from_runtime(&state, relay_id)
}

pub fn test_relay_connection_from_runtime(
    runtime: &RuntimeState,
    relay_id: String,
) -> Result<RelayConnectionTestResult, String> {
    let state = runtime.app_state();
    let summary = relay_summary_by_id_from_state(&state, relay_id.as_str())?;
    if !is_http_endpoint(summary.endpoint.as_str()) {
        return Err("relay endpoint must start with 'http://' or 'https://'".to_string());
    }

    let (status, latency_ms) = match probe_relay_endpoint(summary.endpoint.as_str()) {
        Ok(latency_ms) => ("ok".to_string(), latency_ms),
        Err(_) => ("failed".to_string(), 0),
    };

    Ok(RelayConnectionTestResult {
        relay_id: summary.relay_id,
        endpoint: summary.endpoint,
        status,
        latency_ms,
    })
}

#[tauri::command]
pub fn test_relay_connection(
    relay_id: String,
    state: State<'_, RuntimeState>,
) -> Result<RelayConnectionTestResult, String> {
    test_relay_connection_from_runtime(&state, relay_id)
}

fn list_relays_from_state(state: &AppState) -> Vec<RelaySummary> {
    let mut relays = default_relays()
        .into_iter()
        .map(|relay| relay_summary(&relay))
        .collect::<Vec<_>>();
    relays.extend(
        state
            .iter_managed_relays()
            .map(relay_summary)
            .collect::<Vec<_>>(),
    );
    relays.sort_by(|left, right| left.relay_id.cmp(&right.relay_id));
    relays
}

fn default_relays() -> Vec<ManagedRelay> {
    vec![
        ManagedRelay {
            relay_id: "relay-newapi".into(),
            name: "Local Gateway".into(),
            endpoint: "http://127.0.0.1:8787".into(),
            adapter: RelayBalanceAdapter::NewApi,
        },
        ManagedRelay {
            relay_id: "relay-nobalance".into(),
            name: "Upstream Proxy".into(),
            endpoint: "https://relay.example.test".into(),
            adapter: RelayBalanceAdapter::NoBalance,
        },
        ManagedRelay {
            relay_id: "relay-badpayload".into(),
            name: "Broken Upstream".into(),
            endpoint: "https://badpayload.example.test".into(),
            adapter: RelayBalanceAdapter::NewApi,
        },
    ]
}

fn default_relay_by_id(relay_id: &str) -> Option<ManagedRelay> {
    default_relays()
        .into_iter()
        .find(|relay| relay.relay_id == relay_id)
}

fn relay_by_id_from_state(state: &AppState, relay_id: &str) -> Option<ManagedRelay> {
    if let Some(managed) = state.managed_relay(relay_id) {
        return Some(managed.clone());
    }

    default_relay_by_id(relay_id)
}

fn relay_summary_by_id_from_state(
    state: &AppState,
    relay_id: &str,
) -> Result<RelaySummary, String> {
    relay_by_id_from_state(state, relay_id)
        .map(|relay| relay_summary(&relay))
        .ok_or_else(|| format!("unknown relay id: {relay_id}"))
}

fn relay_adapter_for_state(
    state: &AppState,
    relay_id: &str,
) -> Result<RelayBalanceAdapter, String> {
    relay_by_id_from_state(state, relay_id)
        .map(|relay| relay.adapter)
        .ok_or_else(|| format!("unknown relay id: {relay_id}"))
}

fn relay_balance_fixture_payload(state: &AppState, relay_id: &str) -> &'static str {
    if state.managed_relay(relay_id).is_some() {
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

fn relay_summary(relay: &ManagedRelay) -> RelaySummary {
    RelaySummary {
        relay_id: relay.relay_id.clone(),
        name: relay.name.clone(),
        endpoint: relay.endpoint.clone(),
    }
}

fn build_managed_relay(relay_id: String, input: RelayUpsertInput) -> Result<ManagedRelay, String> {
    let name = validate_non_empty(input.name, "name")?;
    let endpoint = validate_non_empty(input.endpoint, "endpoint")?;
    if !is_http_endpoint(endpoint.as_str()) {
        return Err("relay endpoint must start with 'http://' or 'https://'".to_string());
    }
    let adapter = parse_adapter(input.adapter.as_deref())?;

    Ok(ManagedRelay {
        relay_id,
        name,
        endpoint,
        adapter,
    })
}

fn probe_relay_endpoint(endpoint: &str) -> Result<u64, String> {
    let (host, port) = parse_host_port(endpoint)?;
    let mut addrs = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|error| format!("failed to resolve relay endpoint '{endpoint}': {error}"))?;
    let address = addrs
        .next()
        .ok_or_else(|| format!("failed to resolve relay endpoint '{endpoint}'"))?;

    let started = Instant::now();
    TcpStream::connect_timeout(&address, Duration::from_secs(3))
        .map_err(|error| format!("failed to connect to relay endpoint '{endpoint}': {error}"))?;
    Ok(started.elapsed().as_millis() as u64)
}

fn parse_host_port(endpoint: &str) -> Result<(String, u16), String> {
    let (rest, default_port) = if let Some(value) = endpoint.strip_prefix("http://") {
        (value, 80_u16)
    } else if let Some(value) = endpoint.strip_prefix("https://") {
        (value, 443_u16)
    } else {
        return Err("relay endpoint must start with 'http://' or 'https://'".to_string());
    };

    let authority = rest
        .split('/')
        .next()
        .ok_or_else(|| "relay endpoint must include a host".to_string())?;
    if authority.is_empty() {
        return Err("relay endpoint must include a host".to_string());
    }
    let authority = authority.rsplit('@').next().unwrap_or(authority);

    if authority.starts_with('[') {
        let close = authority
            .find(']')
            .ok_or_else(|| "relay endpoint contains invalid IPv6 host format".to_string())?;
        let host = authority[1..close].to_string();
        if host.is_empty() {
            return Err("relay endpoint must include a host".to_string());
        }
        let remainder = &authority[(close + 1)..];
        if remainder.is_empty() {
            return Ok((host, default_port));
        }
        let port = remainder
            .strip_prefix(':')
            .ok_or_else(|| "relay endpoint contains invalid host:port format".to_string())?
            .parse::<u16>()
            .map_err(|_| "relay endpoint contains invalid port".to_string())?;
        return Ok((host, port));
    }

    if authority.matches(':').count() > 1 {
        return Err("relay endpoint contains invalid host:port format".to_string());
    }
    if let Some((host, port_raw)) = authority.split_once(':') {
        if host.is_empty() {
            return Err("relay endpoint must include a host".to_string());
        }
        let port = port_raw
            .parse::<u16>()
            .map_err(|_| "relay endpoint contains invalid port".to_string())?;
        return Ok((host.to_string(), port));
    }

    Ok((authority.to_string(), default_port))
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
