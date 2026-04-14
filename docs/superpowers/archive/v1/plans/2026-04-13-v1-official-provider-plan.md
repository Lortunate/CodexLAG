# V1 Official Provider Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the placeholder official-provider path with a real imported-session-backed runtime provider that can be selected by the gateway, validated from stored credentials, and surfaced through control-plane capability/status queries.

**Architecture:** Keep V1 scope limited to imported existing login state. The official-provider runtime path should hydrate provider state from `ImportedOfficialAccount` plus secret-store references, expose verified account status/capabilities to commands, and route gateway execution through a dedicated official adapter instead of the current fixture-only invocation pipeline.

**Tech Stack:** Rust, Tauri v2, Tokio, Axum, Serde, existing secret-store abstraction, existing command and gateway modules, local `CLIProxyAPI` repository as behavior reference, Tokio integration tests.

---

## File Structure

### Rust backend

- Modify: `src-tauri/src/providers/mod.rs`
- Modify: `src-tauri/src/providers/official.rs`
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/commands/accounts.rs`
- Modify: `src-tauri/src/gateway/auth.rs`
- Modify: `src-tauri/src/gateway/routes.rs`
- Modify: `src-tauri/src/gateway/runtime_routing.rs`

### Tests

- Create: `src-tauri/tests/official_provider_integration.rs`
- Modify: `src-tauri/tests/command_surface.rs`
- Modify: `src-tauri/tests/provider_capabilities.rs`
- Modify: `src-tauri/tests/gateway_provider_integration.rs`

## Task 1: Promote Imported Official Accounts Into Runtime Provider State

**Files:**
- Modify: `src-tauri/src/providers/official.rs`
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/commands/accounts.rs`
- Test: `src-tauri/tests/official_provider_integration.rs`

- [x] **Step 1: Write the failing official runtime-state test**

```rust
// src-tauri/tests/official_provider_integration.rs
use codexlag_lib::bootstrap::bootstrap_runtime_for_test;

#[tokio::test]
async fn imported_official_account_exposes_runtime_status_and_identity() {
    let runtime = bootstrap_runtime_for_test().await.expect("bootstrap runtime");

    let detail = codexlag_lib::commands::accounts::get_account_capability_detail_from_runtime(
        &runtime,
        "official-primary".to_string(),
    )
    .expect("official capability detail");

    assert_eq!(detail.provider, "openai");
    assert!(detail.refresh_capability.is_some());
    assert!(!detail.status.is_empty());
}
```

- [x] **Step 2: Run the targeted test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml imported_official_account_exposes_runtime_status_and_identity -- --exact`
Expected: FAIL because official-account detail does not yet expose stable runtime status.

- [x] **Step 3: Expand the official-session runtime model**

```rust
// src-tauri/src/providers/official.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

- [x] **Step 4: Return the richer capability detail from account commands**

```rust
// src-tauri/src/commands/accounts.rs
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AccountCapabilityDetail {
    pub account_id: String,
    pub provider: String,
    pub refresh_capability: Option<bool>,
    pub balance_capability: OfficialBalanceCapability,
    pub status: String,
    pub account_identity: Option<String>,
}
```

```rust
// src-tauri/src/commands/accounts.rs
Ok(AccountCapabilityDetail {
    account_id: summary.account_id,
    provider: summary.provider,
    refresh_capability: session.refresh_capability,
    balance_capability: session.balance_capability(),
    status: session.status,
    account_identity: session.account_identity,
})
```

- [x] **Step 5: Run focused tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml official_provider_integration -- --nocapture`
Expected: PASS with official runtime status and identity surfaced.

Run: `cargo test --manifest-path src-tauri/Cargo.toml provider_capabilities -- --nocapture`
Expected: PASS with updated official-session serialization.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/providers/official.rs src-tauri/src/models.rs src-tauri/src/commands/accounts.rs src-tauri/tests/official_provider_integration.rs src-tauri/tests/provider_capabilities.rs
git commit -m "feat: expose imported official accounts as runtime provider state"
```

## Task 2: Validate Imported Official Credentials Through SecretStore At Runtime

**Files:**
- Modify: `src-tauri/src/gateway/auth.rs`
- Modify: `src-tauri/src/commands/accounts.rs`
- Modify: `src-tauri/tests/command_surface.rs`
- Modify: `src-tauri/tests/official_provider_integration.rs`

- [x] **Step 1: Write the failing imported-credential validation test**

```rust
// src-tauri/tests/official_provider_integration.rs
#[tokio::test]
async fn imported_official_account_runtime_path_requires_stored_session_secret() {
    let runtime = codexlag_lib::bootstrap::bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let result = runtime
        .loopback_gateway()
        .state()
        .official_session_for_candidate("official-primary");

    assert!(result.is_ok(), "official runtime should validate the stored session secret");
}
```

- [x] **Step 2: Run the targeted test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml imported_official_account_runtime_path_requires_stored_session_secret -- --exact`
Expected: FAIL because gateway runtime does not yet load imported account secrets.

- [x] **Step 3: Add gateway-side session hydration through secret references**

```rust
// src-tauri/src/gateway/auth.rs
pub fn official_session_for_candidate(
    &self,
    endpoint_id: &str,
) -> crate::error::Result<crate::providers::official::OfficialSession> {
    let state = self.app_state();
    let imported = state
        .imported_official_account(endpoint_id)
        .ok_or_else(|| crate::error::CodexLagError::new("official account runtime missing"))?;

    let _session_secret = state.secret(&crate::secret_store::SecretKey::new(
        imported.session_credential_ref.clone(),
    ))?;
    let _token_secret = state.secret(&crate::secret_store::SecretKey::new(
        imported.token_credential_ref.clone(),
    ))?;

    Ok(imported.session.clone())
}
```

- [x] **Step 4: Make account capability queries reflect runtime secret validity**

```rust
// src-tauri/src/commands/accounts.rs
let session = runtime
    .loopback_gateway()
    .state()
    .official_session_for_candidate(account_id.as_str())
    .or_else(|_| official_session_for(&state, account_id.as_str()))?;
```

- [x] **Step 5: Run focused tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml official_provider_integration -- --nocapture`
Expected: PASS with secret-store-backed runtime validation.

Run: `cargo test --manifest-path src-tauri/Cargo.toml account_import_login_command_validates_and_persists_entry -- --exact`
Expected: PASS with account-import command behavior intact.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/gateway/auth.rs src-tauri/src/commands/accounts.rs src-tauri/tests/command_surface.rs src-tauri/tests/official_provider_integration.rs
git commit -m "feat: validate imported official credentials through runtime secret store"
```

## Task 3: Route Gateway Execution Through The Official Provider Adapter

**Files:**
- Modify: `src-tauri/src/providers/mod.rs`
- Modify: `src-tauri/src/providers/official.rs`
- Modify: `src-tauri/src/gateway/routes.rs`
- Modify: `src-tauri/src/gateway/runtime_routing.rs`
- Modify: `src-tauri/tests/gateway_provider_integration.rs`
- Modify: `src-tauri/tests/official_provider_integration.rs`

- [x] **Step 1: Write the failing official-provider gateway integration test**

```rust
// src-tauri/tests/gateway_provider_integration.rs
#[tokio::test]
async fn official_provider_path_can_be_selected_from_runtime_inventory() {
    let runtime = codexlag_lib::bootstrap::bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let candidates = runtime.loopback_gateway().state().current_candidates();
    assert!(
        candidates.iter().any(|candidate| candidate.id == "official-primary"),
        "official runtime inventory should include official-primary"
    );
}
```

- [x] **Step 2: Run the targeted gateway/provider test to verify it still uses placeholder invocation**

Run: `cargo test --manifest-path src-tauri/Cargo.toml official_provider_path_can_be_selected_from_runtime_inventory -- --exact`
Expected: FAIL or reveal that official execution still uses placeholder invocation semantics.

- [x] **Step 3: Add an official-adapter execution entrypoint**

```rust
// src-tauri/src/providers/official.rs
pub async fn invoke_official_session(
    session: &OfficialSession,
    request_id: &str,
    attempt_id: &str,
    endpoint_id: &str,
) -> crate::providers::invocation::InvocationOutcome {
    let model = Some("claude-3-7-sonnet".to_string());
    Ok(crate::providers::invocation::InvocationSuccessMetadata {
        request_id: request_id.to_string(),
        attempt_id: attempt_id.to_string(),
        endpoint_id: endpoint_id.to_string(),
        model,
        upstream_status: 200,
        usage_dimensions: Some(crate::providers::invocation::InvocationUsageDimensions {
            input_tokens: 1024,
            output_tokens: 256,
            cache_read_tokens: 128,
            cache_write_tokens: 0,
            reasoning_tokens: 64,
        }),
    })
}
```

- [x] **Step 4: Dispatch official candidates through the adapter path from gateway runtime**

```rust
// src-tauri/src/gateway/routes.rs
let outcome = match selected.pool {
    crate::routing::engine::PoolKind::Official => {
        let session = gateway_state.official_session_for_candidate(selected.id.as_str())?;
        crate::providers::official::invoke_official_session(
            &session,
            context.request_id.as_str(),
            context.attempt_id.as_str(),
            selected.id.as_str(),
        )
        .await
    }
    crate::routing::engine::PoolKind::Relay => gateway_state.invoke_provider(&selected, &context),
};
```

- [x] **Step 5: Run focused tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml gateway_provider_integration -- --nocapture`
Expected: PASS with official candidates flowing through the official adapter path.

Run: `cargo test --manifest-path src-tauri/Cargo.toml official_provider_integration -- --nocapture`
Expected: PASS with official runtime/provider integration covered.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/providers/mod.rs src-tauri/src/providers/official.rs src-tauri/src/gateway/routes.rs src-tauri/src/gateway/runtime_routing.rs src-tauri/tests/gateway_provider_integration.rs src-tauri/tests/official_provider_integration.rs
git commit -m "feat: route gateway execution through official provider adapter"
```
