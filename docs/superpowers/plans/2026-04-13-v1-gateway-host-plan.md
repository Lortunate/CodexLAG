# V1 Gateway Host Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the in-process placeholder gateway lifecycle with a real loopback Axum host owned by the Tauri runtime and restartable from runtime controls.

**Architecture:** Introduce a dedicated gateway host type that owns the bound listener, shutdown signal, and serving task. `RuntimeState` becomes the lifecycle owner, and tray/runtime actions restart the real server rather than rebuilding only in-memory state. Keep the existing router composition, but move host startup and status inspection into explicit runtime services.

**Tech Stack:** Rust, Tokio, Axum, Tauri v2, Serde, existing CodexLAG runtime and tray modules, Tokio integration tests.

**Status:** Completed locally on 2026-04-14. Historical implementation steps are retained for traceability; checkbox state below reflects the completed repository state.

---

## File Structure

### Rust backend

- Create: `src-tauri/src/gateway/host.rs`
- Modify: `src-tauri/src/gateway/mod.rs`
- Modify: `src-tauri/src/gateway/server.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/bootstrap.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/tray.rs`

### Tests

- Create: `src-tauri/tests/gateway_host.rs`
- Modify: `src-tauri/tests/runtime_composition.rs`
- Modify: `src-tauri/tests/tray_restart.rs`

## Task 1: Add A Dedicated Gateway Host Type

**Files:**
- Create: `src-tauri/src/gateway/host.rs`
- Modify: `src-tauri/src/gateway/mod.rs`
- Test: `src-tauri/tests/gateway_host.rs`

- [x] **Step 1: Write the failing gateway-host lifecycle test**

```rust
// src-tauri/tests/gateway_host.rs
use codexlag_lib::bootstrap::bootstrap_runtime_for_test;

#[tokio::test]
async fn runtime_starts_and_restarts_a_real_loopback_gateway_host() {
    let runtime = bootstrap_runtime_for_test().await.expect("bootstrap runtime");

    let status = runtime.gateway_host_status();
    assert!(status.is_running, "gateway host should be running after bootstrap");
    assert_eq!(status.listen_addr.ip().to_string(), "127.0.0.1");
    assert_eq!(status.listen_addr.port(), 8787);

    runtime.restart_gateway().expect("restart gateway");

    let restarted = runtime.gateway_host_status();
    assert!(restarted.is_running);
    assert_eq!(restarted.listen_addr.ip().to_string(), "127.0.0.1");
}
```

- [x] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml runtime_starts_and_restarts_a_real_loopback_gateway_host -- --exact`
Expected: FAIL with missing `gateway_host_status`, missing host state, or no live loopback listener.

- [x] **Step 3: Create the host type with explicit bind, shutdown, and task ownership**

```rust
// src-tauri/src/gateway/host.rs
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use axum::Router;
use tokio::{
    net::TcpListener,
    sync::oneshot,
    task::JoinHandle,
};

use crate::error::{CodexLagError, Result};

pub const LOOPBACK_BIND_ADDR: SocketAddr =
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8787));

pub struct GatewayHost {
    listen_addr: SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayHostStatus {
    pub is_running: bool,
    pub listen_addr: SocketAddr,
}

impl GatewayHost {
    pub async fn start(router: Router) -> Result<Self> {
        let listener = TcpListener::bind(LOOPBACK_BIND_ADDR)
            .await
            .map_err(|error| CodexLagError::new(format!("failed to bind loopback gateway: {error}")))?;
        let listen_addr = listener
            .local_addr()
            .map_err(|error| CodexLagError::new(format!("failed to read gateway listen addr: {error}")))?;
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let task = tokio::spawn(async move {
            let server = axum::serve(listener, router).with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            });
            let _ = server.await;
        });

        Ok(Self {
            listen_addr,
            shutdown_tx: Some(shutdown_tx),
            task,
        })
    }
}
```

- [x] **Step 4: Export the new module from the gateway package**

```rust
// src-tauri/src/gateway/mod.rs
pub mod auth;
pub mod host;
pub mod routes;
pub mod runtime_routing;
pub mod server;
```

- [x] **Step 5: Run the focused host test**

Run: `cargo test --manifest-path src-tauri/Cargo.toml gateway_host -- --nocapture`
Expected: FAIL only on missing runtime wiring, not on missing host types.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/gateway/host.rs src-tauri/src/gateway/mod.rs src-tauri/tests/gateway_host.rs
git commit -m "feat: add dedicated gateway host type"
```

## Task 2: Move Gateway Lifecycle Ownership Into RuntimeState

**Files:**
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/bootstrap.rs`
- Modify: `src-tauri/src/gateway/server.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/tests/runtime_composition.rs`

- [x] **Step 1: Write the failing runtime-composition status test**

```rust
// src-tauri/tests/runtime_composition.rs
#[tokio::test]
async fn runtime_exposes_gateway_host_status_for_the_running_loopback_server() {
    let runtime = codexlag_lib::bootstrap::bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let status = runtime.gateway_host_status();
    assert!(status.is_running);
    assert_eq!(status.listen_addr.ip().to_string(), "127.0.0.1");
}
```

- [x] **Step 2: Run the targeted test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml runtime_exposes_gateway_host_status_for_the_running_loopback_server -- --exact`
Expected: FAIL because `RuntimeState` does not yet own a real host.

- [x] **Step 3: Extend RuntimeState to own gateway host lifecycle**

```rust
// src-tauri/src/state.rs
use crate::gateway::host::{GatewayHost, GatewayHostStatus};

#[derive(Clone)]
pub struct RuntimeState {
    app_state: Arc<RwLock<AppState>>,
    usage_records: Arc<RwLock<Vec<UsageRecord>>>,
    loopback_gateway: Arc<RwLock<LoopbackGateway>>,
    gateway_host: Arc<RwLock<Option<GatewayHost>>>,
    runtime_log: RuntimeLogConfig,
    last_balance_refresh: Arc<RwLock<Option<String>>>,
    last_restart_feedback: Arc<RwLock<Option<String>>>,
}
```

```rust
// src-tauri/src/state.rs
pub fn gateway_host_status(&self) -> GatewayHostStatus {
    self.gateway_host
        .read()
        .expect("runtime gateway host lock poisoned")
        .as_ref()
        .map(|host| GatewayHostStatus {
            is_running: true,
            listen_addr: host.listen_addr(),
        })
        .unwrap_or(GatewayHostStatus {
            is_running: false,
            listen_addr: crate::gateway::host::LOOPBACK_BIND_ADDR,
        })
}
```

- [x] **Step 4: Start the host during bootstrap**

```rust
// src-tauri/src/bootstrap.rs
pub async fn bootstrap_runtime_for_test() -> Result<RuntimeState> {
    let database_path = test_database_path();
    let app_state = bootstrap_state_for_test_at(&database_path).await?;
    let runtime_log = RuntimeLogConfig {
        log_dir: runtime_log_dir(
            database_path
                .parent()
                .ok_or_else(|| CodexLagError::new("runtime database path has no parent directory"))?,
        ),
    };

    RuntimeState::start(app_state, runtime_log).await
}
```

```rust
// src-tauri/src/lib.rs
.setup(|app| -> Result<(), Box<dyn Error>> {
    // ...
    let runtime = tauri::async_runtime::block_on(
        bootstrap::bootstrap_runtime_at_with_log_dir(database_path, app_log_dir)
    )?;
    // ...
})
```

- [x] **Step 5: Run focused runtime-composition tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml runtime_exposes_gateway_host_status_for_the_running_loopback_server -- --exact`
Expected: PASS.

Run: `cargo test --manifest-path src-tauri/Cargo.toml runtime_composition -- --nocapture`
Expected: PASS with real host ownership.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/state.rs src-tauri/src/bootstrap.rs src-tauri/src/gateway/server.rs src-tauri/src/lib.rs src-tauri/tests/runtime_composition.rs
git commit -m "feat: move gateway host lifecycle into runtime state"
```

## Task 3: Make Restart And Status Surfaces Operate On The Real Host

**Files:**
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/tray.rs`
- Modify: `src-tauri/src/tray_summary.rs`
- Modify: `src-tauri/tests/tray_restart.rs`
- Modify: `src-tauri/tests/gateway_host.rs`

- [x] **Step 1: Write the failing tray restart behavior test**

```rust
// src-tauri/tests/tray_restart.rs
#[tokio::test]
async fn restart_tray_action_restarts_the_real_gateway_host() {
    let runtime = codexlag_lib::bootstrap::bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let before = runtime.gateway_host_status();
    runtime.restart_gateway().expect("restart gateway");
    let after = runtime.gateway_host_status();

    assert!(before.is_running);
    assert!(after.is_running);
    assert_eq!(after.listen_addr.ip().to_string(), "127.0.0.1");
}
```

- [x] **Step 2: Run the targeted tray-restart test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml restart_tray_action_restarts_the_real_gateway_host -- --exact`
Expected: FAIL because restart still only rebuilds the in-memory gateway object.

- [x] **Step 3: Implement graceful shutdown and restart on RuntimeState**

```rust
// src-tauri/src/state.rs
pub fn restart_gateway(&self) -> crate::error::Result<()> {
    let runtime = tokio::runtime::Handle::current();
    let router = self.loopback_gateway().router();

    {
        let mut host = self
            .gateway_host
            .write()
            .expect("runtime gateway host lock poisoned");
        if let Some(existing) = host.take() {
            runtime.block_on(existing.shutdown())?;
        }
        *host = Some(runtime.block_on(crate::gateway::host::GatewayHost::start(router))?);
    }

    self.record_restart_feedback("success".to_string());
    Ok(())
}
```

- [x] **Step 4: Surface host-backed status through tray summary**

```rust
// src-tauri/src/tray_summary.rs
let host_status = runtime.gateway_host_status();
let gateway_status_label = if host_status.is_running {
    "Gateway status | ready".to_string()
} else {
    "Gateway status | stopped".to_string()
};
```

- [x] **Step 5: Run final gateway-host tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml gateway_host -- --nocapture`
Expected: PASS.

Run: `cargo test --manifest-path src-tauri/Cargo.toml tray_restart -- --nocapture`
Expected: PASS with tray restart hitting the real host lifecycle.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/state.rs src-tauri/src/tray.rs src-tauri/src/tray_summary.rs src-tauri/tests/tray_restart.rs src-tauri/tests/gateway_host.rs
git commit -m "feat: wire tray restart to the real gateway host"
```
