# V1 Runtime Candidates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the static runtime candidate set with endpoint candidates derived from persisted official accounts, managed relays, and runtime eligibility state so the gateway and tray reflect the real inventory.

**Architecture:** Introduce a dedicated candidate-construction layer that builds runtime `CandidateEndpoint` values from `AppState` inventory instead of hard-coded defaults. `LoopbackGateway`, runtime routing state, and tray summaries all consume that inventory-derived candidate set. This plan stops short of full policy-driven ordering, which remains in the broader routing-policy workstream, but it ensures runtime candidates are real and persistently sourced.

**Tech Stack:** Rust, Tauri v2, Serde, current routing engine, current commands/state modules, Tokio integration tests.

---

## File Structure

### Rust backend

- Create: `src-tauri/src/routing/candidates.rs`
- Modify: `src-tauri/src/routing/mod.rs`
- Modify: `src-tauri/src/gateway/server.rs`
- Modify: `src-tauri/src/gateway/auth.rs`
- Modify: `src-tauri/src/gateway/runtime_routing.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/tray_summary.rs`
- Modify: `src-tauri/src/commands/accounts.rs`
- Modify: `src-tauri/src/commands/relays.rs`

### Tests

- Create: `src-tauri/tests/runtime_candidate_inventory.rs`
- Modify: `src-tauri/tests/runtime_composition.rs`
- Modify: `src-tauri/tests/tray_restart.rs`

## Task 1: Add A Candidate Builder From Persisted Inventory

**Files:**
- Create: `src-tauri/src/routing/candidates.rs`
- Modify: `src-tauri/src/routing/mod.rs`
- Modify: `src-tauri/src/commands/accounts.rs`
- Modify: `src-tauri/src/commands/relays.rs`
- Test: `src-tauri/tests/runtime_candidate_inventory.rs`

- [x] **Step 1: Write the failing candidate-inventory test**

```rust
// src-tauri/tests/runtime_candidate_inventory.rs
use codexlag_lib::bootstrap::bootstrap_runtime_for_test;

#[tokio::test]
async fn runtime_candidates_are_built_from_persisted_accounts_and_relays() {
    let runtime = bootstrap_runtime_for_test().await.expect("bootstrap runtime");

    let candidates = runtime.loopback_gateway().state().current_candidates();
    assert!(
        candidates.iter().any(|candidate| candidate.id == "official-primary"),
        "official inventory should produce a candidate"
    );
    assert!(
        candidates.iter().any(|candidate| candidate.id == "relay-newapi"),
        "relay inventory should produce a candidate"
    );
}
```

- [x] **Step 2: Run the targeted test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml runtime_candidates_are_built_from_persisted_accounts_and_relays -- --exact`
Expected: FAIL because runtime still uses `default_candidates()`.

- [x] **Step 3: Introduce a routing candidate builder**

```rust
// src-tauri/src/routing/candidates.rs
use crate::{
    commands::{accounts::list_accounts_from_state, relays::list_relays_from_state},
    routing::engine::CandidateEndpoint,
    state::AppState,
};

pub fn build_runtime_candidates(state: &AppState) -> Vec<CandidateEndpoint> {
    let mut candidates = Vec::new();

    for account in list_accounts_from_state(state) {
        candidates.push(CandidateEndpoint::official(account.account_id.as_str(), 10, true));
    }

    for relay in list_relays_from_state(state) {
        candidates.push(CandidateEndpoint::relay(relay.relay_id.as_str(), 20, true));
    }

    candidates.sort_by(|left, right| left.id.cmp(&right.id));
    candidates
}
```

- [x] **Step 4: Export the candidate builder from the routing module**

```rust
// src-tauri/src/routing/mod.rs
pub mod candidates;
pub mod engine;
pub mod policy;
```

- [x] **Step 5: Run the focused candidate-builder test**

Run: `cargo test --manifest-path src-tauri/Cargo.toml runtime_candidate_inventory -- --nocapture`
Expected: FAIL only on runtime wiring, not on missing builder definitions.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/routing/candidates.rs src-tauri/src/routing/mod.rs src-tauri/src/commands/accounts.rs src-tauri/src/commands/relays.rs src-tauri/tests/runtime_candidate_inventory.rs
git commit -m "feat: add runtime candidate builder from persisted inventory"
```

## Task 2: Rebuild LoopbackGateway And RuntimeRoutingState From Real Candidates

**Files:**
- Modify: `src-tauri/src/gateway/server.rs`
- Modify: `src-tauri/src/gateway/auth.rs`
- Modify: `src-tauri/src/gateway/runtime_routing.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/tests/runtime_candidate_inventory.rs`
- Modify: `src-tauri/tests/runtime_composition.rs`

- [x] **Step 1: Write the failing gateway composition test**

```rust
// src-tauri/tests/runtime_composition.rs
#[tokio::test]
async fn bootstrapped_runtime_uses_inventory_derived_gateway_candidates() {
    let runtime = codexlag_lib::bootstrap::bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let candidates = runtime.loopback_gateway().state().current_candidates();
    assert!(candidates.iter().any(|candidate| candidate.id == "official-primary"));
    assert!(candidates.iter().any(|candidate| candidate.id == "relay-newapi"));
}
```

- [x] **Step 2: Run the targeted composition test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bootstrapped_runtime_uses_inventory_derived_gateway_candidates -- --exact`
Expected: FAIL because `LoopbackGateway::new` still uses `default_candidates()`.

- [x] **Step 3: Change gateway creation to consume inventory-derived candidates**

```rust
// src-tauri/src/gateway/server.rs
pub fn build_candidates_from_state(
    app_state: &std::sync::RwLockReadGuard<'_, crate::state::AppState>,
) -> Vec<CandidateEndpoint> {
    crate::routing::candidates::build_runtime_candidates(app_state)
}

pub fn default_candidates() -> Vec<CandidateEndpoint> {
    Vec::new()
}
```

```rust
// src-tauri/src/gateway/auth.rs
let candidates = {
    let state = app_state.read().expect("gateway app state lock poisoned");
    crate::routing::candidates::build_runtime_candidates(&state)
};
```

- [x] **Step 4: Add a runtime-side refresh path for candidate inventory changes**

```rust
// src-tauri/src/state.rs
pub fn rebuild_gateway_candidates(&self) {
    let next_gateway = crate::gateway::server::LoopbackGateway::new(
        Arc::clone(&self.app_state),
        Arc::clone(&self.usage_records),
    );
    *self
        .loopback_gateway
        .write()
        .expect("runtime loopback gateway lock poisoned") = next_gateway;
}
```

- [x] **Step 5: Run focused runtime tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml runtime_candidate_inventory -- --nocapture`
Expected: PASS with inventory-derived candidates.

Run: `cargo test --manifest-path src-tauri/Cargo.toml runtime_composition -- --nocapture`
Expected: PASS with runtime gateway rebuilt from persisted inventory.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/gateway/server.rs src-tauri/src/gateway/auth.rs src-tauri/src/gateway/runtime_routing.rs src-tauri/src/state.rs src-tauri/tests/runtime_candidate_inventory.rs src-tauri/tests/runtime_composition.rs
git commit -m "feat: rebuild runtime gateway from inventory-derived candidates"
```

## Task 3: Reflect Real Candidate Inventory In Tray And Runtime Status

**Files:**
- Modify: `src-tauri/src/tray_summary.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/tests/tray_restart.rs`
- Modify: `src-tauri/tests/runtime_composition.rs`

- [x] **Step 1: Write the failing tray-summary candidate test**

```rust
// src-tauri/tests/tray_restart.rs
#[tokio::test]
async fn tray_summary_counts_inventory_derived_official_and_relay_candidates() {
    let runtime = codexlag_lib::bootstrap::bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let model = runtime.tray_model();
    let labels = model
        .items
        .iter()
        .map(|item| item.label.text().to_string())
        .collect::<Vec<_>>();

    assert!(labels.iter().any(|label| label.contains("official:")));
    assert!(labels.iter().any(|label| label.contains("relay:")));
}
```

- [x] **Step 2: Run the targeted tray-summary test to verify it fails or is still placeholder-backed**

Run: `cargo test --manifest-path src-tauri/Cargo.toml tray_summary_counts_inventory_derived_official_and_relay_candidates -- --exact`
Expected: FAIL or reveal counts still sourced from placeholder candidates.

- [x] **Step 3: Base tray summary counts on current runtime candidates**

```rust
// src-tauri/src/tray_summary.rs
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
```

- [x] **Step 4: Refresh tray/runtime status after inventory-changing operations**

```rust
// src-tauri/src/state.rs
pub fn on_inventory_changed(&self) {
    self.rebuild_gateway_candidates();
}
```

- [x] **Step 5: Run final candidate/tray tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml tray_restart -- --nocapture`
Expected: PASS with tray summary derived from the real runtime inventory.

Run: `cargo test --manifest-path src-tauri/Cargo.toml runtime_composition -- --nocapture`
Expected: PASS with status surfaces aligned to current candidates.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/tray_summary.rs src-tauri/src/state.rs src-tauri/tests/tray_restart.rs src-tauri/tests/runtime_composition.rs
git commit -m "feat: reflect inventory-derived candidates in tray status"
```
