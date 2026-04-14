# V1 UI Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current prototype shell styling with a production desktop UI foundation built on Tailwind CSS v4, shadcn/ui, and reusable layout primitives suitable for the CodexLAG operations console.

**Architecture:** Keep the React/Vite app, but replace ad-hoc CSS and page wrappers with a shared shell, shared UI primitives, stable path aliases, and Tailwind-powered design tokens. This plan stops at the foundation layer: app shell, reusable primitives, and UI stack wiring. Page-by-page reconstruction is handled in the follow-on UI pages plan.

**Tech Stack:** React 19, TypeScript, Vite, Tailwind CSS v4, `@tailwindcss/vite`, shadcn/ui, Radix UI, class-variance-authority, lucide-react, Vitest, Testing Library.

---

## File Structure

### Frontend

- Modify: `package.json`
- Modify: `vite.config.ts`
- Modify: `tsconfig.json`
- Modify: `src/main.tsx`
- Replace: `src/styles.css`
- Create: `components.json`
- Create: `src/lib/utils.ts`
- Create: `src/components/app-shell.tsx`
- Create: `src/components/page-header.tsx`
- Create: `src/components/status-badge.tsx`
- Create: `src/components/ui/*`
- Modify: `src/App.tsx`

### Tests

- Modify: `src/test/app-shell.test.tsx`
- Modify: `src/test/tauri.test.ts`

## Task 1: Wire Tailwind v4 And shadcn/ui Into The Vite App

**Files:**
- Modify: `package.json`
- Modify: `vite.config.ts`
- Modify: `tsconfig.json`
- Create: `components.json`
- Test: `src/test/app-shell.test.tsx`

- [ ] **Step 1: Write the failing shell-structure test**

```tsx
// src/test/app-shell.test.tsx
it("renders the production desktop shell with persistent navigation and header chrome", async () => {
  render(<App />);

  expect(screen.getByRole("navigation", { name: /primary/i })).toBeInTheDocument();
  expect(screen.getByText("CodexLAG")).toBeInTheDocument();
  expect(screen.getByText("Gateway Overview")).toBeInTheDocument();
});
```

- [ ] **Step 2: Run the focused test to verify it fails**

Run: `bun run test -- src/test/app-shell.test.tsx`
Expected: FAIL because the current shell still uses the prototype layout and old styling setup.

- [ ] **Step 3: Add the UI foundation dependencies**

```json
// package.json
{
  "dependencies": {
    "@radix-ui/react-dialog": "^1.1.0",
    "@radix-ui/react-select": "^2.1.0",
    "@radix-ui/react-slot": "^1.1.0",
    "class-variance-authority": "^0.7.0",
    "clsx": "^2.1.1",
    "lucide-react": "^0.511.0",
    "tailwind-merge": "^2.6.0"
  },
  "devDependencies": {
    "@tailwindcss/vite": "^4.1.0",
    "tailwindcss": "^4.1.0"
  }
}
```

- [ ] **Step 4: Configure Tailwind and the `@` alias for the app**

```ts
// vite.config.ts
import path from "path";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
});
```

```json
// tsconfig.json
{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@/*": ["./src/*"]
    }
  }
}
```

```json
// components.json
{
  "$schema": "https://ui.shadcn.com/schema.json",
  "style": "radix-nova",
  "rsc": false,
  "tsx": true,
  "tailwind": {
    "config": "",
    "css": "src/styles.css",
    "baseColor": "neutral",
    "cssVariables": true,
    "prefix": ""
  },
  "aliases": {
    "components": "@/components",
    "utils": "@/lib/utils",
    "ui": "@/components/ui",
    "lib": "@/lib"
  },
  "iconLibrary": "lucide"
}
```

- [ ] **Step 5: Run the focused shell test**

Run: `bun run test -- src/test/app-shell.test.tsx`
Expected: FAIL only on missing shell components, not on broken build setup.

- [ ] **Step 6: Commit**

```bash
git add package.json vite.config.ts tsconfig.json components.json src/test/app-shell.test.tsx
git commit -m "feat: wire tailwind and shadcn into the desktop app"
```

## Task 2: Build Reusable Desktop Shell Primitives

**Files:**
- Create: `src/lib/utils.ts`
- Create: `src/components/app-shell.tsx`
- Create: `src/components/page-header.tsx`
- Create: `src/components/status-badge.tsx`
- Create: `src/components/ui/*`
- Modify: `src/main.tsx`
- Replace: `src/styles.css`
- Modify: `src/App.tsx`
- Modify: `src/test/app-shell.test.tsx`

- [ ] **Step 1: Write the failing shell-primitive behavior test**

```tsx
// src/test/app-shell.test.tsx
it("renders the shared desktop shell and highlights the active page", async () => {
  render(<App />);

  const overviewButton = screen.getByRole("button", { name: /overview/i });
  expect(overviewButton).toHaveAttribute("aria-pressed", "true");
  expect(screen.getByRole("main")).toBeInTheDocument();
});
```

- [ ] **Step 2: Run the focused shell-primitive test to verify it fails**

Run: `bun run test -- src/test/app-shell.test.tsx`
Expected: FAIL because reusable shell primitives do not yet exist.

- [ ] **Step 3: Add the utility helper used by shadcn-style components**

```ts
// src/lib/utils.ts
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
```

- [ ] **Step 4: Create the reusable shell and header primitives**

```tsx
// src/components/app-shell.tsx
export function AppShell({
  navigation,
  children,
}: {
  navigation: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div className="min-h-screen bg-[var(--app-bg)] text-[var(--app-fg)]">
      <div className="grid min-h-screen grid-cols-[280px_1fr]">
        <aside aria-label="Primary" className="border-r border-border/50 bg-card/60 p-6">
          {navigation}
        </aside>
        <main className="min-w-0 p-8">{children}</main>
      </div>
    </div>
  );
}
```

```tsx
// src/components/page-header.tsx
export function PageHeader({
  title,
  description,
}: {
  title: string;
  description: string;
}) {
  return (
    <header className="mb-6 space-y-2">
      <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
      <p className="text-sm text-muted-foreground">{description}</p>
    </header>
  );
}
```

- [ ] **Step 5: Replace the old CSS with Tailwind-backed app tokens**

```css
/* src/styles.css */
@import "tailwindcss";

:root {
  --app-bg: #f2f4ef;
  --app-fg: #17201b;
}

body {
  margin: 0;
  min-width: 320px;
  min-height: 100vh;
  background: linear-gradient(180deg, #f8f8f4 0%, #eef2e8 100%);
  color: var(--app-fg);
}
```

- [ ] **Step 6: Run focused frontend tests**

Run: `bun run test -- src/test/app-shell.test.tsx`
Expected: PASS with the shared shell and header primitives in place.

- [ ] **Step 7: Commit**

```bash
git add src/lib/utils.ts src/components/app-shell.tsx src/components/page-header.tsx src/components/status-badge.tsx src/components/ui src/main.tsx src/styles.css src/App.tsx src/test/app-shell.test.tsx
git commit -m "feat: add shared desktop shell primitives"
```

## Task 3: Make The Foundation Safe For Page Rebuild Work

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/test/tauri.test.ts`
- Modify: `src/test/app-shell.test.tsx`

- [ ] **Step 1: Write the failing navigation-stability test**

```tsx
// src/test/app-shell.test.tsx
it("keeps the six primary navigation targets available from the new shell", async () => {
  render(<App />);

  expect(screen.getByRole("button", { name: /overview/i })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /accounts/i })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /relays/i })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /platform keys/i })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /policies/i })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /logs/i })).toBeInTheDocument();
});
```

- [ ] **Step 2: Run the focused navigation-stability test to verify it fails**

Run: `bun run test -- src/test/app-shell.test.tsx`
Expected: FAIL because the new shell has not yet been wired around the existing six-page model.

- [ ] **Step 3: Rebuild `App.tsx` around the shared shell without changing page semantics**

```tsx
// src/App.tsx
return (
  <AppShell
    navigation={
      <nav aria-label="Primary" className="space-y-2">
        <div className="mb-8">
          <p className="text-xs uppercase tracking-[0.22em] text-muted-foreground">
            Local control plane
          </p>
          <h1 className="text-2xl font-semibold">CodexLAG</h1>
        </div>
        {sections.map((section) => (
          <button
            key={section.id}
            type="button"
            className={cn(
              "w-full rounded-xl px-4 py-3 text-left",
              section.id === activePage ? "bg-primary text-primary-foreground" : "bg-transparent",
            )}
            aria-pressed={section.id === activePage ? "true" : "false"}
            onClick={() => setActivePage(section.id)}
          >
            {section.label}
          </button>
        ))}
      </nav>
    }
  >
    {activeContent}
  </AppShell>
);
```

- [ ] **Step 4: Run all foundation-layer frontend tests**

Run: `bun run test -- src/test/app-shell.test.tsx src/test/tauri.test.ts`
Expected: PASS with the shell foundation stable enough for page-by-page rebuild work.

- [ ] **Step 5: Commit**

```bash
git add src/App.tsx src/test/tauri.test.ts src/test/app-shell.test.tsx
git commit -m "feat: stabilize the new desktop UI foundation"
```
