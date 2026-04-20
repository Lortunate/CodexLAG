# Local UI Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deepen the CodexLAG desktop operator workbench UI by strengthening shell hierarchy, visual language, and scanability on the overview and accounts surfaces without pushing remote changes.

**Architecture:** Keep the existing React/Tauri feature layout intact and improve the UI through focused edits to shared shell components, shared CSS tokens/patterns, and two high-value feature pages. Avoid logic churn and avoid expanding scope into unrelated pages. Work in the current dirty workspace and preserve any pre-existing edits.

**Tech Stack:** React 19, TypeScript, Vite, Vitest, shared CSS in `src/styles.css`

---

### Task 1: Workbench shell and shared visual language

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/components/app-shell.tsx`
- Modify: `src/components/page-header.tsx`
- Modify: `src/styles.css`
- Test: `src/test/app-shell.test.tsx`

- [ ] **Step 1: Add or update shell-focused tests**

Add assertions in `src/test/app-shell.test.tsx` for the shell landmarks and page framing that must remain stable after the redesign:
- the primary navigation is still reachable by label
- the active workspace title is visible
- the top-level application heading still exposes `CodexLAG`

- [ ] **Step 2: Run shell tests to confirm current baseline**

Run: `bash -lc "cd /e/Projects/Rust/CodexLAG && npm test -- --run src/test/app-shell.test.tsx"`

Expected: existing shell test suite passes before visual refactor

- [ ] **Step 3: Implement shell hierarchy improvements**

Update `src/App.tsx`, `src/components/app-shell.tsx`, and `src/components/page-header.tsx` to:
- make the rail feel like a persistent operator sidebar rather than a generic nav list
- add stronger contextual framing in the shell header
- keep the current page switch contract intact (`activePageLabel`, `navigation`, `children`)
- preserve accessibility labels and keyboard-click behavior

- [ ] **Step 4: Strengthen shared tokens and shared layout classes**

Update `src/styles.css` to:
- deepen the restrained dark workbench palette without introducing gradients or glassmorphism
- tighten typography hierarchy and surface layering
- improve page rhythm for headers, summary strips, panels, and dense data sections
- add reusable shared classes only when they support multiple pages

- [ ] **Step 5: Re-run shell tests**

Run: `bash -lc "cd /e/Projects/Rust/CodexLAG && npm test -- --run src/test/app-shell.test.tsx"`

Expected: PASS

- [ ] **Step 6: Report changed files and test outcome**

Do not commit in this task. The workspace is already dirty and the user explicitly forbade push. Report exact files changed, test results, and any concerns.

### Task 2: Overview page scanability and operator summary

**Files:**
- Modify: `src/features/overview/overview-page.tsx`
- Modify: `src/styles.css`
- Test: `src/test/app-shell.test.tsx`

- [ ] **Step 1: Add or update overview-oriented assertions**

Extend `src/test/app-shell.test.tsx` with assertions that confirm the overview page still renders:
- the `Gateway Overview` heading
- the capability matrix section
- the runtime diagnostics section

- [ ] **Step 2: Run overview-related tests before changing markup**

Run: `bash -lc "cd /e/Projects/Rust/CodexLAG && npm test -- --run src/test/app-shell.test.tsx"`

Expected: PASS

- [ ] **Step 3: Rework overview information hierarchy**

Update `src/features/overview/overview-page.tsx` to:
- make the opening summary read like an operator status board, not a row of equivalent cards
- improve differentiation between high-signal state, support metrics, and diagnostic detail
- reduce repeated explanatory copy where the heading already carries the meaning
- preserve all current data loading behavior and command usage

- [ ] **Step 4: Add only the shared or page-specific CSS needed by the new overview layout**

Update `src/styles.css` only where needed to support the refined overview hierarchy, keeping selectors scoped and reusable where practical.

- [ ] **Step 5: Re-run tests**

Run: `bash -lc "cd /e/Projects/Rust/CodexLAG && npm test -- --run src/test/app-shell.test.tsx"`

Expected: PASS

- [ ] **Step 6: Report changed files and test outcome**

Do not commit in this task. Report exact files changed, test results, and any concerns.

### Task 3: Accounts page density and auth-state clarity

**Files:**
- Modify: `src/features/accounts/accounts-page.tsx`
- Modify: `src/styles.css`
- Test: `src/test/app-shell.test.tsx`

- [ ] **Step 1: Add or update accounts assertions**

Extend `src/test/app-shell.test.tsx` with assertions that confirm the accounts page still renders:
- the `Official Accounts` navigation target
- the accounts page heading once navigated
- at least one auth/session-oriented panel heading that should remain visible

- [ ] **Step 2: Run the shell/app test file before editing accounts markup**

Run: `bash -lc "cd /e/Projects/Rust/CodexLAG && npm test -- --run src/test/app-shell.test.tsx"`

Expected: PASS

- [ ] **Step 3: Improve accounts page layout and scanability**

Update `src/features/accounts/accounts-page.tsx` to:
- make provider onboarding, account health, and session management easier to scan as separate operator concerns
- increase density without making the page noisy
- clarify auth-profile and session-state emphasis using layout and typography rather than decorative effects
- preserve existing async behavior, actions, and data dependencies

- [ ] **Step 4: Add only the styles required for the refined accounts layout**

Update `src/styles.css` with the smallest set of new selectors or tokens needed to support the accounts page improvements.

- [ ] **Step 5: Re-run tests**

Run: `bash -lc "cd /e/Projects/Rust/CodexLAG && npm test -- --run src/test/app-shell.test.tsx"`

Expected: PASS

- [ ] **Step 6: Report changed files and test outcome**

Do not commit in this task. Report exact files changed, test results, and any concerns.
