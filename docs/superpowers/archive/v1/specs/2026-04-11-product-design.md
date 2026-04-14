# Codex Local Gateway Desktop Product Design

- Date: 2026-04-11
- Status: Design draft completed and pending written-spec review
- Project: CodexLAG
- Related references:
  - `docs/superpowers/specs/foundation/codexlag-foundation.md`
  - `E:/Projects/3rdp/agents/CLIProxyAPI`

> This document preserves the product-wide vision, domain model, and UI/tray design. Shared project rules such as document layering, `CLIProxyAPI` alignment, security/storage constraints, logging/audit boundaries, and release-gate expectations are centralized in `foundation/codexlag-foundation.md`.

## 1. Background

The goal is to build a Windows-first local Codex gateway desktop application with Tauri 2. The product is both a desktop console and a local API gateway that only listens on loopback addresses. It must manage official Codex accounts and third-party relays, route local Codex client traffic using configurable priority and failover rules, and expose quota, logs, token, and cost views.

The first release is not a generic OpenAI proxy and not a remote multi-tenant cloud platform. Its core is a local control plane plus a local data plane:

- the control plane manages accounts, relays, platform keys, policy, health status, balance refresh, and log queries
- the data plane receives local requests, validates platform keys, selects upstreams according to policy, forwards requests, and records request lifecycle and usage data

`CLIProxyAPI` is a useful reference for routing strategy, usage accounting, and management-interface ideas, but this project does not copy its server-platform shape. The first release target is a single-machine desktop product.

For official-feature support, this project follows only what already exists in `CLIProxyAPI`. It does not introduce new desktop-only official feature semantics.

Relevant reusable patterns from the reference project include:

- capability enrichment and constraints derived from model registration metadata, such as max completion tokens or context-related constraints
- pass-through of upstream-specific behavior through dedicated entrypoints or parameters, such as `compact`-style request paths

## 2. Confirmed Scope

The following boundaries were confirmed during design:

- v1 includes a platform-key system, not only a local proxy
- the local gateway protocol is optimized for Codex usage rather than starting from generic OpenAI-compatibility
- official accounts support both imported existing login state and in-app login
- failover is policy-driven and not limited to hard failure switching
- each platform key can define its own rules rather than using a fixed global mode
- sensitive credentials must use Windows system-level secure storage and must not be stored as plaintext in the database
- official account quota display only uses real queryable values; if unavailable, it must be explicitly marked as non-queryable
- official feature support only follows `CLIProxyAPI` implementations and must not invent desktop-specific official behavior
- the local gateway only listens on `127.0.0.1` or `localhost`
- the first target OS is Windows only
- first launch automatically creates a `default key`
- a custom tray right-click panel is required for `default key` mode switching and status summary

## 3. Product Goals

### 3.1 Functional Goals

V1 must deliver the following capabilities:

- manage official Codex accounts
- manage third-party relay endpoints, with `newapi` as the first supported relay type
- display real remaining quota for official accounts when actually queryable
- display real balance for relay endpoints that support a balance API
- configure priority, enabled state, and tags for each account and relay
- create platform keys for local clients
- bind per-key allowed mode and independent routing policy
- route and fail over between official-account pools and relay pools according to policy
- discover, display, and pass through official features already supported by `CLIProxyAPI`, without inventing desktop-only official features
- record request logs, per-attempt chains, token accounting, and estimated cost
- provide first-launch `default key` bootstrap behavior and tray-based quick mode switching

### 3.2 Non-Goals

V1 explicitly does not include:

- LAN or public shared-node exposure
- a remote multi-tenant platform
- complex load balancing or parallel racing
- universal balance adapters for every relay ecosystem
- fake local quota estimation that pretends to be official quota
- a full end-to-end UI automation suite

## 4. Recommended Architecture

The recommended architecture is a single-process desktop control plane with an embedded local gateway.

### 4.1 Option Selection

Recommended approach:

- a Tauri 2 app as the only host process
- a Rust backend that simultaneously hosts the local HTTP gateway, control-plane services, runtime state, and persistence
- a web frontend for the management interface

Why this is the recommended shape:

- deployment stays simple and fits the single-machine use case
- system-level credential storage integrates more naturally with Windows
- tray, windows, command handling, and background tasks can all be managed by Tauri
- although the runtime is single-process, internal boundaries can still be designed as if they could later be split into separate services

### 4.2 Module Boundaries

The system is divided into the following modules:

1. `Desktop UI`  
   Owns account, relay, platform key, policy, log, statistics, and status presentation.

2. `Control Plane`  
   Owns config validation, config mutation, policy parsing, balance refresh, connection tests, health-status queries, and Tauri commands for the frontend.

3. `Loopback Gateway`  
   Listens only on loopback addresses for local Codex clients. Owns platform-key validation, request normalization, routing, forwarding, and logging.

4. `Routing Engine`  
   Selects a currently available endpoint from the official-account pool and relay pool using the policy bound to the platform key. Supports priority, circuit breaking, recovery, and cross-pool failover.

5. `Provider Adapters`  
   Includes official-account adapters and relay adapters. Normalizes authentication, balance, model capability, usage extraction, and error mapping.

6. `Persistence`  
   Uses SQLite for non-sensitive state and Windows Credential Manager for sensitive credentials.

## 5. Core Runtime Model

### 5.1 ProviderEndpoint

`ProviderEndpoint` represents a traffic-bearing upstream node. It is the common abstraction over official accounts and third-party relay endpoints.

Suggested shared fields:

- `id`
- `name`
- `kind`, with values `official_account` or `relay_endpoint`
- `enabled`
- `priority`
- `pool_tags`
- `health_status`
- `last_health_check_at`
- `supports_balance_query`
- `last_balance_snapshot_at`
- `pricing_profile_id`
- `feature_capabilities`

Suggested official-account-specific fields:

- `auth_mode`, with values `imported_session` or `in_app_login`
- `account_identity`
- `quota_capability`
- `refresh_capability`
- `max_context_window`
- `supports_context_compression`
- `context_compression_modes`

Suggested relay-specific fields:

- `relay_type`, with `newapi` as the minimum v1 supported value
- `base_url`
- `model_mapping`
- `balance_capability`

### 5.2 CredentialRef

`CredentialRef` references sensitive data stored in Windows Credential Manager. The database stores only references, never plaintext secrets.

Suggested fields:

- `id`
- `target_name`
- `version`
- `credential_kind`
- `last_verified_at`

### 5.3 FeatureCapabilities

`FeatureCapabilities` describes official feature support at either endpoint level or model level so that capability decisions are not scattered across UI copy and request transformation logic. The source of truth is whatever `CLIProxyAPI` already supports.

Suggested fields:

- `model_id`
- `max_context_window`
- `supports_context_compression`
- `context_compression_modes`
- `supports_prompt_cache`
- `supports_reasoning`
- `supports_streaming`
- `last_capability_check_at`

Design rules:

- capabilities should be maintained at model granularity whenever possible; if model-level capability data is unavailable, the system may temporarily fall back to endpoint-level defaults
- context-window size must be surfaced as an explicit field, not hidden inside model naming
- only capabilities already implemented in `CLIProxyAPI` should be included
- the desktop app must not implement custom context-compression or other official-feature algorithms that do not exist in the reference project

### 5.4 PlatformKey

`PlatformKey` represents a local gateway key issued to a local Codex client.

Suggested fields:

- `id`
- `name`
- `key_prefix`
- `secret_ref`
- `enabled`
- `allowed_mode`, with values `account_only`, `relay_only`, `hybrid`
- `policy_id`
- `created_at`
- `last_used_at`

Where:

- `allowed_mode` defines the top-level permission boundary
- detailed routing behavior is determined by the bound `RoutingPolicy`

### 5.5 RoutingPolicy

`RoutingPolicy` is the core configuration object and should exist as an independent reusable entity instead of scattering routing logic across the platform-key table.

Suggested fields:

- `id`
- `name`
- `selection_order`
- `cross_pool_fallback`
- `default_timeout_ms`
- `retry_budget`
- `failure_rules`
- `recovery_rules`
- `circuit_breaker_config`
- `request_feature_policy`

Where:

- `selection_order` defines pool order, tag order, and endpoint grouping order
- `failure_rules` defines which failures trigger failover
- `recovery_rules` defines circuit-breaker recovery and half-open probing behavior
- `request_feature_policy` defines how explicitly requested official feature requirements are validated, downgraded, rejected, or rewritten

### 5.6 RequestLog

Each client request produces one main request record.

Suggested fields:

- `request_id`
- `platform_key_id`
- `request_type`
- `model`
- `selected_endpoint_id`
- `attempt_count`
- `final_status`
- `http_status`
- `started_at`
- `finished_at`
- `latency_ms`
- `error_code`
- `error_reason`
- `requested_context_window`
- `requested_context_compression`
- `effective_context_window`
- `effective_context_compression`

### 5.7 RequestAttemptLog

Each failover attempt should be stored as its own record so the full downgrade chain is visible.

Suggested fields:

- `attempt_id`
- `request_id`
- `attempt_index`
- `endpoint_id`
- `pool_type`
- `trigger_reason`
- `upstream_status`
- `timeout_ms`
- `latency_ms`
- `token_usage_snapshot`
- `estimated_cost_snapshot`
- `balance_snapshot_id`
- `feature_resolution_snapshot`

### 5.8 UsageLedger

`UsageLedger` is the fact table for token accounting and cost estimation.

Suggested fields:

- `usage_id`
- `request_id`
- `attempt_id`
- `platform_key_id`
- `endpoint_id`
- `model`
- `input_tokens`
- `output_tokens`
- `cache_read_tokens`
- `cache_write_tokens`
- `reasoning_tokens`
- `total_tokens`
- `estimated_cost`
- `currency`
- `usage_source`
- `price_source`
- `recorded_at`

## 6. Routing And Failover Rules

### 6.1 Mode Definitions

The three mode semantics are fixed:

- `account_only`: candidates may only come from the official-account pool
- `relay_only`: candidates may only come from the relay pool
- `hybrid`: candidates may come from both pools, with order and cross-pool failover controlled by policy

### 6.2 Routing Flow

The standard per-request flow is:

1. a local client sends a request to the local gateway with a platform key
2. the gateway validates the platform key and loads the bound policy
3. the gateway parses requested official feature requirements and only processes features already supported by `CLIProxyAPI`
4. candidate queues are built from `allowed_mode`, `selection_order`, and capability requirements
5. candidates are filtered for disabled state, invalid authentication, known quota exhaustion, open-circuit state, or capability mismatch
6. upstreams are attempted in sequence
7. when a failure matches failover rules, the attempt is logged and the next candidate is tried
8. on success, the system writes main request logs, attempt logs, usage data, and health updates

### 6.3 Configurable Failure Conditions

V1 policy configuration must support at least:

- authentication invalidation
- quota exhaustion
- `429`
- timeout
- repeated `5xx`
- network unreachable

V1 does not use parallel probing; it uses sequential failover only. Every request must remain explainable in terms of both “why this endpoint was selected” and “why the previous endpoint was abandoned.”

### 6.4 Official Feature Handling

The official-feature rules for V1 are:

- only official features already implemented in `CLIProxyAPI` are supported
- the desktop app must not introduce its own official-feature protocol or custom algorithm
- supported official features may be discovered, displayed, passed through, logged, and included in routing decisions

Handling rules:

- if a client explicitly declares a feature requirement, the router should prefer candidates that satisfy it
- if policy allows downgrade, the router may select a lower-capability candidate that can still execute the request, but this downgrade must be recorded
- if policy does not allow downgrade, the gateway should return a capability mismatch error instead of silently rewriting behavior
- if an official endpoint supports a capability already represented in the reference project, such as `compact`-style behavior, the gateway should pass that parameter through rather than simulate the feature locally
- the local gateway must not implement context-compression algorithms or other official-feature algorithms that do not exist in the reference project

Reference-priority rules:

- prefer the `CLIProxyAPI` model-registration approach for context-window and max-token constraints
- prefer the `CLIProxyAPI` `compact` path pass-through approach for compact/compression behavior
- if a feature is absent from the reference project, it is out of V1 scope here too

### 6.5 Circuit Breaking And Recovery

The router health-state machine must at minimum support:

- `healthy`
- `degraded`
- `open_circuit`
- `half_open`
- `disabled`

Recovery logic must support:

- circuit open duration
- half-open probing
- successful probe recovery
- failed probe reopening

## 7. Official Account Management

### 7.1 Onboarding Modes

V1 supports two official-account onboarding modes:

- import existing login state
- in-app login

Once an account enters the system, both modes are normalized into the same `OfficialSession` runtime model so that validation, refresh, balance, and routing logic can be shared.

### 7.2 OfficialSession Runtime Model

Suggested fields:

- `session_id`
- `account_identity`
- `token_bundle_ref`
- `expires_at`
- `refresh_capability`
- `quota_capability`
- `last_verified_at`
- `status`
- `default_feature_capabilities`

### 7.3 Capability Probing

Immediately after import or login, the system should perform a capability probe:

- whether the account is usable
- whether it can refresh
- whether it can query real quota
- which model families or interface families it supports
- which `CLIProxyAPI`-backed official capabilities it supports, such as context-window size or `compact`

If an official account cannot expose real quota, V1 must display “balance not queryable” and must not show a fake estimate.

## 8. Third-Party Relay Management

### 8.1 Adapter Model

Relay balance and invocation details must not be hard-coded in the main flow. The system should define `RelayBalanceAdapter` and `RelayInvocationAdapter`.

Each relay type should provide at least:

- `supports_balance_query()`
- `query_balance()`
- `normalize_balance_response()`
- `query_models_if_supported()`
- `extract_usage()`
- `normalize_error()`

### 8.2 V1 Relay Scope

V1 should support:

- `newapi`
- `generic_openai_compatible_no_balance`

The second type acts as a fallback relay type: it may be used for invocation, but must be explicitly marked as not supporting balance queries.

## 9. Balance, Statistics, And Cost Semantics

### 9.1 Official Account Balance

The balance rule is fixed:

- display only real queryable values
- if unavailable, display “not queryable”
- do not use local estimation to imitate real official quota

### 9.2 Relay Balance

The balance rule is fixed:

- only relays with supported balance adapters may display real balances
- all others must be explicitly marked as unsupported

### 9.3 Token Accounting

Prefer real upstream usage data. If real usage is unavailable, the system must explicitly label the result as unknown or partially estimated.

Standard usage fields include:

- input tokens
- output tokens
- cache read tokens
- cache write tokens
- reasoning tokens
- total tokens

If the upstream returns usage or trimming information related to supported official features, the system should normalize that data into attempt snapshots or usage extension fields so that the UI can surface it later.

### 9.4 Cost Estimation

Cost estimation is handled through `PricingProfile`:

- official accounts should prefer local price tables
- relays should prefer relay-specific pricing tables or user-configured multipliers
- all cost records must be marked as `estimated`

`PricingProfile` should at minimum include:

- model-matching rules
- input unit price
- output unit price
- cache-hit unit price
- currency
- effective time

## 10. Credentials And Storage

### 10.1 Secure Storage

Sensitive data belongs in Windows Credential Manager, including:

- official account tokens or sessions
- third-party relay API keys
- platform key secrets

SQLite stores only:

- config
- references
- logs
- statistics
- balance snapshots
- health state

### 10.2 Persistence Rules

- the frontend must not read or write sensitive plaintext directly
- the gateway runtime must load required secrets through `CredentialRef`
- request log and usage writes must be transactional so the main request table, attempt table, and statistics stay consistent

## 11. Management Interfaces And Local Gateway Interface

### 11.1 Management Interface

The desktop frontend talks to the backend through Tauri commands to perform:

- account login and account import
- relay creation and relay testing
- platform-key creation and disable/enable actions
- policy editing
- balance refresh
- status queries
- aggregated log queries

### 11.2 Gateway Interface

The local HTTP gateway is only exposed to local clients. It should include at least:

- the main Codex request entrypoint
- `GET /health`
- `GET /models` if model enumeration or mapping becomes necessary

The request entrypoint should include a lightweight capability-parsing and compatibility-matching layer:

- identify the target model declared by the client
- identify official feature requirements that are already supported by the reference project
- map those requirements into the capability matrix for candidate endpoints
- return explicit capability mismatch errors when the request cannot be satisfied

If an official feature already has a stable entrypoint in the reference project, such as a `compact` path or a model-capability registration rule, the desktop gateway should preserve that compatibility. If the reference project does not implement the feature, it is out of V1 scope.

Control-plane actions and data-plane actions must remain isolated so that connection tests, balance refreshes, and other UI operations do not pollute real request logs or usage statistics.

## 12. Desktop UI And Tray

### 12.1 Main Interface Structure

The V1 main interface should be organized into six primary sections:

1. Overview
2. Official Accounts
3. Third-Party Relays
4. Platform Keys
5. Policy Center
6. Request Logs And Statistics

The main interface must also expose capability information in explicit UI surfaces, at minimum in:

- official account detail pages showing supported reference-project capabilities such as context-window size or `compact`
- model/capability detail overlays showing a model-level capability matrix
- request-log details showing declared capabilities and effective resolved capabilities

### 12.2 Default Objects On First Launch

First launch automatically creates:

- one enabled `default key`
- one built-in `default policy`

Default behavior:

- `default key.allowed_mode = hybrid`
- `default key.policy_id = default policy`

If the user deletes `default key`, the system does not recreate it automatically. If the user only disables it, tray quick-switch behavior becomes read-only status feedback.

### 12.3 Tray Right-Click Panel

In Tauri v2, the tray menu is an enhanced operational panel rather than a complex editing surface.

Suggested tray items:

- gateway status
- current listen address
- current `default key` mode
- switch to `account_only`
- switch to `relay_only`
- switch to `hybrid`
- current available official-account count
- current available relay count
- latest balance-refresh summary
- open main window
- restart gateway
- quit

Behavior rules:

- the tray only operates on `default key`
- the tray does not directly edit complex routing policy
- left click on the tray icon opens the main window
- right click opens the menu
- if the current mode has no available endpoints, switching is still allowed, but the tray must clearly indicate that the selected mode currently has no usable endpoint

## 13. Error Model

V1 should normalize errors into the following categories:

- `CredentialError`
- `QuotaError`
- `RoutingError`
- `UpstreamError`
- `ConfigError`

The UI should show user-actionable copy and must not expose internal stack traces directly.

Examples:

- This official account login state has expired. Please sign in again.
- This relay type does not currently support balance queries.
- This platform key policy does not contain any available endpoints.
- The request degraded multiple times and all candidates failed.
- The requested official feature is unsupported by all available endpoints, or the feature is outside the V1 support boundary.

## 14. Test Strategy

V1 should emphasize backend stability over full UI automation completeness.

### 14.1 Unit Tests

- router priority and mode filtering
- failure-rule triggering
- circuit-breaking and recovery
- official account capability probing
- capability matching for official features already supported by the reference project
- pass-through and rejection logic for `compact` or similar supported features
- NewAPI balance normalization
- usage extraction and price-table matching

### 14.2 Persistence Tests

- transactional consistency across main request table, attempt table, and statistics table
- platform-key secrets never written as plaintext to the database
- credential-reference validity

### 14.3 Integration Tests

Use local test servers to simulate:

- successful requests
- `429`
- timeout
- repeated `5xx`
- quota exhaustion

Verify:

- correct failover behavior
- correct logging
- correct health-state updates

### 14.4 Tauri Backend Tests

Verify commands for:

- creating and reading platform keys
- importing and validating login state
- testing relay connections
- refreshing balances
- querying logs

## 15. Recommended Implementation Order

Recommended order:

1. project skeleton and baseline runtime
2. SQLite and credential-storage wrapper
3. platform-key and policy model
4. local loopback gateway
5. routing engine, request lifecycle logging, and Tauri runtime logging
6. official account adapter
7. NewAPI relay adapter
8. overview and list-management pages
9. tray and `default key`
10. statistics and balance panels

This order prioritizes the most critical control-plane and data-plane closure before polishing UI completeness.

## 16. Success Criteria

The V1 design is ready to move into implementation planning when the following are true:

- the desktop app and local loopback gateway can start on Windows
- first launch auto-generates `default key`
- a local client can access the local gateway using a platform key
- official accounts and NewAPI relays can both participate in one unified routing layer
- official features already supported by the reference project can be discovered, displayed, logged, and used in routing
- failover rules are configurable and produce explainable logs
- sensitive credentials are not stored as plaintext in the database
- the tray can switch the three `default key` modes and show status summary
- request logs, token accounting, and estimated cost can be viewed by key and by endpoint

## 17. Tauri Runtime Logging System (File Logs + Terminal Logs)

`RequestLog` and `RequestAttemptLog` are business-audit records. The runtime logs defined in this section are for diagnosing system behavior. They are related, but one is not a replacement for the other.

### 17.1 Goals And Boundaries

V1 must satisfy all of the following:

- terminal logs exist so development and local debugging behavior is visible in real time
- file logs exist so startup, routing, upstream failure, and exception behavior can be traced after the fact
- the same event can be correlated to `request_id` and `attempt_id`
- log output is redacted by default and must not leak token, API key, session, or similar secrets

### 17.2 Tauri v2 Integration Approach

Integrate `tauri-plugin-log` in `src-tauri/src/lib.rs` and enable both `Stdout` and `LogDir` targets.

Suggested baseline configuration:

```rust
use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};

// Initialize runtime log system: terminal + local file.
let log_plugin = tauri_plugin_log::Builder::new()
    .target(Target::new(TargetKind::Stdout))
    .target(Target::new(TargetKind::LogDir {
        file_name: Some("gateway".to_string()),
    }))
    .max_file_size(10_000_000)
    .rotation_strategy(RotationStrategy::KeepAll)
    .timezone_strategy(TimezoneStrategy::UseLocal)
    .build();
```

Capabilities must include `log:default` in `src-tauri/capabilities/default.json`.

### 17.3 File Log Policy

- on Windows, the default file location should follow the Tauri `LogDir` convention: `%LOCALAPPDATA%/{bundleIdentifier}/logs`
- the file prefix should be `gateway` so it remains distinct from future UI or diagnostics artifacts
- the default log level should be `Info`, with `Debug` allowed in development
- default single-file size should be 10 MB with rotation and retained history

### 17.4 Terminal Log Policy

- terminal and file output must share the same event semantics
- terminal logs are primarily for real-time debugging and should prioritize key state transitions and failure causes
- terminal logs must remain in English for easier cross-team and cross-region diagnostics

Suggested log-message templates:

```text
[gateway.request.accepted] request_id={request_id} model={model} platform_key_id={platform_key_id}
[routing.endpoint.selected] request_id={request_id} attempt_id={attempt_id} endpoint_id={endpoint_id} reason={reason}
[provider.call.failed] request_id={request_id} attempt_id={attempt_id} endpoint_id={endpoint_id} error_code={error_code} retryable={retryable}
```

### 17.5 Field Conventions And Redaction Rules

Suggested common runtime-log fields include:

- `timestamp`
- `level`
- `component` such as `gateway`, `routing`, `provider`, `control_plane`, `tauri_command`
- `event`
- `request_id`
- `attempt_id`
- `platform_key_id`
- `endpoint_id`
- `latency_ms`
- `error_code`

Redaction rules:

- never record a raw `Authorization` value
- never record raw official session values, third-party API keys, or platform-key secrets
- likely secret-bearing URL query fields must be cleaned before being written to logs

### 17.6 Comment And Log Language Constraints

- new code comments must be in English
- runtime log messages must be in English
- error codes must remain stable and machine-readable, while error messages should remain grep-friendly

## 18. Suggested Superpowers Follow-Up Work

On top of `docs/superpowers/plans/2026-04-13-v1-completion-plan.md`, the following incremental priorities should remain high:

1. **P0: Runtime Log Foundation**
   - add `tauri-plugin-log` and capability permissions
   - initialize `Stdout + LogDir` targets in `lib.rs`
   - define common runtime-log field helpers for request/attempt/component/event
   - provide common redaction helpers for sensitive fields

2. **P1: Gateway / Routing / Provider Runtime Log Wiring**
   - write `gateway.request.accepted` at the gateway entrypoint
   - write `routing.endpoint.selected` and `routing.fallback.triggered` at selection and downgrade points
   - write `provider.call.failed` on upstream failure, including `error_code` and `retryable`
   - guarantee correlation between runtime logs and `RequestLog` / `RequestAttemptLog` through `request_id` and `attempt_id`

3. **P2: Control-Plane Observability Closure**
   - add Tauri commands for log directory and recent log-file metadata
   - add a UI action for exporting diagnostic logs without exporting secrets
   - add integration coverage for file-log persistence, terminal output, and redaction correctness

4. **P3: Pre-Release Validation**
   - verify Windows packaged builds and development mode use consistent log paths and rotation behavior
   - verify `429` / `5xx` / timeout scenarios remain diagnosable through runtime logs
   - verify all new comments and runtime log messages remain English
