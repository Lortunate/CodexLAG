use crate::{routing::engine::CandidateEndpoint, secret_store::SecretKey, state::AppState};

pub fn build_runtime_candidates(state: &AppState) -> Vec<CandidateEndpoint> {
    let selection_order = state
        .default_policy()
        .map(|policy| policy.selection_order.clone())
        .unwrap_or_default();
    let mut candidates = Vec::new();

    for account in state.iter_imported_official_accounts() {
        candidates.push(CandidateEndpoint::official(
            account.account_id.as_str(),
            candidate_priority(
                account.account_id.as_str(),
                base_official_priority(account.account_id.as_str()),
                &selection_order,
            ),
            official_candidate_available(state, account),
        ));
    }

    for relay in state.iter_managed_relays() {
        candidates.push(CandidateEndpoint::relay(
            relay.relay_id.as_str(),
            candidate_priority(
                relay.relay_id.as_str(),
                base_relay_priority(relay.relay_id.as_str()),
                &selection_order,
            ),
            relay_candidate_available(state, relay),
        ));
    }

    candidates.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.id.cmp(&right.id))
    });
    candidates
}

fn official_candidate_available(
    state: &AppState,
    account: &crate::models::ImportedOfficialAccount,
) -> bool {
    account.session.status == "active"
        && state
            .secret(&SecretKey::new(account.session_credential_ref.clone()))
            .is_ok()
        && state
            .secret(&SecretKey::new(account.token_credential_ref.clone()))
            .is_ok()
}

fn relay_candidate_available(state: &AppState, relay: &crate::models::ManagedRelay) -> bool {
    state
        .secret(&SecretKey::new(relay.api_key_credential_ref.clone()))
        .is_ok()
}

fn candidate_priority(endpoint_id: &str, base_priority: i32, selection_order: &[String]) -> i32 {
    if selection_order.is_empty() {
        return base_priority;
    }

    match selection_order
        .iter()
        .position(|candidate| candidate == endpoint_id)
    {
        Some(index) => ((index as i32) + 1) * 10,
        None => 1_000 + base_priority,
    }
}

fn base_official_priority(account_id: &str) -> i32 {
    if account_id == "official-primary" {
        10
    } else {
        15
    }
}

fn base_relay_priority(relay_id: &str) -> i32 {
    match relay_id {
        "relay-newapi" => 20,
        "relay-badpayload" => 30,
        "relay-nobalance" => 40,
        _ => 50,
    }
}
