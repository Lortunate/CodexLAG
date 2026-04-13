# Runtime Observability Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add production-grade Tauri runtime observability: terminal + file logs, structured gateway/routing events, log metadata commands, and UI diagnostics visibility.

**Architecture:** Keep business usage logs (`UsageRecord`) and runtime diagnostics logs separate. Runtime diagnostics are emitted through `tauri-plugin-log` + `log` macros and tagged with stable event names and request correlation IDs. The control plane exposes log-directory metadata to the UI through dedicated commands without exposing any secret material.

**Tech Stack:** Tauri v2, Rust, `tauri-plugin-log`, `log`, Axum, Serde, React, TypeScript, Vitest, Tokio integration tests.

---

## File Structure

### Rust backend

- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/bootstrap.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/logging/mod.rs`
- Create: `src-tauri/src/logging/runtime.rs`
- Modify: `src-tauri/src/gateway/routes.rs`
- Modify: `src-tauri/src/commands/logs.rs`
- Modify: `src-tauri/src/commands/mod.rs`

### Frontend

- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauri.ts`
- Modify: `src/features/overview/overview-page.tsx`

### Tests

- Create: `src-tauri/tests/runtime_logging.rs`
- Modify: `src-tauri/tests/runtime_composition.rs`
- Modify: `src-tauri/tests/command_surface.rs`
- Modify: `src/test/app-shell.test.tsx`

## Task 1: Wire Tauri Runtime Logging Plugin And Runtime Log Location

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/bootstrap.rs`
- Modify: `src-tauri/src/state.rs`
- Test: `src-tauri/tests/runtime_logging.rs`

- [ ] **Step 1: Write the failing runtime log location test**

```rust
// src-tauri/tests/runtime_logging.rs
use codexlag_lib::bootstrap::{runtime_database_path, runtime_log_dir};

#[test]
fn runtime_log_dir_uses_app_local_data_logs_subfolder() {
    let app_local_data_dir = std::path::Path::new("/tmp/codexlag-app");

    assert_eq!(
        runtime_log_dir(app_local_data_dir),
        std::path::PathBuf::from("/tmp/codexlag-app").join("logs")
    );
    assert_eq!(
        runtime_database_path(app_local_data_dir),
        std::path::PathBuf::from("/tmp/codexlag-app").join("codexlag.sqlite3")
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test runtime_logging runtime_log_dir_uses_app_local_data_logs_subfolder`
Expected: FAIL with unresolved import/function `runtime_log_dir`.

- [ ] **Step 3: Implement plugin dependency, capability, and runtime log path plumbing**

```toml
# src-tauri/Cargo.toml
[dependencies]
tauri-plugin-log = "2"
log = "0.4"
```

```json
// src-tauri/capabilities/default.json
{
  "identifier": "default",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:menu:default",
    "core:tray:default",
    "log:default"
  ]
}
```

```rust
// src-tauri/src/bootstrap.rs
pub fn runtime_log_dir(app_local_data_dir: impl AsRef<Path>) -> PathBuf {
    app_local_data_dir.as_ref().join("logs")
}
```

```rust
// src-tauri/src/state.rs
#[derive(Debug, Clone)]
pub struct RuntimeLogConfig {
    pub log_dir: std::path::PathBuf,
}

#[derive(Clone)]
pub struct RuntimeState {
    app_state: Arc<RwLock<AppState>>,
    usage_records: Arc<RwLock<Vec<UsageRecord>>>,
    loopback_gateway: LoopbackGateway,
    runtime_log: RuntimeLogConfig,
}
```

```rust
// src-tauri/src/lib.rs
use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .target(Target::new(TargetKind::Stdout))
                .target(Target::new(TargetKind::LogDir {
                    file_name: Some("gateway".to_string()),
                }))
                .max_file_size(10_000_000)
                .rotation_strategy(RotationStrategy::KeepAll)
                .timezone_strategy(TimezoneStrategy::UseLocal)
                .build(),
        )
        // existing setup + invoke_handler...
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test runtime_logging`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/capabilities/default.json src-tauri/src/lib.rs src-tauri/src/bootstrap.rs src-tauri/src/state.rs src-tauri/tests/runtime_logging.rs
git commit -m "feat(logging): bootstrap tauri runtime logging targets and log dir metadata"
```

## Task 2: Add Structured Runtime Event API With Redaction

**Files:**
- Create: `src-tauri/src/logging/runtime.rs`
- Modify: `src-tauri/src/logging/mod.rs`
- Test: `src-tauri/tests/runtime_logging.rs`

- [ ] **Step 1: Write failing redaction and event-format tests**

```rust
// src-tauri/tests/runtime_logging.rs
use codexlag_lib::logging::runtime::{format_event_fields, redact_secret_value};

#[test]
fn redact_secret_value_masks_sensitive_tokens() {
    assert_eq!(redact_secret_value(""), "");
    assert_eq!(redact_secret_value("abcd"), "****");
    assert_eq!(redact_secret_value("ck_local_1234567890"), "ck_l****************");
}

#[test]
fn format_event_fields_outputs_stable_key_value_pairs() {
    let line = format_event_fields(&[
        ("event", "gateway.request.accepted"),
        ("request_id", "req-1"),
        ("endpoint_id", "relay-default"),
    ]);

    assert!(line.contains("event=gateway.request.accepted"));
    assert!(line.contains("request_id=req-1"));
    assert!(line.contains("endpoint_id=relay-default"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test runtime_logging redact_secret_value_masks_sensitive_tokens`
Expected: FAIL because `logging::runtime` module and helpers do not exist.

- [ ] **Step 3: Implement runtime event helpers and migrate route logging to `log` macros**

```rust
// src-tauri/src/logging/runtime.rs
pub fn redact_secret_value(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    if value.len() <= 4 {
        return "****".to_string();
    }
    let prefix = &value[..4];
    format!("{prefix}{}", "*".repeat(value.len() - 4))
}

pub fn format_event_fields(fields: &[(&str, &str)]) -> String {
    fields
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(" ")
}
```

```rust
// src-tauri/src/logging/mod.rs
pub mod runtime;
pub mod usage;

pub fn log_route_downgrade(/* existing args */) {
    // ...
    log::warn!(
        "{}",
        runtime::format_event_fields(&[
            ("event", "routing.endpoint.downgraded"),
            ("mode", mode),
            ("selected", selected.id.as_str()),
            ("reasons", reasons.join(",").as_str()),
        ])
    );
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test runtime_logging`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/logging/mod.rs src-tauri/src/logging/runtime.rs src-tauri/tests/runtime_logging.rs
git commit -m "feat(logging): add structured runtime event helpers and redaction primitives"
```

## Task 3: Instrument Gateway Routing Flow With Correlation IDs

**Files:**
- Modify: `src-tauri/src/gateway/routes.rs`
- Modify: `src-tauri/src/logging/mod.rs`
- Modify: `src-tauri/tests/command_surface.rs`
- Test: `src-tauri/tests/runtime_logging.rs`

- [ ] **Step 1: Write failing correlation-id test around generated request IDs**

```rust
// src-tauri/tests/runtime_logging.rs
use codexlag_lib::logging::runtime::build_attempt_id;

#[test]
fn build_attempt_id_uses_request_id_and_zero_based_index() {
    assert_eq!(build_attempt_id("req-abc", 0), "req-abc:0");
    assert_eq!(build_attempt_id("req-abc", 2), "req-abc:2");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test runtime_logging build_attempt_id_uses_request_id_and_zero_based_index`
Expected: FAIL because `build_attempt_id` helper does not exist yet.

- [ ] **Step 3: Implement request/attempt correlation logging at accept/select/reject points**

```rust
// src-tauri/src/gateway/routes.rs (inside codex_request)
let request_id = gateway_state.next_request_id(&platform_key.name, now_ms, "pending");
log::info!(
    "{}",
    crate::logging::runtime::format_event_fields(&[
        ("event", "gateway.request.accepted"),
        ("request_id", request_id.as_str()),
        ("platform_key", platform_key.name.as_str()),
        ("mode", mode),
    ])
);

let selected = choose_endpoint_at(mode, &candidates, now_ms).map_err(|error| {
    crate::logging::log_route_rejection(mode, &error, &candidates, now_ms, request_id.as_str());
    map_routing_error(mode, error)
})?;

let attempt_id = crate::logging::runtime::build_attempt_id(request_id.as_str(), 0);
log::info!(
    "{}",
    crate::logging::runtime::format_event_fields(&[
        ("event", "routing.endpoint.selected"),
        ("request_id", request_id.as_str()),
        ("attempt_id", attempt_id.as_str()),
        ("endpoint_id", selected.id.as_str()),
    ])
);
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test command_surface usage_commands_reflect_runtime_gateway_requests_only`
Expected: PASS with unchanged functional behavior for request recording.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/gateway/routes.rs src-tauri/src/logging/mod.rs src-tauri/tests/runtime_logging.rs src-tauri/tests/command_surface.rs
git commit -m "feat(gateway): emit structured runtime events with request and attempt correlation"
```

## Task 4: Expose Runtime Log Metadata To Control Plane And Overview UI

**Files:**
- Modify: `src-tauri/src/commands/logs.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauri.ts`
- Modify: `src/features/overview/overview-page.tsx`
- Modify: `src-tauri/tests/runtime_composition.rs`
- Modify: `src/test/app-shell.test.tsx`

- [ ] **Step 1: Write failing backend test for log metadata**

```rust
// src-tauri/tests/runtime_composition.rs
use codexlag_lib::commands::logs::runtime_log_metadata_from_runtime;

#[tokio::test]
async fn runtime_log_metadata_exposes_log_dir_and_existing_files() {
    let runtime = bootstrap_runtime_for_test().await.expect("bootstrap runtime");

    let metadata = runtime_log_metadata_from_runtime(&runtime).expect("runtime metadata");
    assert!(metadata.log_dir.contains("logs"));
    assert!(metadata.files.len() <= 20);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test runtime_composition runtime_log_metadata_exposes_log_dir_and_existing_files`
Expected: FAIL because metadata API does not exist yet.

- [ ] **Step 3: Implement command + frontend bindings + overview rendering**

```rust
// src-tauri/src/commands/logs.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeLogMetadata {
    pub log_dir: String,
    pub files: Vec<String>,
}

#[tauri::command]
pub fn get_runtime_log_metadata(state: State<'_, RuntimeState>) -> Result<RuntimeLogMetadata, String> {
    runtime_log_metadata_from_runtime(&state)
}
```

```ts
// src/lib/types.ts
export interface RuntimeLogMetadata {
  log_dir: string;
  files: string[];
}
```

```ts
// src/lib/tauri.ts
export function getRuntimeLogMetadata() {
  return invoke<RuntimeLogMetadata>("get_runtime_log_metadata");
}
```

```tsx
// src/features/overview/overview-page.tsx
const [runtimeLogMetadata, setRuntimeLogMetadata] = useState<RuntimeLogMetadata | null>(null);
// load with Promise.all(...)
<article className="status-card">
  <h3>Runtime diagnostics</h3>
  <p>Log directory: {runtimeLogMetadata?.log_dir ?? "loading"}</p>
  <p>Log files tracked: {runtimeLogMetadata?.files.length ?? 0}</p>
</article>
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test runtime_composition`
Expected: PASS.

Run: `bun run test src/test/app-shell.test.tsx`
Expected: PASS with updated overview assertions.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/logs.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs src/lib/types.ts src/lib/tauri.ts src/features/overview/overview-page.tsx src-tauri/tests/runtime_composition.rs src/test/app-shell.test.tsx
git commit -m "feat(observability): expose runtime log metadata in tauri command and overview ui"
```

## Task 5: Verification And Release Readiness For Logging Baseline

**Files:**
- Modify: `src-tauri/src/commands/logs.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauri.ts`
- Modify: `src/features/overview/overview-page.tsx`
- Test: `src-tauri/tests/runtime_composition.rs`
- Test: `src/test/app-shell.test.tsx`

- [ ] **Step 1: Add failing test for runtime diagnostics export**

```rust
// src-tauri/tests/runtime_composition.rs
use codexlag_lib::commands::logs::export_runtime_diagnostics_from_runtime;

#[tokio::test]
async fn diagnostics_export_returns_manifest_path() {
    let runtime = bootstrap_runtime_for_test().await.expect("bootstrap runtime");
    let manifest_path = export_runtime_diagnostics_from_runtime(&runtime).expect("export diagnostics");

    assert!(manifest_path.ends_with("diagnostics-manifest.txt"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test runtime_composition diagnostics_export_returns_manifest_path`
Expected: FAIL because export helper/command does not exist.

- [ ] **Step 3: Implement diagnostics export command and UI entry**

```rust
// src-tauri/src/commands/logs.rs
#[tauri::command]
pub fn export_runtime_diagnostics(state: State<'_, RuntimeState>) -> Result<String, String> {
    export_runtime_diagnostics_from_runtime(&state)
}

pub fn export_runtime_diagnostics_from_runtime(runtime: &RuntimeState) -> Result<String, String> {
    let metadata = runtime_log_metadata_from_runtime(runtime)?;
    let diagnostics_dir = std::path::PathBuf::from(&metadata.log_dir).join("diagnostics");
    std::fs::create_dir_all(&diagnostics_dir).map_err(|error| error.to_string())?;
    let manifest_path = diagnostics_dir.join("diagnostics-manifest.txt");
    let generated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_secs();
    std::fs::write(
        &manifest_path,
        format!(
            "generated_at_unix={}\nlog_dir={}\nfiles={}\n",
            generated_at,
            metadata.log_dir,
            metadata.files.join(",")
        ),
    )
    .map_err(|error| error.to_string())?;
    Ok(manifest_path.to_string_lossy().to_string())
}
```

```tsx
// src/features/overview/overview-page.tsx
const [diagnosticsPath, setDiagnosticsPath] = useState<string | null>(null);
// ...
<button
  type="button"
  onClick={async () => {
    const path = await exportRuntimeDiagnostics();
    setDiagnosticsPath(path);
  }}
>
  Export diagnostics
</button>
<p>{diagnosticsPath ? `Diagnostics manifest: ${diagnosticsPath}` : "No diagnostics export yet."}</p>
```

- [ ] **Step 4: Run full verification suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS.

Run: `bun run test`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/logs.rs src-tauri/src/lib.rs src/lib/types.ts src/lib/tauri.ts src/features/overview/overview-page.tsx src-tauri/tests/runtime_composition.rs src/test/app-shell.test.tsx
git commit -m "feat(observability): add runtime diagnostics export command and overview action"
```

## Spec Coverage Check

- Runtime terminal + file logging: covered by Task 1.
- Structured event naming and redaction: covered by Task 2.
- Request-level correlation across routing flow: covered by Task 3.
- Control-plane observability in command/UI: covered by Task 4.
- Diagnostics export loop for support workflows: covered by Task 5.

## Placeholder Scan

- No `TODO`, `TBD`, or deferred pseudo-steps.
- Every task has explicit files, code snippets, commands, and expected outcomes.

## Type Consistency Check

- Rust `RuntimeLogMetadata` uses snake_case fields to match existing Tauri payload conventions.
- TypeScript `RuntimeLogMetadata` mirrors backend field names exactly (`log_dir`, `files`).
- Event names are consistent across backend logging helper and gateway instrumentation.
