# V1 Release Gates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Consolidate the release-gate workflow, final documentation tree, and verification surface so v1 completion is judged from one current plan, one current doc structure, and repeatable frontend/backend verification.

**Architecture:** Keep the release-gate logic simple: one active v1 plan, one active spec tree, and one CI path that runs the same high-signal checks used locally. The release-gate plan is the close-out layer after implementation workstreams have landed, not a place to hide unfinished product work.

**Tech Stack:** GitHub Actions (Windows), Bun, Rust/Cargo, existing docs tree, existing test suites.

**Status:** Completed locally on 2026-04-14. Historical implementation steps are retained for traceability; checkbox state below reflects the completed repository state.

---

## File Structure

### CI

- Modify: `.github/workflows/windows-release-gates.yml`

### Docs

- Modify: `docs/superpowers/specs/2026-04-11-product-design.md`
- Modify: `docs/superpowers/specs/2026-04-13-v1-completion-design.md`
- Modify: `docs/superpowers/specs/foundation/codexlag-foundation.md`
- Modify: `docs/superpowers/plans/2026-04-13-v1-completion-plan.md`

### Verification

- No new code files required

## Task 1: Align CI Release Gates With The Actual V1 Verification Contract

**Files:**
- Modify: `.github/workflows/windows-release-gates.yml`

- [x] **Step 1: Write the failing CI-content check as a local expectation**

```bash
cat .github/workflows/windows-release-gates.yml
```

Expected additions:

```text
- bun install --frozen-lockfile
- bun run test
- cargo test --manifest-path src-tauri/Cargo.toml
```

- [x] **Step 2: Verify the current workflow is missing or incomplete**

Run: `rg -n "bun run test|cargo test --manifest-path src-tauri/Cargo.toml" .github/workflows/windows-release-gates.yml`
Expected: MISS or incomplete coverage for the current v1 release gate.

- [x] **Step 3: Update the Windows release-gate workflow**

```yaml
# .github/workflows/windows-release-gates.yml
- name: Install frontend dependencies
  run: bun install --frozen-lockfile

- name: Run frontend tests
  run: bun run test

- name: Run Rust tests
  run: cargo test --manifest-path src-tauri/Cargo.toml
```

- [x] **Step 4: Verify the workflow contains the expected gates**

Run: `rg -n "bun install --frozen-lockfile|bun run test|cargo test --manifest-path src-tauri/Cargo.toml" .github/workflows/windows-release-gates.yml`
Expected: 3 matches.

- [x] **Step 5: Commit**

```bash
git add .github/workflows/windows-release-gates.yml
git commit -m "chore: align windows release gates with v1 verification"
```

## Task 2: Lock The Spec Tree To Foundation + Product + Completion

**Files:**
- Modify: `docs/superpowers/specs/2026-04-11-product-design.md`
- Modify: `docs/superpowers/specs/2026-04-13-v1-completion-design.md`
- Modify: `docs/superpowers/specs/foundation/codexlag-foundation.md`

- [x] **Step 1: Verify the expected spec tree**

Run: `find docs/superpowers/specs -maxdepth 2 -type f | sort`
Expected:

```text
docs/superpowers/specs/2026-04-11-product-design.md
docs/superpowers/specs/2026-04-13-v1-completion-design.md
docs/superpowers/specs/foundation/codexlag-foundation.md
```

- [x] **Step 2: Confirm the product and completion specs reference the foundation spec**

Run: `rg -n "codexlag-foundation" docs/superpowers/specs/2026-04-11-product-design.md docs/superpowers/specs/2026-04-13-v1-completion-design.md`
Expected: matches in both files.

- [x] **Step 3: Keep only shared rules in the foundation spec**

```markdown
<!-- docs/superpowers/specs/foundation/codexlag-foundation.md -->
- document layering
- version boundary notation
- CLIProxyAPI alignment
- secret and storage rules
- runtime log vs business audit boundaries
- language rules
- testing and release-gate rules
```

- [x] **Step 4: Re-run the spec-tree verification**

Run: `find docs/superpowers/specs -maxdepth 2 -type f | sort && rg -n "codexlag-foundation" docs/superpowers/specs/2026-04-11-product-design.md docs/superpowers/specs/2026-04-13-v1-completion-design.md`
Expected: expected spec tree plus references intact.

- [x] **Step 5: Commit**

```bash
git add docs/superpowers/specs/2026-04-11-product-design.md docs/superpowers/specs/2026-04-13-v1-completion-design.md docs/superpowers/specs/foundation/codexlag-foundation.md
git commit -m "docs: lock codexlag specs to foundation product and completion"
```

## Task 3: Lock The Plan Tree To One Master Plan Plus Dispatchable Workstream Plans

**Files:**
- Modify: `docs/superpowers/plans/2026-04-13-v1-completion-plan.md`

- [x] **Step 1: Verify the expected plan tree**

Run: `find docs/superpowers/plans -maxdepth 1 -type f | sort`
Expected:

```text
docs/superpowers/plans/2026-04-13-v1-completion-plan.md
docs/superpowers/plans/2026-04-13-v1-gateway-host-plan.md
docs/superpowers/plans/2026-04-13-v1-newapi-relay-plan.md
docs/superpowers/plans/2026-04-13-v1-official-provider-plan.md
docs/superpowers/plans/2026-04-13-v1-platform-keys-plan.md
docs/superpowers/plans/2026-04-13-v1-request-lifecycle-plan.md
docs/superpowers/plans/2026-04-13-v1-routing-policy-plan.md
docs/superpowers/plans/2026-04-13-v1-runtime-candidates-plan.md
docs/superpowers/plans/2026-04-13-v1-ui-foundation-plan.md
docs/superpowers/plans/2026-04-13-v1-ui-pages-plan.md
docs/superpowers/plans/2026-04-13-v1-release-gates-plan.md
```

- [x] **Step 2: Ensure the master plan references the dispatch-ready subplans**

Run: `rg -n "Dispatch-ready subplans|v1-gateway-host-plan|v1-release-gates-plan" docs/superpowers/plans/2026-04-13-v1-completion-plan.md`
Expected: matches covering the subplan references.

- [x] **Step 3: Update the master plan doc section if any subplan links are missing**

```markdown
<!-- docs/superpowers/plans/2026-04-13-v1-completion-plan.md -->
- Dispatch-ready subplans for tasks one through ten:
  - gateway host
  - platform keys
  - runtime candidates
  - official provider
  - newapi relay
  - routing policy
  - request lifecycle
  - UI foundation
  - UI pages
  - release gates
```

- [x] **Step 4: Re-run the plan-tree verification**

Run: `find docs/superpowers/plans -maxdepth 1 -type f | sort && rg -n "Dispatch-ready subplans" docs/superpowers/plans/2026-04-13-v1-completion-plan.md`
Expected: full workstream plan tree plus master-plan references.

- [x] **Step 5: Commit**

```bash
git add docs/superpowers/plans/2026-04-13-v1-completion-plan.md
git commit -m "docs: lock v1 plan tree to master and workstream plans"
```
