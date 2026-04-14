# V1 Platform Key Issuance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make platform keys real local credentials by issuing secrets, persisting them in the secret store, exposing the initial secret to the UI once, and ensuring newly created keys can authenticate against the loopback gateway.

**Architecture:** Extend the existing platform-key model to carry stable metadata while keeping the raw secret in `SecretStore`. The bootstrap path continues to create `default key`, but key issuance becomes a shared primitive used by both bootstrap and user-created keys. Backend command results and frontend types are updated so the one-time secret can be surfaced safely at creation time.

**Tech Stack:** Rust, Tauri v2, Serde, Rusqlite, existing secret-store abstraction, React/TypeScript contract types, Vitest, Tokio integration tests.

**Status:** Completed locally on 2026-04-14. Historical implementation steps are retained for traceability; checkbox state below reflects the completed repository state.

---

## File Structure

### Rust backend

- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/bootstrap.rs`
- Modify: `src-tauri/src/secret_store.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/db/migrations.rs`
- Modify: `src-tauri/src/db/repositories.rs`
- Modify: `src-tauri/src/commands/keys.rs`
- Modify: `src-tauri/src/gateway/auth.rs`

### Frontend

- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauri.ts`
- Modify: `src/features/keys/keys-page.tsx`
- Modify: `src/features/keys/key-management-panel.tsx`
- Modify: `src/test/tauri.test.ts`
- Modify: `src/test/app-shell.test.tsx`

### Tests

- Create: `src-tauri/tests/platform_key_issuance.rs`
- Modify: `src-tauri/tests/bootstrap_default_key.rs`
- Modify: `src-tauri/tests/secret_store_persistence.rs`
- Modify: `src-tauri/tests/gateway_auth.rs`
- Modify: `src-tauri/tests/command_surface.rs`

## Task 1: Extend Platform Key Persistence For Real Issuance

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/bootstrap.rs`
- Modify: `src-tauri/src/db/migrations.rs`
- Modify: `src-tauri/src/db/repositories.rs`
- Test: `src-tauri/tests/bootstrap_default_key.rs`

- [x] **Step 1: Write the failing metadata test**

```rust
// src-tauri/tests/bootstrap_default_key.rs
#[tokio::test]
async fn bootstrap_default_key_contains_runtime_metadata_fields() {
    let state = codexlag_lib::bootstrap::bootstrap_state_for_test()
        .await
        .expect("bootstrap");

    let key = state
        .get_platform_key_by_name("default")
        .expect("default key");

    assert_eq!(key.key_prefix, "ck_local_");
    assert!(key.created_at_ms > 0);
    assert_eq!(key.last_used_at_ms, None);
}
```

- [x] **Step 2: Run the targeted test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bootstrap_default_key_contains_runtime_metadata_fields -- --exact`
Expected: FAIL because `PlatformKey` does not yet carry issuance metadata.

- [x] **Step 3: Expand the platform-key model**

```rust
// src-tauri/src/models.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

- [x] **Step 4: Update bootstrap and persistence to store the expanded metadata**

```rust
// src-tauri/src/bootstrap.rs
let default_key = PlatformKey {
    id: DEFAULT_PLATFORM_KEY_ID.into(),
    name: DEFAULT_PLATFORM_KEY_NAME.into(),
    key_prefix: DEFAULT_PLATFORM_KEY_SECRET_PREFIX.into(),
    allowed_mode: HYBRID.into(),
    policy_id: default_policy.id.clone(),
    enabled: true,
    created_at_ms: now_ms(),
    last_used_at_ms: None,
};
```

```rust
// src-tauri/src/db/migrations.rs
ALTER TABLE platform_keys ADD COLUMN key_prefix TEXT NOT NULL DEFAULT 'ck_local_';
ALTER TABLE platform_keys ADD COLUMN created_at_ms INTEGER NOT NULL DEFAULT 0;
ALTER TABLE platform_keys ADD COLUMN last_used_at_ms INTEGER NULL;
```

- [x] **Step 5: Run focused tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bootstrap_default_key -- --nocapture`
Expected: PASS with expanded default-key metadata.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/bootstrap.rs src-tauri/src/db/migrations.rs src-tauri/src/db/repositories.rs src-tauri/tests/bootstrap_default_key.rs
git commit -m "feat: persist platform key issuance metadata"
```

## Task 2: Generate And Store Secrets For Newly Created Platform Keys

**Files:**
- Modify: `src-tauri/src/bootstrap.rs`
- Modify: `src-tauri/src/secret_store.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/commands/keys.rs`
- Test: `src-tauri/tests/platform_key_issuance.rs`
- Modify: `src-tauri/tests/secret_store_persistence.rs`
- Modify: `src-tauri/tests/command_surface.rs`

- [x] **Step 1: Write the failing issuance test**

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

- [x] **Step 2: Run the targeted test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml create_platform_key_issues_a_real_secret_and_stores_it -- --exact`
Expected: FAIL because key creation still only inserts metadata.

- [x] **Step 3: Add a reusable platform-key secret generator**

```rust
// src-tauri/src/bootstrap.rs
pub fn generate_platform_key_secret() -> String {
    let mut bytes = [0_u8; 24];
    rand::rngs::OsRng.fill_bytes(&mut bytes);

    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push_str(&format!("{byte:02x}"));
    }

    format!("{DEFAULT_PLATFORM_KEY_SECRET_PREFIX}{encoded}")
}
```

- [x] **Step 4: Return a secret-bearing result from the create-key command and persist the secret**

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

```rust
// src-tauri/src/commands/keys.rs
let secret = crate::bootstrap::generate_platform_key_secret();
app_state.store_secret(&SecretKey::platform_key(key_id.clone()), secret.clone())?;

Ok(CreatedPlatformKey {
    id: key_id,
    name,
    policy_id,
    allowed_mode,
    enabled: true,
    secret,
})
```

- [x] **Step 5: Run focused secret issuance tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml platform_key_issuance -- --nocapture`
Expected: PASS with stored secret and returned one-time secret.

Run: `cargo test --manifest-path src-tauri/Cargo.toml secret_store_persistence -- --nocapture`
Expected: PASS with secret-store contract unchanged.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/bootstrap.rs src-tauri/src/secret_store.rs src-tauri/src/state.rs src-tauri/src/commands/keys.rs src-tauri/tests/platform_key_issuance.rs src-tauri/tests/secret_store_persistence.rs src-tauri/tests/command_surface.rs
git commit -m "feat: issue and store new platform key secrets"
```

## Task 3: Prove New Keys Can Authenticate And Expose The Secret In The UI

**Files:**
- Modify: `src-tauri/src/gateway/auth.rs`
- Modify: `src-tauri/tests/gateway_auth.rs`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauri.ts`
- Modify: `src/features/keys/keys-page.tsx`
- Modify: `src/features/keys/key-management-panel.tsx`
- Modify: `src/test/tauri.test.ts`
- Modify: `src/test/app-shell.test.tsx`

- [x] **Step 1: Write the failing auth-path test for a newly created key**

```rust
// src-tauri/tests/gateway_auth.rs
#[tokio::test]
async fn newly_created_platform_key_can_authenticate_against_the_gateway() {
    let runtime = codexlag_lib::bootstrap::bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let created = codexlag_lib::commands::keys::create_platform_key_from_runtime(
        &runtime,
        codexlag_lib::commands::keys::CreatePlatformKeyInput {
            key_id: "key-secondary".into(),
            name: "secondary".into(),
            policy_id: "policy-default".into(),
            allowed_mode: "hybrid".into(),
        },
    )
    .expect("create platform key");

    let router = runtime.loopback_gateway().router();
    let response = router
        .oneshot(
            axum::http::Request::builder()
                .uri("/health")
                .header("authorization", format!("bearer {}", created.secret))
                .body(axum::body::Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
}
```

- [x] **Step 2: Run the targeted auth test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml newly_created_platform_key_can_authenticate_against_the_gateway -- --exact`
Expected: FAIL because newly created keys are not yet fully wired into the auth path.

- [x] **Step 3: Keep auth lookup aligned with the secret-store-backed key inventory**

```rust
// src-tauri/src/gateway/auth.rs
fn authenticate_platform_key(&self, provided_secret: &str) -> Option<PlatformKey> {
    self.app_state().authenticate_platform_key(provided_secret)
}
```

- [x] **Step 4: Update frontend contracts and show the secret once after creation**

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
```

```tsx
// src/features/keys/keys-page.tsx
{createdKey ? (
  <div role="status">
    <h3>Generated secret</h3>
    <code>{createdKey.secret}</code>
  </div>
) : null}
```

- [x] **Step 5: Run focused backend and frontend tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml gateway_auth -- --nocapture`
Expected: PASS with newly created key authentication covered.

Run: `bun run test -- src/test/tauri.test.ts src/test/app-shell.test.tsx`
Expected: PASS with UI contract updated to expose the one-time secret.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/gateway/auth.rs src-tauri/tests/gateway_auth.rs src/lib/types.ts src/lib/tauri.ts src/features/keys/keys-page.tsx src/features/keys/key-management-panel.tsx src/test/tauri.test.ts src/test/app-shell.test.tsx
git commit -m "feat: expose and verify one-time platform key secret issuance"
```
