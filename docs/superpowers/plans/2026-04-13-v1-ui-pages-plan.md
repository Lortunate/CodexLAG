# V1 UI Pages Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild the six primary CodexLAG pages on top of the new UI foundation so the desktop app becomes a usable operations console for v1.

**Architecture:** Keep the six current page boundaries, but replace prototype-only layouts with structured operations screens using shared shell primitives, data-display cards, and form flows that reflect the real backend contract. This plan assumes the UI foundation is already in place and focuses on page-level behavior, not on adding another design system layer.

**Tech Stack:** React 19, TypeScript, Tailwind CSS v4, shadcn/ui, Radix UI, React Hook Form, Zod, TanStack Table, Vitest, Testing Library.

---

## File Structure

### Frontend

- Modify: `src/App.tsx`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauri.ts`
- Modify: `src/features/overview/overview-page.tsx`
- Modify: `src/features/accounts/accounts-page.tsx`
- Modify: `src/features/accounts/account-import-form.tsx`
- Modify: `src/features/relays/relays-page.tsx`
- Modify: `src/features/relays/relay-editor.tsx`
- Modify: `src/features/keys/keys-page.tsx`
- Modify: `src/features/keys/key-management-panel.tsx`
- Modify: `src/features/policies/policies-page.tsx`
- Modify: `src/features/policies/policy-editor.tsx`
- Modify: `src/features/logs/logs-page.tsx`
- Modify: `src/features/logs/request-detail-capability-panel.tsx`
- Modify: `src/features/default-key/default-key-mode-toggle.tsx`

### Tests

- Modify: `src/test/app-shell.test.tsx`
- Modify: `src/test/tauri.test.ts`

## Task 1: Rebuild Overview, Accounts, And Relays As Production Operations Pages

**Files:**
- Modify: `src/features/overview/overview-page.tsx`
- Modify: `src/features/accounts/accounts-page.tsx`
- Modify: `src/features/accounts/account-import-form.tsx`
- Modify: `src/features/relays/relays-page.tsx`
- Modify: `src/features/relays/relay-editor.tsx`
- Modify: `src/features/default-key/default-key-mode-toggle.tsx`
- Modify: `src/test/app-shell.test.tsx`

- [ ] **Step 1: Write the failing top-of-funnel page test**

```tsx
// src/test/app-shell.test.tsx
it("shows the overview diagnostics panel and default key operations in the rebuilt pages", async () => {
  render(<App />);

  expect(await screen.findByText(/runtime diagnostics/i)).toBeInTheDocument();
  expect(screen.getByText(/default key mode/i)).toBeInTheDocument();
});

it("renders account import and relay creation as structured operations panels", async () => {
  render(<App />);

  await user.click(screen.getByRole("button", { name: /accounts/i }));
  expect(await screen.findByRole("heading", { name: /import official account/i })).toBeInTheDocument();

  await user.click(screen.getByRole("button", { name: /relays/i }));
  expect(await screen.findByRole("heading", { name: /manage relays/i })).toBeInTheDocument();
});
```

- [ ] **Step 2: Run the focused page test to verify it fails**

Run: `bun run test -- src/test/app-shell.test.tsx`
Expected: FAIL because the current page layouts are still prototype-style.

- [ ] **Step 3: Rebuild the Overview page around production status cards**

```tsx
// src/features/overview/overview-page.tsx
return (
  <section>
    <PageHeader
      title="Gateway Overview"
      description="Monitor local runtime health, balance observability, diagnostics, and default key controls."
    />
    <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
      <StatusCard title="Runtime status" value={logSummary?.level ?? "loading"} />
      <StatusCard title="Balance observability" value={`${queryableRelayCount} queryable relays`} />
      <StatusCard title="Usage ledger" value={`${usageLedger?.total_tokens ?? 0} tokens`} />
      <StatusCard title="Runtime diagnostics" value={runtimeLogMetadata?.log_dir ?? "loading"} />
    </div>
    <DefaultKeyModeToggle ... />
  </section>
);
```

- [ ] **Step 4: Rebuild Accounts and Relays as operator pages instead of raw detail grids**

```tsx
// src/features/accounts/accounts-page.tsx
<PageHeader
  title="Official Accounts"
  description="Import existing login state, review provider identity, and inspect capability status."
/>
<AccountImportForm ... />
<div className="grid gap-4 xl:grid-cols-2">
  {accounts.map((panel) => (
    <Card key={panel.account.account_id}>
      <CardHeader>
        <CardTitle>{panel.account.name}</CardTitle>
      </CardHeader>
      <CardContent>{/* capability + balance summary */}</CardContent>
    </Card>
  ))}
</div>
```

```tsx
// src/features/relays/relays-page.tsx
<PageHeader
  title="Relays"
  description="Manage NewAPI relay endpoints, validate connectivity, and inspect balance support."
/>
<RelayEditor ... />
```

- [ ] **Step 5: Run focused frontend tests**

Run: `bun run test -- src/test/app-shell.test.tsx`
Expected: PASS with rebuilt overview, account, and relay pages.

- [ ] **Step 6: Commit**

```bash
git add src/features/overview/overview-page.tsx src/features/accounts/accounts-page.tsx src/features/accounts/account-import-form.tsx src/features/relays/relays-page.tsx src/features/relays/relay-editor.tsx src/features/default-key/default-key-mode-toggle.tsx src/test/app-shell.test.tsx
git commit -m "feat: rebuild overview accounts and relays pages"
```

## Task 2: Rebuild Keys And Policies As Real Control Surfaces

**Files:**
- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauri.ts`
- Modify: `src/features/keys/keys-page.tsx`
- Modify: `src/features/keys/key-management-panel.tsx`
- Modify: `src/features/policies/policies-page.tsx`
- Modify: `src/features/policies/policy-editor.tsx`
- Modify: `src/test/app-shell.test.tsx`
- Modify: `src/test/tauri.test.ts`

- [ ] **Step 1: Write the failing control-surface test**

```tsx
// src/test/app-shell.test.tsx
it("shows the generated platform key secret after key creation", async () => {
  render(<App />);

  await user.click(screen.getByRole("button", { name: /platform keys/i }));
  expect(await screen.findByText(/generated secret/i)).toBeInTheDocument();
});

it("renders policy fields from hydrated backend data", async () => {
  render(<App />);

  await user.click(screen.getByRole("button", { name: /policies/i }));
  expect(await screen.findByLabelText(/retry budget/i)).toBeEnabled();
});
```

- [ ] **Step 2: Run the focused control-surface test to verify it fails**

Run: `bun run test -- src/test/app-shell.test.tsx src/test/tauri.test.ts`
Expected: FAIL because keys and policies still expose incomplete prototype behavior.

- [ ] **Step 3: Update the frontend contract for secret-bearing key creation and hydrated policy fields**

```ts
// src/lib/types.ts
export interface CreatedPlatformKey {
  id: string;
  name: string;
  policy_id: string;
  allowed_mode: DefaultKeyMode;
  enabled: boolean;
  secret: string;
}

export interface PolicySummary {
  policy_id: string;
  name: string;
  status: string;
  selection_order: string[];
  cross_pool_fallback: boolean;
  retry_budget: number;
  timeout_open_after: number;
  server_error_open_after: number;
  cooldown_ms: number;
  half_open_after_ms: number;
  success_close_after: number;
}
```

- [ ] **Step 4: Rebuild Keys and Policies pages as formal control panels**

```tsx
// src/features/keys/keys-page.tsx
<PageHeader
  title="Platform Keys"
  description="Issue local gateway keys, bind runtime policy, and review mode access."
/>
{createdKey ? (
  <Alert>
    <AlertTitle>Generated secret</AlertTitle>
    <AlertDescription>{createdKey.secret}</AlertDescription>
  </Alert>
) : null}
<KeyManagementPanel ... />
```

```tsx
// src/features/policies/policies-page.tsx
<PageHeader
  title="Policies"
  description="Edit runtime endpoint order, retry budget, and recovery thresholds."
/>
<PolicyEditor policies={policies} endpointIds={endpointIds} onSave={handleSavePolicy} />
```

- [ ] **Step 5: Run focused frontend tests**

Run: `bun run test -- src/test/app-shell.test.tsx src/test/tauri.test.ts`
Expected: PASS with key issuance and policy editing pages rebuilt.

- [ ] **Step 6: Commit**

```bash
git add src/lib/types.ts src/lib/tauri.ts src/features/keys/keys-page.tsx src/features/keys/key-management-panel.tsx src/features/policies/policies-page.tsx src/features/policies/policy-editor.tsx src/test/app-shell.test.tsx src/test/tauri.test.ts
git commit -m "feat: rebuild keys and policies control surfaces"
```

## Task 3: Rebuild Logs As A Master-Detail Request Lifecycle Console

**Files:**
- Modify: `src/features/logs/logs-page.tsx`
- Modify: `src/features/logs/request-detail-capability-panel.tsx`
- Modify: `src/test/app-shell.test.tsx`

- [ ] **Step 1: Write the failing logs-page detail test**

```tsx
// src/test/app-shell.test.tsx
it("renders logs as a request history with detail and capability panels", async () => {
  render(<App />);

  await user.click(screen.getByRole("button", { name: /logs/i }));
  expect(await screen.findByRole("heading", { name: /request history/i })).toBeInTheDocument();
  expect(screen.getByText(/usage provenance/i)).toBeInTheDocument();
});
```

- [ ] **Step 2: Run the focused logs-page test to verify it fails**

Run: `bun run test -- src/test/app-shell.test.tsx`
Expected: FAIL because the current logs page is still a simple prototype list.

- [ ] **Step 3: Rebuild Logs page as a master-detail operational console**

```tsx
// src/features/logs/logs-page.tsx
<PageHeader
  title="Logs And Usage"
  description="Inspect persisted request history, attempt-level outcomes, and pricing/usage provenance."
/>
<div className="grid gap-6 xl:grid-cols-[1.1fr_0.9fr]">
  <section>{/* request history table/list */}</section>
  <section>{/* selected request detail */}</section>
</div>
```

- [ ] **Step 4: Promote capability detail into a first-class request detail panel**

```tsx
// src/features/logs/request-detail-capability-panel.tsx
return (
  <section className="rounded-2xl border bg-card p-4" aria-label="Request capability detail">
    <h4 className="mb-3 text-sm font-medium">Capability resolution</h4>
    <pre className="rounded-xl bg-muted p-3">{formatMaybeJson(detail.declared_capability_requirements)}</pre>
    <pre className="rounded-xl bg-muted p-3">{formatMaybeJson(detail.effective_capability_result)}</pre>
  </section>
);
```

- [ ] **Step 5: Run final UI-page tests**

Run: `bun run test`
Expected: PASS for all frontend test files with the rebuilt logs page included.

- [ ] **Step 6: Commit**

```bash
git add src/features/logs/logs-page.tsx src/features/logs/request-detail-capability-panel.tsx src/test/app-shell.test.tsx
git commit -m "feat: rebuild logs page as request lifecycle console"
```
