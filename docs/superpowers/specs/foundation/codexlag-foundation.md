# CodexLAG Foundation

- Scope: Project-wide rules shared by product specs, completion specs, and implementation plans
- Last Updated: 2026-04-13
- Project: CodexLAG

## 1. Purpose

This document holds the rules that should not be repeated across every CodexLAG spec:

- document layering
- version boundary notation
- `CLIProxyAPI` alignment
- security and secret-storage rules
- runtime log vs business audit log rules
- language rules for comments and runtime messages
- testing and release-gate rules

Business specs should focus on product and implementation intent, then reference this document for common constraints.

## 2. Document Layering

Use the documentation tree with the following responsibilities:

- `docs/superpowers/specs/foundation/`
  - shared project rules and conventions
- `docs/superpowers/specs/`
  - product design, scope, domain model, and release-specific design decisions
- `docs/superpowers/plans/`
  - execution order, tasks, tests, and rollout steps

Specs answer:

- what is being built
- what is in scope
- what is out of scope
- what counts as complete

Plans answer:

- in what order the work will happen
- which files will change
- which tests prove each step

## 3. Version Boundary Notation

Use version labels consistently:

- `v1`
  - release-blocking scope required for the first usable product
- `v1.1`
  - immediate follow-on scope that is intentionally deferred after v1 release gates pass
- `non-goal`
  - work that should not be folded into the active release line

Do not hide unresolved v1 core defects inside a `v1.1` bucket.

## 4. Project Boundaries

The project-wide product boundaries are:

- Windows-first desktop application
- local single-machine control plane plus local single-machine data plane
- loopback-only gateway binding
- no LAN exposure and no public remote serving in v1
- no remote multi-tenant platform behavior in v1

## 5. CLIProxyAPI Alignment

`CLIProxyAPI` is a behavior baseline, not a template to be cloned.

Use it to:

- confirm official feature support boundaries
- confirm request and error semantics when they materially affect compatibility
- confirm usage and routing concepts already implemented there

Do not use it to:

- force 1:1 management-surface parity
- import unrelated server-platform concepts into the desktop app
- invent desktop-only official features that are absent from `CLIProxyAPI`

## 6. Secrets And Storage

The secret-storage rules are stable across all workstreams:

- sensitive credentials must not be stored in SQLite as plaintext
- Windows credential storage is the source of truth for platform key secrets, official sessions/tokens, and relay API keys
- SQLite stores configuration, references, logs, pricing data, health state, and other non-secret state
- frontends and exported diagnostics must not expose secret material

## 7. Logging And Audit Model

CodexLAG has two distinct logging surfaces:

### 7.1 Runtime Logs

Runtime logs are for diagnosis and operator troubleshooting.

They must:

- be emitted to terminal and file
- use stable event names
- support redaction
- be correlatable through request and attempt identifiers

### 7.2 Business Audit Records

Business audit records are for request lifecycle visibility.

They include:

- request main records
- request attempt records
- usage data
- pricing and estimate context

Runtime logs do not replace business audit records, and business audit records do not replace runtime logs.

## 8. Language Rules

The language rules are:

- new code comments must be in English
- runtime log messages must be in English
- error codes should stay stable and machine-readable
- user-facing UI copy can be product-appropriate, but internal diagnostic text should stay grep-friendly

## 9. Testing And Release Gates

Every release-line spec and plan should assume:

- backend stability matters more than full UI automation breadth
- integration tests are required for gateway, provider, routing, and persistence closure
- release readiness is proven by explicit verification, not by partial green tests

For v1-class work, release gates should confirm:

- frontend tests pass
- Rust tests pass
- loopback gateway serves real traffic
- provider routing works on real runtime state
- request, attempt, usage, and runtime logs can be correlated
- secret material remains redacted and out of persisted artifacts
