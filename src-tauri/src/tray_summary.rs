use crate::{
    gateway::server::LOOPBACK_GATEWAY_LISTEN_ADDRESS,
    routing::engine::{endpoint_rejection_reason, wall_clock_now_ms, PoolKind},
    routing::policy::RoutingMode,
    state::RuntimeState,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraySummaryModel {
    pub current_mode: RoutingMode,
    pub current_mode_label: String,
    pub gateway_status_label: String,
    pub listen_address_label: String,
    pub available_endpoints_label: String,
    pub last_balance_refresh_label: String,
}

pub fn build_tray_summary_for_runtime(runtime: &RuntimeState) -> TraySummaryModel {
    let current_mode = runtime.current_mode();
    let gateway_state = runtime.loopback_gateway().state();
    let unavailable_reason = gateway_state.unavailable_reason_for_mode(current_mode.as_str());
    let current_mode_label = match unavailable_reason.as_ref() {
        Some(reason) => {
            format!(
                "Default key state | Current mode: {} ({reason})",
                current_mode.as_str()
            )
        }
        None => format!(
            "Default key state | Current mode: {}",
            current_mode.as_str()
        ),
    };

    let mut gateway_status_label = match unavailable_reason {
        Some(reason) => format!("Gateway status | unavailable ({reason})"),
        None => "Gateway status | ready".to_string(),
    };
    if let Some(feedback) = runtime.last_restart_feedback() {
        gateway_status_label.push_str(" | last restart: ");
        gateway_status_label.push_str(&feedback);
    }

    let now_ms = wall_clock_now_ms();
    let mut available_official = 0usize;
    let mut available_relay = 0usize;
    for candidate in gateway_state.current_candidates() {
        if endpoint_rejection_reason(&candidate, now_ms).is_some() {
            continue;
        }

        match candidate.pool {
            PoolKind::Official => available_official += 1,
            PoolKind::Relay => available_relay += 1,
        }
    }

    let available_endpoints_label =
        format!("Available endpoints | official: {available_official}, relay: {available_relay}");
    let last_balance_refresh_label = match runtime.last_balance_refresh_summary() {
        Some(summary) => format!("Last balance refresh | {summary}"),
        None => "Last balance refresh | none".to_string(),
    };

    TraySummaryModel {
        current_mode,
        current_mode_label,
        gateway_status_label,
        listen_address_label: format!("Listen address | {LOOPBACK_GATEWAY_LISTEN_ADDRESS}"),
        available_endpoints_label,
        last_balance_refresh_label,
    }
}
