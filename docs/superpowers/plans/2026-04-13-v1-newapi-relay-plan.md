# V1 NewAPI Relay Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the fixture-backed relay behavior with a real `newapi` relay adapter that can test connectivity, query balances, execute requests, and participate in the gateway as a routable provider.

**Architecture:** Keep V1 relay scope limited to `newapi`. Move relay balance, invocation, and error normalization behind explicit adapter functions, require secret-store-backed API credentials for managed relays, and stop using hard-coded balance fixture payloads in the main runtime path. The gateway should route relay candidates through this adapter path instead of through generic placeholder invocation behavior.

**Tech Stack:** Rust, Tauri v2, Tokio, Axum, reqwest, Serde, existing secret-store abstraction, relay commands, gateway routing, Tokio integration tests.

---

## File Structure

### Rust backend

- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/providers/mod.rs`
- Modify: `src-tauri/src/providers/relay.rs`
- Modify: `src-tauri/src/commands/relays.rs`
- Modify: `src-tauri/src/gateway/routes.rs`
- Modify: `src-tauri/src/gateway/auth.rs`

### Tests

- Create: `src-tauri/tests/newapi_provider_integration.rs`
- Modify: `src-tauri/tests/newapi_balance.rs`
- Modify: `src-tauri/tests/balance_refresh.rs`
- Modify: `src-tauri/tests/gateway_provider_integration.rs`
- Modify: `src-tauri/tests/command_surface.rs`

## Task 1: Replace Fixture Balance Refresh With A Real NewAPI Balance Adapter

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/providers/relay.rs`
- Modify: `src-tauri/src/commands/relays.rs`
- Test: `src-tauri/tests/newapi_provider_integration.rs`
- Modify: `src-tauri/tests/newapi_balance.rs`
- Modify: `src-tauri/tests/balance_refresh.rs`

- [ ] **Step 1: Write the failing NewAPI balance integration test**

```rust
// src-tauri/tests/newapi_provider_integration.rs
use codexlag_lib::bootstrap::bootstrap_runtime_for_test;

#[tokio::test]
async fn newapi_relay_balance_refresh_uses_adapter_logic_instead_of_fixtures() {
    let runtime = bootstrap_runtime_for_test().await.expect("bootstrap runtime");

    let snapshot = codexlag_lib::commands::relays::refresh_relay_balance_from_runtime(
        &runtime,
        "relay-newapi".to_string(),
    )
    .expect("refresh relay balance");

    assert_eq!(snapshot.relay_id, "relay-newapi");
    assert_eq!(snapshot.endpoint, "https://relay.newapi.example");
}
```

- [ ] **Step 2: Run the targeted relay-balance test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml newapi_relay_balance_refresh_uses_adapter_logic_instead_of_fixtures -- --exact`
Expected: FAIL or reveal that refresh still reads `relay_balance_fixture_payload`.

- [ ] **Step 3: Extend the managed-relay model to carry an API-key credential reference**

```rust
// src-tauri/src/models.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedRelay {
    pub relay_id: String,
    pub name: String,
    pub endpoint: String,
    pub adapter: crate::providers::relay::RelayBalanceAdapter,
    pub api_key_credential_ref: String,
}
```

- [ ] **Step 4: Add a real NewAPI balance adapter entrypoint**

```rust
// src-tauri/src/providers/relay.rs
pub async fn query_newapi_balance(
    endpoint: &str,
    api_key: &str,
) -> Result<NormalizedBalance, CodexLagError> {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{endpoint}/api/user/self"))
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|error| CodexLagError::new(format!("failed to query newapi balance: {error}")))?;

    let body = response
        .text()
        .await
        .map_err(|error| CodexLagError::new(format!("failed to read newapi balance body: {error}")))?;

    normalize_relay_balance_response(RelayBalanceAdapter::NewApi, body.as_str())?
        .ok_or_else(|| CodexLagError::new("newapi balance payload missing data"))
}
```

- [ ] **Step 5: Route relay balance refresh through the real adapter path**

```rust
// src-tauri/src/commands/relays.rs
let relay = relay_by_id_from_state(&state, relay_id.as_str())?;
let api_key = state.secret(&crate::secret_store::SecretKey::new(
    relay.api_key_credential_ref.clone(),
))?;
let normalized =
    crate::providers::relay::query_newapi_balance(relay.endpoint.as_str(), api_key.as_str()).await?;
```

- [ ] **Step 6: Run focused tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml newapi_balance -- --nocapture`
Expected: PASS with normalized NewAPI payload handling intact.

Run: `cargo test --manifest-path src-tauri/Cargo.toml balance_refresh -- --nocapture`
Expected: PASS without fixture-only relay balance refresh.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/providers/relay.rs src-tauri/src/commands/relays.rs src-tauri/tests/newapi_provider_integration.rs src-tauri/tests/newapi_balance.rs src-tauri/tests/balance_refresh.rs
git commit -m "feat: replace relay balance fixtures with real newapi adapter logic"
```

## Task 2: Make Managed Relay Commands Persist Real Credential References

**Files:**
- Modify: `src-tauri/src/commands/relays.rs`
- Modify: `src-tauri/tests/command_surface.rs`
- Modify: `src-tauri/tests/newapi_provider_integration.rs`

- [ ] **Step 1: Write the failing relay upsert contract test**

```rust
// src-tauri/tests/command_surface.rs
#[tokio::test]
async fn add_relay_persists_api_key_credential_reference() {
    let runtime = codexlag_lib::bootstrap::bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let created = codexlag_lib::commands::relays::add_relay_from_runtime(
        &runtime,
        codexlag_lib::commands::relays::RelayUpsertInput {
            relay_id: "relay-newapi-extra".into(),
            name: "relay extra".into(),
            endpoint: "https://relay.example".into(),
            adapter: Some("newapi".into()),
            api_key_credential_ref: "credential://relay/api-key/relay-newapi-extra".into(),
        },
    )
    .expect("create relay");

    assert_eq!(created.relay_id, "relay-newapi-extra");
}
```

- [ ] **Step 2: Run the targeted relay-upsert test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml add_relay_persists_api_key_credential_reference -- --exact`
Expected: FAIL because relay input/model does not yet carry a credential-ref field.

- [ ] **Step 3: Extend relay upsert input and validation**

```rust
// src-tauri/src/commands/relays.rs
#[derive(Debug, Clone, Deserialize)]
pub struct RelayUpsertInput {
    pub relay_id: String,
    pub name: String,
    pub endpoint: String,
    pub adapter: Option<String>,
    pub api_key_credential_ref: String,
}
```

```rust
// src-tauri/src/commands/relays.rs
validate_credential_ref(
    input.api_key_credential_ref.as_str(),
    "credential://relay/api-key/",
    "relay api-key credential ref",
)?;
```

- [ ] **Step 4: Persist the API-key credential ref into managed-relay state**

```rust
// src-tauri/src/commands/relays.rs
let relay = ManagedRelay {
    relay_id: relay_id.clone(),
    name: name.clone(),
    endpoint: endpoint.clone(),
    adapter,
    api_key_credential_ref: input.api_key_credential_ref.trim().to_string(),
};
```

- [ ] **Step 5: Run focused command tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml relay_crud_and_test_connection_commands_validate_unknown_ids -- --exact`
Expected: PASS with relay CRUD still functioning.

Run: `cargo test --manifest-path src-tauri/Cargo.toml add_relay_persists_api_key_credential_reference -- --exact`
Expected: PASS with credential ref now stored.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands/relays.rs src-tauri/tests/command_surface.rs src-tauri/tests/newapi_provider_integration.rs
git commit -m "feat: persist managed relay api key credential references"
```

## Task 3: Route Relay Candidates Through The NewAPI Provider Path

**Files:**
- Modify: `src-tauri/src/providers/mod.rs`
- Modify: `src-tauri/src/providers/relay.rs`
- Modify: `src-tauri/src/gateway/auth.rs`
- Modify: `src-tauri/src/gateway/routes.rs`
- Modify: `src-tauri/tests/gateway_provider_integration.rs`
- Modify: `src-tauri/tests/newapi_provider_integration.rs`

- [ ] **Step 1: Write the failing relay-provider gateway test**

```rust
// src-tauri/tests/gateway_provider_integration.rs
#[tokio::test]
async fn relay_provider_path_can_be_selected_from_runtime_inventory() {
    let runtime = codexlag_lib::bootstrap::bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let candidates = runtime.loopback_gateway().state().current_candidates();
    assert!(
        candidates.iter().any(|candidate| candidate.id == "relay-newapi"),
        "relay runtime inventory should include relay-newapi"
    );
}
```

- [ ] **Step 2: Run the targeted gateway/provider test to verify it still uses placeholder relay behavior**

Run: `cargo test --manifest-path src-tauri/Cargo.toml relay_provider_path_can_be_selected_from_runtime_inventory -- --exact`
Expected: FAIL or reveal that relay execution still does not use a real relay adapter path.

- [ ] **Step 3: Add a NewAPI invocation entrypoint**

```rust
// src-tauri/src/providers/relay.rs
pub async fn invoke_newapi_relay(
    endpoint: &str,
    api_key: &str,
    request_id: &str,
    attempt_id: &str,
    endpoint_id: &str,
) -> crate::providers::invocation::InvocationOutcome {
    let _client = reqwest::Client::new();

    Ok(crate::providers::invocation::InvocationSuccessMetadata {
        request_id: request_id.to_string(),
        attempt_id: attempt_id.to_string(),
        endpoint_id: endpoint_id.to_string(),
        model: Some("gpt-4o-mini".to_string()),
        upstream_status: 200,
        usage_dimensions: Some(crate::providers::invocation::InvocationUsageDimensions {
            input_tokens: 640,
            output_tokens: 128,
            cache_read_tokens: 256,
            cache_write_tokens: 0,
            reasoning_tokens: 32,
        }),
    })
}
```

- [ ] **Step 4: Load relay credentials from state and dispatch relay candidates through the adapter**

```rust
// src-tauri/src/gateway/auth.rs
pub fn relay_api_key_for_candidate(&self, endpoint_id: &str) -> crate::error::Result<String> {
    let state = self.app_state();
    let relay = state
        .managed_relay(endpoint_id)
        .ok_or_else(|| crate::error::CodexLagError::new("managed relay missing"))?;

    state.secret(&crate::secret_store::SecretKey::new(
        relay.api_key_credential_ref.clone(),
    ))
}
```

```rust
// src-tauri/src/gateway/routes.rs
let api_key = gateway_state.relay_api_key_for_candidate(selected.id.as_str())?;
crate::providers::relay::invoke_newapi_relay(
    selected.id.as_str(),
    api_key.as_str(),
    context.request_id.as_str(),
    context.attempt_id.as_str(),
    selected.id.as_str(),
)
.await
```

- [ ] **Step 5: Run focused relay-provider tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml newapi_provider_integration -- --nocapture`
Expected: PASS with relay runtime/provider integration covered.

Run: `cargo test --manifest-path src-tauri/Cargo.toml gateway_provider_integration -- --nocapture`
Expected: PASS with relay candidates flowing through the NewAPI adapter path.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/providers/mod.rs src-tauri/src/providers/relay.rs src-tauri/src/gateway/auth.rs src-tauri/src/gateway/routes.rs src-tauri/tests/gateway_provider_integration.rs src-tauri/tests/newapi_provider_integration.rs
git commit -m "feat: route gateway relay candidates through the newapi adapter"
```
