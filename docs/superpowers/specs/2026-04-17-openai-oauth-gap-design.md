# CodexLAG OpenAI Official OAuth Gap Assessment

## Goal

Define the shortest complete path to bring CodexLAG's OpenAI official account login flow up to the practical completeness already evidenced in local CLIProxyAPI, without inventing unsupported OpenAI platform behaviors.

## Baseline

- Reference repo: `E:/Projects/3rdp/agents/CLIProxyAPI`
- CodexLAG currently has a working browser PKCE skeleton:
  - browser launch
  - loopback callback server
  - state validation
  - authorization-code exchange
  - refresh-token refresh
  - secret persistence
- CodexLAG does not currently turn a successful OAuth session into a gateway-usable imported official account.
- CodexLAG stores `id_token` but does not parse or project plan/subscription claims from it.
- CLIProxyAPI does not appear to call a separate OpenAI plan endpoint. It derives `plan_type` and subscription metadata from JWT claims carried in the OAuth `id_token`.

## Design Decision

CodexLAG should treat OpenAI official account completion as a three-layer feature:

1. Auth transport completion
2. Runtime identity and inventory completion
3. Claim-derived entitlement projection

These layers should be implemented in this order. Do not start with UI polish or model gating before the runtime can trust and persist the account identity.

## Current State

### Implemented

- PKCE browser login flow in `src-tauri/src/auth/openai.rs`
- loopback callback handling and code exchange in `src-tauri/src/auth/openai.rs`
- refresh support and session persistence in `src-tauri/src/auth/openai.rs` and `src-tauri/src/auth/session_store.rs`
- Tauri command surface in `src-tauri/src/commands/accounts.rs`
- frontend invocation hooks in `src/lib/tauri.ts` and `src/features/accounts/accounts-page.tsx`

### Missing

- post-login bridge from `credential://auth/...` session storage into imported official account storage using `credential://official/...`
- trusted account identity derivation from `id_token`
- `plan_type` and subscription field extraction from `id_token`
- capability projection based on parsed claims
- explicit degraded-state handling for callback failure, refresh failure, or malformed claims
- tests proving gateway usability of OAuth-created official accounts

## Complete Gap Checklist

### 1. Complete the OAuth-to-official-account bridge

- Add a runtime conversion step after successful code exchange and persistence.
- The conversion must create or update an imported official account record automatically.
- The resulting official account must point at secrets that the gateway already knows how to consume.
- Choose one storage strategy and keep it consistent:
  - preferred: write imported official account records that reference the existing OAuth secrets directly by moving the gateway to accept `credential://auth/...`
  - fallback: duplicate the OAuth secrets into `credential://official/...` and keep both stores synchronized
- Recommendation: prefer the first option to avoid duplicated secret lifecycles.

### 2. Parse and trust identity claims

- Add `id_token` parsing logic for OpenAI official sessions.
- Extract at minimum:
  - stable account identifier
  - email
  - display identity fields if present
  - claim block under `https://api.openai.com/auth`
- Validate only what CodexLAG can locally validate with confidence.
- If signature verification is not yet feasible in the desktop runtime, mark the parsed identity as unverified but still usable with explicit status.

### 3. Implement plan and subscription projection

- Mirror CLIProxyAPI's behavior boundary:
  - parse `chatgpt_plan_type`
  - parse subscription active start
  - parse subscription active until
  - carry last-checked timestamp when available
- Treat these as claim-derived metadata, not billing truth.
- Add a normalized internal structure for entitlement projection.
- Expose the normalized projection through account capability detail and provider inventory summaries.

### 4. Fix capability semantics

- Stop hardcoding official account quota capability to `false`.
- Replace current capability logic with:
  - `refresh_capability`: derived from refresh token availability and last refresh result
  - `balance_capability`: still `NonQueryable` unless a supported public endpoint exists
  - `quota_capability`: unknown or claim-derived, not hardcoded false
  - `plan_capability`: new claim-derived field
- Preserve a strict distinction between:
  - unsupported by OpenAI public APIs
  - unknown because claims are absent
  - known from claims

### 5. Make OAuth-created accounts gateway-usable

- Ensure `project_provider_inventory_summary` includes OAuth-created official accounts as available candidates.
- Ensure `official_session_for_candidate` can load the secrets produced by the OAuth path.
- Add an end-to-end backend test where:
  - browser login completes
  - official account becomes visible in inventory
  - gateway resolves it as a usable candidate

### 6. Add degraded-state and recovery behavior

- Persist callback failure and refresh failure summaries instead of silently dropping them.
- Surface status values such as:
  - `pending`
  - `active`
  - `degraded`
  - `reauth_required`
- Add user-facing guidance for:
  - state mismatch
  - missing refresh token
  - invalid claim payload
  - expired session

### 7. Expand the management surface

- Add account capability output fields for claim-derived plan metadata.
- Show plan data as:
  - source = `id_token_claim`
  - trust = `derived`
- Do not label the result as billing balance or platform subscription truth.

### 8. Test matrix

- Unit tests:
  - JWT claim parsing
  - missing-claim behavior
  - malformed `id_token`
  - account bridge creation/update
- Integration tests:
  - callback exchange persists session and official account
  - refresh updates account status
  - inventory exposes OAuth-created official account
  - gateway accepts OAuth-created official account
- UI tests:
  - onboarding reflects active/degraded/reauth-required status
  - capability panel shows claim-derived plan metadata

## Recommended Implementation Order

1. Add JWT parsing and normalized entitlement model.
2. Add OAuth-to-official-account bridge.
3. Update gateway and inventory to consume bridged accounts.
4. Replace hardcoded capability fields with derived semantics.
5. Add degraded-state persistence and UI/status projection.
6. Add tests, then only after that add model-gating behavior tied to `plan_type`.

## Non-Goals

- Do not invent a remote OpenAI plan-fetch API.
- Do not present claim-derived plan data as authoritative billing data.
- Do not expand to other providers in the same change.
- Do not redesign the whole account UI before runtime parity exists.

## Acceptance Criteria

- A successful OpenAI browser login produces a gateway-usable official account without manual import.
- The account inventory shows active status and derived identity metadata.
- If `id_token` contains OpenAI auth claims, the runtime exposes normalized `plan_type` and subscription window metadata.
- Official account capability output no longer hardcodes quota capability to `false`.
- Failure states are persisted and surfaced as actionable recovery states.
- Tests cover the full login-to-usable-account path plus claim parsing behavior.
