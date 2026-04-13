# V1 Spec Conformance And Control-Plane Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the remaining gaps between the current codebase and `docs/superpowers/specs/2026-04-11-codex-local-gateway-desktop-design.md`, with priority on control-plane completeness, typed error contracts, persistent request lifecycle data, and operator-facing observability.

**Architecture:** Keep the existing Tauri v2 single-process topology, but complete the missing domain and API layers: move from read-only/manage-lite surfaces to full control-plane CRUD/test flows, move from synthetic gateway outcomes to real provider invocation contracts, and move from in-memory usage snapshots to persistent request/attempt and pricing-backed accounting. Keep runtime diagnostics and tray actions as first-class operational controls.

**Tech Stack:** Tauri v2, Rust, Axum, Tokio, Serde, Rusqlite, tauri-plugin-log, React 19, TypeScript, Vitest, GitHub Actions (Windows).

---

## File Structure

### Rust backend

- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/error.rs`
- Modify: `src-tauri/src/db/migrations.rs`
- Modify: `src-tauri/src/db/repositories.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/gateway/auth.rs`
- Modify: `src-tauri/src/gateway/routes.rs`
- Modify: `src-tauri/src/gateway/runtime_routing.rs`
- Modify: `src-tauri/src/gateway/server.rs`
- Modify: `src-tauri/src/providers/mod.rs`
- Modify: `src-tauri/src/providers/official.rs`
- Modify: `src-tauri/src/providers/relay.rs`
- Modify: `src-tauri/src/routing/policy.rs`
- Modify: `src-tauri/src/routing/engine.rs`
- Modify: `src-tauri/src/logging/mod.rs`
- Modify: `src-tauri/src/logging/runtime.rs`
- Modify: `src-tauri/src/logging/usage.rs`
- Modify: `src-tauri/src/tray.rs`
- Modify: `src-tauri/src/commands/accounts.rs`
- Modify: `src-tauri/src/commands/relays.rs`
- Modify: `src-tauri/src/commands/keys.rs`
- Modify: `src-tauri/src/commands/policies.rs`
- Modify: `src-tauri/src/commands/logs.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

### Rust backend (new files)

- Create: `src-tauri/src/providers/invocation.rs`
- Create: `src-tauri/src/logging/redaction.rs`
- Create: `src-tauri/src/tray_summary.rs`

### Frontend

- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauri.ts`
- Modify: `src/features/accounts/accounts-page.tsx`
- Modify: `src/features/relays/relays-page.tsx`
- Modify: `src/features/keys/keys-page.tsx`
- Modify: `src/features/policies/policies-page.tsx`
- Modify: `src/features/logs/logs-page.tsx`
- Modify: `src/features/overview/overview-page.tsx`

### Frontend (new files)

- Create: `src/features/accounts/account-import-form.tsx`
- Create: `src/features/relays/relay-editor.tsx`
- Create: `src/features/keys/key-management-panel.tsx`
- Create: `src/features/policies/policy-editor.tsx`
- Create: `src/features/overview/runtime-log-files-table.tsx`
- Create: `src/features/logs/request-detail-capability-panel.tsx`

### Tests

- Modify: `src-tauri/tests/command_surface.rs`
- Modify: `src-tauri/tests/runtime_logging.rs`
- Modify: `src-tauri/tests/security_regression.rs`
- Modify: `src-tauri/tests/gateway_failover.rs`
- Modify: `src-tauri/tests/failure_recovery.rs`
- Modify: `src-tauri/tests/runtime_composition.rs`
- Modify: `src-tauri/tests/bootstrap_default_key.rs`
- Modify: `src/test/app-shell.test.tsx`
- Create: `src-tauri/tests/error_contract.rs`
- Create: `src-tauri/tests/request_attempt_logging.rs`
- Create: `src-tauri/tests/gateway_provider_integration.rs`
- Create: `src-tauri/tests/runtime_event_schema.rs`
- Create: `src-tauri/tests/runtime_log_metadata.rs`
- Create: `src-tauri/tests/policy_schema_roundtrip.rs`
- Create: `src-tauri/tests/tray_restart.rs`
- Create: `src-tauri/tests/observability_e2e.rs`

### CI

- Modify: `.github/workflows/windows-release-gates.yml`

## Task 1: Expand Core Domain + SQLite Schema To Match Spec Models

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/db/migrations.rs`
- Modify: `src-tauri/src/db/repositories.rs`
- Test: `src-tauri/tests/policy_schema_roundtrip.rs`
- Test: `src-tauri/tests/request_attempt_logging.rs`

- [ ] **Step 1: Write failing persistence tests for policy/request schema**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test policy_schema_roundtrip --test request_attempt_logging`
Expected: FAIL due missing columns/types (`selection_order`, `retry_budget`, request attempt fields).

- [ ] **Step 2: Add missing domain structs/enums in `models.rs`**

Add concrete types for:
- `ProviderEndpoint`
- `CredentialRef`
- `PricingProfile`
- `RequestLog`
- `RequestAttemptLog`
- Policy expansion fields (`selection_order`, `cross_pool_fallback`, `retry_budget`, `failure_rules`, `recovery_rules`)

- [ ] **Step 3: Add migration for new tables/columns and backfill defaults**

Implement migration statements for:
- request log/attempt tables
- pricing profile table
- policy expansion columns
- endpoint/credential reference table(s)

- [ ] **Step 4: Extend repositories with typed CRUD/lookup methods**

Implement repository methods used by commands/gateway for:
- loading/saving expanded policies
- appending request/attempt rows transactionally
- reading active pricing profile by model

- [ ] **Step 5: Re-run schema tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test policy_schema_roundtrip --test request_attempt_logging`
Expected: PASS with migration + roundtrip assertions.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/db/migrations.rs src-tauri/src/db/repositories.rs src-tauri/tests/policy_schema_roundtrip.rs src-tauri/tests/request_attempt_logging.rs
git commit -m "feat(domain): align core models and sqlite schema with v1 spec"
```

## Task 2: Complete Control-Plane Command Surface (Accounts/Relays/Keys/Policies)

**Files:**
- Modify: `src-tauri/src/commands/accounts.rs`
- Modify: `src-tauri/src/commands/relays.rs`
- Modify: `src-tauri/src/commands/keys.rs`
- Modify: `src-tauri/src/commands/policies.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/tests/command_surface.rs`

- [ ] **Step 1: Write failing command-surface tests for missing management actions**

Add failing tests for:
- account import/login command
- relay add/update/delete/test command
- key create/list/disable command
- policy update command

- [ ] **Step 2: Implement account commands**

Add command handlers for:
- importing official session/token credential refs
- validating and persisting account entry

- [ ] **Step 3: Implement relay CRUD + test-connection commands**

Add command handlers for:
- create/update/delete relay
- relay connectivity test (non-data-plane accounting)

- [ ] **Step 4: Implement key inventory commands**

Add command handlers for:
- create platform key
- list keys
- disable/enable key

- [ ] **Step 5: Implement policy update command with validation**

Add command handler for:
- updating routing policy fields with strict validation

- [ ] **Step 6: Register commands in `mod.rs` and `lib.rs`**

Ensure new commands are exported to frontend.

- [ ] **Step 7: Run command tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test command_surface`
Expected: PASS with explicit error messages for invalid input and unknown ids.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/commands/accounts.rs src-tauri/src/commands/relays.rs src-tauri/src/commands/keys.rs src-tauri/src/commands/policies.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs src-tauri/tests/command_surface.rs
git commit -m "feat(commands): complete v1 control-plane management surface"
```

## Task 3: Replace Synthetic Gateway Outcome Flow With Real Provider Invocation + `/models`

**Files:**
- Create: `src-tauri/src/providers/invocation.rs`
- Modify: `src-tauri/src/providers/mod.rs`
- Modify: `src-tauri/src/gateway/auth.rs`
- Modify: `src-tauri/src/gateway/routes.rs`
- Modify: `src-tauri/src/gateway/runtime_routing.rs`
- Test: `src-tauri/tests/gateway_provider_integration.rs`
- Test: `src-tauri/tests/gateway_failover.rs`

- [ ] **Step 1: Write failing integration tests for real invocation routing**

Cover:
- provider failure then fallback
- all candidates exhausted
- request/attempt ids carried across attempts
- `/models` returns allowed model list for current policy/mode

- [ ] **Step 2: Introduce provider invocation contract**

Create a provider invocation module with typed outcome:
- success payload metadata
- failure class (`timeout`, `429`, `5xx`, auth, config)

- [ ] **Step 3: Refactor `codex_request` to use invocation contract**

Remove synthetic header-plan simulation path and call invocation pipeline.

- [ ] **Step 4: Add `/models` route and capability-aware response**

Expose model list according to active policy and endpoint capability matrix.

- [ ] **Step 5: Re-run gateway tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test gateway_provider_integration --test gateway_failover --test failure_recovery`
Expected: PASS with real invocation fallback behavior.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/providers/invocation.rs src-tauri/src/providers/mod.rs src-tauri/src/gateway/auth.rs src-tauri/src/gateway/routes.rs src-tauri/src/gateway/runtime_routing.rs src-tauri/tests/gateway_provider_integration.rs src-tauri/tests/gateway_failover.rs
git commit -m "feat(gateway): use provider invocation pipeline and add models route"
```

## Task 4: Enforce Typed Error Taxonomy + Stable Gateway Error Contract

**Files:**
- Modify: `src-tauri/src/error.rs`
- Modify: `src-tauri/src/gateway/routes.rs`
- Modify: `src-tauri/src/providers/official.rs`
- Modify: `src-tauri/src/providers/relay.rs`
- Create: `src-tauri/tests/error_contract.rs`
- Modify: `src/lib/tauri.ts`
- Modify: `src/lib/types.ts`

- [ ] **Step 1: Write failing tests for spec error categories**

Cover:
- `CredentialError`
- `QuotaError`
- `RoutingError`
- `UpstreamError`
- `ConfigError`

- [ ] **Step 2: Replace string-only error type with typed enum hierarchy**

Implement structured error with:
- stable `code`
- category
- user-safe message
- internal context (non-UI)

- [ ] **Step 3: Map provider and routing failures into taxonomy**

Ensure consistent mapping in gateway response and command response paths.

- [ ] **Step 4: Update frontend parsers/types to consume structured error payload**

Add client-side decoder and narrow types in `tauri.ts` and `types.ts`.

- [ ] **Step 5: Run error and integration tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test error_contract --test gateway_provider_integration`
Expected: PASS with stable machine-readable codes.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/error.rs src-tauri/src/gateway/routes.rs src-tauri/src/providers/official.rs src-tauri/src/providers/relay.rs src-tauri/tests/error_contract.rs src/lib/tauri.ts src/lib/types.ts
git commit -m "feat(error): introduce typed error taxonomy and stable contracts"
```

## Task 5: Standardize Runtime Observability Schema + Redaction + Rich Metadata

**Files:**
- Create: `src-tauri/src/logging/redaction.rs`
- Modify: `src-tauri/src/logging/runtime.rs`
- Modify: `src-tauri/src/logging/mod.rs`
- Modify: `src-tauri/src/commands/logs.rs`
- Modify: `src-tauri/src/state.rs`
- Create: `src-tauri/tests/runtime_event_schema.rs`
- Create: `src-tauri/tests/runtime_log_metadata.rs`
- Modify: `src-tauri/tests/security_regression.rs`
- Modify: `src-tauri/tests/runtime_logging.rs`

- [ ] **Step 1: Write failing runtime schema and metadata tests**

Cover required fields:
- `component`, `event`, `request_id`, `attempt_id`, `endpoint_id`, `latency_ms`, `error_code`
- metadata file entries with name/path/size/mtime

- [ ] **Step 2: Add centralized redaction policy module**

Implement redaction for:
- bearer tokens
- `ck_local_`/API-key patterns
- query params with key/token/session semantics

- [ ] **Step 3: Apply schema and redaction to all runtime event emitters**

Wire logger helpers in gateway/routing/provider/log export paths.

- [ ] **Step 4: Upgrade runtime log metadata command payload**

Return bounded recent file metadata list instead of filename-only list.

- [ ] **Step 5: Run logging/security tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test runtime_event_schema --test runtime_log_metadata --test runtime_logging --test security_regression`
Expected: PASS with sanitized payloads and complete event fields.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/logging/redaction.rs src-tauri/src/logging/runtime.rs src-tauri/src/logging/mod.rs src-tauri/src/commands/logs.rs src-tauri/src/state.rs src-tauri/tests/runtime_event_schema.rs src-tauri/tests/runtime_log_metadata.rs src-tauri/tests/runtime_logging.rs src-tauri/tests/security_regression.rs
git commit -m "feat(observability): standardize runtime schema and strengthen redaction"
```

## Task 6: Complete Usage + PricingProfile Accounting Pipeline

**Files:**
- Modify: `src-tauri/src/logging/usage.rs`
- Modify: `src-tauri/src/commands/logs.rs`
- Modify: `src-tauri/src/db/repositories.rs`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauri.ts`
- Modify: `src/features/logs/logs-page.tsx`
- Create: `src/features/logs/request-detail-capability-panel.tsx`
- Create: `src-tauri/tests/observability_e2e.rs`

- [ ] **Step 1: Write failing tests for usage dimensions and provenance**

Cover required fields:
- input/output/cache-read/cache-write/reasoning/total
- provenance (`actual`, `estimated`, `unknown`)
- pricing profile selection by model/time

- [ ] **Step 2: Extend usage record/detail models**

Add:
- `reasoning_tokens`
- declared capability requirements
- effective capability result
- final upstream status/error details

- [ ] **Step 3: Integrate `PricingProfile` lookup into cost calculation**

Calculate cost using active profile rules and persist `estimated` marker.

- [ ] **Step 4: Expose enriched request detail through command + frontend types**

Ensure logs page can render capability match and final status context.

- [ ] **Step 5: Run usage and e2e tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test request_attempt_logging --test observability_e2e`
Expected: PASS with complete usage dimensions and pricing provenance.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/logging/usage.rs src-tauri/src/commands/logs.rs src-tauri/src/db/repositories.rs src/lib/types.ts src/lib/tauri.ts src/features/logs/logs-page.tsx src/features/logs/request-detail-capability-panel.tsx src-tauri/tests/observability_e2e.rs
git commit -m "feat(usage): add pricing-profile accounting and enriched request detail"
```

## Task 7: Expand Tray To Full Operations Summary + Real Restart Behavior

**Files:**
- Create: `src-tauri/src/tray_summary.rs`
- Modify: `src-tauri/src/tray.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/gateway/server.rs`
- Modify: `src-tauri/src/commands/accounts.rs`
- Modify: `src-tauri/src/commands/relays.rs`
- Create: `src-tauri/tests/tray_restart.rs`
- Modify: `src-tauri/tests/runtime_composition.rs`
- Modify: `src-tauri/tests/bootstrap_default_key.rs`

- [ ] **Step 1: Write failing tray tests for required summary lines and restart**

Cover:
- gateway status
- listen address
- available official/relay counts
- last balance refresh summary
- functional restart menu action

- [ ] **Step 2: Implement tray summary model builder**

Create a dedicated summary builder with deterministic label format.

- [ ] **Step 3: Wire restart action to runtime gateway restart path**

Implement state transition and failure-safe restart handling.

- [ ] **Step 4: Update tray rendering and event handling**

Use summary model + restart action result feedback.

- [ ] **Step 5: Run tray/runtime tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test tray_restart --test runtime_composition --test bootstrap_default_key`
Expected: PASS with restart and summary assertions.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/tray_summary.rs src-tauri/src/tray.rs src-tauri/src/state.rs src-tauri/src/gateway/server.rs src-tauri/src/commands/accounts.rs src-tauri/src/commands/relays.rs src-tauri/tests/tray_restart.rs src-tauri/tests/runtime_composition.rs src-tauri/tests/bootstrap_default_key.rs
git commit -m "feat(tray): add operational summary and functional gateway restart"
```

## Task 8: Build Full Frontend Management UX For Accounts/Relays/Keys/Policies + Runtime Files Table

**Files:**
- Create: `src/features/accounts/account-import-form.tsx`
- Create: `src/features/relays/relay-editor.tsx`
- Create: `src/features/keys/key-management-panel.tsx`
- Create: `src/features/policies/policy-editor.tsx`
- Create: `src/features/overview/runtime-log-files-table.tsx`
- Modify: `src/features/accounts/accounts-page.tsx`
- Modify: `src/features/relays/relays-page.tsx`
- Modify: `src/features/keys/keys-page.tsx`
- Modify: `src/features/policies/policies-page.tsx`
- Modify: `src/features/overview/overview-page.tsx`
- Modify: `src/lib/tauri.ts`
- Modify: `src/lib/types.ts`
- Modify: `src/test/app-shell.test.tsx`

- [ ] **Step 1: Write failing UI tests for management workflows**

Cover:
- account import form submit flow
- relay create/test flow
- key create/disable flow
- policy edit/save flow
- runtime log file metadata table render

- [ ] **Step 2: Implement reusable editor components**

Add dedicated components for account/relay/key/policy flows.

- [ ] **Step 3: Integrate components into page containers**

Wire components to Tauri commands and error states.

- [ ] **Step 4: Update type-safe client APIs**

Align `types.ts` and `tauri.ts` with new command payloads.

- [ ] **Step 5: Run frontend tests**

Run: `npm run test -- src/test/app-shell.test.tsx`
Expected: PASS with new workflow assertions.

- [ ] **Step 6: Commit**

```bash
git add src/features/accounts/account-import-form.tsx src/features/relays/relay-editor.tsx src/features/keys/key-management-panel.tsx src/features/policies/policy-editor.tsx src/features/overview/runtime-log-files-table.tsx src/features/accounts/accounts-page.tsx src/features/relays/relays-page.tsx src/features/keys/keys-page.tsx src/features/policies/policies-page.tsx src/features/overview/overview-page.tsx src/lib/tauri.ts src/lib/types.ts src/test/app-shell.test.tsx
git commit -m "feat(ui): complete control-plane workflows and runtime metadata views"
```

## Task 9: Run Final Reliability + Release Gate Verification

**Files:**
- Modify: `.github/workflows/windows-release-gates.yml`
- Modify: `src-tauri/tests/observability_e2e.rs`
- Modify: `src-tauri/tests/runtime_logging.rs`
- Modify: `src-tauri/tests/failure_recovery.rs`
- Modify: `src-tauri/tests/gateway_provider_integration.rs`

- [ ] **Step 1: Add failing CI assertions for new contract requirements**

Include:
- typed error contract suite
- request/attempt persistence suite
- runtime schema + metadata suite
- tray restart suite

- [ ] **Step 2: Update Windows release gate workflow**

Run complete backend + frontend test matrix used by this plan.

- [ ] **Step 3: Execute local full test pass**

Run:
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `npm run test`

Expected: PASS with no skipped critical suites.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/windows-release-gates.yml src-tauri/tests/observability_e2e.rs src-tauri/tests/runtime_logging.rs src-tauri/tests/failure_recovery.rs src-tauri/tests/gateway_provider_integration.rs
git commit -m "ci(windows): enforce v1 spec conformance release gates"
```

## Spec Coverage Check

- `§9 余额、统计与费用口径`: covered by Task 6.
- `§10 凭据与存储`: covered by Tasks 1, 2, 5.
- `§11 管理接口与本地网关接口`: covered by Tasks 2, 3.
- `§12 Desktop UI 与托盘`: covered by Tasks 7, 8.
- `§13 错误模型`: covered by Task 4.
- `§14 测试策略`: covered by Tasks 1, 3, 5, 6, 7, 9.
- `§16 成功标准`: covered by all tasks; final acceptance in Task 9.
- `§18 Superpowers 下一步计划（增量）`: hardened and extended by Tasks 5, 6, 9.

## Placeholder Scan

- No `TODO`, `TBD`, or deferred pseudo-steps.
- Every task includes concrete files, concrete test commands, and concrete commit boundaries.

## Type Consistency Check

- Backend and frontend contracts are kept in sync through paired changes in `src-tauri/src/commands/*`, `src/lib/types.ts`, and `src/lib/tauri.ts`.
- Request lifecycle, capability, and error fields are introduced once in domain/schema and reused across command + UI surfaces.
