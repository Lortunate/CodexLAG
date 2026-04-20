# CodexLAG Single-Task Shell Design

## 1. Goal

Reduce the desktop UI from a verbose multi-layer workbench into a restrained single-task operator console.

The shell should stop repeating context across sidebar, topbar, and page headers. Each page should open directly into its primary task, with non-essential framing removed, collapsed, or demoted.

## 2. Confirmed Scope

### 2.1 Must Deliver

- preserve the existing six primary surfaces:
  - `Overview`
  - `Official Accounts`
  - `Relays`
  - `Platform Keys`
  - `Policies`
  - `Logs & Usage`
- keep the current dark operator-console direction from `.impeccable.md`
- simplify the global shell so it only provides:
  - product identity
  - compact primary navigation
  - current page title
- remove repeated shell copy such as:
  - long workspace descriptions
  - repeated “active workspace” framing
  - `Surface / Session / Scope` summary blocks
  - persistent per-route detail text in navigation
- shift page entry from “explain the workspace” to “start the task”
- compress or remove first-screen summary strips when they only restate information already visible elsewhere
- reduce visible panel count and flatten hierarchy where adjacent boxes currently fragment one task into many containers
- keep degraded states, operational risk, and trust-significant system status explicit

### 2.2 Explicitly Excludes

- no route or information architecture rewrite beyond shell simplification
- no change to backend contracts or Tauri command behavior
- no consumer-marketing redesign, glassmorphism, gradients, or decorative hero treatments
- no hiding of critical runtime health, auth state, or routing consequences

## 3. Design Context

CodexLAG is used by programmers operating a local desktop control plane for accounts, relays, keys, policies, and diagnostics. The interface should feel concise, precise, and programmer-friendly. It should behave like a calm dark console rather than a promotional dashboard.

The user selected this direction explicitly:

- primary problem: the current interface feels too verbose, redundant, and not minimal
- highest priority: the current page’s primary task
- preferred aesthetic: `extreme minimal operator console`
- acceptable tradeoff: secondary information can be folded or demoted by default
- chosen shell direction: `single-task shell`

## 4. Problem Summary

The current shell over-explains the same context in multiple places:

- the sidebar shows product framing, active workspace framing, and navigation detail text
- the topbar repeats product/workspace framing and adds another summary block
- many pages then begin with another title/description layer plus a summary strip before useful work starts

This creates avoidable noise:

- the shell competes with page content
- the user must scan multiple layers before reaching the real task
- the interface looks heavier than the actual amount of actionable information

## 5. Design Direction

### 5.1 Shell Responsibility

The shell should do exactly three things:

- identify the product
- let the operator switch pages
- state the current page

It should not describe the entire workspace every time the route changes.

### 5.2 Sidebar

The left rail should become narrower and quieter.

It should contain:

- a compact CodexLAG brand block
- one vertical list of the six routes

It should not contain:

- long product captions
- “active workspace” explainer text
- route detail subtitles displayed at all times

Navigation items should default to a single-line label. The active state should rely on clean contrast, spacing, and a restrained position marker rather than large glowing fills or oversized cards.

### 5.3 Topbar

The topbar should collapse into a thin title row.

It should contain:

- the current page title
- optionally one short operational state token if the page truly needs it

It should not contain:

- eyebrow copy
- descriptive marketing-style sentences
- `dl` summary framing blocks

### 5.4 Page Entry

Each page should begin with a single clear task start, not a stack of framing sections.

The first visible block on a page should answer:

- what is the operator here to do right now?

This means page entry should prefer:

- primary status with consequences
- primary list or table
- primary editor or action surface

It should not lead with explanatory text that restates the route name or shell context.

### 5.5 Content Compression Rules

The following rules apply across the frontend:

- any piece of information should appear once at the highest-value layer only
- if the page title already explains the surface, a summary strip must not restate it
- if a module title already explains the section, the first paragraph should not paraphrase the same meaning
- default to showing decision-making content before explanatory content
- reduce boxed surfaces where one task is fragmented into multiple neighboring panels
- keep empty-state and helper text, but only where absence or next-step ambiguity would otherwise block the operator

## 6. Page-Level Implications

### 6.1 Overview

`Overview` should stop behaving like a landing page for the whole app. It should behave like a compact runtime snapshot.

The first screen should prioritize:

- current runtime health
- key posture
- the most consequential status board or capability surface

It should not stack title copy, summary cards, and additional framing before the meaningful operational surface.

### 6.2 Accounts / Relays / Keys / Policies / Logs

These pages should move toward:

- one concise page intro at most
- immediate access to the primary editable surface
- reduced explanatory duplication around forms and tables

Where a summary strip only mirrors what the main editor/list already says, it should be removed rather than restyled.

## 7. Implementation Shape

The implementation should likely concentrate in:

- `src/components/app-shell.tsx`
- `src/App.tsx`
- `src/styles.css`
- selected page entry components that currently stack title + summary + panel framing

This is intentionally a shell-first cleanup. It should establish reusable compression patterns before touching deeper page internals.

## 8. Acceptance Criteria

This design is successful when all of the following are true:

- the sidebar presents only compact product identity and single-line route labels
- the topbar is reduced to a minimal current-page title row
- shell-level repeated workspace framing is removed
- the first screen of each major page emphasizes the task surface rather than repeated explanation
- summary strips are removed or reduced wherever they only restate visible page context
- the UI still feels explicitly operational, trustworthy, and dark-console oriented
- no critical degraded state, auth risk, routing consequence, or diagnostic warning becomes hidden by the simplification
