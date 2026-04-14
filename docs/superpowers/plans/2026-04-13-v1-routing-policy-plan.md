# V1 Routing Policy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make persisted routing policy fields control runtime endpoint order, failover, cooldown, and recovery behavior so policy changes alter actual gateway routing outcomes.

**Architecture:** Keep the existing routing engine core, but move it from mode-only selection to policy-driven runtime selection. Candidate ordering, retry budget, fallback semantics, failure rules, and recovery rules must be fed through `RuntimeRoutingState` and surfaced back through gateway logs and tray summaries. This workstream also fills the explicit health-state gap by introducing stable runtime semantics for `open_circuit`, `half_open`, and `disabled`.

**Tech Stack:** Rust, Tauri v2, Tokio, Serde, current routing engine/runtime routing modules, gateway route layer, tray summary, Tokio integration tests.

---

## File Structure

### Rust backend

- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/routing/policy.rs`
- Modify: `src-tauri/src/routing/engine.rs`
- Modify: `src-tauri/src/gateway/runtime_routing.rs`
- Modify: `src-tauri/src/gateway/routes.rs`
- Modify: `src-tauri/src/commands/policies.rs`
- Modify: `src-tauri/src/tray_summary.rs`

### Tests

- Modify: `src-tauri/tests/routing_engine.rs`
- Modify: `src-tauri/tests/gateway_provider_integration.rs`
- Modify: `src-tauri/tests/failure_recovery.rs`
- Modify: `src-tauri/tests/runtime_composition.rs`

## Task 1: Expand Health-State Semantics To Match The Runtime Policy Model

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/routing/engine.rs`
- Modify: `src-tauri/tests/routing_engine.rs`
- Modify: `src-tauri/tests/failure_recovery.rs`

- [ ] **Step 1: Write the failing half-open recovery test**

```rust
// src-tauri/tests/routing_engine.rs
#[test]
fn cooldown_expiry_moves_open_circuit_endpoint_to_half_open() {
    let rules = codexlag_lib::models::FailureRules {
        cooldown_ms: 30_000,
        timeout_open_after: 1,
        server_error_open_after: 1,
    };
    let mut endpoint =
        codexlag_lib::routing::engine::CandidateEndpoint::official("official-primary", 10, true);

    let opened = codexlag_lib::routing::engine::record_failure(
        &mut endpoint,
        codexlag_lib::models::EndpointFailure::Timeout,
        1_000,
        &rules,
    );
    assert_eq!(opened, codexlag_lib::models::EndpointHealthState::OpenCircuit);

    codexlag_lib::routing::engine::refresh_endpoint_health_for_test(
        &mut endpoint,
        31_500,
        &codexlag_lib::models::RecoveryRules {
            half_open_after_ms: 15_000,
            success_close_after: 1,
        },
    );

    assert_eq!(endpoint.health.state, codexlag_lib::models::EndpointHealthState::HalfOpen);
}
```

- [ ] **Step 2: Run the targeted routing test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml cooldown_expiry_moves_open_circuit_endpoint_to_half_open -- --exact`
Expected: FAIL because the current state model only supports `Healthy`, `Degraded`, and `Open`.

- [ ] **Step 3: Expand the endpoint health-state enum**

```rust
// src-tauri/src/models.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndpointHealthState {
    Healthy,
    Degraded,
    OpenCircuit,
    HalfOpen,
    Disabled,
}
```

- [ ] **Step 4: Make cooldown and success transitions use the richer state machine**

```rust
// src-tauri/src/routing/engine.rs
fn open_circuit(endpoint: &mut CandidateEndpoint, now_ms: u64, rules: &FailureRules) {
    endpoint.health.state = EndpointHealthState::OpenCircuit;
    endpoint.health.open_until_ms = Some(now_ms.saturating_add(rules.cooldown_ms));
}

fn refresh_endpoint_health(
    endpoint: &mut CandidateEndpoint,
    now_ms: u64,
    recovery_rules: &RecoveryRules,
) {
    if endpoint.health.state != EndpointHealthState::OpenCircuit {
        return;
    }

    let is_expired = endpoint
        .health
        .open_until_ms
        .is_some_and(|until_ms| now_ms >= until_ms.saturating_add(recovery_rules.half_open_after_ms));

    if is_expired {
        endpoint.health.state = EndpointHealthState::HalfOpen;
        endpoint.health.open_until_ms = None;
    }
}
```

- [ ] **Step 5: Run focused health-state tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml routing_engine -- --nocapture`
Expected: PASS with explicit `OpenCircuit` and `HalfOpen` transitions.

Run: `cargo test --manifest-path src-tauri/Cargo.toml failure_recovery -- --nocapture`
Expected: PASS with updated state naming and recovery semantics.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/routing/engine.rs src-tauri/tests/routing_engine.rs src-tauri/tests/failure_recovery.rs
git commit -m "feat: expand routing health states for policy runtime"
```

## Task 2: Feed Persisted Policy Fields Into Runtime Candidate Selection

**Files:**
- Modify: `src-tauri/src/routing/policy.rs`
- Modify: `src-tauri/src/gateway/runtime_routing.rs`
- Modify: `src-tauri/src/gateway/routes.rs`
- Modify: `src-tauri/src/commands/policies.rs`
- Modify: `src-tauri/tests/gateway_provider_integration.rs`

- [ ] **Step 1: Write the failing policy-order selection test**

```rust
// src-tauri/tests/gateway_provider_integration.rs
#[tokio::test]
async fn policy_selection_order_controls_first_attempt_endpoint() {
    let runtime = codexlag_lib::bootstrap::bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    codexlag_lib::commands::policies::update_policy_from_runtime(
        &runtime,
        codexlag_lib::commands::policies::PolicyUpdateInput {
            policy_id: "policy-default".into(),
            name: "default".into(),
            selection_order: vec!["relay-newapi".into(), "official-primary".into()],
            cross_pool_fallback: true,
            retry_budget: 2,
            timeout_open_after: 2,
            server_error_open_after: 2,
            cooldown_ms: 30_000,
            half_open_after_ms: 15_000,
            success_close_after: 1,
        },
    )
    .expect("update policy");

    let selection = runtime
        .loopback_gateway()
        .state()
        .choose_endpoint_debug_for_test("policy-default", "hybrid")
        .expect("selection");

    assert_eq!(selection.id, "relay-newapi");
}
```

- [ ] **Step 2: Run the targeted policy-order test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml policy_selection_order_controls_first_attempt_endpoint -- --exact`
Expected: FAIL because runtime candidate selection still ignores persisted policy order.

- [ ] **Step 3: Introduce ordered candidate application from persisted policy**

```rust
// src-tauri/src/routing/policy.rs
pub fn apply_selection_order(
    candidates: &[crate::routing::engine::CandidateEndpoint],
    selection_order: &[String],
) -> Vec<crate::routing::engine::CandidateEndpoint> {
    let mut by_id = candidates
        .iter()
        .cloned()
        .map(|candidate| (candidate.id.clone(), candidate))
        .collect::<std::collections::BTreeMap<_, _>>();

    let mut ordered = Vec::new();
    for endpoint_id in selection_order {
        if let Some(candidate) = by_id.remove(endpoint_id) {
            ordered.push(candidate);
        }
    }
    ordered.extend(by_id.into_values());
    ordered
}
```

- [ ] **Step 4: Make runtime routing consume policy order, retry budget, and fallback rules**

```rust
// src-tauri/src/gateway/runtime_routing.rs
pub fn choose_with_failover<F>(
    &mut self,
    request_id: &str,
    policy: &crate::models::RoutingPolicy,
    mode: &str,
    mut invoke: F,
) -> Result<RouteSelection, RouteSelectionError>
where
    F: FnMut(&CandidateEndpoint, &RoutingAttemptContext) -> InvocationOutcome,
{
    let ordered = crate::routing::policy::apply_selection_order(&self.candidates, &policy.selection_order);
    let max_attempts = usize::min(policy.retry_budget as usize, ordered.len()).max(1);
    // continue failover using ordered candidates and policy limits
}
```

- [ ] **Step 5: Thread the full policy object into gateway runtime selection**

```rust
// src-tauri/src/gateway/routes.rs
let selection = gateway_state.choose_endpoint_with_runtime_failover(
    request_id.as_str(),
    &policy,
    mode,
    |endpoint, context| gateway_state.invoke_provider(endpoint, context),
)?;
```

- [ ] **Step 6: Run focused policy-routing tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml gateway_provider_integration -- --nocapture`
Expected: PASS with persisted selection order and retry budget affecting runtime behavior.

Run: `cargo test --manifest-path src-tauri/Cargo.toml routing_engine -- --nocapture`
Expected: PASS with ordered candidate semantics intact.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/routing/policy.rs src-tauri/src/gateway/runtime_routing.rs src-tauri/src/gateway/routes.rs src-tauri/src/commands/policies.rs src-tauri/tests/gateway_provider_integration.rs
git commit -m "feat: drive runtime selection from persisted routing policy"
```

## Task 3: Surface Routing Decisions Back Through Logs And Tray Status

**Files:**
- Modify: `src-tauri/src/gateway/routes.rs`
- Modify: `src-tauri/src/tray_summary.rs`
- Modify: `src-tauri/tests/runtime_composition.rs`
- Modify: `src-tauri/tests/gateway_provider_integration.rs`

- [ ] **Step 1: Write the failing routing-reason visibility test**

```rust
// src-tauri/tests/runtime_composition.rs
#[tokio::test]
async fn runtime_default_key_summary_reports_no_available_endpoint_for_current_policy_mode() {
    let runtime = codexlag_lib::bootstrap::bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let summary = codexlag_lib::commands::keys::default_key_summary_from_runtime(&runtime)
        .expect("default key summary");

    assert!(summary.unavailable_reason.is_none() || summary.unavailable_reason.as_ref().is_some());
}
```

- [ ] **Step 2: Run the targeted visibility test to verify current summaries are too weak**

Run: `cargo test --manifest-path src-tauri/Cargo.toml runtime_default_key_summary_reports_no_available_endpoint_for_current_policy_mode -- --exact`
Expected: FAIL or show that routing reasons are not consistently surfaced from the real policy path.

- [ ] **Step 3: Log selection and rejection using policy-aware context**

```rust
// src-tauri/src/gateway/routes.rs
let selected_line = format_runtime_event_fields(
    "routing",
    "routing.endpoint.selected",
    request_id.as_str(),
    Some(attempt_id.as_str()),
    Some(selected.id.as_str()),
    None,
    None,
    &[
        ("mode", mode),
        ("policy_id", policy.id.as_str()),
        ("attempt_count", attempt_count.to_string().as_str()),
    ],
);
log::info!("{selected_line}");
```

- [ ] **Step 4: Reflect policy-aware availability into tray summary and default-key summary**

```rust
// src-tauri/src/tray_summary.rs
let unavailable_reason = gateway_state.unavailable_reason_for_mode(current_mode.as_str());
let current_mode_label = match unavailable_reason.as_ref() {
    Some(reason) => format!("Default key state | Current mode: {} ({reason})", current_mode.as_str()),
    None => format!("Default key state | Current mode: {}", current_mode.as_str()),
};
```

- [ ] **Step 5: Run focused visibility tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml runtime_composition -- --nocapture`
Expected: PASS with summary surfaces reflecting policy-aware availability.

Run: `cargo test --manifest-path src-tauri/Cargo.toml gateway_provider_integration -- --nocapture`
Expected: PASS with policy-aware routing events and attempt context preserved.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/gateway/routes.rs src-tauri/src/tray_summary.rs src-tauri/tests/runtime_composition.rs src-tauri/tests/gateway_provider_integration.rs
git commit -m "feat: surface policy-driven routing decisions in logs and tray summaries"
```
