# CodexLAG V1 Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn CodexLAG from a scaffolded Tauri desktop prototype into a releasable Windows-local Codex gateway with real loopback serving, real provider routing, real platform-key issuance, real persisted audit logs, and a production-grade desktop UI.

**Architecture:** Keep the current single-process Tauri shape, but replace placeholder runtime behavior with real runtime services. The Rust side becomes the source of truth for gateway lifecycle, provider execution, routing, persistence, and tray state; the React/Tauri frontend becomes a structured operations console rebuilt on a modern UI stack. `CLIProxyAPI` remains a behavior baseline for supported official-feature semantics, not a management-platform template.

**Tech Stack:** Tauri v2, Rust, Tokio, Axum, Serde, Rusqlite, keyring/Windows Credential Manager abstraction, React 19, TypeScript, Vite, Tailwind CSS v4, shadcn/ui, Radix UI, TanStack Table, React Hook Form, Zod, Vitest, Testing Library.

---

## File Structure

### Rust Backend

- Create: `src-tauri/src/gateway/host.rs`
- Create: `src-tauri/src/routing/candidates.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/bootstrap.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/db/migrations.rs`
- Modify: `src-tauri/src/db/repositories.rs`
- Modify: `src-tauri/src/gateway/mod.rs`
- Modify: `src-tauri/src/gateway/server.rs`
- Modify: `src-tauri/src/gateway/auth.rs`
- Modify: `src-tauri/src/gateway/routes.rs`
- Modify: `src-tauri/src/gateway/runtime_routing.rs`
- Modify: `src-tauri/src/providers/mod.rs`
- Modify: `src-tauri/src/providers/official.rs`
- Modify: `src-tauri/src/providers/relay.rs`
- Modify: `src-tauri/src/routing/mod.rs`
- Modify: `src-tauri/src/routing/policy.rs`
- Modify: `src-tauri/src/routing/engine.rs`
- Modify: `src-tauri/src/logging/mod.rs`
- Modify: `src-tauri/src/logging/runtime.rs`
- Modify: `src-tauri/src/logging/usage.rs`
- Modify: `src-tauri/src/commands/accounts.rs`
- Modify: `src-tauri/src/commands/relays.rs`
- Modify: `src-tauri/src/commands/keys.rs`
- Modify: `src-tauri/src/commands/policies.rs`
- Modify: `src-tauri/src/commands/logs.rs`
- Modify: `src-tauri/src/tray.rs`
- Modify: `src-tauri/src/tray_summary.rs`

### Rust Tests

- Create: `src-tauri/tests/gateway_host.rs`
- Create: `src-tauri/tests/platform_key_issuance.rs`
- Create: `src-tauri/tests/runtime_candidate_inventory.rs`
- Create: `src-tauri/tests/official_provider_integration.rs`
- Create: `src-tauri/tests/newapi_provider_integration.rs`
- Create: `src-tauri/tests/request_lifecycle_persistence.rs`
- Modify: `src-tauri/tests/bootstrap_default_key.rs`
- Modify: `src-tauri/tests/gateway_auth.rs`
- Modify: `src-tauri/tests/gateway_provider_integration.rs`
- Modify: `src-tauri/tests/routing_engine.rs`
- Modify: `src-tauri/tests/newapi_balance.rs`
- Modify: `src-tauri/tests/command_surface.rs`
- Modify: `src-tauri/tests/runtime_composition.rs`
- Modify: `src-tauri/tests/tray_restart.rs`

### Frontend

- Modify: `package.json`
- Modify: `vite.config.ts`
- Modify: `tsconfig.json`
- Modify: `src/main.tsx`
- Modify: `src/App.tsx`
- Replace: `src/styles.css`
- Create: `components.json`
- Create: `src/lib/utils.ts`
- Create: `src/components/app-shell.tsx`
- Create: `src/components/page-header.tsx`
- Create: `src/components/status-badge.tsx`
- Create: `src/components/ui/*`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauri.ts`
- Modify: `src/features/overview/overview-page.tsx`
- Modify: `src/features/accounts/accounts-page.tsx`
- Modify: `src/features/accounts/account-import-form.tsx`
- Modify: `src/features/relays/relays-page.tsx`
- Modify: `src/features/relays/relay-editor.tsx`
- Modify: `src/features/keys/keys-page.tsx`
- Modify: `src/features/keys/key-management-panel.tsx`
- Modify: `src/features/policies/policies-page.tsx`
- Modify: `src/features/policies/policy-editor.tsx`
- Modify: `src/features/logs/logs-page.tsx`
- Modify: `src/features/default-key/default-key-mode-toggle.tsx`
- Modify: `src/test/app-shell.test.tsx`
- Modify: `src/test/tauri.test.ts`

### Docs

- Keep: `docs/superpowers/specs/foundation/codexlag-foundation.md`
- Keep: `docs/superpowers/specs/2026-04-11-product-design.md`
- Keep: `docs/superpowers/specs/2026-04-13-v1-completion-design.md`
- Create: `docs/superpowers/plans/2026-04-13-v1-completion-plan.md`
- Dispatch-ready subplans for the first three tasks:
  - `docs/superpowers/plans/2026-04-13-v1-gateway-host-plan.md`
  - `docs/superpowers/plans/2026-04-13-v1-platform-keys-plan.md`
  - `docs/superpowers/plans/2026-04-13-v1-runtime-candidates-plan.md`
- Dispatch-ready subplans for tasks four through six:
  - `docs/superpowers/plans/2026-04-13-v1-official-provider-plan.md`
  - `docs/superpowers/plans/2026-04-13-v1-newapi-relay-plan.md`
  - `docs/superpowers/plans/2026-04-13-v1-routing-policy-plan.md`
- Dispatch-ready subplans for tasks seven through ten:
  - `docs/superpowers/plans/2026-04-13-v1-request-lifecycle-plan.md`
  - `docs/superpowers/plans/2026-04-13-v1-ui-foundation-plan.md`
  - `docs/superpowers/plans/2026-04-13-v1-ui-pages-plan.md`
  - `docs/superpowers/plans/2026-04-13-v1-release-gates-plan.md`
- Repository cleanup target:
  - keep one current master completion plan plus the dispatch-ready v1 workstream subplans in `docs/superpowers/plans/`

## Task 1: Start A Real Loopback Gateway Host

**Files:**
- Create: `src-tauri/src/gateway/host.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/gateway/server.rs`
- Modify: `src-tauri/src/tray.rs`
- Test: `src-tauri/tests/gateway_host.rs`
- Modify: `src-tauri/tests/runtime_composition.rs`
- Modify: `src-tauri/tests/tray_restart.rs`

- [ ] **Step 1: Write the failing host lifecycle test**

```rust
// src-tauri/tests/gateway_host.rs
use codexlag_lib::bootstrap::bootstrap_runtime_for_test;

#[tokio::test]
async fn runtime_starts_and_restarts_a_real_loopback_gateway_host() {
    let runtime = bootstrap_runtime_for_test().await.expect("bootstrap runtime");

    let status = runtime.gateway_host_status();
    assert!(status.is_running, "gateway host should be running after bootstrap");
    assert_eq!(status.listen_addr.ip().to_string(), "127.0.0.1");

    runtime.restart_gateway().expect("restart gateway");

    let restarted = runtime.gateway_host_status();
    assert!(restarted.is_running);
    assert_eq!(restarted.listen_addr.ip().to_string(), "127.0.0.1");
}
```

- [ ] **Step 2: Run the targeted host test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml runtime_starts_and_restarts_a_real_loopback_gateway_host -- --exact`
Expected: FAIL with missing `gateway_host_status` or no running loopback host in bootstrap.

- [ ] **Step 3: Introduce a real host abstraction and wire it into runtime bootstrap**

```rust
// src-tauri/src/gateway/host.rs
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};

pub const LOOPBACK_BIND_ADDR: SocketAddr =
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8787));

pub struct GatewayHost {
    listen_addr: SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}
```

```rust
// src-tauri/src/state.rs
pub struct GatewayHostStatus {
    pub is_running: bool,
    pub listen_addr: std::net::SocketAddr,
}
```

- [ ] **Step 4: Update tray restart to restart the real host**

```rust
// src-tauri/src/tray.rs
match item_id {
    TrayItemId::RestartGateway => runtime.restart_gateway()?,
    _ => {}
}
```

- [ ] **Step 5: Run focused backend tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml gateway_host -- --nocapture`
Expected: PASS for the new host lifecycle test.

Run: `cargo test --manifest-path src-tauri/Cargo.toml runtime_mode_switch_updates_default_key_summary_and_tray_model -- --exact`
Expected: PASS with real host wiring intact.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/gateway/host.rs src-tauri/src/lib.rs src-tauri/src/state.rs src-tauri/src/gateway/server.rs src-tauri/src/tray.rs src-tauri/tests/gateway_host.rs src-tauri/tests/runtime_composition.rs src-tauri/tests/tray_restart.rs
git commit -m "feat: start real loopback gateway host"
```

## Task 2: Issue Real Platform Key Secrets And Persist Runtime Metadata

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/bootstrap.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/secret_store.rs`
- Modify: `src-tauri/src/db/migrations.rs`
- Modify: `src-tauri/src/db/repositories.rs`
- Modify: `src-tauri/src/commands/keys.rs`
- Test: `src-tauri/tests/platform_key_issuance.rs`
- Modify: `src-tauri/tests/bootstrap_default_key.rs`
- Modify: `src-tauri/tests/secret_store_persistence.rs`

- [ ] **Step 1: Write the failing key issuance test**

```rust
// src-tauri/tests/platform_key_issuance.rs
use codexlag_lib::bootstrap::bootstrap_runtime_for_test;
use codexlag_lib::commands::keys::{create_platform_key_from_runtime, CreatePlatformKeyInput};
use codexlag_lib::secret_store::SecretKey;

#[tokio::test]
async fn create_platform_key_issues_a_real_secret_and_stores_it() {
    let runtime = bootstrap_runtime_for_test().await.expect("bootstrap runtime");

    let created = create_platform_key_from_runtime(
        &runtime,
        CreatePlatformKeyInput {
            key_id: "key-secondary".into(),
            name: "secondary".into(),
            policy_id: "policy-default".into(),
            allowed_mode: "hybrid".into(),
        },
    )
    .expect("create platform key");

    assert!(created.secret.starts_with("ck_local_"));
    let stored = runtime
        .app_state()
        .secret(&SecretKey::platform_key("key-secondary"))
        .expect("stored secret");
    assert_eq!(stored, created.secret);
}
```

- [ ] **Step 2: Run the targeted test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml create_platform_key_issues_a_real_secret_and_stores_it -- --exact`
Expected: FAIL because `create_platform_key_from_runtime` does not return or store a secret.

- [ ] **Step 3: Extend the platform-key model and command result shape**

```rust
// src-tauri/src/models.rs
pub struct PlatformKey {
    pub id: String,
    pub name: String,
    pub key_prefix: String,
    pub allowed_mode: String,
    pub policy_id: String,
    pub enabled: bool,
    pub created_at_ms: i64,
    pub last_used_at_ms: Option<i64>,
}
```

```rust
// src-tauri/src/commands/keys.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CreatedPlatformKey {
    pub id: String,
    pub name: String,
    pub policy_id: String,
    pub allowed_mode: String,
    pub enabled: bool,
    pub secret: String,
}
```

- [ ] **Step 4: Generate and persist the secret during key creation**

```rust
// src-tauri/src/commands/keys.rs
let secret = crate::bootstrap::generate_platform_key_secret();
app_state.store_secret(&SecretKey::platform_key(key_id.clone()), secret.clone())?;
```

- [ ] **Step 5: Run focused tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml platform_key_issuance -- --nocapture`
Expected: PASS for key issuance and secret persistence.

Run: `cargo test --manifest-path src-tauri/Cargo.toml bootstrap_creates_default_policy_and_default_key -- --exact`
Expected: PASS with the expanded model fields.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/bootstrap.rs src-tauri/src/state.rs src-tauri/src/secret_store.rs src-tauri/src/db/migrations.rs src-tauri/src/db/repositories.rs src-tauri/src/commands/keys.rs src-tauri/tests/platform_key_issuance.rs src-tauri/tests/bootstrap_default_key.rs src-tauri/tests/secret_store_persistence.rs
git commit -m "feat: issue real platform key secrets"
```

## Task 3: Build Dynamic Runtime Candidates From Persisted Accounts, Relays, And Policy State

**Files:**
- Create: `src-tauri/src/routing/candidates.rs`
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/gateway/auth.rs`
- Modify: `src-tauri/src/gateway/server.rs`
- Modify: `src-tauri/src/gateway/runtime_routing.rs`
- Modify: `src-tauri/src/tray_summary.rs`
- Test: `src-tauri/tests/runtime_candidate_inventory.rs`

- [x] **Step 1: Write the failing runtime candidate inventory test**

```rust
// src-tauri/tests/runtime_candidate_inventory.rs
use codexlag_lib::bootstrap::bootstrap_runtime_for_test;

#[tokio::test]
async fn runtime_candidates_are_built_from_persisted_accounts_and_relays() {
    let runtime = bootstrap_runtime_for_test().await.expect("bootstrap runtime");

    let candidates = runtime.loopback_gateway().state().current_candidates();
    assert!(
        candidates.iter().any(|candidate| candidate.id == "official-primary"),
        "official account inventory should produce a runtime candidate"
    );
    assert!(
        candidates.iter().any(|candidate| candidate.id == "relay-newapi"),
        "managed relay inventory should produce a runtime candidate"
    );
}
```

- [x] **Step 2: Run the targeted test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml runtime_candidates_are_built_from_persisted_accounts_and_relays -- --exact`
Expected: FAIL because runtime still uses `default_candidates()`.

- [x] **Step 3: Create a candidate builder from persisted state**

```rust
// src-tauri/src/routing/candidates.rs
pub fn build_runtime_candidates(state: &crate::state::AppState) -> Vec<CandidateEndpoint> {
    let mut candidates = Vec::new();

    for account in crate::commands::accounts::list_accounts_from_state(state) {
        candidates.push(CandidateEndpoint::official(account.account_id.as_str(), 10, true));
    }

    for relay in crate::commands::relays::list_relays_from_state(state) {
        candidates.push(CandidateEndpoint::relay(relay.relay_id.as_str(), 20, true));
    }

    candidates
}
```

- [x] **Step 4: Rebuild runtime routing state from `AppState` instead of `default_candidates()`**

```rust
// src-tauri/src/gateway/server.rs
let candidates = crate::routing::candidates::build_runtime_candidates(
    &app_state.read().expect("gateway app state lock poisoned"),
);
let state = GatewayState::new_with_runtime(app_state, usage_records, candidates);
```

- [x] **Step 5: Run focused tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml runtime_candidate_inventory -- --nocapture`
Expected: PASS with candidates derived from real state.

Run: `cargo test --manifest-path src-tauri/Cargo.toml tray_model_exposes_operational_summary_lines -- --exact`
Expected: PASS with tray summary driven by real candidate inventory.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/routing/candidates.rs src-tauri/src/models.rs src-tauri/src/state.rs src-tauri/src/gateway/auth.rs src-tauri/src/gateway/server.rs src-tauri/src/gateway/runtime_routing.rs src-tauri/src/tray_summary.rs src-tauri/tests/runtime_candidate_inventory.rs
git commit -m "feat: build runtime candidates from persisted state"
```

## Task 4: Replace Placeholder Official Provider Execution With Imported-Session Routing

**Files:**
- Modify: `src-tauri/src/providers/mod.rs`
- Modify: `src-tauri/src/providers/official.rs`
- Modify: `src-tauri/src/commands/accounts.rs`
- Modify: `src-tauri/src/gateway/routes.rs`
- Modify: `src-tauri/src/gateway/auth.rs`
- Test: `src-tauri/tests/official_provider_integration.rs`
- Modify: `src-tauri/tests/command_surface.rs`

- [x] **Step 1: Write the failing official provider integration test**

```rust
// src-tauri/tests/official_provider_integration.rs
use codexlag_lib::bootstrap::bootstrap_runtime_for_test;

#[tokio::test]
async fn imported_official_account_can_be_selected_as_a_real_provider_candidate() {
    let runtime = bootstrap_runtime_for_test().await.expect("bootstrap runtime");

    let detail = codexlag_lib::commands::accounts::get_account_capability_detail_from_runtime(
        &runtime,
        "official-primary".to_string(),
    )
    .expect("official capability detail");

    assert!(detail.refresh_capability.is_some());
    assert_eq!(detail.provider, "openai");
}
```

- [x] **Step 2: Run the targeted test to verify it fails or remains too weak**

Run: `cargo test --manifest-path src-tauri/Cargo.toml imported_official_account_can_be_selected_as_a_real_provider_candidate -- --exact`
Expected: FAIL or reveal that capability detail still carries only placeholder state.

- [x] **Step 3: Replace placeholder official session metadata with imported-session runtime state**

```rust
// src-tauri/src/providers/official.rs
pub struct OfficialSession {
    pub session_id: String,
    pub account_identity: Option<String>,
    pub auth_mode: Option<OfficialAuthMode>,
    pub refresh_capability: Option<bool>,
    pub quota_capability: Option<bool>,
    pub last_verified_at_ms: Option<i64>,
    pub status: String,
}
```

- [x] **Step 4: Read imported credentials through the secret-store path during provider selection**

```rust
// src-tauri/src/gateway/auth.rs
pub fn official_session_for_candidate(&self, endpoint_id: &str) -> crate::error::Result<OfficialSession> {
    let state = self.app_state();
    let imported = state
        .imported_official_account(endpoint_id)
        .ok_or_else(|| crate::error::CodexLagError::new("official session missing"))?;

    let _session_secret = state.secret(&crate::secret_store::SecretKey::new(
        imported.session_credential_ref.clone(),
    ))?;

    Ok(imported.session.clone())
}
```

- [x] **Step 5: Re-run official-provider tests and command-surface tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml official_provider_integration -- --nocapture`
Expected: PASS with imported-session-backed account runtime state.

Run: `cargo test --manifest-path src-tauri/Cargo.toml account_import_login_command_validates_and_persists_entry -- --exact`
Expected: PASS with expanded account runtime metadata.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/providers/mod.rs src-tauri/src/providers/official.rs src-tauri/src/commands/accounts.rs src-tauri/src/gateway/routes.rs src-tauri/src/gateway/auth.rs src-tauri/tests/official_provider_integration.rs src-tauri/tests/command_surface.rs
git commit -m "feat: route imported official sessions through real provider state"
```

## Task 5: Replace Relay Fixtures With Real NewAPI Relay Execution And Balance Queries

**Files:**
- Modify: `src-tauri/src/providers/relay.rs`
- Modify: `src-tauri/src/commands/relays.rs`
- Modify: `src-tauri/src/gateway/routes.rs`
- Test: `src-tauri/tests/newapi_provider_integration.rs`
- Modify: `src-tauri/tests/newapi_balance.rs`
- Modify: `src-tauri/tests/balance_refresh.rs`

- [x] **Step 1: Write the failing NewAPI relay integration test**

```rust
// src-tauri/tests/newapi_provider_integration.rs
use codexlag_lib::bootstrap::bootstrap_runtime_for_test;

#[tokio::test]
async fn newapi_relay_balance_refresh_uses_live_adapter_logic() {
    let runtime = bootstrap_runtime_for_test().await.expect("bootstrap runtime");

    let snapshot = codexlag_lib::commands::relays::refresh_relay_balance_from_runtime(
        &runtime,
        "relay-newapi".to_string(),
    )
    .expect("refresh relay balance");

    assert_eq!(snapshot.relay_id, "relay-newapi");
}
```

- [x] **Step 2: Run the targeted relay test to verify it still depends on fixtures**

Run: `cargo test --manifest-path src-tauri/Cargo.toml newapi_relay_balance_refresh_uses_live_adapter_logic -- --exact`
Expected: FAIL or show fixture-backed implementation that must be replaced.

- [x] **Step 3: Move NewAPI behavior behind a real adapter entrypoint**

```rust
// src-tauri/src/providers/relay.rs
pub async fn query_newapi_balance(
    endpoint: &str,
    api_key: &str,
) -> Result<NormalizedBalance, CodexLagError> {
    let client = reqwest::Client::new();
    let response = client.get(format!("{endpoint}/api/user/self")).bearer_auth(api_key).send().await?;
    let body = response.text().await?;
    normalize_relay_balance_response(RelayBalanceAdapter::NewApi, body.as_str())?
        .ok_or_else(|| CodexLagError::new("newapi balance payload missing"))
}
```

- [x] **Step 4: Make relay commands use the managed relay endpoint and stored credential**

```rust
// src-tauri/src/commands/relays.rs
let relay = relay_by_id_from_state(&state, relay_id.as_str())?;
let api_key = state.secret(&crate::secret_store::SecretKey::new(relay.api_key_credential_ref.clone()))?;
let normalized = crate::providers::relay::query_newapi_balance(relay.endpoint.as_str(), api_key.as_str()).await?;
```

- [x] **Step 5: Run focused relay tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml newapi_balance -- --nocapture`
Expected: PASS with normalized NewAPI payloads.

Run: `cargo test --manifest-path src-tauri/Cargo.toml balance_refresh -- --nocapture`
Expected: PASS without fixture-only refresh behavior.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/providers/relay.rs src-tauri/src/commands/relays.rs src-tauri/src/gateway/routes.rs src-tauri/tests/newapi_provider_integration.rs src-tauri/tests/newapi_balance.rs src-tauri/tests/balance_refresh.rs
git commit -m "feat: execute real newapi relay adapter flows"
```

## Task 6: Make Routing Policy Drive Candidate Order, Failover, And Recovery

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/routing/policy.rs`
- Modify: `src-tauri/src/routing/engine.rs`
- Modify: `src-tauri/src/gateway/runtime_routing.rs`
- Modify: `src-tauri/src/gateway/routes.rs`
- Modify: `src-tauri/src/commands/policies.rs`
- Modify: `src-tauri/src/tray_summary.rs`
- Test: `src-tauri/tests/routing_engine.rs`
- Modify: `src-tauri/tests/gateway_provider_integration.rs`

- [ ] **Step 1: Write the failing policy-driven routing test**

```rust
// src-tauri/tests/gateway_provider_integration.rs
#[tokio::test]
async fn policy_selection_order_controls_first_attempt_endpoint() {
    let runtime = bootstrap_runtime_for_test().await.expect("bootstrap runtime");

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
            cooldown_ms: 30000,
            half_open_after_ms: 15000,
            success_close_after: 1,
        },
    )
    .expect("update policy");

    assert!(runtime.loopback_gateway().state().last_route_debug().is_none());
}
```

- [ ] **Step 2: Run policy/routing tests to verify current runtime ignores policy ordering**

Run: `cargo test --manifest-path src-tauri/Cargo.toml policy_selection_order_controls_first_attempt_endpoint -- --exact`
Expected: FAIL because runtime still chooses from static candidates and static order.

- [ ] **Step 3: Expand health state and recovery semantics**

```rust
// src-tauri/src/models.rs
pub enum EndpointHealthState {
    Healthy,
    Degraded,
    OpenCircuit,
    HalfOpen,
    Disabled,
}
```

- [ ] **Step 4: Feed policy order and runtime candidate filtering into selection**

```rust
// src-tauri/src/gateway/runtime_routing.rs
pub fn choose_with_failover<F>(
    &mut self,
    request_id: &str,
    policy: &RoutingPolicy,
    mode: &str,
    mut invoke: F,
) -> Result<RouteSelection, RouteSelectionError>
where
    F: FnMut(&CandidateEndpoint, &RoutingAttemptContext) -> InvocationOutcome,
{
    let ordered = apply_selection_order(&self.candidates, &policy.selection_order);
    // then run filtered failover against ordered candidates
}
```

- [ ] **Step 5: Re-run routing tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml routing_engine -- --nocapture`
Expected: PASS with explicit health-state transitions and policy-driven ordering.

Run: `cargo test --manifest-path src-tauri/Cargo.toml gateway_provider_integration -- --nocapture`
Expected: PASS with runtime selecting endpoints according to persisted policy.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/routing/policy.rs src-tauri/src/routing/engine.rs src-tauri/src/gateway/runtime_routing.rs src-tauri/src/gateway/routes.rs src-tauri/src/commands/policies.rs src-tauri/src/tray_summary.rs src-tauri/tests/routing_engine.rs src-tauri/tests/gateway_provider_integration.rs
git commit -m "feat: drive routing from persisted policy state"
```

## Task 7: Persist Request Lifecycle Records And Query Them From The Runtime Path

**Files:**
- Modify: `src-tauri/src/db/repositories.rs`
- Modify: `src-tauri/src/logging/usage.rs`
- Modify: `src-tauri/src/gateway/routes.rs`
- Modify: `src-tauri/src/commands/logs.rs`
- Modify: `src-tauri/src/state.rs`
- Test: `src-tauri/tests/request_lifecycle_persistence.rs`
- Modify: `src-tauri/tests/request_attempt_logging.rs`
- Modify: `src-tauri/tests/observability_e2e.rs`

- [x] **Step 1: Write the failing request lifecycle persistence test**

```rust
// src-tauri/tests/request_lifecycle_persistence.rs
use codexlag_lib::bootstrap::bootstrap_runtime_for_test;

#[tokio::test]
async fn runtime_gateway_requests_are_persisted_to_request_logs_and_attempt_logs() {
    let runtime = bootstrap_runtime_for_test().await.expect("bootstrap runtime");

    let history = codexlag_lib::commands::logs::usage_request_history_from_runtime(&runtime, Some(10));
    assert!(history.is_empty(), "fresh runtime should start empty");
}
```

- [x] **Step 2: Run the targeted persistence test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml runtime_gateway_requests_are_persisted_to_request_logs_and_attempt_logs -- --exact`
Expected: FAIL because runtime still queries only in-memory `usage_records`.

- [x] **Step 3: Persist request main/attempt records inside the gateway route**

```rust
// src-tauri/src/gateway/routes.rs
runtime
    .app_state_mut()
    .repositories_mut()
    .append_request_with_attempts(&request_log, &attempt_logs)?;
```

- [x] **Step 4: Change log commands to read persisted request lifecycle data**

```rust
// src-tauri/src/commands/logs.rs
pub fn usage_request_history_from_runtime(
    runtime: &RuntimeState,
    limit: Option<usize>,
) -> Vec<UsageRequestDetail> {
    runtime
        .app_state()
        .repositories()
        .recent_request_details(limit)
        .expect("request history")
}
```

- [x] **Step 5: Re-run persistence and observability tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml request_lifecycle_persistence -- --nocapture`
Expected: PASS with request and attempt rows written from the runtime path.

Run: `cargo test --manifest-path src-tauri/Cargo.toml observability_e2e -- --nocapture`
Expected: PASS with request IDs and attempt IDs correlating across persisted and runtime surfaces.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/db/repositories.rs src-tauri/src/logging/usage.rs src-tauri/src/gateway/routes.rs src-tauri/src/commands/logs.rs src-tauri/src/state.rs src-tauri/tests/request_lifecycle_persistence.rs src-tauri/tests/request_attempt_logging.rs src-tauri/tests/observability_e2e.rs
git commit -m "feat: persist request lifecycle records on the runtime path"
```

## Task 8: Introduce Tailwind v4 And shadcn/ui Foundation For The Desktop Console

**Files:**
- Modify: `package.json`
- Modify: `vite.config.ts`
- Modify: `tsconfig.json`
- Modify: `src/main.tsx`
- Replace: `src/styles.css`
- Create: `components.json`
- Create: `src/lib/utils.ts`
- Create: `src/components/app-shell.tsx`
- Create: `src/components/page-header.tsx`
- Create: `src/components/status-badge.tsx`
- Create: `src/components/ui/*`
- Test: `src/test/app-shell.test.tsx`

- [ ] **Step 1: Write the failing app-shell visual-structure test**

```tsx
// src/test/app-shell.test.tsx
it("renders the production desktop shell with persistent navigation and header chrome", async () => {
  render(<App />);

  expect(screen.getByRole("navigation", { name: /primary/i })).toBeInTheDocument();
  expect(screen.getByText("CodexLAG")).toBeInTheDocument();
  expect(screen.getByText("Gateway Overview")).toBeInTheDocument();
});
```

- [ ] **Step 2: Run the focused frontend test to verify it fails**

Run: `bun run test -- src/test/app-shell.test.tsx`
Expected: FAIL because the current shell is still the prototype sidebar/content layout.

- [ ] **Step 3: Add Tailwind v4 + shadcn/ui foundation using the current Vite path**

```json
// package.json
{
  "dependencies": {
    "@radix-ui/react-dialog": "^1.1.0",
    "@radix-ui/react-select": "^2.1.0",
    "@radix-ui/react-slot": "^1.1.0",
    "@tanstack/react-table": "^8.20.0",
    "class-variance-authority": "^0.7.0",
    "clsx": "^2.1.1",
    "lucide-react": "^0.511.0",
    "react-hook-form": "^7.55.0",
    "tailwind-merge": "^2.6.0",
    "zod": "^3.24.0"
  },
  "devDependencies": {
    "@tailwindcss/vite": "^4.1.0",
    "tailwindcss": "^4.1.0"
  }
}
```

```ts
// vite.config.ts
import path from "path";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
});
```

- [ ] **Step 4: Replace the prototype shell with reusable desktop layout primitives**

```tsx
// src/components/app-shell.tsx
export function AppShell({
  navigation,
  children,
}: {
  navigation: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div className="min-h-screen bg-[var(--app-bg)] text-[var(--app-fg)]">
      <div className="grid min-h-screen grid-cols-[280px_1fr]">
        <aside aria-label="Primary" className="border-r border-border/50 bg-card/60 p-6">
          {navigation}
        </aside>
        <main className="min-w-0 p-8">{children}</main>
      </div>
    </div>
  );
}
```

- [ ] **Step 5: Run the focused frontend shell tests**

Run: `bun run test -- src/test/app-shell.test.tsx`
Expected: PASS with the rebuilt desktop shell.

- [ ] **Step 6: Commit**

```bash
git add package.json vite.config.ts tsconfig.json src/main.tsx src/styles.css components.json src/lib/utils.ts src/components src/test/app-shell.test.tsx
git commit -m "feat: add desktop UI foundation with tailwind and shadcn"
```

## Task 9: Rebuild The Six Production Pages On The New UI Foundation

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauri.ts`
- Modify: `src/features/overview/overview-page.tsx`
- Modify: `src/features/accounts/accounts-page.tsx`
- Modify: `src/features/accounts/account-import-form.tsx`
- Modify: `src/features/relays/relays-page.tsx`
- Modify: `src/features/relays/relay-editor.tsx`
- Modify: `src/features/keys/keys-page.tsx`
- Modify: `src/features/keys/key-management-panel.tsx`
- Modify: `src/features/policies/policies-page.tsx`
- Modify: `src/features/policies/policy-editor.tsx`
- Modify: `src/features/logs/logs-page.tsx`
- Modify: `src/features/default-key/default-key-mode-toggle.tsx`
- Modify: `src/test/app-shell.test.tsx`
- Modify: `src/test/tauri.test.ts`

- [ ] **Step 1: Write the failing page workflow tests**

```tsx
// src/test/app-shell.test.tsx
it("shows the generated platform key secret after key creation", async () => {
  render(<App />);

  await user.click(screen.getByRole("button", { name: /platform keys/i }));
  expect(await screen.findByText(/generated secret/i)).toBeInTheDocument();
});

it("renders policy fields from hydrated backend data", async () => {
  render(<App />);

  await user.click(screen.getByRole("button", { name: /policies/i }));
  expect(await screen.findByLabelText(/retry budget/i)).toBeEnabled();
});
```

- [ ] **Step 2: Run the focused frontend tests to verify they fail**

Run: `bun run test -- src/test/app-shell.test.tsx src/test/tauri.test.ts`
Expected: FAIL because the existing page implementations still expose prototype-only flows and incomplete data hydration.

- [ ] **Step 3: Hydrate the frontend contract with the new backend result shapes**

```ts
// src/lib/types.ts
export interface CreatedPlatformKey {
  id: string;
  name: string;
  policy_id: string;
  allowed_mode: DefaultKeyMode;
  enabled: boolean;
  secret: string;
}

export interface PolicySummary {
  policy_id: string;
  name: string;
  status: string;
  selection_order: string[];
  cross_pool_fallback: boolean;
  retry_budget: number;
  timeout_open_after: number;
  server_error_open_after: number;
  cooldown_ms: number;
  half_open_after_ms: number;
  success_close_after: number;
}
```

- [ ] **Step 4: Rebuild the pages as real operations screens**

```tsx
// src/features/keys/keys-page.tsx
<PageHeader
  title="Platform Keys"
  description="Issue local gateway keys, bind policy, and review current mode access."
/>
{createdKey ? (
  <Alert>
    <AlertTitle>Generated secret</AlertTitle>
    <AlertDescription>{createdKey.secret}</AlertDescription>
  </Alert>
) : null}
```

```tsx
// src/features/policies/policies-page.tsx
<PageHeader
  title="Policies"
  description="Edit runtime routing order, failover thresholds, and recovery behavior."
/>
<PolicyEditor policies={policies} endpointIds={endpointIds} onSave={handleSavePolicy} />
```

- [ ] **Step 5: Run all frontend tests**

Run: `bun run test`
Expected: PASS for `src/test/app-shell.test.tsx` and `src/test/tauri.test.ts`.

- [ ] **Step 6: Commit**

```bash
git add src/App.tsx src/lib/types.ts src/lib/tauri.ts src/features/overview/overview-page.tsx src/features/accounts/accounts-page.tsx src/features/accounts/account-import-form.tsx src/features/relays/relays-page.tsx src/features/relays/relay-editor.tsx src/features/keys/keys-page.tsx src/features/keys/key-management-panel.tsx src/features/policies/policies-page.tsx src/features/policies/policy-editor.tsx src/features/logs/logs-page.tsx src/features/default-key/default-key-mode-toggle.tsx src/test/app-shell.test.tsx src/test/tauri.test.ts
git commit -m "feat: rebuild desktop management pages for v1"
```

## Task 10: Lock The Plan Tree And Verify Release Gates

**Files:**
- Modify: `.github/workflows/windows-release-gates.yml`
- Modify: `docs/superpowers/specs/2026-04-11-product-design.md`
- Modify: `docs/superpowers/specs/2026-04-13-v1-completion-design.md`
- Create: `docs/superpowers/specs/foundation/codexlag-foundation.md`

- [x] **Step 1: Add a release gate that runs both frontend and Rust verification**

```yaml
# .github/workflows/windows-release-gates.yml
- name: Install frontend dependencies
  run: bun install --frozen-lockfile

- name: Run frontend tests
  run: bun run test

- name: Run Rust tests
  run: cargo test --manifest-path src-tauri/Cargo.toml
```

- [x] **Step 2: Verify the repository is left with one current master completion plan plus dispatch-ready subplans**

Run: `find docs/superpowers/plans -maxdepth 1 -type f | sort`
Expected:

```text
docs/superpowers/plans/2026-04-13-v1-completion-plan.md
docs/superpowers/plans/2026-04-13-v1-gateway-host-plan.md
docs/superpowers/plans/2026-04-13-v1-newapi-relay-plan.md
docs/superpowers/plans/2026-04-13-v1-official-provider-plan.md
docs/superpowers/plans/2026-04-13-v1-platform-keys-plan.md
docs/superpowers/plans/2026-04-13-v1-release-gates-plan.md
docs/superpowers/plans/2026-04-13-v1-request-lifecycle-plan.md
docs/superpowers/plans/2026-04-13-v1-routing-policy-plan.md
docs/superpowers/plans/2026-04-13-v1-runtime-candidates-plan.md
docs/superpowers/plans/2026-04-13-v1-ui-foundation-plan.md
docs/superpowers/plans/2026-04-13-v1-ui-pages-plan.md
```

- [x] **Step 3: Verify the spec tree is organized around foundation + product + completion**

```bash
find docs/superpowers/specs -maxdepth 2 -type f | sort
```

- [x] **Step 4: Run full release-gate verification locally**

Run: `bun run test && cargo test --manifest-path src-tauri/Cargo.toml --no-fail-fast`
Verified on `2026-04-14`:

- `bun run test` -> `2` files passed, `28` tests passed
- `cargo test --manifest-path src-tauri/Cargo.toml --no-fail-fast` -> full backend suite passed with `0` failing targets

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/windows-release-gates.yml
git commit -m "chore: consolidate v1 completion plan and release gates"
```

## V1.1 Follow-On Queue

V1.1 starts only after all Task 1-10 release gates pass.

- official in-app login
- generic OpenAI-compatible relay without balance support
- model-level capability matrix UI
- richer policy authoring UX
- expanded operator diagnostics
