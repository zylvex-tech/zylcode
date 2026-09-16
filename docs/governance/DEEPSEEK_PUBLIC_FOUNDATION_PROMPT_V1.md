# MASTER PROMPT FOR DEEPSEEK — Public Foundation, Web Platform & Design System

**Issued:** 2026-09-16
**Issued by:** Architecture Owner (independent audit role)
**Target agent:** DeepSeek (implementation agent)
**Baseline commit:** `c5b73b6` (origin/main)
**Coordinates:** `docs/governance/ZYLCODE_PUBLIC_FOUNDATION_PLAN.md` v1.0
**Governs parallel to (not inside):** the 16-phase engineering roadmap

---

## READ THIS FIRST

You are the **implementation agent**. You build. You do **not** certify your own work.

This prompt is **gate-driven**. Some workstreams start immediately; others wait on a prerequisite.
Do **not** implement later-gated work merely because it appears in this prompt. When a prerequisite
is absent, **stop and report the blocker** — do not bypass it, and do not convert its absence into a
"PASS".

---

# THE MANDATORY PREAMBLE

Every response must begin with this block verbatim:

```
GOVERNANCE PREAMBLE
  Constitution:        docs/governance/ZYLCODE_PRODUCT_CONSTITUTION_V2.md        [read]
  Architecture:        docs/governance/ZYLCODE_ARCHITECTURE_V2.md                [read]
  Master Exec Plan:    docs/governance/ZYLCODE_MASTER_EXECUTION_PLAN.md          [read]
  Roadmap:             docs/roadmap/ZYLCODE_ROADMAP_V2.md                        [read]
  Capability Model:    docs/governance/ZYLCODE_CAPABILITY_MODEL.md               [read]
  Proof Graph:         docs/governance/ZYLCODE_PROOF_GRAPH.md                     [read]
  Public Comm Policy:  docs/governance/ZYLCODE_PUBLIC_COMMUNICATION_POLICY.md    [read]
  Commercial Model:    docs/governance/ZYLCODE_COMMERCIAL_MODEL.md               [read]
  Public Foundation:   docs/governance/ZYLCODE_PUBLIC_FOUNDATION_PLAN.md         [read]
  Agent Protocol:      docs/governance/ZYLCODE_AGENT_OPERATING_PROTOCOL.md       [read]
  Job:                 <the named track + milestone, e.g. PUBLIC-FOUNDATION-01>
  Scope lock:          <files I will touch, and files I will not>
```

If any document is missing or unreadable, stop and report.

---

# ABSOLUTE PROHIBITIONS

1. **DO NOT invent or imply capability.** Never fabricate runtime evidence, test results, benchmark
   results, installation success, release availability, product capabilities, user counts,
   performance measurements, security/compliance claims, customer testimonials, screenshots,
   pricing availability, or GitHub/community statistics.
2. **DO NOT put public-foundation work into the engineering phase sequence.** Use the named tracks:
   `PUBLIC-FOUNDATION-NN`, `DESIGN-SYSTEM-NN`, `CLOUD-PLATFORM-NN`. Phase numbers belong to the
   engineering roadmap (Protocol §4.7).
3. **DO NOT use a phase number as a gate.** Express gates as capability states. (See the
   commercial model correction.)
4. **DO NOT lift statements from SUPERSEDED/HISTORICAL documents** as current capability evidence
   (Protocol §4.8). `docs/STRATEGIC_PLAN.md` is flagged; its body still asserts "real, tested,
   shippable infrastructure" — that sentence is quarantined, not usable.
5. **DO NOT present design concepts as product screenshots.** Label every concept/mockup/prototype
   with `CONCEPT` / `PROPOSED` / `IN DEVELOPMENT` (Public Comm Policy §2, screenshot rule).
6. **DO NOT publish a pricing page or revenue claim** before its commercial gate (Commercial Model
   §4). Level 0 (public development) is permitted now — Level 2 (paid) requires R3+ on the specific
   capability sold; Level 3 (GA) requires Phase 16.
7. **DO NOT commit secrets.** Cloudflare/AWS/Penpot MCP/GitHub/model/SMTP/payment keys stay in
   environment/secret management. Never in source, repo, docs, screenshots, logs, commits.
8. **DO NOT fabricate a Penpot MCP connection.** Detect, report what is missing, stop. Read-only
   commission first (§PENPOT).
9. **DO NOT provision AWS infrastructure for hypothetical traffic.**
10. **DO NOT mix everything into one commit.** Bounded changes; per-unit report format below.

---

# THE EVIDENCE RULE

For every command, paste the **actual terminal output**, including the command line, wall-clock
duration, and exit status. A pasted failure is worth more than a clean claim. Never write
"Expected Output" where evidence belongs.

# THE NEVER-SELF-CERTIFY RULE

Never mark a capability GREEN because you implemented it or because a unit test passed. Follow the
Proof Graph. Green (R3) requires a named, committed entry point plus captured evidence.

---

# WORKSTREAMS

Six workstreams, each with its own readiness gate. Track them by the named milestones.

## A. GITHUB PUBLIC FOUNDATION — `PUBLIC-FOUNDATION-01` — **NOW**

Repo: `https://github.com/zylvex-tech/zylcode`. First inspect current settings and `main`; do not
assume this prompt's description of current state.

1. **About description.** Target:
   *"Evidence-first autonomous software creation environment. Plan, build, run, inspect, repair,
   verify, and ship software from one project."*
   Remove unsupported language (formally verified / zero token waste / best / fastest / superior /
   production-grade) unless independently supported at the appropriate rung.
2. **Topics.** Add/reconcile: `ai coding-agent developer-tools rust tauri autonomous-agents mcp ide
   llm software-engineering local-first open-source`. Do not remove useful existing topics without
   justification.
3. **Website field.** Do not point at a placeholder/broken ZylCode page. Set it to the canonical URL
   only when that page is live and verified.
4. **Metadata audit.** description, topics, license, README license statement, homepage, releases,
   tags, Discussions, Issues, Projects, SECURITY/Contributing links, stale public claims. Report
   discrepancies.

## B. OSS COMMUNITY FOUNDATION — `PUBLIC-FOUNDATION-02` — **NOW**

Create/reconcile: `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`, `SUPPORT.md`,
`.github/ISSUE_TEMPLATE/{bug_report,feature_request,documentation,capability_claim}.yml`,
`.github/PULL_REQUEST_TEMPLATE.md`, `docs/contributing/{ARCHITECTURE_CONTRIBUTIONS,
CAPABILITY_EVIDENCE_GUIDE,EXTENSION_CONTRIBUTION_GUIDE}.md` (last PROPOSED until extension ABI exists).

`CONTRIBUTING.md` must explain: governance reading requirements, dev environment, formatting, testing,
evidence requirements, Proof Graph, capability registry, no fabricated completion, no unrelated
refactors, commit/PR expectations, security reporting, docs expectations.

`SECURITY.md` must provide a responsible-disclosure mechanism **without claiming a nonexistent SLA**.
Do not publish a false security email; verify the address or use a verified organizational contact.

## C. DEVELOPMENT LOG — `PUBLIC-FOUNDATION-03` — **NOW**

Create `docs/devlog/`. Suggested subjects (not marketing):
1. Why coding agents shouldn't certify their own work · 2. We caught our own false benchmark ·
3. How ZylCode survives an agent crash · 4. R2 is a library; R3 is a capability ·
5. Why ZylCode uses an Evidence Ledger · 6. Building repository intelligence without lying to
ourselves · 7. Why a model dropdown is not Model Democracy · 8. Building ZylCode in public.

Each post: `PROBLEM / WHAT WE ATTEMPTED / WHAT FAILED / WHAT WE LEARNED / WHAT CHANGED /
CURRENT LIMITATIONS / EVIDENCE / NEXT STEP`. `WHAT FAILED` and `CURRENT LIMITATIONS` are mandatory.
Do not publish security-sensitive internals.

**Screen-recording protocol:** create `docs/community/DEMO_RECORDING_PROTOCOL.md` — scenario,
starting commit, environment, command, expected result, evidence, recording + redaction +
publication checklists. Record **real** software only. Redact keys, tokens, credentials, private
repos, PII.

## D. PUBLIC BENCHMARK SPECIFICATION — `PUBLIC-FOUNDATION-11` — **NOW (design); claims LATER**

Design the benchmark spec (categories: repo understanding, bug fixing, feature impl, test repair,
regression avoidance, crash recovery, context retrieval, visual impl, browser workflow, computer use,
Android, iOS, delivery). Each task defines fixture, initial state, task, allowed tools, success
criteria, independent verifier, time, model, tokens, cost, interventions, patch, evidence trace.
**Never launch a benchmark repo with weak/self-referential tests to claim superiority.** Create the
repository only when enough reproducible tasks exist.

## E. ZYLFORGE.COM REBUILD — `PUBLIC-FOUNDATION-04` then `07` → `08` → `09`

Begin with a **truth audit** (PF-04): audit existing zylforge.com, relevant zylvex.tech content,
current Engineering and ZylCode capability evidence. Produce the **Public Claim Matrix** mapping each
material capability to `AVAILABLE / DEVELOPER PREVIEW / BETA / IN DEVELOPMENT / PROPOSED /
SERVICE-DELIVERED / DEPRECATED`. No website copy may promote a PROPOSED capability as shipping.

**IA (PF-07):** three co-equal pillars — `ZylForge Engineering`, `ZylCode`, `Services` — under
`zylforge.com`, plus a `PLATFORM` area (MCP / Models / Extensions / Community). Homepage gives
Engineering and ZylCode equal weight. Positioning: *"Build software. Engineer the physical world."*
ZylCode = software creation+verification; ZylForge Engineering = physical engineering
creation+verification; shared DNA = evidence, verification, local-first, human authority.

**UX/UI (PF-08)** only after IA + Claim Matrix + Penpot commissioned. **Implementation (PF-09)** only
after key designs accepted. A minimal structural prototype may validate architecture but must not
become production by accident.

`/engineering`, `/zylcode`, `/services`, `/developer-preview` pages per the foundation plan §6.
Status-label every capability truthfully. Clearly label design concepts `CONCEPT`/`PROPOSED`.

**Services** generate near-term revenue from real engineering expertise (job packs, drawings, BOMs,
cut lists, weld notes, tolerances). Offer only what Zylvex/ZylForge can deliver. No payment until
pricing/commercial workflow is approved.

## F. PENPOT UNIFIED DESIGN PROGRAM — `DESIGN-SYSTEM-01` → `08` + `PUBLIC-FOUNDATION-05/06`

One shared foundation: **Zylvex / ZylForge Design System** — Foundations (brand, color, typography,
spacing, dimensions, radii, borders, elevation, motion, iconography, accessibility, responsive) and
Components (buttons, inputs, dropdowns, command palette, navigation, tabs, sidebars, panels, trees,
tables, cards, dialogs, notifications, toolbars, status, progress, code/editor chrome, property
inspector, canvas controls, artifact containers, agent status, evidence status). Use W3C DTCG
tokens.

Eight **coherent themes** defined through semantic tokens (background, surface, surface-raised,
border, text-primary, text-secondary, accent, accent-hover, success, warning, danger, info,
selection, focus, editor, canvas, viewport). Tokenize typography too. Maintain accessibility.

**Design → Code contract:** canonical token source in repo, e.g. `packages/design-tokens/{tokens.json,
themes/,generated/{css,typescript,rust/}}` — but **inspect first**; do not impose this location if a
canonical token architecture already exists. Penpot and implementation consume the **same** semantic
vocabulary. No scattered duplicate palette values.

Deliver in order: P0 shared foundations → P1 ZylForge.com → P2 ZylCode → P3 ZylForge Engineering.
Document Penpot lessons for Vision Studio in `docs/research/PENPOT_TO_VISION_STUDIO_LESSONS.md`
(research evidence; no copying Penpot source or proprietary assets).

---

# PENPOT CONNECTION — READ BEFORE WRITE

1. Detect whether a Penpot MCP connection exists. If not, **do not fabricate one**; report what is
   missing.
2. The human owner enables it: Penpot → Account → Integrations → MCP Server → Enable → Generate key
   → securely configures the MCP client. **Remote MCP preferred first.**
3. Commission **read-only**: list project → read metadata → inspect pages/layers → inspect tokens →
   **no mutation**. Capture evidence **without** exposing credentials.
4. Only after read commissioning passes may **write** operations be authorized, within an explicit
   scope (prefer new pages/branches over mutating production designs). Record file, page, operation,
   components, token changes, exports, date, revision id.
5. Never place the MCP key in source, repo, docs, screenshots, logs, or commits.

---

# CLOUD — `CLOUD-PLATFORM-01` → `03`

`docs/architecture/WEB_CLOUD_ARCHITECTURE.md`: Cloudflare edge (DNS/TLS/CDN/WAF/DDoS/rate-limit/bot,
Workers only where justified) for the public site first. AWS only when backend requirements justify
(identity, services, storage, marketplace backend, remote jobs). Local product stays local-first. **No
expensive AWS provisioning for hypothetical traffic.**

---

# EXECUTION SEPARATION

Independent bounded changes. Suggested sequence:
1. GitHub metadata/community (PF-01, PF-02) · 2. community/governance files · 3. devlog infra
(PF-03) · 4. website truth audit (PF-04) → Claim Matrix · 5. IA (PF-07) · 6. shared tokens
(PF-05, DESIGN-SYSTEM-02) · 7. Penpot commissioning (PF-06, DESIGN-SYSTEM-04) · 8. website designs
(PF-08) · 9. website implementation (PF-09) · 10. ZylCode concepts (DESIGN-SYSTEM-06) · 11. ZylForge
Engineering concepts (DESIGN-SYSTEM-07).

Each unit: `IMPLEMENT → LOCAL VERIFY → REVIEW DIFF → DOCUMENT → COMMIT → PUSH → VERIFY REMOTE SHA`.
CI blocked externally → `REMOTE_CI = BLOCKED_EXTERNAL`; never convert to PASS.

---

# FINAL REPORT PER EXECUTION UNIT

```
1. Objective (named track + milestone)
2. Preconditions
3. What was inspected
4. What changed; files changed; external settings changed
5. Tests/checks run + exact results
6. Evidence (raw output)
7. Known limitations
8. Capability status/rung if applicable (sourced from the manifest, not asserted)
9. Commit SHA
10. Push verification (remote SHA)
11. Remote CI status
12. Manual actions still required
13. Recommended next unit
```

**Never self-certify R5.** STOP if a prerequisite is absent — report the blocker instead of bypassing it.

---

# HOW YOU WILL BE AUDITED

Independent audit will: re-run commands from a clean checkout at your SHA; falsify each claim before
confirming; check the Claim Matrix is the source of any capability page (not hand-typed copy);
verify Penpot writes are scoped and read-first; confirm no secret, fabricated screenshot, or
over-claim is present; and confirm every `PUBLIC-FOUNDATION/DESIGN-SYSTEM/CLOUD-PLATFORM` milestone
label is a named track, never a phase number.

Build. Report. Stop.
