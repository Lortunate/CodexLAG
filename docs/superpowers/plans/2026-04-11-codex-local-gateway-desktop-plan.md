# Codex Local Gateway Desktop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Windows-first Tauri v2 desktop app that runs a loopback Codex gateway, manages official accounts and NewAPI relays, issues local platform keys, applies per-key routing policy, records request logs and usage, and exposes tray controls for the `default key`.

**Architecture:** The app is a single Tauri v2 process with clear internal boundaries: a Rust control plane, a Rust loopback HTTP gateway, a SQLite persistence layer, a Windows credential abstraction, and a React/TypeScript desktop UI. Official features must stay at parity with `CLIProxyAPI`: if the reference project does not implement a feature, this app does not invent it.

**Tech Stack:** Tauri v2, Rust, Tokio, Axum, Serde, Rusqlite, Keyring or Windows Credential Manager wrapper, React, TypeScript, Vite, Vitest, Testing Library.

---

## File Structure

### Frontend

- Create: `package.json`
- Create: `tsconfig.json`
- Create: `vite.config.ts`
- Create: `src/main.tsx`
- Create: `src/App.tsx`
- Create: `src/styles.css`
- Create: `src/lib/tauri.ts`
- Create: `src/lib/types.ts`
- Create: `src/features/overview/overview-page.tsx`
- Create: `src/features/accounts/accounts-page.tsx`
- Create: `src/features/relays/relays-page.tsx`
- Create: `src/features/keys/keys-page.tsx`
- Create: `src/features/policies/policies-page.tsx`
- Create: `src/features/logs/logs-page.tsx`
- Create: `src/features/default-key/default-key-mode-toggle.tsx`
- Create: `src/test/app-shell.test.tsx`

### Rust backend

- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/capabilities/default.json`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`
- Create: `src-tauri/src/error.rs`
- Create: `src-tauri/src/state.rs`
- Create: `src-tauri/src/bootstrap.rs`
- Create: `src-tauri/src/models.rs`
- Create: `src-tauri/src/db/mod.rs`
- Create: `src-tauri/src/db/migrations.rs`
- Create: `src-tauri/src/db/repositories.rs`
- Create: `src-tauri/src/secret_store.rs`
- Create: `src-tauri/src/gateway/mod.rs`
- Create: `src-tauri/src/gateway/server.rs`
- Create: `src-tauri/src/gateway/routes.rs`
- Create: `src-tauri/src/gateway/auth.rs`
- Create: `src-tauri/src/routing/mod.rs`
- Create: `src-tauri/src/routing/policy.rs`
- Create: `src-tauri/src/routing/engine.rs`
- Create: `src-tauri/src/providers/mod.rs`
- Create: `src-tauri/src/providers/official.rs`
- Create: `src-tauri/src/providers/relay.rs`
- Create: `src-tauri/src/providers/capabilities.rs`
- Create: `src-tauri/src/logging/mod.rs`
- Create: `src-tauri/src/logging/usage.rs`
- Create: `src-tauri/src/commands/mod.rs`
- Create: `src-tauri/src/commands/accounts.rs`
- Create: `src-tauri/src/commands/relays.rs`
- Create: `src-tauri/src/commands/keys.rs`
- Create: `src-tauri/src/commands/policies.rs`
- Create: `src-tauri/src/commands/logs.rs`
- Create: `src-tauri/src/tray.rs`

### Rust tests

- Create: `src-tauri/tests/bootstrap_default_key.rs`
- Create: `src-tauri/tests/gateway_auth.rs`
- Create: `src-tauri/tests/routing_engine.rs`
- Create: `src-tauri/tests/provider_capabilities.rs`
- Create: `src-tauri/tests/newapi_balance.rs`
- Create: `src-tauri/tests/request_logging.rs`

## Task 1: Scaffold The Tauri + React Shell

**Files:**
- Create: `package.json`
- Create: `tsconfig.json`
- Create: `vite.config.ts`
- Create: `src/main.tsx`
- Create: `src/App.tsx`
- Create: `src/styles.css`
- Create: `src/test/app-shell.test.tsx`
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/capabilities/default.json`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing frontend shell test**

```tsx
// src/test/app-shell.test.tsx
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "../App";

describe("App shell", () => {
  it("renders the six primary navigation sections", () => {
    render(<App />);

    expect(screen.getByText("Overview")).toBeInTheDocument();
    expect(screen.getByText("Official Accounts")).toBeInTheDocument();
    expect(screen.getByText("Relays")).toBeInTheDocument();
    expect(screen.getByText("Platform Keys")).toBeInTheDocument();
    expect(screen.getByText("Policies")).toBeInTheDocument();
    expect(screen.getByText("Logs & Usage")).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- src/test/app-shell.test.tsx`
Expected: FAIL with `Cannot find module '../App'` or missing Vite/Vitest config.

- [ ] **Step 3: Write the minimal frontend and Tauri skeleton**

```json
// package.json
{
  "name": "codexlag",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "test": "vitest run",
    "test:watch": "vitest",
    "tauri:dev": "tauri dev"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "react": "^19.0.0",
    "react-dom": "^19.0.0"
  },
  "devDependencies": {
    "@testing-library/jest-dom": "^6.6.3",
    "@testing-library/react": "^16.3.0",
    "@tauri-apps/cli": "^2.0.0",
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "@vitejs/plugin-react": "^4.3.4",
    "typescript": "^5.8.3",
    "vite": "^6.3.0",
    "vitest": "^3.1.1"
  }
}
```

```tsx
// src/App.tsx
const sections = [
  "Overview",
  "Official Accounts",
  "Relays",
  "Platform Keys",
  "Policies",
  "Logs & Usage",
];

export default function App() {
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <h1>CodexLAG</h1>
        <nav>
          {sections.map((section) => (
            <button key={section} type="button">
              {section}
            </button>
          ))}
        </nav>
      </aside>
      <main className="content">
        <h2>Overview</h2>
        <p>Windows-first local Codex gateway desktop console.</p>
      </main>
    </div>
  );
}
```

```rust
// src-tauri/src/lib.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running CodexLAG");
}
```

```rust
// src-tauri/src/main.rs
fn main() {
    codexlag_lib::run();
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm test -- src/test/app-shell.test.tsx`
Expected: PASS

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS with `0 failed`

- [ ] **Step 5: Commit**

```bash
git add package.json tsconfig.json vite.config.ts src src-tauri
git commit -m "chore: scaffold tauri desktop shell"
```

## Task 2: Build The Domain Model And SQLite Bootstrap

**Files:**
- Create: `src-tauri/src/error.rs`
- Create: `src-tauri/src/models.rs`
- Create: `src-tauri/src/db/mod.rs`
- Create: `src-tauri/src/db/migrations.rs`
- Create: `src-tauri/src/db/repositories.rs`
- Create: `src-tauri/src/state.rs`
- Test: `src-tauri/tests/bootstrap_default_key.rs`

- [ ] **Step 1: Write the failing database bootstrap test**

```rust
// src-tauri/tests/bootstrap_default_key.rs
use codexlag_lib::bootstrap::bootstrap_state_for_test;

#[tokio::test]
async fn bootstrap_creates_default_policy_and_default_key() {
    let state = bootstrap_state_for_test().await.expect("bootstrap");

    let policy = state.db.get_policy_by_name("default").expect("default policy");
    let key = state.db.get_platform_key_by_name("default").expect("default key");

    assert_eq!(policy.name, "default");
    assert_eq!(key.name, "default");
    assert_eq!(key.allowed_mode.as_str(), "hybrid");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bootstrap_creates_default_policy_and_default_key -- --exact`
Expected: FAIL with `unresolved import codexlag_lib::bootstrap::bootstrap_state_for_test`

- [ ] **Step 3: Write the minimal model and DB bootstrap implementation**

```rust
// src-tauri/src/models.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformKey {
    pub id: String,
    pub name: String,
    pub allowed_mode: String,
    pub policy_id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingPolicy {
    pub id: String,
    pub name: String,
}
```

```rust
// src-tauri/src/db/repositories.rs
use crate::models::{PlatformKey, RoutingPolicy};

#[derive(Default)]
pub struct Repositories {
    pub policies: Vec<RoutingPolicy>,
    pub keys: Vec<PlatformKey>,
}

impl Repositories {
    pub fn get_policy_by_name(&self, name: &str) -> Option<RoutingPolicy> {
        self.policies.iter().find(|item| item.name == name).cloned()
    }

    pub fn get_platform_key_by_name(&self, name: &str) -> Option<PlatformKey> {
        self.keys.iter().find(|item| item.name == name).cloned()
    }
}
```

```rust
// src-tauri/src/bootstrap.rs
use crate::{db::repositories::Repositories, models::{PlatformKey, RoutingPolicy}};

pub struct AppStateForTest {
    pub db: Repositories,
}

pub async fn bootstrap_state_for_test() -> Result<AppStateForTest, String> {
    let default_policy = RoutingPolicy {
        id: "policy-default".into(),
        name: "default".into(),
    };

    let default_key = PlatformKey {
        id: "key-default".into(),
        name: "default".into(),
        allowed_mode: "hybrid".into(),
        policy_id: default_policy.id.clone(),
        enabled: true,
    };

    Ok(AppStateForTest {
        db: Repositories {
            policies: vec![default_policy],
            keys: vec![default_key],
        },
    })
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bootstrap_creates_default_policy_and_default_key -- --exact`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/error.rs src-tauri/src/models.rs src-tauri/src/db src-tauri/src/state.rs src-tauri/src/bootstrap.rs src-tauri/tests/bootstrap_default_key.rs
git commit -m "feat: bootstrap core models and default objects"
```

## Task 3: Add Secret Storage And Default Key Secret Bootstrapping

**Files:**
- Create: `src-tauri/src/secret_store.rs`
- Modify: `src-tauri/src/bootstrap.rs`
- Modify: `src-tauri/src/models.rs`
- Test: `src-tauri/tests/bootstrap_default_key.rs`

- [ ] **Step 1: Extend the failing test to require secret storage**

```rust
#[tokio::test]
async fn bootstrap_persists_default_key_secret_in_secret_store() {
    let state = bootstrap_state_for_test().await.expect("bootstrap");

    let secret = state
        .secret_store
        .get("platform-key/default")
        .expect("default key secret");

    assert!(secret.starts_with("ck_local_"));
    assert!(secret.len() > "ck_local_".len());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bootstrap_persists_default_key_secret_in_secret_store -- --exact`
Expected: FAIL with `no field 'secret_store' on type 'AppStateForTest'`

- [ ] **Step 3: Write the minimal secret-store abstraction**

```rust
// src-tauri/src/secret_store.rs
use std::collections::HashMap;

#[derive(Default)]
pub struct SecretStore {
    secrets: HashMap<String, String>,
}

impl SecretStore {
    pub fn set(&mut self, key: &str, value: String) {
        self.secrets.insert(key.to_string(), value);
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.secrets.get(key).cloned()
    }
}
```

```rust
// src-tauri/src/bootstrap.rs
use crate::secret_store::SecretStore;

pub struct AppStateForTest {
    pub db: Repositories,
    pub secret_store: SecretStore,
}

pub async fn bootstrap_state_for_test() -> Result<AppStateForTest, String> {
    let default_policy = RoutingPolicy {
        id: "policy-default".into(),
        name: "default".into(),
    };

    let default_key = PlatformKey {
        id: "key-default".into(),
        name: "default".into(),
        allowed_mode: "hybrid".into(),
        policy_id: default_policy.id.clone(),
        enabled: true,
    };

    let mut secret_store = SecretStore::default();
    secret_store.set("platform-key/default", "ck_local_default_seed".into());

    Ok(AppStateForTest {
        db: Repositories {
            policies: vec![default_policy],
            keys: vec![default_key],
        },
        secret_store,
    })
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bootstrap_ -- --nocapture`
Expected: PASS for both bootstrap tests

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/secret_store.rs src-tauri/src/bootstrap.rs src-tauri/tests/bootstrap_default_key.rs
git commit -m "feat: store default key secret in secure secret store abstraction"
```

## Task 4: Implement Loopback Gateway Health And Platform Key Authentication

**Files:**
- Create: `src-tauri/src/gateway/mod.rs`
- Create: `src-tauri/src/gateway/server.rs`
- Create: `src-tauri/src/gateway/routes.rs`
- Create: `src-tauri/src/gateway/auth.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/tests/gateway_auth.rs`

- [ ] **Step 1: Write the failing gateway auth tests**

```rust
// src-tauri/tests/gateway_auth.rs
use axum::{body::Body, http::{Request, StatusCode}};
use tower::ServiceExt;
use codexlag_lib::gateway::routes::build_router_for_test;

#[tokio::test]
async fn health_route_returns_ok() {
    let app = build_router_for_test("ck_local_default_seed");

    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn codex_route_rejects_invalid_platform_key() {
    let app = build_router_for_test("ck_local_default_seed");

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/codex/request")
                .header("authorization", "Bearer wrong-key")
                .body(Body::from("{\"model\":\"test-model\"}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml gateway_auth -- --nocapture`
Expected: FAIL with `cannot find function build_router_for_test`

- [ ] **Step 3: Write the minimal Axum router and auth guard**

```rust
// src-tauri/src/gateway/routes.rs
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde_json::Value;

#[derive(Clone)]
pub struct GatewayState {
    pub default_secret: String,
}

pub fn build_router_for_test(secret: &str) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/codex/request", post(codex_request))
        .with_state(GatewayState {
            default_secret: secret.to_string(),
        })
}

async fn health() -> &'static str {
    "ok"
}

async fn codex_request(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(_body): Json<Value>,
) -> impl IntoResponse {
    let auth = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    let expected = format!("Bearer {}", state.default_secret);
    if auth != expected {
        return StatusCode::UNAUTHORIZED;
    }

    StatusCode::OK
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml gateway_auth -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/gateway src-tauri/tests/gateway_auth.rs src-tauri/src/lib.rs
git commit -m "feat: add loopback gateway health and key auth"
```

## Task 5: Implement The Routing Engine And Per-Key Mode Filtering

**Files:**
- Create: `src-tauri/src/routing/mod.rs`
- Create: `src-tauri/src/routing/policy.rs`
- Create: `src-tauri/src/routing/engine.rs`
- Modify: `src-tauri/src/models.rs`
- Test: `src-tauri/tests/routing_engine.rs`

- [ ] **Step 1: Write the failing routing tests**

```rust
// src-tauri/tests/routing_engine.rs
use codexlag_lib::routing::engine::{choose_endpoint, CandidateEndpoint};

#[test]
fn hybrid_mode_prefers_official_then_relay() {
    let endpoints = vec![
        CandidateEndpoint::official("official-1", 10, true),
        CandidateEndpoint::relay("relay-1", 20, true),
    ];

    let selected = choose_endpoint("hybrid", &endpoints).expect("selected endpoint");
    assert_eq!(selected.id, "official-1");
}

#[test]
fn relay_only_skips_official_candidates() {
    let endpoints = vec![
        CandidateEndpoint::official("official-1", 10, true),
        CandidateEndpoint::relay("relay-1", 20, true),
    ];

    let selected = choose_endpoint("relay_only", &endpoints).expect("selected endpoint");
    assert_eq!(selected.id, "relay-1");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml routing_engine -- --nocapture`
Expected: FAIL with `cannot find module routing`

- [ ] **Step 3: Write the minimal routing engine**

```rust
// src-tauri/src/routing/engine.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoolKind {
    Official,
    Relay,
}

#[derive(Debug, Clone)]
pub struct CandidateEndpoint {
    pub id: String,
    pub priority: i32,
    pub available: bool,
    pub pool: PoolKind,
}

impl CandidateEndpoint {
    pub fn official(id: &str, priority: i32, available: bool) -> Self {
        Self { id: id.into(), priority, available, pool: PoolKind::Official }
    }

    pub fn relay(id: &str, priority: i32, available: bool) -> Self {
        Self { id: id.into(), priority, available, pool: PoolKind::Relay }
    }
}

pub fn choose_endpoint(mode: &str, endpoints: &[CandidateEndpoint]) -> Option<CandidateEndpoint> {
    let mut candidates: Vec<_> = endpoints
        .iter()
        .filter(|item| item.available)
        .filter(|item| match mode {
            "account_only" => item.pool == PoolKind::Official,
            "relay_only" => item.pool == PoolKind::Relay,
            _ => true,
        })
        .cloned()
        .collect();

    candidates.sort_by_key(|item| item.priority);
    candidates.into_iter().next()
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml routing_engine -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/routing src-tauri/src/models.rs src-tauri/tests/routing_engine.rs
git commit -m "feat: add per-key routing mode engine"
```

## Task 6: Add Official Provider Sessions And CLIProxyAPI-Parity Capability Discovery

**Files:**
- Create: `src-tauri/src/providers/mod.rs`
- Create: `src-tauri/src/providers/official.rs`
- Create: `src-tauri/src/providers/capabilities.rs`
- Modify: `src-tauri/src/models.rs`
- Test: `src-tauri/tests/provider_capabilities.rs`

- [ ] **Step 1: Write the failing capability tests**

```rust
// src-tauri/tests/provider_capabilities.rs
use codexlag_lib::providers::capabilities::merge_cli_proxyapi_capabilities;

#[test]
fn capability_merge_includes_registered_max_tokens() {
    let capability = merge_cli_proxyapi_capabilities(
        "claude-3-5-sonnet",
        Some(8192),
        Some(false),
        Some(true),
    );

    assert_eq!(capability.max_context_window, Some(8192));
    assert_eq!(capability.supports_context_compression, Some(false));
    assert_eq!(capability.supports_compact_path, Some(true));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml capability_merge_includes_registered_max_tokens -- --exact`
Expected: FAIL with `cannot find function merge_cli_proxyapi_capabilities`

- [ ] **Step 3: Write the minimal capability model**

```rust
// src-tauri/src/providers/capabilities.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeatureCapability {
    pub model_id: String,
    pub max_context_window: Option<u32>,
    pub supports_context_compression: Option<bool>,
    pub supports_compact_path: Option<bool>,
}

pub fn merge_cli_proxyapi_capabilities(
    model_id: &str,
    max_context_window: Option<u32>,
    supports_context_compression: Option<bool>,
    supports_compact_path: Option<bool>,
) -> FeatureCapability {
    FeatureCapability {
        model_id: model_id.to_string(),
        max_context_window,
        supports_context_compression,
        supports_compact_path,
    }
}
```

```rust
// src-tauri/src/providers/official.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficialSession {
    pub session_id: String,
    pub account_identity: String,
    pub auth_mode: String,
    pub refresh_capability: bool,
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml capability_merge_includes_registered_max_tokens -- --exact`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/providers src-tauri/src/models.rs src-tauri/tests/provider_capabilities.rs
git commit -m "feat: model official sessions and cli proxyapi parity capabilities"
```

## Task 7: Add NewAPI Relay Adapter And Balance Query Normalization

**Files:**
- Create: `src-tauri/src/providers/relay.rs`
- Modify: `src-tauri/src/providers/mod.rs`
- Test: `src-tauri/tests/newapi_balance.rs`

- [ ] **Step 1: Write the failing NewAPI balance test**

```rust
// src-tauri/tests/newapi_balance.rs
use codexlag_lib::providers::relay::normalize_newapi_balance_response;

#[test]
fn normalize_newapi_balance_response_maps_amounts() {
    let normalized = normalize_newapi_balance_response(r#"{"data":{"total_balance":"25.00","used_balance":"7.50"}}"#)
        .expect("normalized");

    assert_eq!(normalized.total, "25.00");
    assert_eq!(normalized.used, "7.50");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml normalize_newapi_balance_response_maps_amounts -- --exact`
Expected: FAIL with `cannot find function normalize_newapi_balance_response`

- [ ] **Step 3: Write the minimal relay adapter**

```rust
// src-tauri/src/providers/relay.rs
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedBalance {
    pub total: String,
    pub used: String,
}

#[derive(Debug, Deserialize)]
struct NewApiPayload {
    data: NewApiBalanceData,
}

#[derive(Debug, Deserialize)]
struct NewApiBalanceData {
    total_balance: String,
    used_balance: String,
}

pub fn normalize_newapi_balance_response(body: &str) -> Result<NormalizedBalance, serde_json::Error> {
    let payload: NewApiPayload = serde_json::from_str(body)?;
    Ok(NormalizedBalance {
        total: payload.data.total_balance,
        used: payload.data.used_balance,
    })
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml normalize_newapi_balance_response_maps_amounts -- --exact`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/providers/relay.rs src-tauri/src/providers/mod.rs src-tauri/tests/newapi_balance.rs
git commit -m "feat: add newapi relay balance normalization"
```

## Task 8: Record Request Logs, Usage, And Cost Snapshots

**Files:**
- Create: `src-tauri/src/logging/mod.rs`
- Create: `src-tauri/src/logging/usage.rs`
- Modify: `src-tauri/src/models.rs`
- Test: `src-tauri/tests/request_logging.rs`

- [ ] **Step 1: Write the failing request logging test**

```rust
// src-tauri/tests/request_logging.rs
use codexlag_lib::logging::usage::{record_request, UsageRecordInput};

#[test]
fn record_request_captures_input_output_cache_and_estimated_cost() {
    let record = record_request(UsageRecordInput {
        request_id: "req-1".into(),
        endpoint_id: "official-1".into(),
        input_tokens: 120,
        output_tokens: 30,
        cache_read_tokens: 10,
        cache_write_tokens: 0,
        estimated_cost: "0.0123".into(),
    });

    assert_eq!(record.total_tokens, 160);
    assert_eq!(record.estimated_cost, "0.0123");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml record_request_captures_input_output_cache_and_estimated_cost -- --exact`
Expected: FAIL with `cannot find module logging`

- [ ] **Step 3: Write the minimal logging model**

```rust
// src-tauri/src/logging/usage.rs
#[derive(Debug, Clone)]
pub struct UsageRecordInput {
    pub request_id: String,
    pub endpoint_id: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_write_tokens: u32,
    pub estimated_cost: String,
}

#[derive(Debug, Clone)]
pub struct UsageRecord {
    pub request_id: String,
    pub endpoint_id: String,
    pub total_tokens: u32,
    pub estimated_cost: String,
}

pub fn record_request(input: UsageRecordInput) -> UsageRecord {
    UsageRecord {
        request_id: input.request_id,
        endpoint_id: input.endpoint_id,
        total_tokens: input.input_tokens + input.output_tokens + input.cache_read_tokens + input.cache_write_tokens,
        estimated_cost: input.estimated_cost,
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml record_request_captures_input_output_cache_and_estimated_cost -- --exact`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/logging src-tauri/tests/request_logging.rs src-tauri/src/models.rs
git commit -m "feat: add request usage and cost snapshots"
```

## Task 9: Expose Tauri Commands And Build The Six Main Pages

**Files:**
- Create: `src-tauri/src/commands/mod.rs`
- Create: `src-tauri/src/commands/accounts.rs`
- Create: `src-tauri/src/commands/relays.rs`
- Create: `src-tauri/src/commands/keys.rs`
- Create: `src-tauri/src/commands/policies.rs`
- Create: `src-tauri/src/commands/logs.rs`
- Create: `src/lib/tauri.ts`
- Create: `src/lib/types.ts`
- Create: `src/features/overview/overview-page.tsx`
- Create: `src/features/accounts/accounts-page.tsx`
- Create: `src/features/relays/relays-page.tsx`
- Create: `src/features/keys/keys-page.tsx`
- Create: `src/features/policies/policies-page.tsx`
- Create: `src/features/logs/logs-page.tsx`
- Create: `src/features/default-key/default-key-mode-toggle.tsx`
- Modify: `src/App.tsx`
- Test: `src/test/app-shell.test.tsx`

- [ ] **Step 1: Extend the failing UI test to require the default-key mode widget**

```tsx
it("shows default key mode controls in the overview shell", () => {
  render(<App />);

  expect(screen.getByText("Default Key Mode")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "account_only" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "relay_only" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "hybrid" })).toBeInTheDocument();
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- src/test/app-shell.test.tsx`
Expected: FAIL with missing `Default Key Mode`

- [ ] **Step 3: Write the minimal command surface and UI pages**

```rust
// src-tauri/src/commands/keys.rs
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DefaultKeySummary {
    pub name: String,
    pub allowed_mode: String,
}

#[tauri::command]
pub fn get_default_key_summary() -> DefaultKeySummary {
    DefaultKeySummary {
        name: "default".into(),
        allowed_mode: "hybrid".into(),
    }
}
```

```tsx
// src/features/default-key/default-key-mode-toggle.tsx
export function DefaultKeyModeToggle() {
  return (
    <section>
      <h3>Default Key Mode</h3>
      <div>
        <button type="button">account_only</button>
        <button type="button">relay_only</button>
        <button type="button">hybrid</button>
      </div>
    </section>
  );
}
```

```tsx
// src/App.tsx
import { DefaultKeyModeToggle } from "./features/default-key/default-key-mode-toggle";

const sections = [
  "Overview",
  "Official Accounts",
  "Relays",
  "Platform Keys",
  "Policies",
  "Logs & Usage",
];

export default function App() {
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <h1>CodexLAG</h1>
        <nav>
          {sections.map((section) => (
            <button key={section} type="button">
              {section}
            </button>
          ))}
        </nav>
      </aside>
      <main className="content">
        <h2>Overview</h2>
        <DefaultKeyModeToggle />
      </main>
    </div>
  );
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm test -- src/test/app-shell.test.tsx`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands src/lib src/features src/App.tsx src/test/app-shell.test.tsx
git commit -m "feat: add command surface and management pages"
```

## Task 10: Add Tray Controls For The Default Key

**Files:**
- Create: `src-tauri/src/tray.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/tests/bootstrap_default_key.rs`

- [ ] **Step 1: Write the failing tray summary test**

```rust
#[test]
fn tray_model_contains_default_key_mode_actions() {
    let model = codexlag_lib::tray::build_tray_model("hybrid");

    assert!(model.items.contains(&"mode:account_only".to_string()));
    assert!(model.items.contains(&"mode:relay_only".to_string()));
    assert!(model.items.contains(&"mode:hybrid".to_string()));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml tray_model_contains_default_key_mode_actions -- --exact`
Expected: FAIL with `cannot find module tray`

- [ ] **Step 3: Write the minimal tray model and wire it into Tauri**

```rust
// src-tauri/src/tray.rs
#[derive(Debug, Clone)]
pub struct TrayModel {
    pub items: Vec<String>,
}

pub fn build_tray_model(current_mode: &str) -> TrayModel {
    TrayModel {
        items: vec![
            format!("current-mode:{current_mode}"),
            "mode:account_only".into(),
            "mode:relay_only".into(),
            "mode:hybrid".into(),
            "action:open".into(),
            "action:restart-gateway".into(),
            "action:quit".into(),
        ],
    }
}
```

```rust
// src-tauri/src/lib.rs
mod tray;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let _model = crate::tray::build_tray_model("hybrid");
            let _ = app;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running CodexLAG");
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml tray_model_contains_default_key_mode_actions -- --exact`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/tray.rs src-tauri/src/lib.rs src-tauri/tests/bootstrap_default_key.rs
git commit -m "feat: add tray model for default key controls"
```

## Spec Coverage Check

- Gateway loopback, platform key auth, request logs, usage, and routing are covered by Tasks 2, 4, 5, and 8.
- Official accounts, capability parity, and CLIProxyAPI feature constraints are covered by Task 6.
- NewAPI relay support and balance normalization are covered by Task 7.
- `default key`, tray behavior, and six-page UI shell are covered by Tasks 2, 3, 9, and 10.
- Remaining gap to watch during execution: replace in-memory bootstrap and secret-store stubs with real SQLite and Windows Credential Manager implementations before closing V1. Do not treat the minimal stubs in early tasks as final architecture.

## Placeholder Scan

- No `TODO`, `TBD`, or “implement later” placeholders are permitted during execution.
- If a task introduces a stub to make the red-green cycle small, the next task touching that area must replace it with the real implementation before V1 is declared complete.

## Type Consistency Check

- `PlatformKey.allowed_mode` uses `account_only | relay_only | hybrid` across all tasks.
- `FeatureCapability` is the single Rust type for CLIProxyAPI-parity feature metadata.
- `DefaultKeyModeToggle` is the single frontend component for the overview and tray-aligned mode controls.
