# ZYLCODE_COMMERCIAL_MODEL.md — ZylCode Commercial Model

**Status: GOVERNING (deferred — not yet active)**
**Version:** 1.1
**Ratified:** 2026-09-16 · **Amended:** 2026-09-16 (v1.1 — gate ladder, BYOK terminology)
**Governing docs:** `ZYLCODE_PRODUCT_CONSTITUTION_V2.md` §11 (Truth-in-Advertising),
§13 (Amendment Procedure)

> **v1.1 correction.** v1.0 contained a material contradiction: §4 gated activation on an R3
> capability while §6 gated it on Phase 16 "whichever is later", which would have blocked a paid
> Developer Preview until Public Commissioning. Replaced with the graduated four-level ladder
> (§4). Also resolved the "managed key" / never-resell-tokens ambiguity (§3.1).
> **Level 0 (public development) is permitted immediately.**

> **⚠️ This document is deferred by design.**
> ZylCode is **not for sale** and has **no pricing page** until the gate in §4 is met. This
> document exists so that commercial intent is *recorded and governed* rather than absent —
> an absence that would otherwise invite a future agent to re-derive pricing strategy from a
> superseded plan.

---

## 1. Why This Document Exists

Before 2026-09-16, the only record of ZylCode's commercial model was
`docs/STRATEGIC_PLAN.md` §5–6 — a document that had been **superseded** and whose phase
numbering was obsolete. The Constitution, Architecture and Master Execution Plan were entirely
silent on revenue.

That is a governance gap, not a detail. Two failure modes follow from it:

1. **Silent re-derivation.** An agent asked to "add a pricing page" would invent terms, with
   nothing to check them against.
2. **Truth-in-advertising breach.** The Constitution's §11 binds every public claim to evidence.
   A pricing claim is a public claim. Without a recorded model, there is no standard for what
   may be advertised about the product's commercial state.

This document closes the gap. It is deliberately thin: it records **what we charge for and when
we are allowed to say so**, not a growth plan.

---

## 2. The Governing Principle

> **Monetization is gated on shipping progress. Nothing is sold that cannot be demonstrated.**

This is the commercial expression of the Constitution's evidence principle. ZylCode's core claim
is that it produces verifiable work. Selling that claim before the verification infrastructure
exists would be the exact overclaim the governance package was written to prevent.

**Consequence:** the pricing gate (§4) is not a business preference. It is a **truth-in-advertising
control**, and it is enforceable in the same way as the PROPOSED marking rule.

---

## 3. The Model

### 3.1 What is sold — and what is not

**Sold:** the verification infrastructure. Proof, evidence, audit trails, governance controls.

**Not sold, ever:**

- **Model access is never bundled or marked up.** ZylCode routes to the user's own provider key
  (Anthropic, OpenRouter, Ollama, or a self-hosted endpoint) and the user pays that provider
  directly. ZylCode does not touch or resell tokens.

**Terminology — "managed key" is prohibited.** The phrase is ambiguous between two incompatible
things:

| Meaning | Compatible? |
|---|---|
| **Secure BYOK credential vault** — ZylCode stores and injects *the user's own* key | ✅ **Yes.** This is what a tier may include. |
| **ZylCode-provided managed model API key** — ZylCode supplies access to models | ❌ **No.** This is reselling model access, which §3.1 forbids. |

Tier copy must say **secure BYOK credential management** or **BYOK credential vault**. The
unqualified phrase "managed key" must never appear in a public claim, because the reader cannot
tell which is meant — and only one of the two is permitted.
- **The user's code and artifacts are never the product.** Local-first is a Constitution-level
  commitment (§10.3), not a pricing lever.
- **Evidence is never paywalled from the user who produced it.** A user can always export their
  own proof. Tiers differ in *governance features*, not in whether the user owns their results.

The BYOK stance is a structural position, not a promotion. It exists because a tool that marks up
token costs has an incentive to encourage token consumption — which is incompatible with the
efficiency the Evidence Principle demands.

### 3.2 Tiers (PROPOSED — not active, not advertised)

| Tier | Price | Includes | Intended for |
|---|---|---|---|
| **Free** | $0 | Rungs R1–R2 (structured logs, attestation manifest), BYOK, community support | Individual developers, OSS contributors |
| **Pro** | $9/mo flat | R3 (verified, reachable capability), **secure BYOK credential vault**, priority support | Solo developers, small teams evaluating |
| **Team** | $19/seat | Audit trail, governance controls, SSO | Teams with compliance requirements |
| **Enterprise** | Custom | Self-hosted, contractual verification SLA, dedicated support | Regulated industries |

**Tier mapping note.** The original tier table referenced "Rungs 1–4" — a **four-rung** ladder from
the superseded plan. It is restated here against the authoritative **R0–R5** ladder
(`ZYLCODE_PROOF_GRAPH.md`), because two ladders sharing one name is precisely the defect recorded
in `PHASE_NUMBERING_RECONCILIATION.md`. The mapping:

| Old (superseded) | Authoritative |
|---|---|
| Rung 1 — structured logs | **R1 OBSERVED** |
| Rung 2 — attestation manifest | **R2 EXECUTED** |
| Rung 3 — permission gate | **R3 VERIFIED** |
| Rung 4 — Z3/Dafny proofs | **R4 REPRODUCIBLE** (in part; formal proof exceeds it) |

**Prices are indicative.** They are recorded so the model is complete; they are not commitments
and must not be published before the gate.

---

## 4. The Commercial Activation Ladder

> **Correction (2026-09-16).** An earlier version of this document stated the gate as *"a
> capability at R3"* in §4 and *"Phase 16, whichever is later"* in §6. Those are materially
> different. The second would have blocked a paid Developer Preview until Public Commissioning —
> including for a capability that had already been independently accepted. **§6 was the defect and
> is withdrawn.** The ladder below replaces both.
>
> **Governing rule: the scope of the paid claim determines the evidence gate.**

Commercial activation is **graduated**, not binary. Four levels, each with its own gate.

| Level | Gate | What is permitted |
|---|---|---|
| **0 — PUBLIC DEVELOPMENT** | **None. May start now.** | Build in public: devlog, community, roadmap, status-labelled capability descriptions. No commercial offer. |
| **1 — DEVELOPER PREVIEW** | The included capabilities satisfy their **defined acceptance gates**. May be free or invite-only. | A preview offer, explicitly labelled pre-release. |
| **2 — PAID EARLY ACCESS / PRO** | The **specific** capabilities being sold are **R3+ and independently accepted**. **Does NOT require Phase 16.** | Charge for those capabilities. |
| **3 — GENERAL AVAILABILITY / ENTERPRISE** | **Public Commissioning (Phase 16)**, plus release, security and support gates. | GA claims, enterprise contractual claims, SLAs, compliance representations. |

### 4.1 Why the levels are scoped, not global

Level 2 does **not** require the whole product to be complete. It requires the **sold capability**
to be proven.

> Selling *repository intelligence* requires repository intelligence at R3.
> It does not require Vision Studio, Computer Use, Android, iOS, or multi-agent engineering.

The alternative — blocking all revenue until every phase lands — is not a truth-in-advertising
control. It is a business decision wearing the costume of one. It would forbid charging $9/month
for a genuinely commissioned capability because an unrelated later capability is unfinished, which
the Evidence Principle does not require and which would encourage the very behaviour this
governance exists to prevent: publishing optimistic claims in the absence of a legitimate path to
publishing true ones.

Level 3 is the exception. **GA and enterprise claims are global**, because they represent the
whole product as dependable — so they require Public Commissioning.

### 4.2 Current status

| Level | Status | Why |
|---|---|---|
| 0 — Public Development | ✅ **Permitted now** | No gate |
| 1 — Developer Preview | ❌ Not yet | No capability has passed its acceptance gate; Phase 2A is **RE-OPENED**, Intelligence Graph is R2 |
| 2 — Paid Early Access | ❌ Not yet | No capability at R3; the Computer-Use Engine is R0 |
| 3 — GA / Enterprise | ❌ Not yet | Requires Phase 16 |

### 4.3 The general rule

**Express a gate as a capability state, never as a phase number.** Phase numbers drift when
phases are renumbered or split — as happened when Phase 7 became 7A–7D. Capability states do not.

The former gate, *"do not publish pricing until Phase 10 ships"* (from the superseded plan),
referenced an obsolete phase number and is **withdrawn**.

Level 3's reference to Phase 16 is the one deliberate exception, and it is stated as an
**evidence condition** (Public Commissioning) with the phase given only as a locator — not as the
gate itself.

---

## 5. Anti-Requirements

- **Do not publish pricing before the gate for its level.** (§4)
- **Do not bundle or mark up model access.** (§3.1)
- **Do not use the unqualified phrase "managed key".** Name it *secure BYOK credential
  management*. (§3.1)
- **Do not block a paid capability because an unrelated capability is unfinished.** The scope of
  the paid claim sets the gate. (§4.1)
- **Do not paywall a user's own evidence, code, or artifacts.** (§3.1)
- **Do not describe a tier's included capability above its actual rung.** A tier that sells "R3
  verification" while R3 is unreachable is a truth-in-advertising breach (Constitution §11).
- **Do not let this document become a roadmap.** It governs commercial *claims*, not engineering
  sequence. Feature ordering lives in the roadmap.
- **Do not derive pricing from `STRATEGIC_PLAN.md`.** That document is superseded; this one
  governs. Where they differ, this document wins.

---

## 6. Dependency — Withdrawn and Replaced

> **This section previously read:** *"Activating the commercial model is gated on Phase 16
> (Public Commissioning) at the earliest, and on the §4 gate unconditionally — whichever is
> later."* **That is withdrawn.** It contradicted §4 and would have blocked a paid Developer
> Preview until Public Commissioning. See the correction notice at §4.

**Replaced by the graduated ladder in §4.** The only dependency on Phase 16 is at **Level 3
(GA / Enterprise)**, which is correct, because GA represents the whole product as dependable.

| Level | Depends on |
|---|---|
| 0 — Public Development | nothing |
| 1 — Developer Preview | acceptance gates for the included capabilities |
| 2 — Paid Early Access | R3+ and independent acceptance of **the capabilities being sold** |
| 3 — GA / Enterprise | Public Commissioning (Phase 16) + release/security/support gates |

**This document governs commercial claims, not engineering sequence.** It must not be read as
licence to accelerate engineering, nor as a reason to delay commercial activity that has met its
gate.

Recording the model is the point of this document. The gap is now a **decision**, not an oversight.
