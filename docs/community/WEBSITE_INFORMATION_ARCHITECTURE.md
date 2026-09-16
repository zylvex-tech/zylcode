# ZylForge.com — Information Architecture

**Track:** `PUBLIC-FOUNDATION-07`
**Status:** SPECIFICATION — no page may be built from this until the Claim Matrix entries it cites are live.
**Depends on:** `PUBLIC-FOUNDATION-04` (`WEBSITE_TRUTH_AUDIT.md`, `PUBLIC_CLAIM_MATRIX.yaml`) — satisfied.
**Governed by:** `ZYLCODE_PUBLIC_COMMUNICATION_POLICY.md` ·
`ZYLCODE_PUBLIC_FOUNDATION_PLAN.md` §6, §8 · `ZYLCODE_COMMERCIAL_MODEL.md` §4 ·
`ZYLCODE_AGENT_OPERATING_PROTOCOL.md` §4.7.
**Unlocks:** `PUBLIC-FOUNDATION-08` (UX/UI design in Penpot).

> **This document is not a roadmap.** Nothing here is a phase. Per Protocol §4.7, phase numbers
> belong to the engineering roadmap and appear on this website **only** as roadmap metadata inside a
> capability's detail panel — never as a gate, a section name, or a navigation label.

---

## 0. What problem this IA solves

Most software sites communicate one thing: confidence. Maturity is hidden because exposing it
invites the question "is it finished?" — and the honest answer is usually "no."

ZylForge's product claim is the opposite. The whole thesis of ZylCode is that **work is only real
when it has been re-observed**. A website that hides maturity would contradict the product it
sells. So this IA is built to **expose** maturity:

- every capability is a first-class object with a **status** and an **evidence link**;
- the website's nouns are the same nouns as the Claim Matrix's nouns (`capability_state`,
  `lifecycle`, `proof_rung`);
- no page asserts a capability state; every page **reads** one.

The IA's success criterion is narrow and testable: **a reader can tell, without clicking into a
blog post, which capabilities they can use today and which are planned.** If a reader has to guess,
the IA has failed and the site is making a prohibited claim by omission.

---

## 1. Product hierarchy the site must express

From `ZYLCODE_PUBLIC_FOUNDATION_PLAN.md` §2:

```
Zylvex Technologies Ltd          (parent — corporate/research site: zylvex.tech)
        │
        └── ZylForge               (commercial/product platform: zylforge.com)
              ├── ZylForge Engineering
              ├── ZylCode
              └── Services
```

**Domain split — non-negotiable:**

| Domain | Owns | Does not own |
|---|---|---|
| `zylforge.com` | Products, capabilities, services, platform, community | Corporate history, research, investor/legal |
| `zylvex.tech` | Corporate identity, research notes, the "why Zylvex exists" narrative | Product capability claims |

Rationale: a capability claim on a corporate site is harder to keep synced to the matrix than one on
the product site, and drift is the failure mode this whole program exists to prevent. **`zylvex.tech`
does not carry capability cards.** It links out.

---

## 2. Top-level structure

Three co-equal pillars plus a platform area (plan §6). Not three nav items where one is a dropdown
sitting under another.

```
zylforge.com
├── /                       Home — equal weight to Engineering and ZylCode
├── /engineering/           PILLAR 1 — ZylForge Engineering       (physical systems)
├── /zylcode/               PILLAR 2 — ZylCode                    (software)
├── /services/              PILLAR 3 — Services                   (revenue now)
├── /platform/              PLATFORM  — shared substrate
│   ├── /platform/mcp/
│   ├── /platform/models/
│   ├── /platform/extensions/
│   └── /platform/community/
├── /evidence/              Evidence — the shared endpoint (plan §8)
├── /roadmap/               Roadmap — generated, not written
├── /devlog/                Development log — pulled from docs/devlog/
└── /about/                 Company, contact, legal
```

**Why the three pillars are siblings and not a hierarchy.** Plan §6: *"Services monetizes real
engineering expertise while software matures."* If ZylCode is presented as the parent and Engineering
as a feature, then when ZylCode is IN DEVELOPMENT the whole site reads as a vaporware site. If they
are siblings, a reader who needs a fabricated assembly today can buy it from a pillar that is
`SERVICE_DELIVERED`, while a reader who wants to follow the software sees an honest maturity label.
The structure is doing commercial work, not just layout work.

### 2.1 Home

Constraints:

- Engineering and ZylCode receive **approximately equal visual weight** (plan §6). Neither is the
  "hero" with the other demoted to a secondary card.
- The hero states positioning, not a capability: **"Build software. Engineer the physical world."**
- Immediately below the fold: a **status strip** — a compact, generated row of the site's four
  `AVAILABLE` capabilities and the count of `IN DEVELOPMENT` / `PLANNED` ones. This is the single
  most important element on the site, because it front-loads the honesty.
- No screenshot appears on the homepage without a §5-compliant label.

---

## 3. The three pillars

### 3.1 `/engineering/` — ZylForge Engineering (physical systems)

**What this pillar is:** human-delivered engineering service. Drawings, BOMs, cut lists, weld notes,
tolerance schedules, job packs. Its matrix entries are `SERVICE_DELIVERED` with
`owner_verify_required: true`.

**Constraint that shapes the whole pillar:** both service entries
(`engineering_job_packs`, `fabrication_documentation`) carry `owner_verify_required: true`. Until the
owner confirms Zylvex/ZylForge can actually deliver, **this pillar publishes no service claim, no
pricing and no turnaround time.** It may publish a plain description of the kind of work and a
contact route. A "Request a job pack" CTA is permitted; a price, a deadline, or a "we have delivered
N projects for M clients" claim is not (Policy §6 — a fabricated statistic is a prohibited claim).

**Pages:**

| Path | Purpose | Claim source |
|---|---|---|
| `/engineering/` | Pillar landing | `engineering_job_packs` |
| `/engineering/job-packs/` | What a job pack contains, scoped per request | `engineering_job_packs` |
| `/engineering/documentation/` | Drawings / BOM / cut list / weld notes | `fabrication_documentation` |
| `/engineering/capabilities/` | Per-discipline detail | service catalogue (owner-supplied) |

**Prohibited on this pillar:** *"generated automatically by software without engineer review"*
(matrix, `prohibited_claim`). The pillar must read as a service, not as a product. The distinction
matters legally and editorially.

### 3.2 `/zylcode/` — ZylCode (software)

**What this pillar is:** the product. Its matrix entries are `product: zylcode` — 13 of the 15
entries, spanning `AVAILABLE` through `PROPOSED`.

This is the pillar most at risk of over-claiming, because the majority of its capabilities are not
available. The structure below is designed so that the not-yet-available majority is *present on the
site* rather than omitted — omitting them would itself be a misrepresentation, and would make the
`AVAILABLE` capabilities look like the whole story.

| Path | Purpose | Claim source |
|---|---|---|
| `/zylcode/` | Pillar landing + status strip | all `product: zylcode` entries |
| `/zylcode/capabilities/` | **Index of every capability card** — generated | matrix, all entries |
| `/zylcode/capabilities/<slug>/` | Capability detail page | single matrix entry |
| `/zylcode/download/` | Install / availability route | `crash_recovery`, `execution_engine_shell` |
| `/zylcode/quickstart/` | First run | `AVAILABLE` entries only |
| `/zylcode/architecture/` | Eight-system map, with per-system rung | `ZYLCODE_ARCHITECTURE_V2.md` §9 |

**The capability index is the site's centre of gravity.** It is generated from the matrix, sorted
`AVAILABLE` → `DEVELOPER PREVIEW` → `IN DEVELOPMENT` → `PROPOSED`. No entry is hidden and no entry is
promoted. A reader landing there sees 4 available and 9 not — which is the honest picture, and is
more credible than a page that shows only the 4.

#### Capability detail page — required anatomy

Every capability page carries these seven blocks, in this order. The first three are non-negotiable;
a page missing any of them is incomplete and must not ship.

```
┌─────────────────────────────────────────────────────────────┐
│ 1  NAME                                                     │
│    STATUS LABEL            ← §4 vocabulary, generated       │
│    one-line description from `allowed_claim`                │
│                                                             │
│    Current proof: R2 EXECUTED       ← from `proof_rung`      │
│                                                             │
│    [Evidence]  [Roadmap]            ← first-class links     │
├─────────────────────────────────────────────────────────────┤
│ 2  WHAT WORKS TODAY                                         │
│    The `allowed_claim`, verbatim. Not paraphrased, not      │
│    enlarged, not softened.                                  │
├─────────────────────────────────────────────────────────────┤
│ 3  WHAT THIS DOES NOT DO (YET)                              │
│    The `prohibited_claim`, restated as a limitation.        │
│    This block is MANDATORY on every page.                   │
├─────────────────────────────────────────────────────────────┤
│ 4  WHAT IT IS                                               │
│    Plain description. Architecture link.                    │
├─────────────────────────────────────────────────────────────┤
│ 5  HOW IT IS VERIFIED                                       │
│    The proof rung's meaning, in one sentence, plus the      │
│    Evidence link to the captured transcript.                │
├─────────────────────────────────────────────────────────────┤
│ 6  LIFECYCLE                                                │
│    ACTIVE / EXPERIMENTAL / DEPRECATED / RETIRED             │
├─────────────────────────────────────────────────────────────┤
│ 7  ROADMAP REFERENCE                                        │
│    Engineering phase, as metadata only, collapsed by        │
│    default. Never a nav item. Never a gate.                 │
└─────────────────────────────────────────────────────────────┘
```

**Block 3 is the design's load-bearing element.** Every other software site omits it. It exists
because the matrix's `prohibited_claim` field is *as binding as* `allowed_claim` (Policy §4 rule 3),
and because a reader who wanted "can it operate my computer?" deserves an explicit no rather than an
inference from a green/grey dot. It is rendered at full text size, not as fine print.

**The status label is computed, never typed.** Policy §11 rule 1: a hand-asserted status is R0. The
page template reads `public_label` from the matrix entry and renders it; it does not contain the
string "Available" in its own source.

### 3.3 `/services/` — Services (revenue now)

**What this pillar is:** the commercial surface where the Claim Matrix says `SERVICE_DELIVERED`.
It exists because plan §6 assigns Services the job of monetizing expertise while the software
matures — meaning this pillar may make a commercial offer at Commercial Level 0 that the software
pillar may not.

| Path | Purpose |
|---|---|
| `/services/` | Services landing — what we do, how to engage |
| `/services/engagements/` | Engagement shapes (scoped per request) |
| `/services/contact/` | Enquiry route |

**Hard constraint:** Services may accept enquiries and describe scope. It may **not** display a price,
an availability guarantee, or a delivery statistic until the corresponding
`owner_verify_required: true` entries are verified. Commercial Model Level 0 permits selling
*existing* capability; it does not permit inventing a delivery record.

**Why Services is a full pillar rather than a Contact page.** If the only revenue-now surface is a
contact form, the site's commercial credibility rests entirely on the not-yet-available software. A
pillar that describes scoped human engineering work is a truthful offer today, and it is the honest
answer to "what can I actually buy from you right now?"

---

## 4. Status vocabulary, as rendered

Normative source: `ZYLCODE_PUBLIC_COMMUNICATION_POLICY.md` §3. The IA does not invent terms and does
not render a term above the matrix evidence. Matrix `public_label` → site rendering:

| `capability_state` | `public_label` | Rendered label | Visual register |
|---|---|---|---|
| `AVAILABLE` | Available | `AVAILABLE` | solid, full-contrast |
| `VERIFIED` | *(verification record)* | `VERIFIED` | solid + proof citation |
| `DEVELOPER_PREVIEW` | Developer preview | `DEVELOPER PREVIEW` | outline, pre-release tag |
| `IN_DEVELOPMENT` | In development | `IN DEVELOPMENT` | outline, low-contrast |
| `PROPOSED` | Planned | `PROPOSED` | dashed, lowest contrast |
| `CONCEPT` | *(exploration)* | `CONCEPT` | dashed + diagonal watermark on visuals |
| `HISTORICAL` | *(older build)* | `HISTORICAL` | muted, carries version/commit |
| `SERVICE_DELIVERED` | Service | `SERVICE-DELIVERED` | service register, not product |

Separately, `lifecycle` (`ACTIVE` / `EXPERIMENTAL` / `DEPRECATED` / `RETIRED`) is rendered as a
small secondary badge. **The two axes are never merged into one visual.** A capability can be
`PROPOSED` (state) and `ACTIVE` (lifecycle — the plan is live) at the same time; collapsing them
would produce a misleading single colour.

**Forbidden on the site:** `BETA`, `best`, `fastest`, `production-grade`, `formally verified`,
`enterprise-ready`, and any other unapproved status word (Policy §6; Claim Matrix header note).
`BETA` is specifically retired — it is not a term in either axis, and its informal meaning
("mostly works") is exactly the ambiguity the canonical vocabulary exists to remove.

---

## 5. Screenshot and visual labelling

Policy §2 is a normative input to this IA, not advice, so the IA specifies where labels physically
go:

1. **Every non-implementation visual** (concept, mockup, Penpot frame, prototype, animation) carries
   its status label **inside the image bounds** — a corner chip or a footer band — not only in
   surrounding prose.
2. **Real-build screenshots** carry version, date, or commit SHA adjacent to the image.
3. **Simulated behaviour** is labelled as simulated. The Computer-Use Engine is the named standing
   case (Policy §3): its UI appears functional while every action is a timer.
4. **Fabricated data** in a screenshot is labelled illustrative.
5. Where a capability's matrix entry does not exist, **no visual may be used to illustrate it at all**
   (Policy §4 rule 2 — a capability absent from the matrix has no permitted public claim).

The design system (`PUBLIC-FOUNDATION-05`) owns the label component. The IA requires only that the
component exists and is mandatory — the component may not be optional, and it may not be dismissible.

---

## 6. The platform area — `/platform/`

Shared substrate, not a fourth product. Its purpose is to explain what the three pillars have in
common — which is the strongest argument the company has for being one company rather than three.

| Path | Purpose | Matrix link |
|---|---|---|
| `/platform/mcp/` | Model Context Protocol surface | `extension_platform` (`PROPOSED`) |
| `/platform/models/` | Provider strategy, BYOK | `model_platform` (`IN DEVELOPMENT`) |
| `/platform/extensions/` | Extension model | `extension_platform` (`PROPOSED`) |
| `/platform/community/` | Devlog, discussions, contribution routes | n/a — community |

**`/platform/models/` carries a specific obligation.** `ZYLCODE_COMMERCIAL_MODEL.md` §3.1 prohibits
the phrase "managed key" and requires **secure BYOK credential management** — model access is never
bundled and never marked up. This page is the only place on the site where that policy becomes
user-visible, so it must state it plainly: *you bring your own model credentials; we do not resell
model access.* A pricing page that quietly implies bundled inference would be a prohibited claim.

**`/platform/extensions/` must not imply a stable ABI.** The matrix entry is `PROPOSED`, and
`EXTENSION_CONTRIBUTION_GUIDE.md` is itself marked PROPOSED. The page may describe the intended
model and invite design input; it may not publish an ABI contract, a compatibility promise, or a
submission process that implies review capacity that does not exist.

---

## 7. `/evidence/` — the shared endpoint

Plan §8: the three products share an interaction grammar ending in **Evidence**.

```
ZylCode               Project → Mission → Work → Evidence
ZylForge Engineering  Project → Engineering Task → Work → Evidence
Website               Product → Capability → Demonstration → Evidence
```

Evidence is the site's organising metaphor **and** its most concrete artifact, and the IA makes it a
destination rather than a footer link:

| Element | Behaviour |
|---|---|
| Every capability card | links to its Evidence entry |
| Every capability detail page | has an `[Evidence]` action in block 1 and block 5 |
| `/evidence/` | index of captured transcripts by capability, newest first |
| Evidence entry | names capability, rung, date, environment, reproduction, raw output |

A capability whose evidence is not captured has **no Evidence link** — the link is omitted, not
pointed at a placeholder. A dead Evidence link would be worse than none: it would assert a
verification that does not exist.

`/evidence/` is the page that most directly differentiates ZylForge from every other software site.
It is also the page that is hardest to fake, which is why it is the one to build first and the one
to keep honest.

---

## 8. `/roadmap/` — generated, not written

The roadmap page is **rendered from** the engineering roadmap + the Claim Matrix. It is not hand-authored
marketing copy.

**Rendering rules:**

1. A capability's roadmap position comes from its matrix entry and the roadmap document — never from
   a designer's or writer's memory.
2. Phase numbers appear as **metadata within a capability's own context**, expressed in plain
   language ("planned; engineering phase not yet started"). They are never used as a section
   heading, a nav label, a URL segment, or a gate statement (Protocol §4.7).
3. Status advances on the site **only** when the matrix advances. There is no editorial path by
   which a label improves.
4. The page states plainly that it is derived, and from what.

**Why this matters concretely.** The failure this IA is guarding against is a roadmap page that
quietly becomes aspirational — where "Phase 3" is shown as "coming Q1" and a reader infers a
commitment. A generated roadmap cannot drift from the matrix, because it has no independent copy.

---

## 9. Data flow — one direction only

```
docs/capability-registry.json          (engineering, 32 entries; 12 DISPUTED)
docs/governance/ZYLCODE_ARCHITECTURE_V2.md §9   (authoritative rung status)
        ↓  where they disagree, the architecture rung wins
public service catalogue                (owner-supplied)
        ↓
PUBLIC_CLAIM_MATRIX.yaml                ← the claim of record
        ↓
┌───────────────┬───────────────┬───────────────┐
↓               ↓               ↓               ↓
Website      README          Roadmap        Devlog
```

**The direction is one-way.** The website never writes back to the matrix. A live page that exceeds
its entry is corrected **at the page**, not by inflating the matrix (Policy §4; `WEBSITE_TRUTH_AUDIT.md`
"How discrepancies flow back"). If the matrix is genuinely wrong — the capability actually advanced —
the correction flows through governance: evidence first, architecture rung table second, matrix third,
website fourth.

**Consequence for the build:** the website is a **consumer** of `PUBLIC_CLAIM_MATRIX.yaml`. Any
implementation that hard-codes capability states into page source has broken this IA, regardless of
whether it renders correctly on the day it ships.

---

## 10. Commercial gating

From `ZYLCODE_COMMERCIAL_MODEL.md` §4 and Policy §10. The IA must be buildable at Level 0 and must
not require a rebuild to move up a level.

| Site surface | L0 Public Development | L1 Developer Preview | L2 Paid Early Access | L3 GA |
|---|---|---|---|---|
| `/` status strip | ✓ | ✓ | ✓ | ✓ |
| `/zylcode/capabilities/` | ✓ | ✓ | ✓ | ✓ |
| `/evidence/`, `/devlog/`, `/roadmap/` | ✓ | ✓ | ✓ | ✓ |
| `/engineering/`, `/services/` (enquiry only) | ✓ | ✓ | ✓ | ✓ |
| `/zylcode/download/` | labelled pre-release | ✓ | ✓ | ✓ |
| `/zylcode/pricing/` | **absent** | preview offer | prices for accepted capabilities only | ✓ |
| SLA / compliance representations | **absent** | **absent** | **absent** | ✓ |

**Two hard rules:**

1. **No pricing page exists at L0.** Not an empty one, not a "contact us for pricing" one. Its
   absence is the honest signal that no commercial offer is being made.
2. **The L3 gate is Public Commissioning acceptance — not a phase number.** The IA must never encode
   GA as "after Phase 16" (Protocol §4.7; the plan's own §7 unlock row expresses the same rule).

Because the surfaces are additive, Level 0 can be built and shipped now, and higher levels are new
routes rather than rewrites.

---

## 11. Gaps this IA does not close

Honest statement of what is missing, so that building cannot proceed on assumptions.

1. **Live-site crawl not performed.** The current contents of `zylforge.com` / `zylvex.tech` were not
   observable from the build sandbox (HTTP 200, body not transferable). What exists on the live
   domains today is **unknown** to this document. `WEBSITE_TRUTH_AUDIT.md` carries the owner
   checklist; until it is run, this IA specifies a target, not a migration.
2. **No site repository exists.** This repository has a single branch (`main`) and a single remote,
   and contains no `website/` path. Whether the site lives in this repo, a new `zylforge-web` repo,
   or a hosted platform is an **owner decision** that affects PF-09 but not this IA's structure.
3. **Brand decisions outstanding.** `PUBLIC-FOUNDATION-05` (design system) is gated on brand assets.
   This IA constrains structure and vocabulary; it deliberately constrains no colour, type, or logo.
4. **Service catalogue not supplied.** `/engineering/` and `/services/` page sets assume an
   owner-supplied catalogue of disciplines and engagement shapes. The matrix establishes the claim
   boundary; it does not enumerate the services.
5. **Penpot not commissioned.** PF-08 (design) is gated on `PUBLIC-FOUNDATION-06` — a read-only
   commissioning that is an owner action.
6. **`owner_verify_required` entries unverified.** Both service entries remain unverified. Until
   verified, the Services pillar publishes enquiry routes only, per §3.3.

---

## 12. Acceptance criteria for PF-07

This document is complete when a reviewer can confirm:

- [x] Three pillars are siblings at top level, with equal weight on the homepage (plan §6).
- [x] A `PLATFORM` area exists at top level with MCP / Models / Extensions / Community (§6).
- [x] Every capability page reads its state from `PUBLIC_CLAIM_MATRIX.yaml`; none asserts it (§9).
- [x] Every capability page carries a mandatory *What this does not do (yet)* block (§3.2).
- [x] Every capability card carries an Evidence link, omitted where evidence is absent (§7).
- [x] Status vocabulary matches Policy §3 exactly; `BETA` and other unapproved terms are excluded (§4).
- [x] Screenshot labelling is specified with placement, not merely required (§5).
- [x] No phase number appears as a nav item, section name, URL segment, or gate (Protocol §4.7).
- [x] The site is buildable at Commercial Level 0 and additive to Levels 1–3 (§10).
- [x] Open gaps are stated rather than assumed (§11).

**Verification method:** an independent reviewer maps each criterion to the section that satisfies
it, and attempts to falsify §10's "buildable at Level 0" and §9's "one-way data flow" by looking for
a page, template, or route in this document that would require a hand-typed capability state.

---

## 13. Next

`PUBLIC-FOUNDATION-08` — website UX/UI design in Penpot. Gate: **IA (this document) + Claim Matrix +
Penpot commissioned**. Penpot commissioning is `PUBLIC-FOUNDATION-06` and is a **read-first owner
action** (plan §11) — read-only access, no mutation, no credentials in the repo.
