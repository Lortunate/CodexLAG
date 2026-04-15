use std::collections::HashMap;

use serde::Serialize;

use crate::{
    providers::{inventory::project_provider_inventory_summary, relay::relay_balance_capability},
    routing::engine::{
        endpoint_downgrade_reason, endpoint_rejection_reason, wall_clock_now_ms, PoolKind,
    },
    state::RuntimeState,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderDiagnosticsSummary {
    pub sections: Vec<DiagnosticsSection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiagnosticsSection {
    pub id: String,
    pub title: String,
    pub status: String,
    pub summary: String,
    pub rows: Vec<DiagnosticsRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiagnosticsRow {
    pub key: String,
    pub label: String,
    pub status: String,
    pub value: String,
    pub details: Vec<DiagnosticsDetail>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiagnosticsDetail {
    pub label: String,
    pub value: String,
}

pub fn build_provider_diagnostics_summary(runtime: &RuntimeState) -> ProviderDiagnosticsSummary {
    let auth_rows = build_auth_rows(runtime);
    let provider_rows = build_provider_health_rows(runtime);
    let capability_rows = build_capability_rows(runtime);
    let routing_rows = build_routing_rows(runtime);

    ProviderDiagnosticsSummary {
        sections: vec![
            DiagnosticsSection {
                id: "auth_health".into(),
                title: "Auth health".into(),
                status: summarize_status(&auth_rows),
                summary: if auth_rows.is_empty() {
                    "No provider sessions stored yet.".into()
                } else {
                    format!("{} provider session(s) available.", auth_rows.len())
                },
                rows: auth_rows,
            },
            DiagnosticsSection {
                id: "provider_health".into(),
                title: "Provider health".into(),
                status: summarize_status(&provider_rows),
                summary: if provider_rows.is_empty() {
                    "No runtime endpoints are registered.".into()
                } else {
                    format!(
                        "{} runtime endpoint(s) projected into routing.",
                        provider_rows.len()
                    )
                },
                rows: provider_rows,
            },
            DiagnosticsSection {
                id: "capability_probe".into(),
                title: "Capability probe".into(),
                status: summarize_status(&capability_rows),
                summary: if capability_rows.is_empty() {
                    "No capability metadata available yet.".into()
                } else {
                    format!(
                        "{} provider capability projection(s) available.",
                        capability_rows.len()
                    )
                },
                rows: capability_rows,
            },
            DiagnosticsSection {
                id: "routing_visibility".into(),
                title: "Routing visibility".into(),
                status: summarize_status(&routing_rows),
                summary: "Current mode, policy, and last-route visibility.".into(),
                rows: routing_rows,
            },
        ],
    }
}

fn build_auth_rows(runtime: &RuntimeState) -> Vec<DiagnosticsRow> {
    runtime
        .list_provider_sessions()
        .unwrap_or_default()
        .into_iter()
        .map(|session| {
            let status = auth_status(
                session.auth_state.as_str(),
                session.last_refresh_error.as_deref(),
            );
            DiagnosticsRow {
                key: session.account_id.clone(),
                label: session.display_name.clone(),
                status: status.into(),
                value: format!(
                    "state={} | expires_at_ms={} | provider={}",
                    session.auth_state,
                    optional_i64(session.expires_at_ms),
                    session.provider_id
                ),
                details: vec![
                    detail("account_id", session.account_id),
                    detail(
                        "last_refresh_at_ms",
                        optional_i64(session.last_refresh_at_ms),
                    ),
                    detail(
                        "last_refresh_error",
                        session.last_refresh_error.unwrap_or_else(|| "none".into()),
                    ),
                ],
            }
        })
        .collect()
}

fn build_provider_health_rows(runtime: &RuntimeState) -> Vec<DiagnosticsRow> {
    let state = runtime.app_state();
    let account_index = state
        .iter_imported_official_accounts()
        .map(|account| {
            (
                account.account_id.clone(),
                (
                    account.name.clone(),
                    account.provider.clone(),
                    Some(account.session.status.clone()),
                ),
            )
        })
        .collect::<HashMap<_, _>>();
    let relay_index = state
        .iter_managed_relays()
        .map(|relay| {
            (
                relay.relay_id.clone(),
                (
                    relay.name.clone(),
                    relay.endpoint.clone(),
                    format!("{:?}", relay.adapter),
                ),
            )
        })
        .collect::<HashMap<_, _>>();
    drop(state);

    let now_ms = wall_clock_now_ms();
    runtime
        .loopback_gateway()
        .state()
        .current_candidates()
        .into_iter()
        .map(|candidate| {
            let rejection_reason = endpoint_rejection_reason(&candidate, now_ms);
            let downgrade_reason = endpoint_downgrade_reason(&candidate, now_ms);
            let status = provider_row_status(rejection_reason, downgrade_reason);
            let pool = match candidate.pool {
                PoolKind::Official => "official",
                PoolKind::Relay => "relay",
            };
            let (label, details) = match candidate.pool {
                PoolKind::Official => {
                    let (display_name, provider_id, session_status) = account_index
                        .get(&candidate.id)
                        .cloned()
                        .unwrap_or_else(|| {
                            (
                                candidate.id.clone(),
                                "unknown".into(),
                                Some("missing".into()),
                            )
                        });
                    (
                        display_name,
                        vec![
                            detail("provider_id", provider_id),
                            detail(
                                "session_status",
                                session_status.unwrap_or_else(|| "unknown".into()),
                            ),
                            detail("health_state", format!("{:?}", candidate.health.state)),
                            detail(
                                "rejection_reason",
                                rejection_reason.unwrap_or("none").to_string(),
                            ),
                            detail(
                                "downgrade_reason",
                                downgrade_reason.unwrap_or("none").to_string(),
                            ),
                        ],
                    )
                }
                PoolKind::Relay => {
                    let (display_name, endpoint, adapter) =
                        relay_index.get(&candidate.id).cloned().unwrap_or_else(|| {
                            (candidate.id.clone(), "unknown".into(), "unknown".into())
                        });
                    (
                        display_name,
                        vec![
                            detail("endpoint", endpoint),
                            detail("adapter", adapter),
                            detail("health_state", format!("{:?}", candidate.health.state)),
                            detail(
                                "rejection_reason",
                                rejection_reason.unwrap_or("none").to_string(),
                            ),
                            detail(
                                "downgrade_reason",
                                downgrade_reason.unwrap_or("none").to_string(),
                            ),
                        ],
                    )
                }
            };

            DiagnosticsRow {
                key: candidate.id.clone(),
                label,
                status: status.into(),
                value: format!(
                    "pool={pool} | available={} | priority={} | health={:?}",
                    candidate.available, candidate.priority, candidate.health.state
                ),
                details,
            }
        })
        .collect()
}

fn build_capability_rows(runtime: &RuntimeState) -> Vec<DiagnosticsRow> {
    let state = runtime.app_state();
    let account_rows = state
        .iter_imported_official_accounts()
        .map(|account| DiagnosticsRow {
            key: account.account_id.clone(),
            label: account.name.clone(),
            status: if account.session.status == "active" {
                "healthy".into()
            } else {
                "warn".into()
            },
            value: format!(
                "refresh_capability={} | balance_capability={:?}",
                optional_bool(account.session.refresh_capability),
                account.session.balance_capability()
            ),
            details: vec![
                detail("provider_id", account.provider.clone()),
                detail(
                    "account_identity",
                    account
                        .session
                        .account_identity
                        .clone()
                        .unwrap_or_else(|| "none".into()),
                ),
                detail(
                    "auth_mode",
                    account
                        .session
                        .auth_mode
                        .as_ref()
                        .map(|mode| format!("{mode:?}"))
                        .unwrap_or_else(|| "none".into()),
                ),
            ],
        })
        .collect::<Vec<_>>();
    let relay_rows = state
        .iter_managed_relays()
        .map(|relay| DiagnosticsRow {
            key: relay.relay_id.clone(),
            label: relay.name.clone(),
            status: "info".into(),
            value: format!(
                "balance_capability={:?}",
                relay_balance_capability(relay.adapter)
            ),
            details: vec![
                detail("endpoint", relay.endpoint.clone()),
                detail("adapter", format!("{:?}", relay.adapter)),
            ],
        })
        .collect::<Vec<_>>();
    let inventory = project_provider_inventory_summary(&state);
    drop(state);

    let mut inventory_rows = inventory
        .providers
        .into_iter()
        .map(|provider| DiagnosticsRow {
            key: format!("inventory:{}", provider.endpoint_id),
            label: format!("{} inventory", provider.display_name),
            status: if provider.available {
                "healthy".into()
            } else if provider.registered {
                "warn".into()
            } else {
                "error".into()
            },
            value: format!(
                "registered={} | models={}",
                provider.registered,
                provider.model_ids.join(", ")
            ),
            details: vec![
                detail("provider_id", provider.provider_id),
                detail(
                    "base_url",
                    provider.base_url.unwrap_or_else(|| "default".into()),
                ),
                detail(
                    "feature_capabilities",
                    if provider.feature_capabilities.is_empty() {
                        "none".into()
                    } else {
                        provider
                            .feature_capabilities
                            .into_iter()
                            .map(|capability| capability.model_id)
                            .collect::<Vec<_>>()
                            .join(", ")
                    },
                ),
            ],
        })
        .collect::<Vec<_>>();

    let mut rows = account_rows;
    rows.extend(relay_rows);
    rows.append(&mut inventory_rows);
    rows
}

fn build_routing_rows(runtime: &RuntimeState) -> Vec<DiagnosticsRow> {
    let state = runtime.app_state();
    let current_mode = runtime.current_mode();
    let gateway_state = runtime.loopback_gateway().state();
    let unavailable_reason = gateway_state.unavailable_reason_for_mode(current_mode.as_str());
    let policy = state.default_policy().cloned();
    let last_balance_refresh = runtime.last_balance_refresh_summary();
    drop(state);

    let mut rows = vec![DiagnosticsRow {
        key: "current-mode".into(),
        label: "Current mode".into(),
        status: if unavailable_reason.is_some() {
            "warn".into()
        } else {
            "healthy".into()
        },
        value: current_mode.as_str().into(),
        details: vec![detail(
            "availability",
            unavailable_reason.unwrap_or_else(|| "ready".into()),
        )],
    }];

    if let Some(policy) = policy {
        rows.push(DiagnosticsRow {
            key: "default-policy".into(),
            label: "Default policy".into(),
            status: "info".into(),
            value: policy.name,
            details: vec![
                detail(
                    "selection_order",
                    if policy.selection_order.is_empty() {
                        "none".into()
                    } else {
                        policy.selection_order.join(", ")
                    },
                ),
                detail(
                    "cross_pool_fallback",
                    if policy.cross_pool_fallback {
                        "true"
                    } else {
                        "false"
                    },
                ),
                detail("retry_budget", policy.retry_budget.to_string()),
            ],
        });
    }

    let last_route_debug = gateway_state.last_route_debug();
    rows.push(DiagnosticsRow {
        key: "last-route".into(),
        label: "Last route".into(),
        status: if last_route_debug.is_some() {
            "info"
        } else {
            "warn"
        }
        .into(),
        value: last_route_debug
            .as_ref()
            .map(|snapshot| snapshot.selected_endpoint_id.clone())
            .unwrap_or_else(|| "no requests routed yet".into()),
        details: vec![
            detail(
                "request_id",
                last_route_debug
                    .as_ref()
                    .map(|snapshot| snapshot.request_id.clone())
                    .unwrap_or_else(|| "none".into()),
            ),
            detail(
                "attempt_count",
                last_route_debug
                    .as_ref()
                    .map(|snapshot| snapshot.attempt_count.to_string())
                    .unwrap_or_else(|| "0".into()),
            ),
        ],
    });

    rows.push(DiagnosticsRow {
        key: "last-balance-refresh".into(),
        label: "Last balance refresh".into(),
        status: "info".into(),
        value: last_balance_refresh.unwrap_or_else(|| "none".into()),
        details: Vec::new(),
    });

    rows
}

fn summarize_status(rows: &[DiagnosticsRow]) -> String {
    if rows.iter().any(|row| row.status == "error") {
        return "error".into();
    }
    if rows.iter().any(|row| row.status == "warn") {
        return "warn".into();
    }
    if rows.iter().any(|row| row.status == "healthy") {
        return "healthy".into();
    }
    "info".into()
}

fn auth_status(auth_state: &str, last_refresh_error: Option<&str>) -> &'static str {
    if last_refresh_error.is_some() || auth_state == "expired" {
        return "error";
    }
    if auth_state == "active" {
        return "healthy";
    }
    "warn"
}

fn provider_row_status(
    rejection_reason: Option<&'static str>,
    downgrade_reason: Option<&'static str>,
) -> &'static str {
    if rejection_reason.is_some() {
        return "error";
    }
    if downgrade_reason.is_some() {
        return "warn";
    }
    "healthy"
}

fn optional_i64(value: Option<i64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".into())
}

fn optional_bool(value: Option<bool>) -> String {
    value
        .map(|value| if value { "true" } else { "false" }.to_string())
        .unwrap_or_else(|| "unknown".into())
}

fn detail(label: impl Into<String>, value: impl Into<String>) -> DiagnosticsDetail {
    DiagnosticsDetail {
        label: label.into(),
        value: value.into(),
    }
}
