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
    pub balance: Option<NormalizedBalance>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RelayCapabilityDetail {
    pub relay_id: String,
    pub endpoint: String,
    pub balance_queryable: bool,
    pub balance_adapter: Option<String>,
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
    ]
}

#[tauri::command]
pub fn refresh_relay_balance(relay_id: String) -> Option<RelayBalanceSnapshot> {
    let summary = list_relays()
        .into_iter()
        .find(|candidate| candidate.relay_id == relay_id)?;
    let adapter = relay_adapter_for(summary.relay_id.as_str())?;
    let payload = relay_balance_fixture_payload(adapter);
    let balance = normalize_relay_balance_response(adapter, payload).ok()?;

    Some(RelayBalanceSnapshot {
        relay_id: summary.relay_id,
        endpoint: summary.endpoint,
        balance,
    })
}

#[tauri::command]
pub fn get_relay_capability_detail(relay_id: String) -> Option<RelayCapabilityDetail> {
    let summary = list_relays()
        .into_iter()
        .find(|candidate| candidate.relay_id == relay_id)?;
    let adapter = relay_adapter_for(summary.relay_id.as_str())?;
    let capability = relay_balance_capability(adapter);

    Some(RelayCapabilityDetail {
        relay_id: summary.relay_id,
        endpoint: summary.endpoint,
        balance_queryable: matches!(capability, RelayBalanceCapability::Queryable { .. }),
        balance_adapter: adapter_name(adapter),
    })
}

fn relay_adapter_for(relay_id: &str) -> Option<RelayBalanceAdapter> {
    match relay_id {
        "relay-newapi" => Some(RelayBalanceAdapter::NewApi),
        "relay-nobalance" => Some(RelayBalanceAdapter::NoBalance),
        _ => None,
    }
}

fn relay_balance_fixture_payload(adapter: RelayBalanceAdapter) -> &'static str {
    match adapter {
        RelayBalanceAdapter::NewApi => {
            r#"{"data":{"total_balance":"25.00","used_balance":"7.50"}}"#
        }
        RelayBalanceAdapter::NoBalance => r#"{"ignored":true}"#,
    }
}

fn adapter_name(adapter: RelayBalanceAdapter) -> Option<String> {
    match adapter {
        RelayBalanceAdapter::NewApi => Some("newapi".into()),
        RelayBalanceAdapter::NoBalance => None,
    }
}
