use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::error::{CodexLagError, ConfigErrorKind, Result};
use crate::models::{relay_api_key_credential_ref, ManagedRelay};
use crate::providers::relay::{
    query_newapi_balance, relay_balance_capability, NormalizedBalance,
    RelayBalanceAdapter, RelayBalanceCapability,
};
use crate::secret_store::SecretKey;
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
    #[serde(default)]
    pub api_key_credential_ref: Option<String>,
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
) -> Result<RelayBalanceSnapshot> {
    let state = runtime.app_state();
    let relay = relay_by_id_from_state(&state, relay_id.as_str()).ok_or_else(|| {
        invalid_payload_error(
            "Unknown relay id.",
            format!("command=relay_lookup;field=relay_id;value={relay_id}"),
        )
    })?;
    let balance = match relay.adapter {
        RelayBalanceAdapter::NewApi => {
            let api_key = state.secret(&SecretKey::new(relay.api_key_credential_ref.clone()))?;
            let normalized =
                query_newapi_balance(relay.endpoint.as_str(), api_key.as_str()).map_err(|error| {
                    with_command_context(
                        error,
                        format!("command=refresh_relay_balance;relay_id={}", relay.relay_id),
                    )
                })?;
            RelayBalanceAvailability::Queryable {
                adapter: adapter_name(relay.adapter),
                balance: normalized,
            }
        }
        RelayBalanceAdapter::NoBalance => RelayBalanceAvailability::Unsupported {
            reason: "relay does not provide a balance endpoint".into(),
        },
    };

    Ok(RelayBalanceSnapshot {
        relay_id: relay.relay_id,
        endpoint: relay.endpoint,
        balance,
    })
    .inspect(|snapshot| {
        runtime.record_balance_refresh_summary(format!(
            "relay:{} @ {} ({})",
            snapshot.relay_id,
            current_unix_timestamp_string(),
            relay_balance_status(snapshot)
        ));
    })
}

#[tauri::command]
pub fn refresh_relay_balance(
    relay_id: String,
    state: State<'_, RuntimeState>,
) -> Result<RelayBalanceSnapshot> {
    refresh_relay_balance_from_runtime(&state, relay_id)
}

pub fn get_relay_capability_detail_from_runtime(
    runtime: &RuntimeState,
    relay_id: String,
) -> Result<RelayCapabilityDetail> {
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
) -> Result<RelayCapabilityDetail> {
    get_relay_capability_detail_from_runtime(&state, relay_id)
}

pub fn add_relay_from_runtime(
    runtime: &RuntimeState,
    input: RelayUpsertInput,
) -> Result<RelaySummary> {
    let relay_id = validate_identifier(input.relay_id.clone(), "relay_id")?;
    {
        let state = runtime.app_state();
        validate_not_conflicting_with_account_id(&state, relay_id.as_str())?;
    }
    if list_relays_from_runtime(runtime)
        .iter()
        .any(|relay| relay.relay_id == relay_id)
    {
        return Err(invalid_payload_error(
            "Relay id already exists.",
            format!("command=add_relay;field=relay_id;value={relay_id}"),
        ));
    }

    let created = build_managed_relay(relay_id, input)?;
    runtime
        .app_state_mut()
        .save_managed_relay(created.clone())
        .map_err(|error| {
            CodexLagError::new("Failed to persist relay.").with_internal_context(format!(
                "command=add_relay;operation=save_managed_relay;relay_id={};cause={error}",
                created.relay_id
            ))
        })?;
    runtime.on_inventory_changed().map_err(|error| {
        with_command_context(
            error,
            format!(
                "command=add_relay;operation=on_inventory_changed;relay_id={}",
                created.relay_id
            ),
        )
    })?;
    Ok(relay_summary(&created))
}

#[tauri::command]
pub fn add_relay(input: RelayUpsertInput, state: State<'_, RuntimeState>) -> Result<RelaySummary> {
    add_relay_from_runtime(&state, input)
}

pub fn update_relay_from_runtime(
    runtime: &RuntimeState,
    input: RelayUpsertInput,
) -> Result<RelaySummary> {
    let relay_id = validate_identifier(input.relay_id.clone(), "relay_id")?;
    {
        let state = runtime.app_state();
        validate_not_conflicting_with_account_id(&state, relay_id.as_str())?;
    }
    if runtime
        .app_state()
        .managed_relay(relay_id.as_str())
        .is_none()
    {
        return Err(invalid_payload_error(
            "Unknown relay id.",
            format!("command=update_relay;field=relay_id;value={relay_id}"),
        ));
    }

    let updated = build_managed_relay(relay_id, input)?;
    runtime
        .app_state_mut()
        .save_managed_relay(updated.clone())
        .map_err(|error| {
            CodexLagError::new("Failed to persist relay.").with_internal_context(format!(
                "command=update_relay;operation=save_managed_relay;relay_id={};cause={error}",
                updated.relay_id
            ))
        })?;
    runtime.on_inventory_changed().map_err(|error| {
        with_command_context(
            error,
            format!(
                "command=update_relay;operation=on_inventory_changed;relay_id={}",
                updated.relay_id
            ),
        )
    })?;
    Ok(relay_summary(&updated))
}

#[tauri::command]
pub fn update_relay(
    input: RelayUpsertInput,
    state: State<'_, RuntimeState>,
) -> Result<RelaySummary> {
    update_relay_from_runtime(&state, input)
}

pub fn delete_relay_from_runtime(runtime: &RuntimeState, relay_id: String) -> Result<()> {
    let relay_id = validate_identifier(relay_id, "relay_id")?;
    let removed = runtime
        .app_state_mut()
        .delete_managed_relay(relay_id.as_str())
        .map_err(|error| {
            CodexLagError::new("Failed to persist relay deletion.").with_internal_context(format!(
                "command=delete_relay;operation=delete_managed_relay;relay_id={relay_id};cause={error}"
            ))
        })?;
    if removed {
        runtime.on_inventory_changed().map_err(|error| {
            with_command_context(
                error,
                format!("command=delete_relay;operation=on_inventory_changed;relay_id={relay_id}"),
            )
        })?;
        Ok(())
    } else {
        Err(invalid_payload_error(
            "Unknown relay id.",
            format!("command=delete_relay;field=relay_id;value={relay_id}"),
        ))
    }
}

#[tauri::command]
pub fn delete_relay(relay_id: String, state: State<'_, RuntimeState>) -> Result<()> {
    delete_relay_from_runtime(&state, relay_id)
}

pub fn test_relay_connection_from_runtime(
    runtime: &RuntimeState,
    relay_id: String,
) -> Result<RelayConnectionTestResult> {
    let summary = {
        let state = runtime.app_state();
        relay_summary_by_id_from_state(&state, relay_id.as_str())?
    };
    if !is_http_endpoint(summary.endpoint.as_str()) {
        return Err(invalid_payload_error(
            "Relay endpoint must start with 'http://' or 'https://'.",
            format!(
                "command=test_relay_connection;field=endpoint;value={}",
                summary.endpoint
            ),
        ));
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
) -> Result<RelayConnectionTestResult> {
    test_relay_connection_from_runtime(&state, relay_id)
}

pub(crate) fn list_relays_from_state(state: &AppState) -> Vec<RelaySummary> {
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
            api_key_credential_ref: relay_api_key_credential_ref("relay-newapi"),
        },
        ManagedRelay {
            relay_id: "relay-nobalance".into(),
            name: "Upstream Proxy".into(),
            endpoint: "https://relay.example.test".into(),
            adapter: RelayBalanceAdapter::NoBalance,
            api_key_credential_ref: relay_api_key_credential_ref("relay-nobalance"),
        },
        ManagedRelay {
            relay_id: "relay-badpayload".into(),
            name: "Broken Upstream".into(),
            endpoint: "https://badpayload.example.test".into(),
            adapter: RelayBalanceAdapter::NewApi,
            api_key_credential_ref: relay_api_key_credential_ref("relay-badpayload"),
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

fn relay_summary_by_id_from_state(state: &AppState, relay_id: &str) -> Result<RelaySummary> {
    relay_by_id_from_state(state, relay_id)
        .map(|relay| relay_summary(&relay))
        .ok_or_else(|| {
            invalid_payload_error(
                "Unknown relay id.",
                format!("command=relay_lookup;field=relay_id;value={relay_id}"),
            )
        })
}

fn relay_adapter_for_state(state: &AppState, relay_id: &str) -> Result<RelayBalanceAdapter> {
    relay_by_id_from_state(state, relay_id)
        .map(|relay| relay.adapter)
        .ok_or_else(|| {
            invalid_payload_error(
                "Unknown relay id.",
                format!("command=relay_lookup;field=relay_id;value={relay_id}"),
            )
        })
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

fn build_managed_relay(relay_id: String, input: RelayUpsertInput) -> Result<ManagedRelay> {
    let name = validate_non_empty(input.name, "name")?;
    let endpoint = validate_non_empty(input.endpoint, "endpoint")?;
    if !is_http_endpoint(endpoint.as_str()) {
        return Err(invalid_payload_error(
            "Relay endpoint must start with 'http://' or 'https://'.",
            format!("command=relay_validation;field=endpoint;value={endpoint}"),
        ));
    }
    let adapter = parse_adapter(input.adapter.as_deref())?;
    let api_key_credential_ref =
        resolve_api_key_credential_ref(input.api_key_credential_ref, relay_id.as_str())?;

    Ok(ManagedRelay {
        relay_id,
        name,
        endpoint,
        adapter,
        api_key_credential_ref,
    })
}

fn resolve_api_key_credential_ref(
    raw: Option<String>,
    relay_id: &str,
) -> Result<String> {
    let value = raw
        .map(|candidate| candidate.trim().to_string())
        .filter(|candidate| !candidate.is_empty())
        .unwrap_or_else(|| relay_api_key_credential_ref(relay_id));
    validate_credential_ref(
        value.as_str(),
        "credential://relay/api-key/",
        "relay api-key credential ref",
    )?;
    Ok(value)
}

fn probe_relay_endpoint(endpoint: &str) -> Result<u64> {
    let (host, port) = parse_host_port(endpoint)?;
    let mut addrs = (host.as_str(), port).to_socket_addrs().map_err(|error| {
        CodexLagError::new("Failed to resolve relay endpoint.").with_internal_context(format!(
            "command=probe_relay_endpoint;endpoint={endpoint};cause={error}"
        ))
    })?;
    let address = addrs.next().ok_or_else(|| {
        CodexLagError::new("Failed to resolve relay endpoint.").with_internal_context(format!(
            "command=probe_relay_endpoint;endpoint={endpoint};cause=no_address"
        ))
    })?;

    let started = Instant::now();
    TcpStream::connect_timeout(&address, Duration::from_secs(3)).map_err(|error| {
        CodexLagError::new("Failed to connect to relay endpoint.").with_internal_context(format!(
            "command=probe_relay_endpoint;endpoint={endpoint};cause={error}"
        ))
    })?;
    Ok(started.elapsed().as_millis() as u64)
}

fn parse_host_port(endpoint: &str) -> Result<(String, u16)> {
    let (rest, default_port) = if let Some(value) = endpoint.strip_prefix("http://") {
        (value, 80_u16)
    } else if let Some(value) = endpoint.strip_prefix("https://") {
        (value, 443_u16)
    } else {
        return Err(invalid_payload_error(
            "Relay endpoint must start with 'http://' or 'https://'.",
            format!("command=parse_host_port;field=endpoint;value={endpoint}"),
        ));
    };

    let authority = rest.split('/').next().ok_or_else(|| {
        invalid_payload_error(
            "Relay endpoint must include a host.",
            format!("command=parse_host_port;field=endpoint;value={endpoint}"),
        )
    })?;
    if authority.is_empty() {
        return Err(invalid_payload_error(
            "Relay endpoint must include a host.",
            format!("command=parse_host_port;field=endpoint;value={endpoint}"),
        ));
    }
    let authority = authority.rsplit('@').next().unwrap_or(authority);

    if authority.starts_with('[') {
        let close = authority.find(']').ok_or_else(|| {
            invalid_payload_error(
                "Relay endpoint contains invalid IPv6 host format.",
                format!("command=parse_host_port;field=endpoint;value={endpoint}"),
            )
        })?;
        let host = authority[1..close].to_string();
        if host.is_empty() {
            return Err(invalid_payload_error(
                "Relay endpoint must include a host.",
                format!("command=parse_host_port;field=endpoint;value={endpoint}"),
            ));
        }
        let remainder = &authority[(close + 1)..];
        if remainder.is_empty() {
            return Ok((host, default_port));
        }
        let port = remainder
            .strip_prefix(':')
            .ok_or_else(|| {
                invalid_payload_error(
                    "Relay endpoint contains invalid host:port format.",
                    format!("command=parse_host_port;field=endpoint;value={endpoint}"),
                )
            })?
            .parse::<u16>()
            .map_err(|_| {
                invalid_payload_error(
                    "Relay endpoint contains invalid port.",
                    format!("command=parse_host_port;field=endpoint;value={endpoint}"),
                )
            })?;
        return Ok((host, port));
    }

    if authority.matches(':').count() > 1 {
        return Err(invalid_payload_error(
            "Relay endpoint contains invalid host:port format.",
            format!("command=parse_host_port;field=endpoint;value={endpoint}"),
        ));
    }
    if let Some((host, port_raw)) = authority.split_once(':') {
        if host.is_empty() {
            return Err(invalid_payload_error(
                "Relay endpoint must include a host.",
                format!("command=parse_host_port;field=endpoint;value={endpoint}"),
            ));
        }
        let port = port_raw.parse::<u16>().map_err(|_| {
            invalid_payload_error(
                "Relay endpoint contains invalid port.",
                format!("command=parse_host_port;field=endpoint;value={endpoint}"),
            )
        })?;
        return Ok((host.to_string(), port));
    }

    Ok((authority.to_string(), default_port))
}

fn validate_identifier(raw: String, field_name: &str) -> Result<String> {
    let value = validate_non_empty(raw, field_name)?;
    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
    {
        Ok(value)
    } else {
        Err(invalid_payload_error(
            format!("{field_name} must use only ascii letters, numbers, '-' or '_'"),
            format!("command=relay_validation;field={field_name};value={value}"),
        ))
    }
}

fn validate_not_conflicting_with_account_id(state: &AppState, relay_id: &str) -> Result<()> {
    if crate::commands::accounts::list_accounts_from_state(state)
        .iter()
        .any(|account| account.account_id == relay_id)
    {
        Err(invalid_payload_error(
            format!("relay_id conflicts with existing account id: {relay_id}"),
            format!(
                "command=relay_validation;field=relay_id;value={relay_id};reason=account_conflict"
            ),
        ))
    } else {
        Ok(())
    }
}

fn validate_non_empty(raw: String, field_name: &str) -> Result<String> {
    let value = raw.trim().to_string();
    if value.is_empty() {
        Err(invalid_payload_error(
            format!("{field_name} must not be empty"),
            format!("command=relay_validation;field={field_name};value=empty"),
        ))
    } else {
        Ok(value)
    }
}

fn validate_credential_ref(value: &str, prefix: &str, label: &str) -> Result<()> {
    if value.starts_with(prefix) {
        Ok(())
    } else {
        Err(invalid_payload_error(
            format!("{label} must start with '{prefix}'"),
            format!("command=relay_validation;field={label};value={value}"),
        ))
    }
}

fn is_http_endpoint(endpoint: &str) -> bool {
    endpoint.starts_with("http://") || endpoint.starts_with("https://")
}

fn parse_adapter(raw: Option<&str>) -> Result<RelayBalanceAdapter> {
    let Some(value) = raw.map(str::trim) else {
        return Ok(RelayBalanceAdapter::NewApi);
    };
    if value.is_empty() {
        return Ok(RelayBalanceAdapter::NewApi);
    }
    match value {
        "newapi" => Ok(RelayBalanceAdapter::NewApi),
        "none" | "nobalance" => Ok(RelayBalanceAdapter::NoBalance),
        _ => Err(invalid_payload_error(
            "Adapter must be one of: newapi, none.",
            format!("command=relay_validation;field=adapter;value={value}"),
        )),
    }
}

fn relay_balance_status(snapshot: &RelayBalanceSnapshot) -> &'static str {
    match snapshot.balance {
        RelayBalanceAvailability::Queryable { .. } => "queryable",
        RelayBalanceAvailability::Unsupported { .. } => "unsupported",
    }
}

fn current_unix_timestamp_string() -> String {
    SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

fn invalid_payload_error(message: impl Into<String>, context: impl Into<String>) -> CodexLagError {
    CodexLagError::config(ConfigErrorKind::InvalidPayload, message).with_internal_context(context)
}

fn with_command_context(error: CodexLagError, context: String) -> CodexLagError {
    let merged_context = match error.internal_context() {
        Some(existing) => format!("{context};{existing}"),
        None => context,
    };
    error.with_internal_context(merged_context)
}
