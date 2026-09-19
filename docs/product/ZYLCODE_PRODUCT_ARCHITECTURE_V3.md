# ZYLCODE PRODUCT ARCHITECTURE V3 — Project → Mission → Runtime → Proof → Forge

**Status:** PROPOSED product model, defined 2026-09-19 during the Product
Convergence execution unit. Subject to governance reconciliation. This
document is the product counterpart to `ZYLCODE_PROOF_GRAPH.md` (which governs
evidence) and the Capability Model (which governs rungs). Where the two
disagree, the proof graph and capability model win — this document describes
the product realization, not new evidence rights.

**The "before" state is preserved:** the 2026-09-19 screenshot of the previous
interface (Stream & Tool Panel, raw MCP Activity, Provider Settings, raw
`invoke` exceptions in normal UI) is the comparison baseline for structural
review. Stored with the convergence evidence.

---

## 1. Definition

> **ZylCode is an evidence-first software creation environment where people
> and AI agents design, build, run, inspect, repair, and ship software through
> verifiable missions.**

ZylCode is not an AI IDE and not a chatbot. A chat answers; a mission is
planned, executed, observed, repaired, and verified, and its outcome is backed
by inspectable evidence.

## 2. Core product loop

```
PROJECT
  ↓
MISSION
  ↓
UNDERSTAND → PLAN → BUILD → RUN → SEE → TEST → REPAIR → VERIFY → PROVE → SHIP
```

Every stage exists to feed the next; the loop closes at SHIP and feeds the
next mission. The product's center of gravity moves from "Generate/Verify an
intent" to **"Run and verify a Mission inside a Project."**

## 3. Primary information architecture

Application-level areas:

| Area | Purpose | Initial status |
|---|---|--- truthful |
| HOME | Cross-project overview: projects, missions, evidence highlights | AVAILABLE (v3 shell) |
| PROJECTS | Create/open projects, repository binding | AVAILABLE (v3 shell) |
| MISSIONS | Mission list, composer, board | LIMITED (single-agent, real Composer) |
| DESIGN | Design workspace (canvas, tokens, screens) | PROPOSED — shell only |
| RUNTIME | Runtime Lab: targets, launch, observe, verify | LIMITED (web preview real; others truthful) |
| EVIDENCE | Evidence Center + Proof Inspector | LIMITED (real evidence from missions/tools) |
| FORGE | Capability marketplace | LIMITED (shell + domain model; no payments) |

Within an open project:

`OVERVIEW · CODE · DESIGN · MISSIONS · RUNTIME · ARTIFACTS · EVIDENCE · EXTENSIONS`

**Rule:** navigation may exist for areas whose backend is not yet commissioned,
but every area renders its truthful status (§8) — navigation is not a claim.

## 4. Relocated developer surfaces

The current harness surfaces move out of the primary experience:

| Before (primary) | After |
|---|---|
| Stream & Tool Panel | Developer Tools drawer (raw stream, model override) |
| MCP Activity | Diagnostics → MCP Activity |
| Provider Settings / fallback chain | Settings → Providers |
| Generate / Verify buttons | Mission Composer → RUN MISSION; verification inside missions |

Developer Tools, Diagnostics, and the Evidence Inspector remain first-class —
engineers live there too — but they no longer dominate the normal product
experience. The normal experience answers: **What are we building? What is
ZylCode doing? What changed? Is it working? What evidence proves it? What
needs my approval?**

## 5. Project model

A Project owns: repository, requirements, product specification, design,
missions, agents, runtime targets, artifacts, evidence, extensions, delivery
state.

Project header exposes (real values only): project name, repository, branch,
runtime state, current mission, evidence status. Repository state comes from
the backend when available; absent data renders as NOT CAPTURED / UNKNOWN —
never invented.

## 6. Mission system

**Mission** is the central unit of meaningful autonomous work.

States: `DRAFT → PLANNING → READY → RUNNING → WAITING_APPROVAL → BLOCKED →
VERIFYING → FAILED | COMPLETE | CANCELLED`.

A Mission contains: goal, acceptance criteria, plan, context, capabilities,
actions, changes, runtime, tests, artifacts, evidence, cost/usage where
measurable, final outcome.

**Mapping to current capability:** the existing Agent/Intent pipeline
(`process_intent_stream` → stream deltas → artifacts → verify) is adapted into
the Mission model — not duplicated. The Mission Composer's RUN MISSION
invokes the real intent pipeline; the resulting stream, tool calls, artifacts,
and verification report populate the mission record. Phases of the current
backend that cannot yet fill a field render that field as NOT CAPTURED.

## 7. Mission Composer

Replaces "Describe what you want to build..." with a composer that has real
affordances and no fake selectors:

| Affordance | Initial truth |
|---|---|
| `+ Context` | Files the composer will attach; real file picker in desktop runtime; manual entries in browser preview |
| `@ Files` | Real: current project file tree (browser preview: workspace picker disabled with status) |
| `# History` | Real: recent missions from the session |
| `/ Commands` | LIMITED: only commands that exist |
| `$ Skills` | Real list from backend when commissioned; otherwise COMING SOON (disabled) |
| `⚡ Capabilities` | Real list from capability registry (R2+/R3 entries; others absent) |

Controls: model (free-text override, as today — real), reasoning, autonomy
(§8), environment (real: current worktree), branch/worktree (real git state
where available).

Primary action: **RUN MISSION**. Simple questions may behave conversationally;
engineering requests become missions. Autonomy modes and permission policy
follow the product's permission engine — see below.

## 8. Autonomy model + capability status language

**Autonomy modes (user-facing):**

- **OBSERVE** — no modification.
- **GUIDED** — ask before consequential changes.
- **BUILD** — may edit/build/test inside project boundaries.
- **MISSION** — may autonomously pursue declared acceptance criteria within
  permission policy.
- **SOVEREIGN** — local/private execution where compatible.

These modes do **not** override the permission engine. High-risk actions
(push, publish, deploy, delete, credential access, purchase, production
infrastructure, privileged operations) remain independently governed.

**Capability status language** (one component, used everywhere):

`AVAILABLE · LIMITED · COMING SOON · BLOCKED · NOT INSTALLED`

Developer/evidence views may additionally expose proof rungs (R0–R5). Ordinary
users are never required to know governance terminology; rungs appear only in
evidence/inspector contexts.

**Autonomy-mode availability** maps to the same status system: in the current
implementation, OBSERVE and GUIDED are AVAILABLE (the backend enforces
approval states), BUILD/MISSION are LIMITED (pipeline exists end-to-end but
capability commissioning is partial), SOVEREIGN is COMING SOON. The composer
disables what is not available, with the status as its explanation.

## 9. Workspace layout

Professional dockable workspace:

```
┌────────────────────────────────────────────────────────────┐
│ Project / Branch        Mission Status       Evidence      │
├───────────┬─────────────────────────────┬──────────────────┤
│ OVERVIEW  │                             │ AGENT            │
│ CODE      │       WORK SURFACE          │ PLAN · ACTIONS   │
│ DESIGN    │                             ├──────────────────┤
│ MISSIONS  │                             │ PROOF            │
│ RUNTIME   │                             │                  │
│ ARTIFACTS │                             │                  │
│ EVIDENCE  │                             │                  │
│ EXTENSIONS│                             │                  │
├───────────┴─────────────────────────────┴──────────────────┤
│ Runtime ●  Branch main  Tests ✓  Evidence ●  Model ...    │
└────────────────────────────────────────────────────────────┘
```

LEFT: project navigation. CENTER: primary work surface. RIGHT: context dock
(AGENT, PREVIEW, BROWSER, TERMINAL, INSPECTOR, PROOF modules). BOTTOM:
runtime/status/evidence strip.

Useful combinations: Code + Agent, Design + Preview, Mission + Proof, Browser
+ Agent, Runtime + Logs. Not a mechanical VS Code recreation — optimized for
AI-assisted software creation. (Full docking/drag-resize is future; v3
implements fixed-region dock with selectable right-dock modules.)

## 10. Runtime Lab

Product shell for runtime targets:

| Target | Initial status |
|---|---|
| WEB | AVAILABLE — real: Vite preview / dev server of the current project |
| DESKTOP | LIMITED — real: Tauri shell exists; in-browser preview shows desktop-runtime-required state |
| TERMINAL | LIMITED — real: CLI exists (`zylcode` binary; repo-context etc.) |
| ANDROID / IOS / CONTAINER / REMOTE | NOT INSTALLED / COMING SOON — truthful, no fake controls |
| LOGS | AVAILABLE where real log streams exist; otherwise NOT CAPTURED |

Long-term abstraction: Mission → Runtime Target → Launch → Observe → Interact
→ Verify → Evidence. The current Preview tab evolves into Runtime; it is not
discarded.

## 11. Environment awareness (fixing the browser-preview failure UX)

Typed runtime environment detection:

```ts
type RuntimeEnvironment = "TAURI_DESKTOP" | "BROWSER_PREVIEW" | "TEST" | "UNKNOWN";
```

Detection order: `TAURI_DESKTOP` when the `__TAURI_INTERNALS__` global exists;
`TEST` under test runners (`vitest`/`jest` globals); `BROWSER_PREVIEW`
otherwise; `UNKNOWN` only if detection throws.

Rules:

1. **Never invoke Tauri IPC outside `TAURI_DESKTOP`** (no repeated exceptions).
2. Backend-dependent UI renders a controlled state — "Desktop runtime
   required" with an explanation — not JavaScript exception text.
3. Raw exception details go to **Diagnostics**, available to users who open it.
4. Event subscriptions (`intent:chunk` etc.) no-op safely outside the desktop
   runtime.
5. Add tests for all four environments and the fallback rendering.

## 12. Evidence Center

Evidence is a first-class product surface, grouped by Mission: Build, Tests,
Runtime, Screenshots, Tool actions, Approvals, Git changes, Artifacts,
Verification.

Each item exposes, where available: timestamp, actor, action, input, output,
state change, commit SHA, environment, reproduction command, proof rung.
**Missing fields render NOT CAPTURED** — never silently absent, never
invented.

The v3 Evidence Center reads real evidence available today: intent
verification reports (checks, pass/fail, duration, rung), tool-call records
from the stream, artifact records, and (in desktop runtime) audit logs. Where
a mission has no evidence of a category, that category shows NOT CAPTURED.

## 13. Proof Inspector

Initial Proof Inspector (labeled as a visualization of currently available
evidence — **not** Proof Engine v2):

```
GOAL → CLAIMS → ACTIONS → STATE CHANGES → TESTS → ARTIFACTS → EVIDENCE → OUTCOME
```

Each node renders from real mission data; unavailable nodes render NOT
CAPTURED with the reason (e.g. "proof engine not commissioned").

## 14. Forge — capability marketplace

Forge is a marketplace for **capability packs**, not merely a plugin browser.

A Capability Pack may contain: skills, commands, agents, MCP integrations,
hooks, templates, knowledge, runtime adapters, UI panels, tests, evidence
rules.

**Domain model first, truthful UI states, no payments:** the manifest schema
(§15), taxonomy (§16), trust labels (§18), and discovery concepts (§17) are
implemented as typed domain models + shell UI seeded with taxonomy metadata
only. Commercial model enums exist (`FREE`, `FREEMIUM`, `TRIAL`, `ONE_TIME`,
`SUBSCRIPTION`, `USAGE_BASED`, `PER_SEAT`, `TEAM`, `ENTERPRISE`) but no
checkout, no licensing backend, no fake plugins.

## 15. Capability Pack manifest schema (v1)

```jsonc
{
  "id": "com.zylvex.pack.example",        // reverse-DNS, unique
  "name": "Example Pack",
  "publisher": "zylvex",                   // publisher id
  "version": "1.0.0",                      // semver
  "description": "…",
  "category": "testing",                   // §16 taxonomy slug
  "supported_platforms": ["windows", "macos", "linux", "web"],
  "skills": ["..."], "commands": ["..."], "agents": ["..."],
  "mcp_servers": ["..."], "hooks": ["..."], "templates": ["..."],
  "runtime_adapters": ["..."],
  "permissions": {
    "filesystem": "read-write",  // none|read|write|read-write
    "shell": "execute",
    "network": ["github.com"],
    "git": "read-commit",
    "browser": "control",
    "secrets": ["github-credential"],
    "external_services": ["aws"]
  },
  "commercial": {
    "model": "free",           // §14 enum
    "trial": null, "price": null, "currency": null,
    "billing_period": null, "seats": null
  },
  "evidence": {
    "test_status": "passed",   // none|partial|passed
    "verification_rung": "R2", // R0..R5 or "unverified"
    "verified_version": null,
    "verified_commit": null,
    "platforms_verified": []
  }
}
```

Parser is strict: unknown category → invalid; missing permissions block →
invalid; semver validated; no secret values ever embedded (patterns rejected).

## 16. Initial marketplace taxonomy

Metadata only — no fake working plugins:

`web-development, mobile, backend, database, devops, cloud, security,
testing, ui-ux, design-to-code, ai-ml, local-ai, mcp, automation,
documentation, release-engineering, game-development, 3d, engineering,
data, team-enterprise`

Example future packs (concepts until implemented): Android Production
Engineer, iOS Production Engineer, React/Next Engineer, Flutter Engineer,
Rust Systems Engineer, Python Backend Engineer, Database Architect, DevOps
Engineer, Security Auditor, Browser QA Engineer, UI/UX Engineer,
Design-to-Code, Visual Regression Engineer, MCP Builder, Plugin Builder,
Legacy Modernizer, Codebase Archaeologist, Release Readiness Auditor, Unity
Engineer, Unreal Engineer, Blender Automation, AI Agent Builder, Local AI
Engineer; Zylvex engineering: CAD Automation, Fabrication Job Pack, BOM/Cut
List, Drawing QA, FreeCAD Automation, Engineering Documentation,
Simulation/FEA, Spatial Engineering.

## 17. Capability discovery

Mission analysis may map task requirements to pack categories and recommend
missing capabilities (e.g. "Flutter Engineer — NOT INSTALLED"). Discovery is
advisory only. **Never automatic purchase or install;** installation and
commercial actions require explicit user action.

## 18. Trust + security

Every pack displays its permissions before installation. Marketplace labels
map to the proof philosophy:

`UNVERIFIED → TESTED → VERIFIED → REPRODUCIBLE → CERTIFIED`

Labels are earned by evidence (test status, rung, verified commit, platforms
verified) and are never awarded without it. Unverified packs show prominent
permission warnings.

## 19. Capability Lab (Try Demo)

"Try Demo" is a core Forge concept: a commercial capability eventually ships a
demo fixture; selecting TRY DEMO creates an isolated demo mission showing
goal, actions, files changed, runtime result, tests, evidence, and capability
limitations, then offers UNLOCK FULL CAPABILITY. **During this phase the demo
engine does not exist;** the UI labels it `DEMO ENGINE — PROPOSED`. No
simulated successful execution.

## 20. Project Knowledge Graph (UX/domain model)

Inspectable project knowledge: requirements, decisions, files, symbols,
components, tests, bugs, designs, dependencies, missions, artifacts, evidence,
releases. Every inferred fact is eventually traceable to evidence. **No
fabricated graph data.** Repository Intelligence (scanning, symbols, package
graph, entry points, retrieval — proven deterministic in P1.2) is the first
real input into this graph; the graph surface itself is PROPOSED in this unit.

## 21. Model democracy UX

`AUTO — ZYLCODE ROUTER` is exposed only as far as real routing exists: the
current model override (free text, honored by the backend) and the provider
fallback chain (real, moved to Settings → Providers). "WHY THIS MODEL?"
explanations (success rate, latency, cost, privacy, role suitability,
benchmarks) are PROPOSED — the current router is not measurement-based and
this UI must not pretend it is.

## 22. Agent Board

Single-agent truth today: the board shows the active agent and its state.
Multi-agent roles (Lead Engineer, Repository Analyst, Frontend Engineer,
Backend Engineer, Security Engineer, Test Engineer, Release Auditor) and
states (RUNNING, WAITING, BLOCKED, COMPLETE, FAILED) are defined domain
models, PROPOSED for the multi-agent roadmap phase. **No fake parallel
agents.**

## 23. Design Studio

Defined future workspace (canvas, components, screens, routes, tokens,
assets, states, interactions; DESIGN ↔ CODE ↔ RUNTIME ↔ VISUAL DIFF ↔ AGENT
REPAIR ↔ PROOF). This unit ships only the architectural shell/status surface.
**No fake Figma clone.** Design navigation exists ≠ Vision Studio implemented.

## 24. Visual design direction

Professional, dense-but-readable, engineering-oriented, futuristic without
gaming aesthetics, high information clarity, strong hierarchy, minimal
decorative chrome. The default dark theme should feel like a premium
engineering workstation. Implementation rules: semantic design tokens only
(no hard-coded colors in components), preserve and extend the existing 8-theme
architecture, excellent typography (consistent scale), focus states and
keyboard navigation everywhere.

## 25. Status-boundary rules (governance reconciliation)

- Forge shell implemented ≠ Marketplace implemented.
- Design navigation exists ≠ Vision Studio implemented.
- Runtime tabs exist ≠ Android/iOS execution implemented.
- Mission UX exists ≠ full mission engine implemented (the intent pipeline is
  real; planning/verification depth evolves with the Mission Engine roadmap).
- Roadmap phases are marked complete only by governance, on evidence.

## 26. Implementation boundary of this unit

Implemented in this unit: Product Architecture v3 (this doc); typed
environment detection + safe IPC bridge; navigation shell (Home, Projects,
Missions, Design, Runtime, Evidence, Forge) + project workspace shell; Mission
domain + Composer + autonomy modes; dev-surface relocation (Developer Tools /
Diagnostics / Settings); Evidence Center + Proof Inspector on real evidence;
Runtime Lab shell; Forge domain (manifest schema + strict parser + taxonomy +
trust labels) + marketplace shell; capability status component; design token
extensions; frontend tests.

Not implemented (explicitly): payments/checkout, fake marketplace packs,
fake Computer Use, fake Android/iOS control, fake cloud deployments, full
Design Studio, multi-agent runtime, Proof Engine v2, licensing backend,
unimplemented MCP servers.

## 27. Success criteria for this unit

1. Raw infrastructure failures no longer appear in normal UI; browser preview
   shows controlled states.
2. The center of gravity moves from Generate/Verify-an-intent to Run-and-verify
   a Mission inside a Project.
3. Every capability surface renders truthful status; no control pretends.
4. Real evidence (verification reports, tool calls, artifacts) is inspectable
   in the Evidence Center.
5. The before/after structural comparison shows raw infrastructure gone from
   the primary experience.
6. All tests green; battery unchanged in backend behavior.
