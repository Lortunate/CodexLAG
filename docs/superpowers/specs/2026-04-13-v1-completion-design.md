# CodexLAG V1 Completion Design

- Date: 2026-04-13
- Status: Release gates verified locally on 2026-04-14; final V1 sign-off still requires explicit closure of the official-provider scope deviation and plan checkbox backfill
- Project: CodexLAG
- Related references:
  - `docs/superpowers/specs/foundation/codexlag-foundation.md`
  - `docs/superpowers/specs/2026-04-11-product-design.md`
  - `docs/superpowers/plans/2026-04-13-v1-completion-plan.md`
  - `E:/Projects/3rdp/agents/CLIProxyAPI`

## 1. Goal

Complete CodexLAG V1 as a Windows-first Tauri desktop application that exposes a real loopback Codex gateway, a real local control plane, a real local audit trail, and a production-grade desktop UI.

This design does not replace the original 2026-04-11 product spec. It narrows and sequences the remaining work needed to convert the current repository from a well-tested scaffold into a releasable V1.

## 2. Confirmed Scope

### 2.1 V1 Must Deliver

V1 must satisfy all of the following:

- Tauri starts a real HTTP gateway bound to `127.0.0.1`
- `default key` and user-created platform keys can be used for real gateway authentication
- official accounts participate in routing through imported existing login state only
- `newapi` relays participate in routing through real upstream calls and real balance queries
- routing policy actually affects endpoint selection, failover, and recovery
- request logs, request attempt logs, usage records, pricing estimates, and runtime logs form one diagnosable chain
- tray operations apply only to `default key` and reflect actual runtime state
- desktop UI exposes the six primary management areas as usable production screens

### 2.2 V1 Explicitly Defers To V1.1

The following are intentionally excluded from V1 and moved to V1.1:

- official in-app login
- `generic_openai_compatible_no_balance`
- richer model-level capability matrix UI
- enhanced policy authoring UX beyond the minimum needed for V1
- expanded diagnostics and operator-facing analytics beyond V1 release gates

### 2.3 Shared Project Rules

This document inherits project-wide rules from `docs/superpowers/specs/foundation/codexlag-foundation.md`.

Those shared rules include:

- document layering
- version boundary notation
- `CLIProxyAPI` alignment
- security and secret-storage rules
- runtime log vs business audit log boundaries
- comment and runtime-message language rules
- testing and release-gate expectations

## 3. Current Gap Summary

The current repository already contains:

- Tauri shell, tray, and command registration
- SQLite schema for key entities
- Windows credential storage abstraction
- routing engine primitives and broad test coverage
- runtime logging foundation
- React shell with six navigation sections

The repository does not yet satisfy V1 because:

- the gateway still routes through fixed placeholder endpoints instead of persisted account/relay state
- provider execution is still dominated by test doubles and placeholder success data
- platform key creation does not issue a real secret
- policy persistence exists, but policy fields do not drive runtime endpoint selection
- business request audit data is not yet fully persisted and queried from the main runtime path
- the desktop UI is still closer to an integration prototype than a release-ready control console

## 4. Architecture Direction

V1 continues to use one Tauri host process, but the implementation must harden the boundaries that already exist:

1. `Desktop UI`
2. `Tauri command control plane`
3. `Loopback Axum gateway`
4. `Routing policy engine`
5. `Official account adapter`
6. `NewAPI relay adapter`
7. `SQLite persistence`
8. `Windows secret storage`
9. `Runtime log and business audit subsystems`

The design principle for the remaining work is:

- remove placeholder runtime behavior
- keep testability
- prefer explicit state transitions and persisted evidence
- keep all new comments and runtime log messages in English

## 5. Workstream Design

## 5.1 Workstream A: Real Runtime And Gateway Lifecycle

### Purpose

Replace the current pseudo-gateway path with a real loopback HTTP server that is started and owned by the Tauri runtime.

### Required outcomes

- start a real Axum server at application bootstrap
- bind only to `127.0.0.1`
- make gateway lifecycle restartable from tray/control-plane actions
- upgrade `/codex/request` from summary endpoint to true request handling entrypoint
- make `/models` reflect real routable endpoints instead of hard-coded placeholders

### Non-goals

- remote exposure
- LAN sharing
- protocol generalization beyond what V1 needs

## 5.2 Workstream B: Real Provider Adapters

### Purpose

Turn official-account import and `newapi` relay definitions into routable upstream providers.

### V1 official provider scope

- import existing login state only
- validate and hydrate runtime session state from imported credentials
- perform minimum viability checks needed for routing
- expose only official feature semantics that already exist in `CLIProxyAPI`

### V1 relay scope

- support `newapi` only
- execute real upstream calls
- execute real balance queries when supported
- normalize upstream errors and usage data

### Design rules

- provider code used in production must not depend on the current in-memory invocation fixture pipeline
- test doubles may remain, but must be isolated to tests and explicit adapter fakes

## 5.3 Workstream C: Routing Policy That Actually Controls Runtime

### Purpose

Make persisted policy state the source of truth for runtime candidate construction and failover behavior.

### Required runtime inputs

- platform key `allowed_mode`
- policy `selection_order`
- policy `cross_pool_fallback`
- policy `failure_rules`
- policy `recovery_rules`
- provider enabled state
- provider credential validity
- provider balance/query capability state where applicable
- provider capability compatibility where applicable

### Required state model

Health state must support:

- `healthy`
- `degraded`
- `open_circuit`
- `half_open`
- `disabled`

### Required observability

Every request path must make it possible to explain:

- why the selected endpoint was chosen
- why an earlier candidate was skipped or abandoned
- why a downgrade or pool crossover occurred

## 5.4 Workstream D: Platform Key Issuance And Default Key Semantics

### Purpose

Promote platform keys from metadata rows into usable local credentials.

### Required outcomes

- creating a platform key generates a real secret
- the secret is written to `SecretStore`
- the UI can show or copy the secret once at creation time
- the new key can immediately authenticate against the loopback gateway
- `default key` keeps its bootstrap behavior
- disabling `default key` changes tray behavior to read-only status feedback

## 5.5 Workstream E: Audit And Observability Closure

### Purpose

Separate runtime diagnostics from business audit data while making them correlate cleanly.

### Required persisted business records

- request main log
- request attempt log
- usage ledger facts
- pricing profile resolution

### Required diagnostic surfaces

- runtime log directory metadata
- diagnostics export
- stable runtime event field schema
- request/attempt correlation across runtime and business logs

### Required rule

The main runtime path must query persisted business records, not only in-memory usage snapshots.

## 5.6 Workstream F: Production Desktop UI

### UI stack

The desktop UI will be rebuilt on:

- `shadcn/ui`
- `Radix`
- `Tailwind CSS`
- `TanStack Table`
- `React Hook Form`
- `Zod`

### Product direction

The UI should look and behave like a desktop operations console, not like a test harness.

Common interaction patterns:

- top-level shell with stable navigation
- data tables for inventories and logs
- right-side detail drawers
- modal or sheet forms for create/edit flows
- explicit empty, loading, and degraded states
- master-detail request log exploration

### Required V1 pages

1. `Overview`
2. `Accounts`
3. `Relays`
4. `Keys`
5. `Policies`
6. `Logs`

### V1 expectations by page

#### Overview

- gateway status
- default key state and mode switch
- account/relay availability summary
- latest balance refresh summary
- runtime log metadata
- diagnostics export action

#### Accounts

- import existing official login state
- list imported accounts
- show account status
- show capability/balance query status

#### Relays

- create and manage `newapi` relays
- test connectivity
- show real balance status

#### Keys

- create key
- display generated secret once
- enable/disable key
- bind key to policy and mode

#### Policies

- edit real runtime policy fields needed by V1
- validate endpoint references against current account/relay inventory

#### Logs

- request history list
- request detail
- attempt chain detail
- usage and estimated cost detail
- runtime diagnostics references

## 6. V1.1 Design Boundary

V1.1 begins only after V1 release gates are met.

Its priority items are:

- official in-app login
- generic OpenAI-compatible relay without balance support
- richer model-level capability matrix and capability details UI
- more expressive policy authoring UX
- expanded diagnostics/operator visibility

V1.1 must not be used to defer unresolved V1 core-closure defects.

## 7. Testing Strategy

V1 completion requires a different test emphasis from the current scaffold:

### 7.1 Keep

- unit coverage for routing transitions and error contracts
- persistence transaction tests
- runtime log redaction tests
- command contract tests

### 7.2 Add

- integration tests for real candidate construction from persisted state
- tests for platform key issuance and real authentication of newly created keys
- tests for policy-driven endpoint order and failover
- tests proving request/attempt persistence on the runtime path
- tests proving tray semantics when `default key` is unavailable or disabled
- focused frontend tests for page-critical workflows

### 7.3 Release Gate Validation

Before V1 is considered complete, the following must be demonstrated:

- frontend tests pass
- Rust tests pass
- official imported account can complete at least one real request
- `newapi` relay can complete at least one real request
- one official failure can downgrade to relay successfully
- request log, request attempt log, usage, and runtime log can be correlated through identifiers
- newly created platform key can authenticate successfully
- tray state and Overview state remain consistent

## 8. Risks And Controls

### Risk: Gateway stays partially fake

Control:
- remove placeholder runtime candidates from production construction paths
- isolate all invocation fakes to tests

### Risk: Policy remains decorative

Control:
- require runtime selection tests that assert persisted policy changes alter real routing output

### Risk: UI rebuild outruns backend readiness

Control:
- rebuild pages in backend-delivery order, not all at once

### Risk: CLIProxyAPI parity scope expands uncontrollably

Control:
- treat `CLIProxyAPI` only as a behavior baseline for explicit spec items

## 9. Success Definition

V1 is complete only when CodexLAG is a real local desktop gateway product rather than a validated scaffold:

- real local server
- real local credentials
- real provider execution
- real policy-driven routing
- real persisted audit trail
- real tray/control-plane semantics
- real operator-facing desktop UI

That is the handoff target for the follow-up implementation plan.
