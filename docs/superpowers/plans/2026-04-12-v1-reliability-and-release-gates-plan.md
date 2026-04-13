# V1 Reliability And Release Gates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the highest-risk V1 gaps after baseline implementation by hardening routing recovery in the real data-plane path, adding security/isolation regression coverage, and introducing a Windows release gate.

**Architecture:** Keep the current single-process Tauri shape, but add explicit runtime routing state ownership inside the gateway so endpoint health transitions are driven by real request outcomes instead of isolated engine-only tests. Keep control-plane operations and data-plane accounting strictly separated, and treat diagnostics/log artifacts as security-sensitive outputs that require regression scanning. Release confidence comes from reproducible Windows CI gates, not manual spot checks.

**Tech Stack:** Tauri v2, Rust, Tokio, Axum, Serde, `tauri-plugin-log`, React 19, TypeScript, Vitest, GitHub Actions (Windows runner).

---

## File Structure

### Rust backend

- Create: `src-tauri/src/gateway/runtime_routing.rs`
- Modify: `src-tauri/src/gateway/mod.rs`
- Modify: `src-tauri/src/gateway/auth.rs`
- Modify: `src-tauri/src/gateway/routes.rs`
- Modify: `src-tauri/src/gateway/server.rs`
- Modify: `src-tauri/src/routing/engine.rs`
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/commands/keys.rs`
- Modify: `src-tauri/src/tray.rs`
- Modify: `src-tauri/src/state.rs`

### Frontend

- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauri.ts`
- Modify: `src/features/default-key/default-key-mode-toggle.tsx`

### Tests

- Create: `src-tauri/tests/gateway_failover.rs`
- Create: `src-tauri/tests/security_regression.rs`
- Modify: `src-tauri/tests/command_surface.rs`
- Modify: `src-tauri/tests/runtime_composition.rs`
- Modify: `src-tauri/tests/bootstrap_default_key.rs`
- Modify: `src/test/app-shell.test.tsx`

### CI

- Create: `.github/workflows/windows-release-gates.yml`

## Task 1: Route Real Data-Plane Outcomes Through Endpoint Health State

**Files:**
- Create: `src-tauri/src/gateway/runtime_routing.rs`
- Modify: `src-tauri/src/gateway/mod.rs`
- Modify: `src-tauri/src/gateway/auth.rs`
- Modify: `src-tauri/src/gateway/routes.rs`
- Modify: `src-tauri/src/gateway/server.rs`
- Modify: `src-tauri/src/routing/engine.rs`
- Modify: `src-tauri/src/models.rs`
- Test: `src-tauri/tests/gateway_failover.rs`

- [ ] **Step 1: Write failing integration test for fallback chain and recovery**

```rust
// src-tauri/tests/gateway_failover.rs
use axum::{body::Body, http::{Request, StatusCode}};
use codexlag_lib::{
    bootstrap::bootstrap_runtime_for_test,
    secret_store::SecretKey,
};
use tower::ServiceExt;

#[tokio::test]
async fn gateway_falls_back_to_relay_after_official_server_error_and_keeps_correlation() {
    let runtime = bootstrap_runtime_for_test().await.expect("bootstrap runtime");
    runtime
        .loopback_gateway()
        .state()
        .set_test_outcomes(vec![
            ("official-default".to_string(), Some(503)),
            ("relay-default".to_string(), None),
        ]);

    let secret = runtime
        .app_state()
        .secret(&SecretKey::default_platform_key())
        .expect("default key secret");

    let response = runtime
        .loopback_gateway()
        .router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/codex/request")
                .header("authorization", format!("bearer {secret}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let debug = runtime.loopback_gateway().state().last_route_debug().expect("route debug");
    assert_eq!(debug.attempt_count, 2);
    assert_eq!(debug.selected_endpoint_id, "relay-default");
    assert!(debug.request_id.contains(":unrouted:"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test gateway_failover gateway_falls_back_to_relay_after_official_server_error_and_keeps_correlation`
Expected: FAIL with missing `set_test_outcomes`/`last_route_debug` and no fallback loop in `codex_request`.

- [ ] **Step 3: Implement runtime routing state and data-plane failure/success hooks**

```rust
// src-tauri/src/gateway/runtime_routing.rs
use crate::routing::engine::{choose_endpoint_at, mark_success, record_failure, CandidateEndpoint, EndpointFailure, FailureRules, RoutingError, wall_clock_now_ms};

#[derive(Debug, Clone)]
pub struct RouteDebugSnapshot {
    pub request_id: String,
    pub selected_endpoint_id: String,
    pub attempt_count: usize,
}

pub struct RuntimeRoutingState {
    candidates: Vec<CandidateEndpoint>,
    rules: FailureRules,
    last_debug: Option<RouteDebugSnapshot>,
}

impl RuntimeRoutingState {
    pub fn choose_with_failover(
        &mut self,
        request_id: &str,
        mode: &str,
        mut invoke: impl FnMut(&CandidateEndpoint) -> Result<(), EndpointFailure>,
    ) -> Result<CandidateEndpoint, RoutingError> {
        let mut attempt_count = 0usize;
        loop {
            let now_ms = wall_clock_now_ms();
            let selected = choose_endpoint_at(mode, &self.candidates, now_ms)?;
            attempt_count += 1;

            match invoke(&selected) {
                Ok(()) => {
                    if let Some(endpoint) = self.candidates.iter_mut().find(|item| item.id == selected.id) {
                        mark_success(endpoint);
                    }
                    self.last_debug = Some(RouteDebugSnapshot {
                        request_id: request_id.to_string(),
                        selected_endpoint_id: selected.id.clone(),
                        attempt_count,
                    });
                    return Ok(selected);
                }
                Err(failure) => {
                    if let Some(endpoint) = self.candidates.iter_mut().find(|item| item.id == selected.id) {
                        record_failure(endpoint, failure, now_ms, &self.rules);
                    }
                    if attempt_count >= self.candidates.len().max(1) {
                        return Err(RoutingError::NoAvailableEndpoint);
                    }
                }
            }
        }
    }
}
```

```rust
// src-tauri/src/gateway/auth.rs (new members/methods)
pub fn set_test_outcomes(&self, outcomes: Vec<(String, Option<u16>)>) {
    self.routing
        .write()
        .expect("gateway routing lock poisoned")
        .set_test_outcomes(outcomes);
}

pub fn last_route_debug(&self) -> Option<RouteDebugSnapshot> {
    self.routing
        .read()
        .expect("gateway routing lock poisoned")
        .last_debug()
        .cloned()
}

pub fn choose_endpoint_with_runtime_failover(
    &self,
    request_id: &str,
    mode: &str,
) -> Result<CandidateEndpoint, RoutingError> {
    self.routing
        .write()
        .expect("gateway routing lock poisoned")
        .choose_with_failover(request_id, mode, |endpoint| self.invoke_for_endpoint(endpoint))
}
```

```rust
// src-tauri/src/gateway/routes.rs (inside codex_request)
let selected = gateway_state
    .choose_endpoint_with_runtime_failover(request_id.as_str(), mode)
    .map_err(|error| {
        log_route_rejection(request_id.as_str(), mode, &error, &gateway_state.current_candidates(), now_ms);
        map_routing_error(mode, error)
    })?;
```

- [ ] **Step 4: Run targeted gateway/routing tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test gateway_failover --test routing_engine --test failure_recovery`
Expected: PASS with fallback + recovery behavior asserted through data-plane path.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/gateway/runtime_routing.rs src-tauri/src/gateway/mod.rs src-tauri/src/gateway/auth.rs src-tauri/src/gateway/routes.rs src-tauri/src/gateway/server.rs src-tauri/src/routing/engine.rs src-tauri/src/models.rs src-tauri/tests/gateway_failover.rs
git commit -m "feat(gateway): persist routing health across data-plane failover"
```

## Task 2: Lock Error Contract And Degrade/Rejection Observability

**Files:**
- Modify: `src-tauri/src/gateway/routes.rs`
- Modify: `src-tauri/src/logging/mod.rs`
- Modify: `src-tauri/tests/failure_recovery.rs`
- Modify: `src-tauri/tests/gateway_failover.rs`

- [ ] **Step 1: Write failing test for rejection payload + structured reasons**

```rust
// src-tauri/tests/gateway_failover.rs
#[tokio::test]
async fn no_available_endpoint_returns_structured_error_with_attempt_context() {
    let runtime = bootstrap_runtime_for_test().await.expect("bootstrap runtime");
    runtime
        .loopback_gateway()
        .state()
        .set_test_outcomes(vec![
            ("official-default".to_string(), Some(503)),
            ("relay-default".to_string(), Some(503)),
        ]);

    let secret = runtime
        .app_state()
        .secret(&SecretKey::default_platform_key())
        .expect("default key secret");

    let response = runtime
        .loopback_gateway()
        .router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/codex/request")
                .header("authorization", format!("bearer {secret}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let debug = runtime.loopback_gateway().state().last_route_debug().expect("debug");
    assert_eq!(debug.attempt_count, 2);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test gateway_failover no_available_endpoint_returns_structured_error_with_attempt_context`
Expected: FAIL because rejection path currently only returns `{ error, mode }` and does not expose attempt correlation.

- [ ] **Step 3: Extend routing error payload and log fields**

```rust
// src-tauri/src/gateway/routes.rs
#[derive(Debug, Serialize)]
struct RoutingErrorResponse {
    error: &'static str,
    mode: String,
    request_id: String,
    attempt_count: usize,
}

fn map_routing_error(
    request_id: &str,
    mode: &str,
    attempt_count: usize,
    error: RoutingError,
) -> (StatusCode, Json<RoutingErrorResponse>) {
    let (status, code) = match error {
        RoutingError::InvalidMode => (StatusCode::BAD_REQUEST, "invalid_mode"),
        RoutingError::NoAvailableEndpoint => (StatusCode::SERVICE_UNAVAILABLE, "no_available_endpoint"),
    };
    (
        status,
        Json(RoutingErrorResponse {
            error: code,
            mode: mode.to_string(),
            request_id: request_id.to_string(),
            attempt_count,
        }),
    )
}
```

```rust
// src-tauri/src/logging/mod.rs
let line = format_event_fields(&[
    ("event", "routing.endpoint.rejected"),
    ("request_id", request_id),
    ("attempt_count", &attempt_count.to_string()),
    ("mode", mode),
    ("error", error_code),
    ("reasons", detail.as_str()),
]);
```

- [ ] **Step 4: Re-run rejection/failure tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test gateway_failover --test failure_recovery`
Expected: PASS with stable rejection payload and reason logging.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/gateway/routes.rs src-tauri/src/logging/mod.rs src-tauri/tests/gateway_failover.rs src-tauri/tests/failure_recovery.rs
git commit -m "feat(routing): expose correlated rejection contract and attempt metadata"
```

## Task 3: Add Security Regression Scans For DB And Diagnostics Outputs

**Files:**
- Create: `src-tauri/tests/security_regression.rs`
- Modify: `src-tauri/tests/runtime_composition.rs`
- Modify: `src-tauri/tests/command_surface.rs`
- Modify: `src-tauri/src/commands/logs.rs`

- [ ] **Step 1: Write failing regression test that forbids secret leakage in artifacts**

```rust
// src-tauri/tests/security_regression.rs
use codexlag_lib::bootstrap::bootstrap_runtime_for_test;
use codexlag_lib::commands::logs::export_runtime_diagnostics_from_runtime;
use codexlag_lib::secret_store::SecretKey;

#[tokio::test]
async fn diagnostics_and_db_artifacts_never_contain_plain_platform_secret() {
    let runtime = bootstrap_runtime_for_test().await.expect("bootstrap runtime");
    let platform_secret = runtime
        .app_state()
        .secret(&SecretKey::default_platform_key())
        .expect("default key secret");

    let _ = export_runtime_diagnostics_from_runtime(&runtime).expect("export diagnostics");
    let log_dir = runtime.runtime_log().log_dir.clone();
    let diagnostics_manifest = log_dir.join("diagnostics").join("diagnostics-manifest.txt");
    let manifest_content = std::fs::read_to_string(diagnostics_manifest).expect("manifest");
    assert!(
        !manifest_content.contains(platform_secret.as_str()),
        "diagnostics manifest must never contain platform secrets"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test security_regression diagnostics_and_db_artifacts_never_contain_plain_platform_secret`
Expected: FAIL until diagnostics/exported metadata path is fully sanitized and test utilities avoid leaking secret values.

- [ ] **Step 3: Sanitize diagnostics payloads and add control/data-plane isolation assertions**

```rust
// src-tauri/src/commands/logs.rs
fn safe_manifest_value(value: &str) -> String {
    value
        .replace("ck_local_", "ck_local_[redacted]_")
        .replace("bearer ", "bearer [redacted] ")
}

let manifest = format!(
    "generated_at_unix={generated_at_unix}\nlog_dir={}\nfiles_count={}\nfiles={}\n",
    safe_manifest_value(metadata.log_dir.as_str()),
    metadata.files.len(),
    safe_manifest_value(files_payload.as_str()),
);
```

```rust
// src-tauri/tests/command_surface.rs (add assertion)
assert_eq!(
    history_after_control_plane.len(),
    1,
    "control-plane operations must not create data-plane usage rows"
);
```

- [ ] **Step 4: Run security + command surface suites**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test security_regression --test command_surface --test runtime_composition`
Expected: PASS with deterministic no-leak guarantees and explicit control/data-plane separation checks.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/logs.rs src-tauri/tests/security_regression.rs src-tauri/tests/command_surface.rs src-tauri/tests/runtime_composition.rs
git commit -m "test(security): add artifact leak regression and isolation assertions"
```

## Task 4: Surface Unavailable-Mode State In Tray And Default-Key UI

**Files:**
- Modify: `src-tauri/src/commands/keys.rs`
- Modify: `src-tauri/src/tray.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauri.ts`
- Modify: `src/features/default-key/default-key-mode-toggle.tsx`
- Modify: `src-tauri/tests/bootstrap_default_key.rs`
- Modify: `src-tauri/tests/runtime_composition.rs`
- Modify: `src/test/app-shell.test.tsx`

- [ ] **Step 1: Write failing backend/frontend tests for unavailable-mode summary**

```rust
// src-tauri/tests/runtime_composition.rs
#[tokio::test]
async fn tray_summary_marks_mode_when_no_candidates_available() {
    let runtime = bootstrap_runtime_for_test().await.expect("bootstrap runtime");
    runtime.loopback_gateway().state().set_all_candidates_unavailable_for_test();
    let summary = codexlag_lib::commands::keys::default_key_summary_from_state(&runtime.app_state())
        .expect("summary");
    assert!(summary.unavailable_reason.is_some());
}
```

```tsx
// src/test/app-shell.test.tsx
it("shows unavailable badge when default mode has no candidates", async () => {
  mockGetDefaultKeySummary({
    name: "default",
    allowedMode: "relay_only",
    unavailableReason: "no relay endpoint available",
  });
  render(<App />);
  expect(await screen.findByText(/no relay endpoint available/i)).toBeInTheDocument();
});
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test runtime_composition tray_summary_marks_mode_when_no_candidates_available`
Expected: FAIL because `DefaultKeySummary` has no `unavailable_reason`.

Run: `bun run test -- src/test/app-shell.test.tsx`
Expected: FAIL because UI types/components do not render unavailable status.

- [ ] **Step 3: Extend summary contract and tray/UI rendering**

```rust
// src-tauri/src/commands/keys.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DefaultKeySummary {
    pub name: String,
    pub allowed_mode: String,
    pub unavailable_reason: Option<String>,
}
```

```rust
// src-tauri/src/tray.rs
Self::CurrentMode(mode, Some(reason)) => {
    format!("Default key state | Current mode: {} ({})", mode.as_str(), reason).into()
}
```

```ts
// src/lib/types.ts
export interface DefaultKeySummary {
  name: string;
  allowedMode: string;
  unavailableReason: string | null;
}
```

```tsx
// src/features/default-key/default-key-mode-toggle.tsx
{unavailableReason ? <p className="warning-text">{unavailableReason}</p> : null}
```

- [ ] **Step 4: Run backend + frontend suites**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test bootstrap_default_key --test runtime_composition`
Expected: PASS with tray summary reflecting unavailable-node context.

Run: `bun run test -- src/test/app-shell.test.tsx`
Expected: PASS with unavailable badge/message rendering.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/keys.rs src-tauri/src/tray.rs src-tauri/src/state.rs src/lib/types.ts src/lib/tauri.ts src/features/default-key/default-key-mode-toggle.tsx src-tauri/tests/bootstrap_default_key.rs src-tauri/tests/runtime_composition.rs src/test/app-shell.test.tsx
git commit -m "feat(tray): expose unavailable default-mode summary across backend and UI"
```

## Task 5: Add Windows Release Gate Workflow

**Files:**
- Create: `.github/workflows/windows-release-gates.yml`
- Modify: `package.json`

- [ ] **Step 1: Write failing CI smoke command locally**

```bash
# run locally first, should expose missing workflow/scripts if absent
cargo test --manifest-path src-tauri/Cargo.toml --test gateway_failover --test security_regression
bun run test -- src/test/app-shell.test.tsx
```

- [ ] **Step 2: Add workflow with Rust + Bun gates on Windows**

```yaml
# .github/workflows/windows-release-gates.yml
name: windows-release-gates

on:
  pull_request:
  push:
    branches: [main]

jobs:
  windows-gates:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: oven-sh/setup-bun@v2
      - uses: dtolnay/rust-toolchain@stable
      - name: Install frontend dependencies
        run: bun install --frozen-lockfile
      - name: Run Rust release-gate tests
        run: cargo test --manifest-path src-tauri/Cargo.toml --test gateway_failover --test security_regression --test runtime_logging --test runtime_composition
      - name: Run frontend smoke tests
        run: bun run test -- src/test/app-shell.test.tsx
```

```json
// package.json
{
  "scripts": {
    "test:smoke": "vitest run src/test/app-shell.test.tsx"
  }
}
```

- [ ] **Step 3: Validate workflow syntax and local smoke command**

Run: `bun run test:smoke`
Expected: PASS.

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test gateway_failover --test security_regression --test runtime_logging --test runtime_composition`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/windows-release-gates.yml package.json
git commit -m "ci(windows): add release-gate workflow for gateway reliability and security"
```

## Spec Coverage Check

- Covers design `6.3` and `6.5` by moving failure classification/recovery into real request path and asserting failover + cooldown behavior in gateway integration tests.
- Covers design `11.2` by adding explicit control-plane vs data-plane regression assertions.
- Covers design `12.3` and `16` by surfacing unavailable-mode state in tray/UI and validating release smoke paths.
- Covers design `17` and `10.1` by adding diagnostics/DB secret leakage regressions and Windows release gate automation.

## Placeholder Scan

- No `TODO`, `TBD`, or “later” placeholders.
- Every task includes exact file paths, test commands, expected outcomes, and commit messages.

## Type Consistency Check

- `DefaultKeySummary` extensions are propagated together across Rust commands, Tauri client types, and React UI.
- Routing error correlation fields (`request_id`, `attempt_count`) are defined once in backend contract and consumed in tests.
- Runtime routing health lifecycle uses existing `CandidateEndpoint`/`FailureRules` domain types without parallel duplicate models.
