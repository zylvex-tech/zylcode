# ZYLCODE_COMMERCIAL_MODEL.md — ZylCode Commercial Model

**Status: GOVERNING (deferred — not yet active)**
**Version:** 1.0
**Ratified:** 2026-09-16
**Governing docs:** `ZYLCODE_PRODUCT_CONSTITUTION_V2.md` §11 (Truth-in-Advertising),
§13 (Amendment Procedure)

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
| **Pro** | $9/mo flat | Rung R3 (verified, reachable capability), managed key, priority support | Solo developers, small teams evaluating |
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

## 4. The Pricing Gate

> **Do not publish a pricing page, price commitment, or revenue projection until the gate below
> is met.**

**Gate condition:** ZylCode has shipped a capability at **R3** — reachable through a named product
surface, with captured evidence — in the area the tier claims to sell.

**Current status: NOT MET.**

| Requirement | Status |
|---|---|
| A capability at R3 in the verification area | ❌ Phase 2A is **RE-OPENED**; the Intelligence Graph is R2 |
| Any phase beyond 1A–1D accepted | ❌ Phases 2B and 3A–16 not started |
| Truth-in-advertising position (§11) clean | ❌ The README previously carried unsupported claims (corrected in `4ddefd3`) |

**The gate is not "when we feel ready".** It is met when the evidence exists. The Proof Engine's
rung computation is the mechanism; a hand-asserted readiness is **R0** by definition
(`ZYLCODE_CAPABILITY_MODEL.md` §6).

**The original plan's gate — "do not publish pricing until Phase 10 ships" — is superseded.** That
referenced the obsolete numbering. The condition is restated above in capability terms, which do
not drift when phases are renumbered. This is the general rule: *express a gate as a capability
state, never as a phase number.*

---

## 5. Anti-Requirements

- **Do not publish pricing before the gate.** (§4)
- **Do not bundle or mark up model access.** (§3.1)
- **Do not paywall a user's own evidence, code, or artifacts.** (§3.1)
- **Do not describe a tier's included capability above its actual rung.** A tier that sells "R3
  verification" while R3 is unreachable is a truth-in-advertising breach (Constitution §11).
- **Do not let this document become a roadmap.** It governs commercial *claims*, not engineering
  sequence. Feature ordering lives in the roadmap.
- **Do not derive pricing from `STRATEGIC_PLAN.md`.** That document is superseded; this one
  governs. Where they differ, this document wins.

---

## 6. Dependency

Activating the commercial model is gated on **Phase 16 (Public Commissioning)** at the earliest,
and on the §4 gate unconditionally — whichever is later.

Recording this dependency is the point of the document. The gap is now a **decision**, not an
oversight.
