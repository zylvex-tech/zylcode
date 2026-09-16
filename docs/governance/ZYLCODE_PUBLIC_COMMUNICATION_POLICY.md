# ZYLCODE_PUBLIC_COMMUNICATION_POLICY.md — Public Communication & Release Communication Policy

**Status: GOVERNING**
**Version:** 1.0
**Ratified:** 2026-09-16
**Governing docs:** `ZYLCODE_PRODUCT_CONSTITUTION_V2.md` §11 (Truth-in-Advertising) ·
`ZYLCODE_CAPABILITY_MODEL.md` · `ZYLCODE_PROOF_GRAPH.md` ·
`ZYLCODE_COMMERCIAL_MODEL.md`

> **Purpose.** The Constitution governs truth broadly. This document governs truth **publicly** —
> on the website, the README, GitHub metadata, the development log, release notes, social posts,
> screenshots, videos, and design artefacts. It exists because public development is now
> significant enough to need an operational policy, not just a principle.

---

## 1. The Core Rule

> **Never state or imply a higher capability state than the implementation possesses.**

This applies to prose, status labels, screenshots, video, design mockups, website animations,
prototypes, and Penpot frames. The medium is irrelevant; only the claim matters.

---

## 2. The Screenshot Rule (most important operational rule)

> **A screenshot, video, design mockup, website animation, prototype, or Penpot frame must never
> imply a higher capability state than the implementation it represents.**

This is the single most likely way this project will accidentally lie. Design work will run ahead
of engineering — that is normal and desirable — but a beautiful ZylCode UI concept shown without a
label becomes a false capability claim the moment a reader assumes it is real.

**Requirements:**

1. Any non-implementation visual — concept, mockup, prototype, Penpot frame, rendered animation —
   must carry a **visible status label** in the image itself, not only in surrounding prose.
2. Labels use the vocabulary in §3. `CONCEPT`, `PROPOSED`, and `IN DEVELOPMENT` are the usual cases.
3. A screenshot of a **real** running build must be identifiable as such: version, date, or commit
   SHA adjacent to it.
4. Footage of **simulated** behaviour must be labelled as simulated. The Computer-Use Engine is
   the standing warning: its UI appears functional while every action is a timer.

**Never:**

- Present a design concept as a product screenshot.
- Animate a workflow the product cannot perform.
- Show fabricated data in a screenshot without labelling it as illustrative.
- Use a mockup to illustrate a capability whose implementation does not exist.

---

## 3. Status Vocabulary (normative)

Every public capability claim carries exactly one of these. They are ordered; a claim must not use
a term above its evidence.

| Term | Meaning | Evidence required |
|---|---|---|
| **AVAILABLE** | Usable now through a named surface | R3+ and independently accepted |
| **VERIFIED** | The appropriate Proof Graph requirement is satisfied | Citation to the proof record |
| **DEVELOPER PREVIEW** | Usable but explicitly pre-release | Acceptance gates met; labelled pre-release |
| **IN DEVELOPMENT** | Real implementation exists; acceptance gate not yet satisfied | Named entry point or merged code |
| **PROPOSED** | Planned or design work only | Documented in the architecture |
| **CONCEPT** | UX/product exploration; **not** an implementation claim | Labelled as exploration |
| **HISTORICAL** | Refers to an older build or release | Named commit/version |
| **SERVICE-DELIVERED** | Human/company service, not software automation | Distinguishable from product capability |

**Mapping from internal status** (`ZYLCODE_CAPABILITY_MODEL.md`):

| Internal rung | Permitted public term |
|---|---|
| R3+ (accepted) | AVAILABLE / VERIFIED |
| R3 (pre-release) | DEVELOPER PREVIEW |
| R2 | IN DEVELOPMENT |
| R1 | IN DEVELOPMENT (only if a real entry point exists) |
| R0 | PROPOSED |
| — (no implementation) | PROPOSED, or CONCEPT for pure UX exploration |

**The standing example.** `computer_use` is R0 with a non-functional skeleton present. Its
permitted claim is *"Computer-use runtime is planned for ZylCode."* Its prohibited claim is
*"ZylCode can operate your computer."* Anything between those two lines is a violation.

---

## 4. The Public Claim Matrix

Every material public capability must be driven from a **public capability manifest**, not typed
by hand into marketing copy.

```
Engineering repositories
        +
ZylCode capability registry
        +
Service capability catalogue
        ↓
   PUBLIC CLAIM MATRIX
        ↓
┌───────────┼───────────┐
↓           ↓           ↓
Website   README     Roadmap
```

**Manifest entry shape:**

```yaml
capability: computer_use
product: zylcode
status: PROPOSED
proof_rung: R0
public_label: In development
allowed_claim: "Computer-use runtime is planned for ZylCode."
prohibited_claim: "ZylCode can operate your computer."
```

**Rules:**

1. The website, README and roadmap **read from** the matrix. They do not independently assert.
2. A capability absent from the matrix has **no permitted public claim**.
3. `prohibited_claim` is as binding as `allowed_claim`.
4. The generator must not upgrade a label. `R2` must never render as "production-ready".

**The manifest is a public artefact and is held to the same standard as the code** (Constitution
§11). It is not marketing metadata; it is the claim of record.

---

## 5. Public Benchmarks

Benchmarks are claims about the product and are governed by this document.

- **Never** publish a benchmark whose tasks are weak or self-referential.
- **Never** publish a competitor ranking without **reproducible equivalent** runs on both sides.
- Every published result must carry: fixture, initial state, task, allowed tools, success
  criterion, independent verifier, duration, model, token usage (where available), cost (where
  available), interventions, resulting patch, and evidence trace.
- A benchmark repository is created **only** when enough independently reproducible tasks exist to
  make it useful — not to have a benchmark.

---

## 6. Testimonials, Statistics and Third-Party Claims

Never fabricate or imply:

- user counts, downloads, stars, or community size;
- customer testimonials or case studies;
- performance measurements or comparisons;
- security, compliance, or certification claims;
- pricing availability before the relevant commercial gate;
- release availability, installer success, or platform support not verified.

**Where a number is published, its scope and collection method travel with it.** A metric without
a definition is an unverifiable claim.

---

## 7. Superseded and Historical Material

> **Superseded and historical documents may be consulted for provenance and rationale, but may
> never be used as current capability evidence.** (Agent Operating Protocol §4.8)

`docs/STRATEGIC_PLAN.md` is the live case. It is correctly marked **SUPERSEDED** and its obsolete
phase numbering is flagged — but its body still contains statements such as *"This is real, tested,
shippable infrastructure. Not a prototype."* and *"It's tested. It ships."*

**Those sentences are not rewritten** — altering them would falsify the historical record. They are
instead **quarantined**: they are evidence about what was believed in September 2026, and nothing
more. An agent must not lift them into a capability claim, a README, or a website page.

The same applies to any document carrying a SUPERSEDED, HISTORICAL, or DEPRECATED banner.

---

## 8. Editorial Standard for the Development Log

Development-log entries are **not** marketing. Each carries:

```
PROBLEM
WHAT WE ATTEMPTED
WHAT FAILED
WHAT WE LEARNED
WHAT CHANGED
CURRENT LIMITATIONS
EVIDENCE
NEXT STEP
```

`WHAT FAILED` and `CURRENT LIMITATIONS` are mandatory. An entry that omits them is a promotion, not
a log. Publishing failure is the mechanism by which this project earns the right to publish success.

**Never publish** unpublished security-sensitive implementation details in a public log.

---

## 9. Security Constraints on Public Material

Never publish or commit:

- API keys, MCP tokens, cloud credentials, or secret environment variables;
- personal information or private repository contents;
- screenshots containing credentials or private paths.

Every public artefact passes a redaction checklist before publication. **Scan staged changes before
every commit.**

---

## 10. Relationship to the Commercial Model

| Commercial level (`ZYLCODE_COMMERCIAL_MODEL.md` §4) | Communication permitted |
|---|---|
| **0 — Public Development** | Build-in-public, devlog, roadmap, status-labelled capability pages. **No commercial offer.** |
| **1 — Developer Preview** | Preview offer, labelled pre-release. |
| **2 — Paid Early Access / PRO** | Pricing for the accepted capabilities **only**. |
| **3 — GA / Enterprise** | GA claims, SLAs, compliance representations. |

Level 0 is **permitted immediately** — truthful communication about work in progress is not a
commercial claim, and the commercial model imposes no gate on it.

---

## 11. Enforcement

1. Claim status is **computed** from the manifest, never asserted by hand (Capability Model §6).
   A hand-asserted status is R0.
2. Any public claim is falsifiable. The auditor attempts to break it before confirming it.
3. A claim that cannot be traced to a manifest entry is withdrawn or corrected.
4. Corrections are **visible**. Where a public claim is corrected, the correction is published —
   not quietly edited (Constitution §11).
