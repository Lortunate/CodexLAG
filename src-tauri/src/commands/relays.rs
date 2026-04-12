use serde::Serialize;

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

#[tauri::command]
pub fn list_relays() -> Vec<RelaySummary> {
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

fn relay_adapter_for(relay_id: &str) -> Result<RelayBalanceAdapter, String> {
    match relay_id {
        "relay-newapi" | "relay-badpayload" => Ok(RelayBalanceAdapter::NewApi),
        "relay-nobalance" => Ok(RelayBalanceAdapter::NoBalance),
        _ => Err(format!("unknown relay id: {relay_id}")),
    }
}

fn relay_balance_fixture_payload(relay_id: &str) -> &'static str {
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
