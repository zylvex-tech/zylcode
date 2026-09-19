# ZYLCODE UX FORENSIC — 2026-09
## UX-01A — Current Product Experience Forensic

**Executed by:** Buffy (Codebuff) — UX-01 unit, UX-01A phase.
**Scope:** inspection only. **No application source was modified in this phase.**
**Governance basis:** this document treats the Capability Model's GREEN rule
(`GREEN requires R3 or above; a module is not a capability`) and the P1.3 report's
own implemented/proposed boundary as ground truth to test against — not as claims
to inherit.

---

## 1. State at inspection

| Item | Value |
|---|---|
| Local HEAD | `da91fa116993d82ff7943fe66a7379fa6c581037` |
| `origin/main` | `da91fa116993d82ff7943fe66a7379fa6c581037` (verified `git ls-remote`) |
| Working tree | clean |
| Branch | `main` |
| Preview | live Vite dev server, `http://localhost:1421/` (pid 39428), used for live DOM/computed-style inspection |

Inspection methods: full read of `src/App.tsx` (1,010 lines), `src/lib/{theme,events,runtime,mission,forge}.ts`, `src/components/*`, `src/styles/themes.css` (749 lines), `tailwind.config.js`, `src/index.css`; grep sweeps for navigation, status language, keyboard handlers, breakpoints, persistence; **live computed-style probes** in the running app; cross-check against `docs/capability-registry.json` (v1.4.1-audited) and `docs/governance/ZYLCODE_CAPABILITY_MODEL.md`.

---

## 2. Current UX architecture (as actually implemented)

```
App (ThemeProvider → AppContent)
├─ StatusBar (theme select — native <select>, 8 themes via data-theme attr)
├─ Two-level text navigation
│  ├─ Global nav: Home / Projects / Missions / Design / Runtime / Evidence /
│  │  Forge / Developer Tools ▸          ← 8 text buttons, 5 carry status pills
│  └─ Project nav (when in project): Overview / Code / Design / Missions /
│     Runtime / Artifacts / Evidence / Extensions   ← 8 more, all but one carry pills
├─ Main surface per (area, projectTab) pair
│  ├─ Home: project card, recent missions, capability spotlight (5 statuses)
│  ├─ Project OVERVIEW: project header, environment block, capabilities list
│  ├─ Project MISSIONS: Mission Composer + mission list + Verify
│  ├─ Project EVIDENCE: Evidence Center (empty state) / Proof Inspector
│  ├─ Project RUNTIME: RuntimeLab (8 target cards)
│  ├─ Project others (CODE/DESIGN/ARTIFACTS/EXTENSIONS): status-note panels
│  └─ Forge: browse grid (32 concept cards) + manifest validator + trust model
├─ Right dock (AGENT / PREVIEW / BROWSER / TERMINAL / INSPECTOR / PROOF selector)
├─ Developer Tools drawer: McpInspector + ProviderSettings + Diagnostics
└─ Bottom status strip: circuit, bridges, theme select
```

Domain foundations present and tested: `lib/runtime.ts` (environment bridge,
`safeInvoke`/`unwrapInvoke`), `lib/events.ts` (env-aware events + backlog),
`lib/mission.ts` (states, autonomy, evidence categories, NOT CAPTURED),
`lib/forge.ts` (manifest v1 parser, taxonomy, trust labels).

---

## 3. Finding F1 — STOP MAKING THE ARCHITECTURE THE NAVIGATION (master prompt §1)

**Confirmed, and worse than described.** The two-level navigation is not merely
duplicated — the two levels are *inconsistent with each other*:

- Global nav `Missions` → sets `area="MISSIONS"`; project tab `Missions` →
  `projectTab="MISSIONS"`; `startMission` forces **both**. The same concept is
  reachable through two independent state variables that must be kept in sync by hand.
- Global `Design` (a top-level destination) and project `Design` both render the
  same "Design Studio — COMING SOON" panel. Two dead destinations for one absent capability.
- Global `Runtime` and project `Runtime` both render `RuntimeLab`. Same component, two doors.
- Global `Evidence` and project `Evidence` both render `EvidenceCenter`.
- `inProject` gating (`area !== "HOME" && area !== "FORGE"`) means global Missions/
  Design/Runtime/Evidence silently turn into the project tabs — the global buttons
  are project tabs wearing global costumes.

**Verdict:** category (E) redundant. The seven-domain IA of P1.3 was implemented
literally as seven destinations; the owner's correction (mission as operating
state, runtime as surfaces, evidence as ambient) is not yet reflected.

---

## 4. Finding F2 — Status language floods the UI (§8), with a hidden rendering defect

**Count:** 42 status-language occurrences in `App.tsx` alone (status pills,
COMING SOON chips, NOT INSTALLED labels), 8 more in `Forge.tsx`. Live snapshot of
one workspace view: the project tab row alone displays **LIMITED ×4, COMING SOON ×2**;
RuntimeLab shows 8 pill cards; Home shows 5 pills; Forge shows 32 × NOT INSTALLED.

**The ambiguity the owner predicted is real**, e.g. RuntimeLab's `DESKTOP — LIMITED /
Requires the Tauri desktop shell` conflates three different facts: (a) you are in a
browser preview, (b) the Tauri shell is required, (c) desktop packaging exists as a
target but is not commissioned as a runtime. One pill, three possible meanings.

**Hidden defect discovered (computed-style proof):** the semantic status colors do
not render at all. `tailwind.config.js` defines only a `zyl` palette (bg/surface/
border/accent/muted); the classes components actually use — `bg-surface`,
`text-text-muted`, `bg-success`, `border-success/30`, `text-warning`, `bg-primary`,
`text-error` — are **not defined in the Tailwind config**, so Tailwind generates no
CSS for them. Live probe of a `LIMITED` badge in the Solarized theme:

```
badge border-color: rgb(229, 231, 235)   ← Tailwind default gray-200 (the bare
                                            `border` class default), NOT the theme
theme --color-success: #859900            ← the intended warning color, unused
theme --color-surface:  #073642           ← unused by bg-surface
```

Consequences: (1) every status badge is colorless — status is communicated by text
only, violating the product's own "no status communicated by colour alone" is
moot because there IS no colour; (2) the 749-line themes.css variable system is
consumed only by `body` background, scrollbar, focus ring and selection
(`index.css`) — **the entire semantic token layer is dead code**; (3) the "large
teal slab" the owner observed is the body background showing through panels whose
intended `bg-surface` never resolved.

**Verdict:** the visual flatness is not just aesthetic conservatism — the design
system is electrically disconnected. Category (H) visually inadequate, root-caused.

---

## 5. Finding F3 — Three competing theme systems

1. `src/components/ui/index.tsx` — **the active one**: 8 themes, `ThemeProvider`
   context, sets `data-theme` on `<html>`, persisted to `localStorage("zylcode-theme")`,
   selector = native `<select>` in `StatusBar`. No visual previews.
2. `src/styles/themes.css` — 749 lines defining the 8 themes as `--color-*`
   variables **plus a full spacing/radius/typography/shadow/z-index scale** —
   mostly unconsumed (see F2).
3. `src/lib/theme.ts` — a **third** theme module (3 themes: dark/oled/cyberpunk,
   Tailwind class strings). Zero imports anywhere. Dead code.

`tailwind.config.js` additionally hard-codes a `zyl` palette that only ~25 legacy
references use. **Verdict:** one system must win (semantic tokens in themes.css +
Tailwind mapping + previews); two must be deleted. Category (E)/(H).

---

## 6. Finding F4 — The center is empty because no work surface exists

`grep` results: **no file explorer**, **no editor** (Monaco is a declared
dependency and mounted in exactly one place — `ArtifactViewer.tsx` — which is
only reachable when a mission stream emits an artifact), **no terminal**
(the dock's TERMINAL button renders a "Desktop runtime required" panel, not a
terminal), **no diff view** (`diffMode` state exists in `useArtifactStream` but
no diff surface renders it), **no source control view**, **no command palette**
(the only keydown listener in the app is ArtifactViewer's Ctrl+S), **no resizable
panels** (zero `onMouseDown` splitter code), **no breadcrumbs/tabs/symbols**.

The 11 `lg:/md:/sm:` breakpoints in App.tsx are grid collapses, not workspace
adaptivity. **Verdict:** category (G) missing — the IDE half of the product does
not exist yet; the workspace currently *describes* work instead of hosting it.

---

## 7. Finding F5 — Mission experience

What exists and is real: Mission Composer (goal, autonomy select with honest
mode availability, model override), mission records folded from **real** stream
state (`useStreamSubscription` deltas + `mcpCalls` + `done.artifacts`), Verify via
`verify_logic`, per-mission evidence views, NOT CAPTURED discipline.

What is not: no mission stages/plan/timeline (the UNDERSTAND→…→PACKAGE model
exists nowhere in UI), no acceptance criteria as first-class objects (field exists,
always `[]`), no persistence (zero `localStorage` in App/mission.ts — missions die
on reload), no WAITING_APPROVAL/BLOCKED reason surfacing, mission "RUNNING" state
is set before any backend confirmation and never derives from a plan.

**Verdict:** honest but shallow — a composer + transcript, not the operating model.

---

## 8. Finding F6 — Claims audit (master prompt §37)

| P1.3 claim | Forensic verdict |
|---|---|
| Repository Intelligence — AVAILABLE (Home spotlight) | **MISLEADING as displayed.** Registry marks the 2A intelligence entries DISPUTED; the capability is reachable via `zylcode repo-context` (committed transcript, P1.2) but the product word AVAILABLE maps to the Capability Model's GREEN-rule territory (implies usable capability). Displayed without qualification. Requires rewording (e.g. "CLI-commissioned, audit pending") or demotion. |
| WEB — AVAILABLE (RuntimeLab) | **MISLEADING.** What was proven: the ZylCode frontend is served by Vite. What the pill implies: projects can be built/run as web targets. That capability does not exist. |
| RUN MISSION wired to real `process_intent_stream` | **PARTIALLY VERIFIED.** Code-verified: `safeInvoke(ENVIRONMENT, "process_intent_stream", …)` at App.tsx:563 through the safe bridge — the wiring is real. NOT evidenced: no end-to-end execution capture exists (browser preview correctly refuses; no desktop transcript committed). The claim's evidentiary status is R2-shaped (test exists in backend), not product-proven. |
| Evidence Center renders actual mission evidence | **TRUE but EPHEMERAL.** Renders only real in-session data (verified). No persistence, no backend evidence store; all evidence evaporates on reload. "Evidence-first" products do not keep evidence in a React state array. |
| Runtime real surfaces | **TRUTHFUL but NOISY.** Each target carries an accurate reason; the presentation (8 pill cards) is the problem, not the facts. |
| Forge implemented | **ACCURATE AS SCOPED** (domain+parser+shell; marketplace explicitly not implemented) — but 32 identical concept cards is taxonomy, not a marketplace experience. |
| 418/0 cargo + 48/48 vitest | **VERIFIED this session** by direct execution (fmt/check/clippy `-D warnings` green, guard 11/0, frontend build green). |

---

## 9. Classification (master prompt §3 A–H)

**A. Useful implementation to preserve**
- `lib/runtime.ts` environment bridge + `safeInvoke`/`unwrapInvoke` semantics (the raw-invoke error class is genuinely gone; console verified clean)
- `lib/events.ts` env-aware subscriptions + backlog
- `lib/mission.ts` domain vocabulary (states, autonomy, evidence categories, NOT CAPTURED)
- `lib/forge.ts` manifest v1 parser, secret scan, trust-label clamping (21 parser/trust tests)
- `Forge.tsx` ManifestValidator + TrustModel panels; `DiagnosticsPanel`; stream→mission fold logic; `TokenMetricsWidget` controlled state

**B. Architectural plumbing to hide from normal users**
- Developer Tools drawer (correctly placed already, keep), environment block on Overview (move to Developer Tools), circuit/bridges status strip items

**C. Developer/debug UI** — McpInspector, ProviderSettings (correctly quarantined; keep quarantined)

**D. Temporary placeholder** — CODE/DESIGN/ARTIFACTS/EXTENSIONS project tabs; RuntimeLab as a page; right-dock module buttons for BROWSER/TERMINAL/INSPECTOR (all render "required/not configured" panels)

**E. Redundant** — two-level navigation (F1); `lib/theme.ts` dead module (F3); `zyl` Tailwind palette vs semantic tokens (F3)

**F. Misleading** — Repository Intelligence AVAILABLE; WEB AVAILABLE (F6); sovereignty-adjacent imagery in autonomy descriptions is fine but BUILD/MISSION LIMITED pills on a select are governance leak (F2 pattern)

**G. Missing** — file explorer, editor, terminal, diff/changes, source control, command palette, resizable docks, mission timeline/stages, acceptance criteria UI, persistence (missions + evidence + layout), theme previews, project switcher, onboarding

**H. Visually inadequate** — dead token layer (F2 root cause), all-caps headings everywhere, 1px-border rectangles as the only visual primitive, no elevation/shadow usage (the scale exists in themes.css!), native `<select>` theme switcher, no iconography, no density hierarchy

---

## 10. Proposed target information architecture (for UX-01 design docs)

The owner's four-zone model, made concrete against what exists:

- **Zone A — Activity rail** (icon + tooltip + shortcut): Explorer, Search, Source
  Control, Mission, Run, Evidence, Forge, Extensions, Settings. Replaces both
  navigation levels. Global Home becomes a mode (no project open), not a nav item.
- **Zone B — Context sidebar** swaps content per activity. Explorer is the missing
  keystone: the repo is real, the scanner/index exists — the sidebar can consume
  `build_repo_query`/scanner output.
- **Zone C — Surface host**: typed surfaces (CODE via Monaco — dependency already
  present; DIFF; TERMINAL; PREVIEW; EVIDENCE; MISSION PLAN). Unsupported types
  (DESIGN canvas, DEVICE) exist as typed unavailable capabilities, hidden by default.
- **Zone D — Agent dock**: promote from today's panel-chooser to the persistent
  agent surface with structured progress (§20 of the master prompt).
- **Bottom panel**: TERMINAL/PROBLEMS/TESTS/OUTPUT/EVIDENCE tabs (architecture first;
  only tabs with real data sources render content; others do not appear).
- **Status translation rule** (§8): internal vocabulary (AVAILABLE/LIMITED/…) stays
  in Developer Tools and the registry; user-facing surfaces translate to
  action-oriented copy ("Device execution isn't configured → [Configure]").
- **Persistence** becomes a requirement: missions, layout, preferences survive reload.
- **Command palette** (Ctrl/Cmd+K): files + commands + settings + themes; only real
  actions listed.

## 11. Proposed implementation sequence (UX-01B onward, pending review)

1. **UX-01B design-system foundation** — single semantic token system: extend
   `themes.css` variables (already 8-theme capable) → map into `tailwind.config.js`
   via `rgb(var(…)/<alpha-value>)`, delete `lib/theme.ts` + `zyl` palette, add
   status/action/neutral component primitives, theme previews.
2. **UX-01C workspace shell** — activity rail + context sidebar + surface host +
   bottom panel + status bar; fold the two navs into one; mount existing real
   surfaces (Mission Composer, Evidence Center, Forge, Diagnostics) into the shell.
3. **UX-01D agent dock + mission presentation** — persistent dock, mission as
   operating state (stages render only real stream/verification data), mission
   persistence via localStorage/Tauri store.
4. **UX-01E command system** — palette + shortcuts, real actions only.
5. **UX-01F Forge UX** — taxonomy-first landing (search + categories + type filters),
   concept packs collapsed into an explicit "Taxonomy (not yet published)" section
   instead of 32 card fronts.
6. **UX-01G tests + visual corrections** — RTL suites for shell/nav/status
   translation; 4-viewport screenshot matrix.
7. **UX-01H governance reconciliation.**

**Files likely to change:** `App.tsx` (rewrite into shell + routers), new
`components/shell/*` (Rail, Sidebar, SurfaceHost, BottomPanel), `components/ui`
(primitives + theme), `tailwind.config.js`, `styles/themes.css`, `lib/theme.ts`
(delete), `lib/mission.ts` (stages/persistence), new `lib/palette.ts` (commands),
`Forge.tsx` (landing rework), tests.

---

## 12. Explicit statement of what UX-01A did NOT do

No source file was modified. No capability status was changed. No claim above was
accepted without code-level or computed-style evidence. The live preview server
(pid 39428, port 1421) was used read-only and remains running.
