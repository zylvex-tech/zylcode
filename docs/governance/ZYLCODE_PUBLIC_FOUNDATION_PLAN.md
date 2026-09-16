# ZYLCODE PUBLIC FOUNDATION MASTER EXECUTION PLAN v1.0

**Status: GOVERNING**
**Version:** 1.0
**Ratified:** 2026-09-16
**Governing docs:** `ZYLCODE_COMMERCIAL_MODEL.md` · `ZYLCODE_PUBLIC_COMMUNICATION_POLICY.md` ·
`ZYLCODE_AGENT_OPERATING_PROTOCOL.md` §4.7–§4.8. Full set in §9.

> **This plan sits *alongside* the 16-phase engineering roadmap, not inside it.**
> The engineering roadmap (`ZYLCODE_ROADMAP_V2.md`) owns product capability. This plan owns public
> identity: GitHub, community, development log, design system, website, cloud architecture, and
> release communication. The two run in parallel. Neither is gated on the other except where §7
> says so explicitly.

---

## 1. The Principle This Plan Defends

> **A phase number belongs to the governance roadmap and to nothing else.** (Protocol §4.7)

The website, community, Penpot, devlog, benchmark and Cloudflare/AWS work are **not** Phase 2C,
Phase 3, Phase 17, or any other phase. They are **named tracks** with their own milestone labels.
This is not cosmetic. Putting marketing, design and infrastructure into the engineering phase
sequence would recreate the phase-number collision that `PHASE_NUMBERING_RECONCILIATION.md` just
fixed — where "Phase 12" meant two different things.

**Tracks use a `PREFIX-NN` label.** Example: `PUBLIC-FOUNDATION-06`, `DESIGN-SYSTEM-04`,
`CLOUD-PLATFORM-01`.

---

## 2. Product Hierarchy (informational, not a roadmap)

```
Zylvex Technologies Ltd            (parent technology company)
        │
        └── ZylForge                (commercial/product platform)
              ├── ZylForge Engineering
              ├── ZylCode
              └── Services          (revenue now, via real engineering expertise)
```

`zylforge.com` is the commercial home. `zylvex.tech` is the corporate/research site.

---

## 3. The Tracks

### 3.1 `PUBLIC-FOUNDATION` — public, community, web, release communication

| Milestone | Work | Gate to start |
|---|---|---|
| **PUBLIC-FOUNDATION-01** | GitHub About/topics/metadata audit + reconciliation | Metadata access — **NOW** |
| **PUBLIC-FOUNDATION-02** | OSS community foundation (CONTRIBUTING, CODE_OF_CONDUCT, SECURITY, SUPPORT, issue/PR templates) | Repo access — **NOW** |
| **PUBLIC-FOUNDATION-03** | Development-log infrastructure (`docs/devlog/`) + editorial | None — **NOW** |
| **PUBLIC-FOUNDATION-04** | Website truth audit → **Public Claim Matrix** | Access to current sites/repos — **NOW** |
| **PUBLIC-FOUNDATION-05** | Unified design system (foundations + components) | Brand assets/decisions — **NOW** |
| **PUBLIC-FOUNDATION-06** | Penpot MCP commissioning (read-only first) | MCP/account connection — **NOW** |
| **PUBLIC-FOUNDATION-07** | ZylForge.com information architecture | PUBLIC-FOUNDATION-04 |
| **PUBLIC-FOUNDATION-08** | Website UX/UI design (Penpot) | IA + Claim Matrix + Penpot commissioned |
| **PUBLIC-FOUNDATION-09** | Website implementation | Key designs accepted |
| **PUBLIC-FOUNDATION-10** | Developer Preview funnel (signup, privacy, confirmation) | Privacy/contact workflow — **NOW** |
| **PUBLIC-FOUNDATION-11** | Public benchmark specification + **foundation** (repository later) | None — **NOW** |
| **PUBLIC-FOUNDATION-12** | Developer Preview launch readiness | Release gates for the preview build |

### 3.2 `DESIGN-SYSTEM` — the design program (replaces any "Phase 7" design conflation)

| Milestone | Work |
|---|---|
| **DESIGN-SYSTEM-01** | Brand foundations |
| **DESIGN-SYSTEM-02** | Semantic tokens (W3C DTCG) |
| **DESIGN-SYSTEM-03** | Core component library |
| **DESIGN-SYSTEM-04** | Penpot MCP commissioning |
| **DESIGN-SYSTEM-05** | ZylForge.com UX |
| **DESIGN-SYSTEM-06** | ZylCode UX (concepts, labelled) |
| **DESIGN-SYSTEM-07** | ZylForge Engineering UX |
| **DESIGN-SYSTEM-08** | Design ↔ implementation validation |

**Later:** feeds governing **Vision Studio 8A–9**. It does **not** replace that phase.

### 3.3 `CLOUD-PLATFORM` — web/cloud architecture (not a roadmap phase)

| Milestone | Work |
|---|---|
| **CLOUD-PLATFORM-01** | Cloudflare edge only (DNS, TLS, CDN, WAF, DDoS) for the public site |
| **CLOUD-PLATFORM-02** | ZylForge.com static + contact/funnel backend |
| **CLOUD-PLATFORM-03** | AWS only when backend requirements justify it (identity, services, storage, marketplace backend, remote jobs) |

**Rule (carried from the master prompt):** do not provision expensive AWS infrastructure for
hypothetical future traffic. Infrastructure follows demonstrated product requirements.

### 3.4 Pre-existing internal milestone tracks (reconciled in `PHASE_NUMBERING_RECONCILIATION.md`)

`MULTIPROVIDER-1`, `MULTIPROVIDER-2`, `CTX-COMPRESSION`, `VECTOR-CACHE`, `ARTIFACT-STREAM`. These
label feature-internal milestones and live in source comments. They are not roadmap phases.

---

## 4. Gating Model — start now vs later

The gating is **per-work-item**, not per-program. A large block of public-foundation work starts
immediately; the part that could outrun engineering is held.

| Track | Start | Gate |
|---|---|---|
| GitHub About/topics | **NOW** | Metadata access |
| Community files | **NOW** | Repository access |
| Discussions | **NOW** | GitHub admin access |
| Devlog infrastructure | **NOW** | None |
| Screen-recording protocol | **NOW** | None |
| Website truth audit | **NOW** | Site/repo access |
| ZylForge.com IA | **NOW** | Truth audit |
| Design system | **NOW** | Brand decisions |
| Penpot commissioning | **NOW** | MCP/account connection |
| ZylCode concept UI | **NOW** | Must be labelled `CONCEPT` |
| Public benchmark spec | **NOW** | None |
| Public benchmark *claims* | **LATER** | Reproducible independent runs |
| Website UX/UI | After IA + Claim Matrix | Penpot commissioned |
| Website implementation | After key designs accepted | IA/design accepted |
| Developer Preview signup | **NOW** | Privacy/contact workflow |
| Developer Preview download | **LATER** | Release gates |
| Paid ZylCode | **LATER** | Relevant commercial gate (Commercial Model §4) |

This gives parallelism **without** letting the marketing surface outrun engineering.

---

## 5. The Public Claim Matrix (the heart of truth-in-public)

Every major ZylForge.com capability is driven from the **Public Claim Matrix**, not hand-typed
marketing copy. The matrix is generated from:

```
Engineering repositories
        + ZylCode capability registry
        + Service capability catalogue
        ↓
   PUBLIC CLAIM MATRIX
        ↓
┌───────────┼───────────┐
↓           ↓           ↓
Website   README     Roadmap
```

**Manifest entry shape** (authoritative — see `ZYLCODE_PUBLIC_COMMUNICATION_POLICY.md` §4):

```yaml
capability: computer_use
product: zylcode
status: PROPOSED
proof_rung: R0
public_label: In development
allowed_claim: "Computer-use runtime is planned for ZylCode."
prohibited_claim: "ZylCode can operate your computer."
```

`repository_intelligence`: current `proof_rung: R2`, `public_label: In development`.

**Hard rule:** the website/README/roadmap **read** from the matrix. They never independently
assert. A capability absent from the matrix has **no permitted public claim**. `prohibited_claim`
is as binding as `allowed_claim`.

PUBLIC-FOUNDATION-04 produces the matrix. Until then, no capability page copy is final.

---

## 6. ZylForge.com Information Architecture (revised)

Three **co-equal** product pillars, plus platform — not a buried subpage.

```
                       ZYLFORGE.COM
                            │
          ┌─────────────────┼─────────────────┐
          │                 │                 │
          ▼                 ▼                 ▼
   ZYLFORGE ENGINEERING   ZYLCODE          SERVICES
   (physical systems)    (software)       (revenue now)
          │                 │                 │
          └─────────────────┼─────────────────┘
                            │
                         PLATFORM
            ┌───────────────┼───────────────┐
            ▼               ▼               ▼
           MCP           Models         Extensions
                            │
                         Community
```

Homepage gives Engineering and ZylCode approximately equal visual weight. Services monetizes real
engineering expertise while software matures.

**Status-labelling is mandatory on every capability page.** A capability card may carry, by design:

```
REPOSITORY INTELLIGENCE
IN DEVELOPMENT
Understands project structure, symbols and dependencies.
Current proof: R2 EXECUTED
[Roadmap] [Evidence]
```

Most software sites hide maturity. ZylForge **exposes** it — this is aligned with the Constitution.

---

## 7. Engineering Unlocks (explicit, narrow)

The engineering roadmap **does not block** website design or community building. It unlocks
*specific* public *activities*, and only those.

```
Engineering Phase 2A            → does NOT block website design
Engineering Phase 3B (Mission) → unlocks Mission product demonstrations
Artifact + Browser capability  → unlocks major public Alpha campaign
Vision + Computer Use           → unlocks corresponding public demos
Android                         → unlocks Android campaign
Public Commissioning (Phase 16) → unlocks GA claims (Commercial Model Level 3)
```

Each unlock is a **permission to run a campaign**, not a prerequisite for the design system, the
devlog, or the community. Conflating the two is the error this plan prevents.

---

## 8. Shared Interaction Language

The three products share an **interaction grammar** ending in **Evidence**:

| Product | Grammar |
|---|---|
| ZylCode | `Project → Mission → Work → Evidence` |
| ZylForge Engineering | `Project → Engineering Task → Work → Evidence` |
| Website | `Product → Capability → Demonstration → Evidence` |

**Evidence is the common endpoint.** This is the strongest unifying characteristic of the ZylForge
design language, and it is constitutional: the product's whole claim is verifiable work. The website
should make the evidence link first-class on every capability card.

---

## 9. Governing Document Set

This plan coordinates the following. All are authoritative; none is a phase.

| Document | Owns |
|---|---|
| `ZYLCODE_PRODUCT_CONSTITUTION_V2.md` | Supreme spec |
| `ZYLCODE_ARCHITECTURE_V2.md` | System topology |
| `ZYLCODE_MASTER_EXECUTION_PLAN.md` | 16-phase engineering program |
| `ZYLCODE_ROADMAP_V2.md` | Per-phase engineering detail |
| `ZYLCODE_CAPABILITY_MODEL.md` | Capability/status vocabulary (R0–R5) |
| `ZYLCODE_PROOF_GRAPH.md` | Proof ladder |
| `ZYLCODE_COMMERCIAL_MODEL.md` | Commercial activation ladder (Level 0 permitted now) |
| `ZYLCODE_PUBLIC_COMMUNICATION_POLICY.md` | What may be said publicly, status vocabulary, screenshot rule, claim matrix |
| `ZYLCODE_PUBLIC_FOUNDATION_PLAN.md` | This plan (tracks + gates) |
| `ZYLCODE_AGENT_OPERATING_PROTOCOL.md` | How agents work (§4.7 phase rule, §4.8 superseded-doc rule) |

The execution prompt given to the implementation agent is
`docs/governance/DEEPSEEK_PUBLIC_FOUNDATION_PROMPT_V1.md`.

---

## 10. Execution Discipline

- **Independent bounded changes.** Do not mix all of this into one commit.
- Each unit: `IMPLEMENT → LOCAL VERIFY → REVIEW DIFF → DOCUMENT → COMMIT → PUSH → VERIFY REMOTE SHA`.
- **Where CI is blocked by external billing:** `REMOTE_CI = BLOCKED_EXTERNAL`. Never convert that to
  PASS.
- **Plain `git push` hangs** in this environment — see `ZYLCODE_AGENT_OPERATING_PROTOCOL.md` note and
  the worktree/secret guidance. Use `GIT_TERMINAL_PROMPT=0 GIT_ASKPASS=echo git push origin main`.
- Penpot writes require an **explicit approved scope** and **read-first commissioning** (§11).
- Never self-certify. Never fabricate evidence, tests, benchmarks, screenshots, or counts.
- STOP if a prerequisite is absent. Report the blocker.

---

## 11. Penpot Connection — Read Before Write (carried rule)

1. Detect whether a Penpot MCP connection already exists.
2. If not, **do not fabricate one.** Report what is missing.
3. Never place the Penpot MCP key in source, repo, docs, screenshots, logs, or commits.
4. Commission with **READ-ONLY** access first: list project → read metadata → inspect
   pages/layers → inspect tokens → **no mutation**.
5. Capture evidence **without** exposing credentials.
6. Only after read commissioning passes may **write** operations be authorized, within an explicit
   scope (new pages/branches preferred over mutating production designs).

**Connection is the human owner's action** (Penpot → Account → Integrations → MCP Server → Enable →
Generate key → securely configure the MCP client). Remote MCP preferred first; local MCP later for
asset workflows.

---

## 12. Status (at ratification)

- Public Foundation 01–06, 10, 11 are **executable now**.
- The engineering roadmap remains at Phase 2A reopened / 2B blocked / 3A–16 not started — **unchanged**.
- No public claim may exceed the current capability rung. The Claim Matrix enforces this.
