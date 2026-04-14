# V1 Request Lifecycle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persist request main records, attempt records, and usage detail on the real runtime path, then query that persisted lifecycle data from command surfaces instead of relying only on in-memory usage snapshots.

**Architecture:** Keep runtime diagnostics and business audit records separate, but make the gateway write request lifecycle data to SQLite as part of normal execution. The logging/usage layer should derive UI-facing request detail from persisted request and attempt rows, while runtime commands stop treating `usage_records` as the primary source of truth.

**Tech Stack:** Rust, Tauri v2, Tokio, Rusqlite, Serde, existing request/attempt tables, existing commands/logging modules, Tokio integration tests.

---

## File Structure

### Rust backend

- Modify: `src-tauri/src/db/repositories.rs`
- Modify: `src-tauri/src/logging/usage.rs`
- Modify: `src-tauri/src/gateway/routes.rs`
- Modify: `src-tauri/src/commands/logs.rs`
- Modify: `src-tauri/src/state.rs`

### Tests

- Create: `src-tauri/tests/request_lifecycle_persistence.rs`
- Modify: `src-tauri/tests/request_attempt_logging.rs`
- Modify: `src-tauri/tests/observability_e2e.rs`
- Modify: `src-tauri/tests/command_surface.rs`

## Task 1: Persist Request And Attempt Rows From The Runtime Gateway Path

**Files:**
- Modify: `src-tauri/src/db/repositories.rs`
- Modify: `src-tauri/src/gateway/routes.rs`
- Create: `src-tauri/tests/request_lifecycle_persistence.rs`

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
Expected: FAIL because the runtime path still writes only in-memory usage history.

- [x] **Step 3: Add a repository write entrypoint that is ready for runtime use**

```rust
// src-tauri/src/db/repositories.rs
pub fn append_runtime_request_lifecycle(
    &self,
    request: &RequestLog,
    attempts: &[RequestAttemptLog],
) -> Result<()> {
    self.append_request_with_attempts(request, attempts)
}
```

- [x] **Step 4: Call the persistence path from gateway execution**

```rust
// src-tauri/src/gateway/routes.rs
runtime
    .app_state_mut()
    .repositories_mut()
    .append_runtime_request_lifecycle(&request_log, &attempt_logs)?;
```

- [x] **Step 5: Run focused persistence tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml request_lifecycle_persistence -- --nocapture`
Expected: PASS with runtime requests now creating persisted request and attempt rows.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/db/repositories.rs src-tauri/src/gateway/routes.rs src-tauri/tests/request_lifecycle_persistence.rs
git commit -m "feat: persist request lifecycle records from runtime gateway path"
```

## Task 2: Derive Usage Request Detail From Persisted Lifecycle Rows

**Files:**
- Modify: `src-tauri/src/logging/usage.rs`
- Modify: `src-tauri/src/db/repositories.rs`
- Modify: `src-tauri/tests/request_attempt_logging.rs`
- Modify: `src-tauri/tests/observability_e2e.rs`

- [x] **Step 1: Write the failing persisted-detail derivation test**

```rust
// src-tauri/tests/request_attempt_logging.rs
#[test]
fn request_detail_can_be_derived_from_persisted_request_and_attempt_rows() {
    let request = codexlag_lib::models::RequestLog {
        request_id: "req-1".into(),
        platform_key_id: "key-default".into(),
        request_type: "codex".into(),
        model: "gpt-4o-mini".into(),
        selected_endpoint_id: Some("relay-newapi".into()),
        attempt_count: 1,
        final_status: "success".into(),
        http_status: Some(200),
        started_at_ms: 1_000,
        finished_at_ms: Some(1_120),
        latency_ms: Some(120),
        error_code: None,
        error_reason: None,
        requested_context_window: None,
        requested_context_compression: None,
        effective_context_window: None,
        effective_context_compression: None,
    };

    let attempts = vec![codexlag_lib::models::RequestAttemptLog {
        attempt_id: "req-1:0".into(),
        request_id: "req-1".into(),
        attempt_index: 0,
        endpoint_id: "relay-newapi".into(),
        pool_type: "relay".into(),
        trigger_reason: "primary".into(),
        upstream_status: Some(200),
        timeout_ms: None,
        latency_ms: Some(120),
        token_usage_snapshot: Some("{\"input_tokens\":640}".into()),
        estimated_cost_snapshot: Some("{\"amount\":\"0.0010\"}".into()),
        balance_snapshot_id: None,
        feature_resolution_snapshot: Some("{\"outcome\":\"success\"}".into()),
    }];

    let detail =
        codexlag_lib::logging::usage::usage_request_detail_from_persisted_rows(&request, &attempts);
    assert_eq!(detail.request_id, "req-1");
    assert_eq!(detail.endpoint_id, "relay-newapi");
}
```

- [x] **Step 2: Run the targeted detail test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml request_detail_can_be_derived_from_persisted_request_and_attempt_rows -- --exact`
Expected: FAIL because the usage layer does not yet derive detail from persisted rows.

- [x] **Step 3: Add a persisted-row detail adapter**

```rust
// src-tauri/src/logging/usage.rs
pub fn usage_request_detail_from_persisted_rows(
    request: &crate::models::RequestLog,
    attempts: &[crate::models::RequestAttemptLog],
) -> UsageRequestDetail {
    let final_attempt = attempts
        .iter()
        .max_by_key(|attempt| attempt.attempt_index)
        .expect("at least one attempt");

    UsageRequestDetail {
        request_id: request.request_id.clone(),
        endpoint_id: final_attempt.endpoint_id.clone(),
        model: Some(request.model.clone()),
        input_tokens: 0,
        output_tokens: 0,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        reasoning_tokens: 0,
        total_tokens: 0,
        cost: UsageCost {
            amount: None,
            provenance: UsageProvenance::Unknown,
            is_estimated: false,
        },
        pricing_profile_id: None,
        declared_capability_requirements: None,
        effective_capability_result: final_attempt.feature_resolution_snapshot.clone(),
        final_upstream_status: final_attempt.upstream_status.map(|status| status as u16),
        final_upstream_error_code: request.error_code.clone(),
        final_upstream_error_reason: request.error_reason.clone(),
    }
}
```

- [x] **Step 4: Add repository helpers that load request+attempt bundles**

```rust
// src-tauri/src/db/repositories.rs
pub fn recent_request_details(&self, limit: Option<usize>) -> Result<Vec<UsageRequestDetail>> {
    let rows = self.load_recent_request_bundles(limit)?;
    Ok(rows
        .into_iter()
        .map(|(request, attempts)| crate::logging::usage::usage_request_detail_from_persisted_rows(&request, &attempts))
        .collect())
}
```

- [x] **Step 5: Run focused persistence/detail tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml request_attempt_logging -- --nocapture`
Expected: PASS with persisted detail derivation covered.

Run: `cargo test --manifest-path src-tauri/Cargo.toml observability_e2e -- --nocapture`
Expected: PASS with persisted lifecycle rows driving request detail surfaces.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/logging/usage.rs src-tauri/src/db/repositories.rs src-tauri/tests/request_attempt_logging.rs src-tauri/tests/observability_e2e.rs
git commit -m "feat: derive request detail from persisted lifecycle rows"
```

## Task 3: Move Log Commands To Persisted Lifecycle Queries

**Files:**
- Modify: `src-tauri/src/commands/logs.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/tests/command_surface.rs`
- Modify: `src-tauri/tests/observability_e2e.rs`

- [x] **Step 1: Write the failing persisted-command test**

```rust
// src-tauri/tests/command_surface.rs
#[tokio::test]
async fn usage_commands_read_from_persisted_request_lifecycle_data() {
    let runtime = codexlag_lib::bootstrap::bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let history = codexlag_lib::commands::logs::usage_request_history_from_runtime(&runtime, Some(10));
    assert!(history.is_empty());
}
```

- [x] **Step 2: Run the targeted command-surface test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml usage_commands_read_from_persisted_request_lifecycle_data -- --exact`
Expected: FAIL because log commands still depend on `runtime.usage_records()`.

- [x] **Step 3: Change command handlers to use repository-backed request detail**

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

```rust
// src-tauri/src/commands/logs.rs
pub fn usage_request_detail_from_runtime(
    runtime: &RuntimeState,
    request_id: &str,
) -> Option<UsageRequestDetail> {
    runtime
        .app_state()
        .repositories()
        .request_detail(request_id)
        .expect("request detail")
}
```

- [x] **Step 4: Keep runtime usage memory only as a short-lived observability cache**

```rust
// src-tauri/src/state.rs
pub fn usage_records(&self) -> Vec<UsageRecord> {
    self.usage_records
        .read()
        .expect("runtime usage records lock poisoned")
        .clone()
}
```

- [x] **Step 5: Run final lifecycle/log command tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml command_surface -- --nocapture`
Expected: PASS with command surfaces now backed by persisted lifecycle data.

Run: `cargo test --manifest-path src-tauri/Cargo.toml observability_e2e -- --nocapture`
Expected: PASS with persisted business logs and runtime logs remaining correlatable.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/commands/logs.rs src-tauri/src/state.rs src-tauri/tests/command_surface.rs src-tauri/tests/observability_e2e.rs
git commit -m "feat: query persisted request lifecycle data from log commands"
```
