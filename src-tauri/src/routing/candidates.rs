use crate::{
    commands::{accounts::list_accounts_from_state, relays::list_relays_from_state},
    routing::engine::CandidateEndpoint,
    state::AppState,
};

pub fn build_runtime_candidates(state: &AppState) -> Vec<CandidateEndpoint> {
    let mut candidates = Vec::new();

    for account in list_accounts_from_state(state) {
        candidates.push(CandidateEndpoint::official(
            account.account_id.as_str(),
            10,
            true,
        ));
    }

    for relay in list_relays_from_state(state) {
        candidates.push(CandidateEndpoint::relay(relay.relay_id.as_str(), 20, true));
    }

    candidates.sort_by(|left, right| left.id.cmp(&right.id));
    candidates
}
