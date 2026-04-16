# OpenAI Official OAuth Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make a successful OpenAI browser login produce a gateway-usable official account in CodexLAG, with claim-derived plan/subscription metadata projected from `id_token` in a CLIProxyAPI-aligned way.

**Architecture:** Keep the existing PKCE browser login transport in `src-tauri/src/auth/openai.rs`, then add a narrow claim-parsing layer, an OAuth-to-official-account bridge layer, and a capability/inventory projection layer. Reuse the current gateway official-account path instead of inventing a parallel runtime, and keep plan data explicitly marked as claim-derived rather than remote billing truth.

**Tech Stack:** Rust, Tauri v2, Tokio, Serde, Reqwest, React 19, TypeScript, Vitest, Cargo tests.

---

## File Map

- Create: `src-tauri/src/auth/openai_claims.rs`
- Create: `src-tauri/tests/openai_claims.rs`
- Create: `src-tauri/tests/openai_oauth_bridge.rs`
- Modify: `src-tauri/src/auth/mod.rs`
- Modify: `src-tauri/src/auth/openai.rs`
- Modify: `src-tauri/src/auth/session_store.rs`
- Modify: `src-tauri/src/commands/accounts.rs`
- Modify: `src-tauri/src/gateway/auth.rs`
- Modify: `src-tauri/src/providers/official.rs`
- Modify: `src-tauri/src/providers/inventory.rs`
- Modify: `src-tauri/src/models.rs`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauri.ts`
- Modify: `src/features/accounts/accounts-page.tsx`
- Modify: `src/test/tauri.test.ts`

## Parallelization Notes

- Task 1 is the foundation and should land first.
- After Task 1, Task 2 and Task 3 can proceed in parallel if they coordinate on the shared entitlement structs.
- Task 4 depends on Task 2 and Task 3 backend contract completion.
- Task 5 is the final verification and closeout pass.

### Task 1: Add OpenAI Claim Parsing Foundation

**Files:**
- Create: `src-tauri/src/auth/openai_claims.rs`
- Create: `src-tauri/tests/openai_claims.rs`
- Modify: `src-tauri/src/auth/mod.rs`

- [ ] **Step 1: Write the failing Rust tests for OpenAI `id_token` claim parsing**

```rust
use codexlag::auth::openai_claims::{parse_openai_id_token_claims, OpenAiEntitlementSnapshot};

#[test]
fn parses_plan_and_subscription_window_from_openai_id_token_claims() {
    let token = test_openai_jwt(
        r#"{
            "email":"user@example.com",
            "https://api.openai.com/auth":{
                "chatgpt_account_id":"acc_123",
                "chatgpt_plan_type":"pro",
                "chatgpt_subscription_active_start":"2026-04-01T00:00:00Z",
                "chatgpt_subscription_active_until":"2026-05-01T00:00:00Z"
            }
        }"#,
    );

    let claims = parse_openai_id_token_claims(&token).expect("claims should parse");

    assert_eq!(claims.email.as_deref(), Some("user@example.com"));
    assert_eq!(claims.account_id.as_deref(), Some("acc_123"));
    assert_eq!(claims.plan_type.as_deref(), Some("pro"));
    assert_eq!(
        claims.subscription_active_until.as_deref(),
        Some("2026-05-01T00:00:00Z")
    );
}

#[test]
fn returns_empty_snapshot_when_openai_claim_block_is_missing() {
    let token = test_openai_jwt(r#"{"email":"user@example.com"}"#);

    let claims = parse_openai_id_token_claims(&token).expect("claims should parse");

    assert_eq!(
        claims,
        OpenAiEntitlementSnapshot {
            email: Some("user@example.com".into()),
            account_id: None,
            plan_type: None,
            subscription_active_start: None,
            subscription_active_until: None,
            claim_source: "id_token_claim".into(),
        }
    );
}
```

- [ ] **Step 2: Run the new Rust tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml openai_claims -- --nocapture`

Expected: FAIL because `openai_claims` module and parsing helpers do not exist yet.

- [ ] **Step 3: Implement the parser and entitlement snapshot types**

```rust
// src-tauri/src/auth/openai_claims.rs
use base64::Engine;
use serde::Deserialize;

use crate::error::{CodexLagError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAiEntitlementSnapshot {
    pub email: Option<String>,
    pub account_id: Option<String>,
    pub plan_type: Option<String>,
    pub subscription_active_start: Option<String>,
    pub subscription_active_until: Option<String>,
    pub claim_source: String,
}

#[derive(Debug, Deserialize)]
struct JwtClaims {
    #[serde(default)]
    email: Option<String>,
    #[serde(rename = "https://api.openai.com/auth", default)]
    auth: Option<OpenAiAuthClaims>,
}

#[derive(Debug, Deserialize)]
struct OpenAiAuthClaims {
    #[serde(default)]
    chatgpt_account_id: Option<String>,
    #[serde(default)]
    chatgpt_plan_type: Option<String>,
    #[serde(default)]
    chatgpt_subscription_active_start: Option<String>,
    #[serde(default)]
    chatgpt_subscription_active_until: Option<String>,
}

pub fn parse_openai_id_token_claims(id_token: &str) -> Result<OpenAiEntitlementSnapshot> {
    let payload = id_token
        .split('.')
        .nth(1)
        .ok_or_else(|| CodexLagError::new("openai id_token missing payload segment"))?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|error| CodexLagError::new(format!("failed to decode openai id_token: {error}")))?;
    let claims: JwtClaims = serde_json::from_slice(&decoded)
        .map_err(|error| CodexLagError::new(format!("failed to parse openai id_token claims: {error}")))?;
    let auth = claims.auth;

    Ok(OpenAiEntitlementSnapshot {
        email: claims.email,
        account_id: auth.as_ref().and_then(|value| value.chatgpt_account_id.clone()),
        plan_type: auth.as_ref().and_then(|value| value.chatgpt_plan_type.clone()),
        subscription_active_start: auth
            .as_ref()
            .and_then(|value| value.chatgpt_subscription_active_start.clone()),
        subscription_active_until: auth
            .as_ref()
            .and_then(|value| value.chatgpt_subscription_active_until.clone()),
        claim_source: "id_token_claim".into(),
    })
}
```

- [ ] **Step 4: Export the new module through the auth namespace**

```rust
// src-tauri/src/auth/mod.rs
pub mod callback;
pub mod openai;
pub mod openai_claims;
pub mod session_store;
```

- [ ] **Step 5: Run the Rust tests and verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml openai_claims -- --nocapture`

Expected: PASS with both claim parsing tests green.

- [ ] **Step 6: Commit the foundation**

```bash
git add src-tauri/src/auth/mod.rs src-tauri/src/auth/openai_claims.rs src-tauri/tests/openai_claims.rs
git commit -m "feat: add openai claim parsing foundation"
```

### Task 2: Bridge OAuth Sessions Into Gateway-Usable Official Accounts

**Files:**
- Create: `src-tauri/tests/openai_oauth_bridge.rs`
- Modify: `src-tauri/src/auth/openai.rs`
- Modify: `src-tauri/src/commands/accounts.rs`
- Modify: `src-tauri/src/gateway/auth.rs`
- Modify: `src-tauri/src/models.rs`

- [ ] **Step 1: Write the failing integration test for OAuth-to-official-account bridging**

```rust
#[tokio::test]
async fn openai_browser_login_persists_gateway_usable_official_account() {
    let runtime = test_runtime_with_fake_openai_exchange();

    let pending = runtime
        .app_state_mut()
        .openai_auth_runtime()
        .start_default_browser_login("openai-primary".into(), "OpenAI Primary".into())
        .expect("pending login");

    complete_fake_openai_callback(&pending.callback_url, "auth-code-123").await;

    let inventory = crate::commands::accounts::list_provider_inventory_from_runtime(&runtime);
    let account = inventory
        .accounts
        .iter()
        .find(|account| account.account.account_id == "openai-primary")
        .expect("oauth-created official account must be visible");

    assert!(account.account.available);
    assert_eq!(account.account.provider_id, "openai");

    let session = runtime
        .loopback_gateway()
        .state()
        .official_session_for_candidate("openai-primary")
        .expect("gateway should accept bridged account");

    assert_eq!(session.account_identity.as_deref(), Some("user@example.com"));
}
```

- [ ] **Step 2: Run the integration test and verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml openai_browser_login_persists_gateway_usable_official_account -- --nocapture`

Expected: FAIL because successful OAuth login does not currently create an imported official account.

- [ ] **Step 3: Add a normalized entitlement model to backend response structs**

```rust
// src-tauri/src/models.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountEntitlementSummary {
    pub plan_type: Option<String>,
    pub subscription_active_start: Option<String>,
    pub subscription_active_until: Option<String>,
    pub claim_source: Option<String>,
}
```

- [ ] **Step 4: Bridge successful code exchange into imported official account persistence**

```rust
// inside persist_exchanged_openai_session(...)
let stored = ProviderSessionStore::load(&app_state, OPENAI_PROVIDER_ID, request.account_id.as_str())?
    .expect("persisted session must reload");
let entitlement = stored
    .token_secret_struct()
    .id_token
    .as_deref()
    .map(parse_openai_id_token_claims)
    .transpose()?;

let account_identity = entitlement
    .as_ref()
    .and_then(|value| value.email.clone());

app_state.save_imported_official_account(ImportedOfficialAccount {
    account_id: request.account_id.clone(),
    name: request.display_name.clone(),
    provider: "openai".into(),
    session: OfficialSession {
        session_id: format!("session:{}", request.account_id),
        account_identity,
        auth_mode: Some(OfficialAuthMode::BrowserOauthPkce),
        refresh_capability: Some(true),
        quota_capability: None,
        last_verified_at_ms: Some(now_ms()),
        status: "active".into(),
        entitlement: entitlement.map(Into::into),
    },
    session_credential_ref: format!("credential://auth/openai_official/session/{}", request.account_id),
    token_credential_ref: format!("credential://auth/openai_official/token/{}", request.account_id),
})?;
```

- [ ] **Step 5: Update gateway official-session loading to accept bridged OAuth-backed official accounts**

```rust
// src-tauri/src/gateway/auth.rs
let imported = state
    .imported_official_account(endpoint_id)
    .ok_or_else(|| CodexLagError::new("official account runtime missing"))?;

let _session_secret = state.secret(&SecretKey::new(imported.session_credential_ref.clone()))?;
let _token_secret = state.secret(&SecretKey::new(imported.token_credential_ref.clone()))?;

Ok(imported.session.clone())
```

Constraint: do not reintroduce `credential://official/...` validation in the gateway; let the import path accept the bridged `credential://auth/...` references when `auth_mode == browser_oauth_pkce`.

- [ ] **Step 6: Relax import validation so browser OAuth official accounts can reference the bridged secret path**

```rust
fn validate_official_secret_ref(value: &str, field: &str, auth_mode: Option<&str>) -> Result<()> {
    let prefixes = if auth_mode == Some("browser_oauth_pkce") {
        &["credential://auth/openai_official/session/", "credential://auth/openai_official/token/"][..]
    } else {
        &["credential://official/session/", "credential://official/token/"][..]
    };

    if prefixes.iter().any(|prefix| value.starts_with(prefix)) {
        return Ok(());
    }

    Err(invalid_payload_error(
        format!("{field} has unsupported credential ref").as_str(),
        format!("command=account_import_validation;field={field};value=invalid").as_str(),
    ))
}
```

- [ ] **Step 7: Re-run the bridge integration test and existing OAuth tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml openai_ -- --nocapture`

Expected: PASS for `openai_auth_flow`, `openai_session_refresh`, and the new bridge test.

- [ ] **Step 8: Commit the runtime bridge**

```bash
git add src-tauri/src/auth/openai.rs src-tauri/src/commands/accounts.rs src-tauri/src/gateway/auth.rs src-tauri/src/models.rs src-tauri/tests/openai_oauth_bridge.rs
git commit -m "feat: bridge oauth sessions into official accounts"
```

### Task 3: Project Plan and Status Data Through Capability and Inventory APIs

**Files:**
- Modify: `src-tauri/src/providers/official.rs`
- Modify: `src-tauri/src/providers/inventory.rs`
- Modify: `src-tauri/src/commands/accounts.rs`
- Modify: `src-tauri/tests/official_provider_integration.rs`
- Modify: `src-tauri/tests/provider_inventory.rs`

- [ ] **Step 1: Write the failing backend tests for plan projection**

```rust
#[test]
fn capability_detail_exposes_claim_derived_plan_metadata() {
    let runtime = runtime_with_imported_openai_account(Some("pro"));

    let detail = crate::commands::accounts::get_account_capability_detail_from_runtime(
        &runtime,
        "openai-primary".into(),
    )
    .expect("capability detail");

    assert_eq!(detail.entitlement.plan_type.as_deref(), Some("pro"));
    assert_eq!(detail.entitlement.claim_source.as_deref(), Some("id_token_claim"));
    assert_eq!(detail.balance_capability.status_code(), "non_queryable");
}

#[test]
fn provider_inventory_marks_oauth_created_account_available() {
    let runtime = runtime_with_imported_openai_account(Some("plus"));

    let inventory = crate::commands::accounts::list_provider_inventory_from_runtime(&runtime);
    let account = inventory
        .accounts
        .iter()
        .find(|item| item.account.account_id == "openai-primary")
        .expect("account must exist");

    assert!(account.account.available);
    assert_eq!(account.account.status.as_deref(), Some("active"));
}
```

- [ ] **Step 2: Run the backend tests and verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml capability_detail_exposes_claim_derived_plan_metadata provider_inventory_marks_oauth_created_account_available -- --nocapture`

Expected: FAIL because entitlement fields are not exposed yet.

- [ ] **Step 3: Extend official session structs to carry entitlement metadata**

```rust
// src-tauri/src/providers/official.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct OfficialEntitlement {
    pub plan_type: Option<String>,
    pub subscription_active_start: Option<String>,
    pub subscription_active_until: Option<String>,
    pub claim_source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OfficialSession {
    pub session_id: String,
    pub account_identity: Option<String>,
    pub auth_mode: Option<OfficialAuthMode>,
    pub refresh_capability: Option<bool>,
    pub quota_capability: Option<bool>,
    pub last_verified_at_ms: Option<i64>,
    pub status: String,
    #[serde(default)]
    pub entitlement: OfficialEntitlement,
}
```

- [ ] **Step 4: Replace hardcoded quota semantics with derived semantics**

```rust
// src-tauri/src/commands/accounts.rs
Ok(AccountCapabilityDetail {
    account_id: summary.account_id,
    provider: summary.provider,
    refresh_capability: session.refresh_capability,
    balance_capability: session.balance_capability(),
    status: session.status,
    account_identity: session.account_identity,
    entitlement: session.entitlement,
})
```

Constraint: keep `balance_capability` as `NonQueryable` for official OpenAI accounts; do not invent a remote balance endpoint.

- [ ] **Step 5: Update inventory projection to surface active/degraded state and entitlement-backed availability**

```rust
let available = adapter.is_some()
    && account.session.status == "active"
    && token_secret.is_some()
    && (!requires_session_secret(canonical_provider_id) || has_session_secret);
```

Also project `status` and `plan_type` through the account summary shape used by the frontend.

- [ ] **Step 6: Re-run provider integration tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml official_provider_integration provider_inventory -- --nocapture`

Expected: PASS with entitlement and availability assertions green.

- [ ] **Step 7: Commit the capability and inventory changes**

```bash
git add src-tauri/src/providers/official.rs src-tauri/src/providers/inventory.rs src-tauri/src/commands/accounts.rs src-tauri/tests/official_provider_integration.rs src-tauri/tests/provider_inventory.rs
git commit -m "feat: project openai entitlement data through account APIs"
```

### Task 4: Surface Status and Plan Metadata in the Desktop UI

**Files:**
- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauri.ts`
- Modify: `src/features/accounts/accounts-page.tsx`
- Modify: `src/test/tauri.test.ts`

- [ ] **Step 1: Write the failing frontend tests**

```ts
it("shows claim-derived openai plan metadata on the accounts page", async () => {
  invokeMock.mockResolvedValueOnce([
    {
      account_id: "openai-primary",
      provider: "openai",
      refresh_capability: true,
      balance_capability: { kind: "non_queryable", reason: "official accounts do not expose a balance endpoint" },
      status: "active",
      account_identity: "user@example.com",
      entitlement: {
        plan_type: "pro",
        subscription_active_start: "2026-04-01T00:00:00Z",
        subscription_active_until: "2026-05-01T00:00:00Z",
        claim_source: "id_token_claim",
      },
    },
  ]);

  render(<AccountsPage />);

  expect(await screen.findByText("Plan: pro")).toBeInTheDocument();
  expect(screen.getByText("Source: id_token_claim")).toBeInTheDocument();
});
```

- [ ] **Step 2: Run the frontend test and verify it fails**

Run: `bun x vitest run src/test/tauri.test.ts`

Expected: FAIL because the TypeScript contracts and UI do not expose entitlement fields yet.

- [ ] **Step 3: Extend frontend contracts**

```ts
export interface AccountEntitlementSummary {
  plan_type: string | null;
  subscription_active_start: string | null;
  subscription_active_until: string | null;
  claim_source: string | null;
}

export interface AccountCapabilityDetail {
  account_id: string;
  provider: string;
  refresh_capability: boolean | null;
  balance_capability: AccountBalanceAvailability;
  status: string;
  account_identity: string | null;
  entitlement: AccountEntitlementSummary;
}
```

- [ ] **Step 4: Render plan/status metadata with explicit trust language**

```tsx
{panel.capabilityDetail?.entitlement.plan_type ? (
  <p>Plan: {panel.capabilityDetail.entitlement.plan_type}</p>
) : null}
{panel.capabilityDetail?.entitlement.claim_source ? (
  <p>Source: {panel.capabilityDetail.entitlement.claim_source}</p>
) : null}
{panel.capabilityDetail?.entitlement.subscription_active_until ? (
  <p>Active until: {panel.capabilityDetail.entitlement.subscription_active_until}</p>
) : null}
```

Constraint: do not label these values as “billing”, “wallet”, or “authoritative subscription state”.

- [ ] **Step 5: Re-run the frontend tests**

Run: `bun x vitest run src/test/tauri.test.ts`

Expected: PASS with entitlement rendering assertions green.

- [ ] **Step 6: Commit the UI contract changes**

```bash
git add src/lib/types.ts src/lib/tauri.ts src/features/accounts/accounts-page.tsx src/test/tauri.test.ts
git commit -m "feat: show claim-derived openai plan metadata"
```

### Task 5: Persist Degraded States and Run Final Verification

**Files:**
- Modify: `src-tauri/src/auth/openai.rs`
- Modify: `src-tauri/src/commands/accounts.rs`
- Modify: `src/features/accounts/accounts-page.tsx`
- Modify: `src-tauri/tests/openai_auth_flow.rs`
- Modify: `src-tauri/tests/openai_session_refresh.rs`
- Modify: `src/test/tauri.test.ts`

- [ ] **Step 1: Write failing tests for degraded and reauth-required state handling**

```rust
#[tokio::test]
async fn callback_state_mismatch_marks_session_reauth_required() {
    let runtime = test_runtime_with_fake_openai_exchange();

    let pending = runtime
        .app_state_mut()
        .openai_auth_runtime()
        .start_default_browser_login("openai-primary".into(), "OpenAI Primary".into())
        .expect("pending login");

    complete_fake_openai_callback_with_wrong_state(&pending.callback_url, "bad-state").await;

    let sessions = runtime.list_provider_sessions().expect("sessions");
    assert_eq!(sessions[0].auth_state, "reauth_required");
    assert!(sessions[0].last_refresh_error.is_some());
}
```

- [ ] **Step 2: Run degraded-state tests and verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml openai_session_refresh openai_auth_flow -- --nocapture`

Expected: FAIL because callback errors are not currently persisted as session state.

- [ ] **Step 3: Persist callback and refresh failures into session state**

```rust
fn persist_refresh_error(&mut self, account_id: &str, message: String) -> Result<()> {
    if let Some(mut stored) = ProviderSessionStore::load(self.app_state(), OPENAI_PROVIDER_ID, account_id)? {
        stored.summary.auth_state = "reauth_required".into();
        stored.summary.last_refresh_error = Some(message);
        ProviderSessionStore::persist(self.app_state_mut(), stored)?;
    }
    Ok(())
}
```

Use the same pattern when callback processing fails due to state mismatch, missing code, or malformed token claims.

- [ ] **Step 4: Re-run targeted backend and frontend verification**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml openai_ -- --nocapture
bun x vitest run src/test/tauri.test.ts
```

Expected: PASS for auth, refresh, bridge, inventory, and UI tests.

- [ ] **Step 5: Run repo-level auth-focused regression verification**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml official_provider_integration provider_inventory command_surface -- --nocapture
bun x vitest run src/test/app-shell.test.tsx src/test/tauri.test.ts
```

Expected: PASS with no regressions in official account commands or desktop surface bindings.

- [ ] **Step 6: Run GitNexus change detection before commit**

Run the project’s required pre-commit scope check and confirm only expected auth, provider, inventory, and UI symbols changed.

- [ ] **Step 7: Commit the degraded-state and verification pass**

```bash
git add src-tauri/src/auth/openai.rs src-tauri/src/commands/accounts.rs src/features/accounts/accounts-page.tsx src-tauri/tests/openai_auth_flow.rs src-tauri/tests/openai_session_refresh.rs src/test/tauri.test.ts
git commit -m "feat: persist degraded openai oauth session state"
```

## Self-Review Summary

- Spec coverage: this plan covers the OAuth bridge, claim parsing, capability projection, UI surfacing, degraded-state handling, and verification requirements from the approved spec.
- Placeholder scan: no `TODO`, `TBD`, or deferred “handle later” language remains in the tasks.
- Type consistency: `OpenAiEntitlementSnapshot`, `OfficialEntitlement`, and `AccountEntitlementSummary` are the shared naming line across backend and frontend tasks.

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-04-17-openai-oauth-completion.md`.

Two execution options:

1. Subagent-Driven (recommended) - dispatch a fresh subagent per task, review between tasks, fast iteration
2. Inline Execution - execute tasks in one session using executing-plans with checkpoints
