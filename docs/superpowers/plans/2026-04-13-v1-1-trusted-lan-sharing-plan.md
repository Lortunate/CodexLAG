# V1.1 Trusted LAN Sharing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the current single-machine gateway to optional trusted-LAN access with explicit access control, while keeping V1 security posture and preserving single-tenant assumptions.

**Architecture:** Keep the existing Tauri single-process control-plane/data-plane design, but introduce a persisted `GatewayAccessProfile` that controls bind scope (`loopback` vs `lan`), listen address/port, and CIDR allow-list. Enforce access policy at gateway auth middleware before provider selection, and surface full observability + UI controls for safe rollout and rollback.

**Tech Stack:** Tauri v2, Rust, Axum, Tokio, Serde, Rusqlite, `ipnet`, React 19, TypeScript, Vitest, GitHub Actions (Windows).

---

## Scope Alignment With Spec

This plan is the first post-V1 increment after all existing plans are complete.

- It directly addresses spec section `3.2 非目标` item: `局域网或公网共享节点能力` (now moving from non-goal to V1.1 goal in a constrained form).
- It explicitly **does not** implement multi-tenant remote platform capabilities (`多租户远程平台` remains out of scope).
- It keeps control-plane/data-plane isolation and runtime redaction rules unchanged.

---

## File Structure

### Rust backend

- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/db/migrations.rs`
- Modify: `src-tauri/src/db/repositories.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/gateway/auth.rs`
- Modify: `src-tauri/src/gateway/server.rs`
- Modify: `src-tauri/src/gateway/routes.rs`
- Modify: `src-tauri/src/logging/runtime.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

### Rust backend (new files)

- Create: `src-tauri/src/commands/network.rs`
- Create: `src-tauri/src/gateway/access.rs`

### Frontend

- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauri.ts`
- Modify: `src/App.tsx`
- Modify: `src/features/overview/overview-page.tsx`

### Frontend (new files)

- Create: `src/features/network/network-page.tsx`

### Tests

- Modify: `src-tauri/tests/gateway_auth.rs`
- Modify: `src-tauri/tests/command_surface.rs`
- Modify: `src-tauri/tests/security_regression.rs`
- Create: `src-tauri/tests/lan_access.rs`
- Create: `src/test/network-page.test.tsx`

### CI

- Modify: `.github/workflows/windows-release-gates.yml`

---

## Task 1: Add Persisted Gateway Access Profile

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/db/migrations.rs`
- Modify: `src-tauri/src/db/repositories.rs`
- Test: `src-tauri/tests/command_surface.rs`

- [ ] **Step 1: Write failing persistence test for gateway access profile**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test command_surface gateway_access_profile`
Expected: FAIL because profile table/model/repository methods are missing.

- [ ] **Step 2: Add concrete domain model in `models.rs`**

Add:
- `GatewayBindMode` enum (`Loopback`, `TrustedLan`)
- `GatewayAccessProfile` struct (`bind_mode`, `bind_host`, `bind_port`, `allow_cidrs`, `updated_at_ms`)

- [ ] **Step 3: Add migration for access profile table and defaults**

Create migration with:
- single-row profile table
- default mode = loopback (`127.0.0.1`)
- default allow-list = empty (ignored in loopback mode)

- [ ] **Step 4: Add repository read/update API**

Implement:
- `load_gateway_access_profile()`
- `save_gateway_access_profile(profile)`
- strict validation guard for invalid port and malformed CIDR entries

- [ ] **Step 5: Re-run persistence tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test command_surface gateway_access_profile`
Expected: PASS with migration + repository roundtrip.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/db/migrations.rs src-tauri/src/db/repositories.rs src-tauri/tests/command_surface.rs
git commit -m "feat(network): add persisted gateway access profile model and storage"
```

## Task 2: Implement Bind Policy + Runtime Restart Wiring

**Files:**
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/gateway/server.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/tests/lan_access.rs`

- [ ] **Step 1: Write failing runtime test for bind target selection**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test lan_access bind_mode_switches_listener`
Expected: FAIL because gateway listener still binds only loopback hardcoded path.

- [ ] **Step 2: Add runtime access-profile cache in app state**

State should expose:
- current effective profile snapshot
- atomic profile refresh on command update
- restart signal channel for gateway listener

- [ ] **Step 3: Refactor gateway server bootstrap to derive socket from profile**

Implement:
- loopback mode: `127.0.0.1:<port>`
- trusted-LAN mode: configurable host (default `0.0.0.0`) + allow-list required
- explicit startup log event with bind mode and host/port (no secrets)

- [ ] **Step 4: Wire controlled restart from state/lib initialization path**

Ensure profile updates trigger graceful server restart and no orphan listener remains.

- [ ] **Step 5: Re-run runtime bind tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test lan_access bind_mode_switches_listener`
Expected: PASS with restart + bind assertions.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/state.rs src-tauri/src/gateway/server.rs src-tauri/src/lib.rs src-tauri/tests/lan_access.rs
git commit -m "feat(network): derive gateway binding from access profile with restart wiring"
```

## Task 3: Enforce Trusted-LAN Access Control In Auth Layer

**Files:**
- Modify: `src-tauri/src/gateway/auth.rs`
- Modify: `src-tauri/src/gateway/routes.rs`
- Modify: `src-tauri/src/logging/runtime.rs`
- Modify: `src-tauri/tests/gateway_auth.rs`
- Modify: `src-tauri/tests/security_regression.rs`

- [ ] **Step 1: Write failing auth tests for CIDR allow-list and rejection events**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test gateway_auth --test security_regression lan_allowlist`
Expected: FAIL because auth currently checks platform key only.

- [ ] **Step 2: Add request-client-IP extraction and policy checks**

Implement:
- extraction from socket address / forwarded metadata (trusted local source only)
- loopback mode: reject any non-loopback source
- trusted-LAN mode: require source IP in configured CIDR list

- [ ] **Step 3: Add deterministic error taxonomy for access-denied**

Return stable machine-readable code (for example: `gateway_access_denied`) with concise reason (`not_loopback`, `cidr_not_allowed`, `missing_client_ip`).

- [ ] **Step 4: Emit structured runtime events for accept/reject**

Add events:
- `gateway.client.accepted`
- `gateway.client.rejected`

Include: `request_id`, `bind_mode`, `client_ip`, `reason`, `platform_key_id` (when present), redacted and grep-friendly.

- [ ] **Step 5: Re-run auth/security tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test gateway_auth --test security_regression lan_allowlist`
Expected: PASS with explicit allow/deny behavior and no secret leakage.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/gateway/auth.rs src-tauri/src/gateway/routes.rs src-tauri/src/logging/runtime.rs src-tauri/tests/gateway_auth.rs src-tauri/tests/security_regression.rs
git commit -m "feat(network): enforce trusted-lan cidr policy in gateway auth"
```

## Task 4: Expose Network Settings Command Surface

**Files:**
- Create: `src-tauri/src/commands/network.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/tests/command_surface.rs`

- [ ] **Step 1: Write failing command-surface tests for network settings CRUD**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test command_surface network_settings`
Expected: FAIL because network commands are not registered.

- [ ] **Step 2: Implement network command handlers**

Add commands:
- `get_gateway_access_profile`
- `update_gateway_access_profile`
- `test_gateway_access_profile` (validation only, no bind change)

- [ ] **Step 3: Register commands and enforce validation contract**

Validation rules:
- trusted-LAN mode cannot save empty CIDR list
- CIDR parsing must be strict
- disallow privileged/invalid port values

- [ ] **Step 4: Re-run command tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test command_surface network_settings`
Expected: PASS with stable response payload and error codes.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/network.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs src-tauri/tests/command_surface.rs
git commit -m "feat(commands): add gateway network settings command surface"
```

## Task 5: Build Network Settings UI And Overview Warnings

**Files:**
- Create: `src/features/network/network-page.tsx`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauri.ts`
- Modify: `src/App.tsx`
- Modify: `src/features/overview/overview-page.tsx`
- Create: `src/test/network-page.test.tsx`

- [ ] **Step 1: Write failing frontend tests for mode switch + CIDR editor flow**

Run: `bun test src/test/network-page.test.tsx`
Expected: FAIL because network page and bindings do not exist.

- [ ] **Step 2: Add typed API bindings and DTO definitions**

Add type-safe frontend contracts for network profile read/update/test commands.

- [ ] **Step 3: Implement network settings page**

Page requirements:
- mode selector (`loopback`, `trusted_lan`)
- host/port fields
- CIDR list editor with inline validation
- safe apply flow (`validate` -> `save` -> restart prompt/result)

- [ ] **Step 4: Integrate route/nav and overview risk banner**

Overview should show warning banner when `trusted_lan` mode is enabled, with effective bind and allow-list summary.

- [ ] **Step 5: Re-run frontend tests**

Run: `bun test src/test/network-page.test.tsx src/test/app-shell.test.tsx`
Expected: PASS with navigation + form validation assertions.

- [ ] **Step 6: Commit**

```bash
git add src/features/network/network-page.tsx src/lib/types.ts src/lib/tauri.ts src/App.tsx src/features/overview/overview-page.tsx src/test/network-page.test.tsx src/test/app-shell.test.tsx
git commit -m "feat(ui): add trusted-lan network settings and overview warnings"
```

## Task 6: Add Release Gates For LAN Security Regression

**Files:**
- Modify: `.github/workflows/windows-release-gates.yml`
- Modify: `src-tauri/tests/lan_access.rs`
- Modify: `src-tauri/tests/security_regression.rs`

- [ ] **Step 1: Write failing CI assertion for LAN security suite**

Run locally:
`cargo test --manifest-path src-tauri/Cargo.toml --test lan_access --test security_regression`
Expected: FAIL if workflow and/or tests do not include LAN cases.

- [ ] **Step 2: Add deterministic LAN test matrix**

Cover:
- loopback mode rejecting non-loopback clients
- trusted-LAN mode accepting allowed CIDR
- trusted-LAN mode rejecting disallowed CIDR
- diagnostics/runtime events contain no secrets

- [ ] **Step 3: Update Windows release workflow**

Require LAN suite in release gates and fail build on missing LAN regression coverage.

- [ ] **Step 4: Re-run full release gate command set**

Run:
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `bun test`

Expected: PASS with LAN checks integrated.

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/windows-release-gates.yml src-tauri/tests/lan_access.rs src-tauri/tests/security_regression.rs
git commit -m "ci(release): gate trusted-lan mode with security regression suite"
```

## Task 7: End-To-End Verification And Rollback Safety

**Files:**
- Modify: `docs/superpowers/specs/2026-04-11-codex-local-gateway-desktop-design.md`
- Modify: `docs/superpowers/plans/2026-04-13-v1-1-trusted-lan-sharing-plan.md`

- [ ] **Step 1: Run full verification suite**

Run:
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `bun test`

Expected: PASS with no flaky network-mode tests.

- [ ] **Step 2: Manual smoke checklist (Windows)**

Verify:
- loopback default remains unchanged after upgrade
- switching to trusted-LAN restarts listener and updates overview/tray state
- switching back to loopback is immediate rollback path

- [ ] **Step 3: Update spec status markers**

Mark that previous V1 non-goal `局域网或公网共享节点能力` is now partially delivered as `trusted-lan single-tenant` in V1.1, while multi-tenant/public exposure remains deferred.

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/specs/2026-04-11-codex-local-gateway-desktop-design.md docs/superpowers/plans/2026-04-13-v1-1-trusted-lan-sharing-plan.md
git commit -m "docs(v1.1): record trusted-lan rollout and remaining deferred scope"
```

---

## Spec Coverage Check

- `3.2 非目标` #1 (`局域网或公网共享节点能力`): covered by Tasks 1-6 with constrained trusted-LAN rollout.
- `11.2 网关接口` local-access discipline: preserved and expanded with explicit policy commands and auth checks (Tasks 3-4).
- Security/redaction constraints from runtime logging section: preserved via Task 3 + Task 6 regression requirements.

Remaining deferred items (explicitly still out of this plan):
- 多租户远程平台
- 复杂负载均衡和并行竞速请求
- 所有中转站的通用余额适配
- 不能提供真实额度时的官方额度估算
- 完整 UI 自动化测试体系

## Placeholder Scan

- No `TODO`, `TBD`, or deferred pseudo-steps.
- Every task includes concrete files, concrete commands, and explicit pass/fail expectations.

## Type Consistency Check

- Backend naming remains consistent around `GatewayAccessProfile` and `GatewayBindMode`.
- Frontend DTO naming is aligned with backend command surface (`get/update/test_gateway_access_profile`).
- Error contract in auth layer uses a single stable category for access policy rejection.

