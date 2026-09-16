# PROOF_ENGINE.md — ZylCode Proof Engine

**Status: PROPOSED**
**Rung: R0 (CLAIMED)**
**Phase: 12**
**Governing docs:** `ZYLCODE_PROOF_GRAPH.md` · `ZYLCODE_ARCHITECTURE_V2.md` §3.6

> ⚠️ **This system does not exist.** A verification rung exists in the pipeline; the Proof Graph
> does not. The two are not the same thing.

---

## 1. Responsibility

Unify all verification and make every capability claim **provable**.

**Verification domains:** compile · lint · unit tests · integration tests · browser tests ·
visual tests · accessibility · Android runtime · iOS runtime · security · packaging · deployment.

---

## 2. The Core Distinction

| | Verification (exists today) | Proof Engine (Phase 12) |
|---|---|---|
| Scope | one artifact, one rung | every capability, every claim |
| Output | a pass/fail | a **Proof node** in a graph |
| Citation | none | mandatory, resolves to the ledger |
| Status | asserted by a human | **computed** from evidence |
| Invalidations | not tracked | tracked; proofs go STALE |

> **A verification rung is a verdict. A proof is a verdict with a citation, a dependency set, and
> a lifetime.**

---

## 3. The Proof Graph

```
Mission
  └── Criterion[]                    (from the Mission definition)
        └── Proof[]                  (each with a rung)
              ├── Artifact[]         (citable outputs)
              ├── Evidence[]         (ledger entries)
              └── Proof[]            (dependencies — proofs can depend on proofs)
```

### 3.1 Node definitions

| Node | Definition |
|---|---|
| **Claim** | something asserted to be true |
| **Criterion** | an acceptance condition attached to a Mission |
| **Proof** | an evaluated claim with a rung and ≥ 1 citation |
| **Artifact** | a citable, inspectable output |
| **Evidence** | an immutable ledger entry recording what happened |
| **Decision** | a recorded choice (routing, permission, approval) |

### 3.2 The ladder

```
R0 CLAIMED · R1 OBSERVED · R2 EXECUTED · R3 VERIFIED · R4 REPRODUCIBLE · R5 COMMISSIONED
```

Full definitions in `ZYLCODE_PROOF_GRAPH.md`.

---

## 4. Rules (enforced, not advisory)

| # | Rule | Enforcement |
|---|---|---|
| 1 | Every Proof cites ≥ 1 Artifact | a proof without a citation is downgraded to Claim |
| 2 | Every Artifact resolves to a Ledger entry | citation resolution is validated |
| 3 | A Criterion is satisfied only by a Proof at **R3+** | Mission cannot reach COMPLETE otherwise |
| 4 | A Mission cannot complete with an unsatisfied Criterion | fail closed |
| 5 | Downgrades are recorded, not deleted | supersession entries |
| 6 | An index is not a ledger | separate stores |
| 7 | A proof whose inputs changed becomes **STALE** | invalidation tracking |

### 4.1 Rule 3 is the load-bearing one

> **Tests alone do not satisfy a user-facing acceptance criterion.**

A unit test is R2. If a Mission's criterion is "users can complete a password reset", no number of
unit tests satisfies it — the criterion requires a proof at R3 that a user or agent exercised the
flow.

This is precisely the rule that would have prevented Phase 2A's mis-classification.

---

## 5. Invalidation

A Proof becomes **STALE** when:

- the artifact it cites changes;
- a dependency's rung drops;
- the producing environment is no longer reproducible.

Stale proofs must be re-established or explicitly marked. **Silently retaining a stale proof is
a defect**, and the registry must surface it.

---

## 6. Rung Computation

```
rung(capability) = min(rung(component) for component in critical_path(capability))
```

The **rung ceiling** (`ZYLCODE_PROOF_GRAPH.md` §2.2): a system with four R3 components and one
R2 component is R2. Optimism is not additive.

**Rungs are computed, never asserted.** If a rung cannot be justified by an attached artifact, it
is R0 by definition.

---

## 7. Verification Providers

Phase 12 unifies verification through the Extension Platform (Phase 5):

```
verification provider contract
├── compile       (cargo, tsc, gradle, xcodebuild)
├── lint          (clippy, eslint, ktlint)
├── unit          (test runners per ecosystem)
├── integration
├── browser       (Phase 7 backend)
├── visual        (Phase 9 difference model)
├── accessibility
├── android       (Phase 10 device lab)
├── ios           (Phase 11 worker)
├── security      (scan, dependency audit)
├── packaging     (Phase 13)
└── deployment    (Phase 13)
```

A verification provider is a **package**, not a core special case. That is why Phase 5 precedes
Phase 12.

---

## 8. Interfaces

```
zylcode verify <claim>
zylcode verify mission <mission_id>
zylcode proof show <proof_id>
zylcode proof graph <mission_id>
zylcode proof stale
```

Plus the Evidence / Proof panel.

**Internal API**

```
ProofEngine::verify(claim) -> Proof
ProofEngine::criterion_satisfied(criterion) -> Option<Proof>
ProofEngine::invalidate(proof_id, reason)
ProofEngine::rung_of(capability) -> Rung
```

---

## 9. Benchmark / Acceptance

| # | Criterion | Method |
|---|---|---|
| 1 | Every registry capability carries a **computed** rung | registry dump |
| 2 | A capability claiming a rung above its evidence is **flagged as a defect** | seeded violation test |
| 3 | A Mission with one unsatisfied criterion **cannot** reach COMPLETE | negative test |
| 4 | A proof citing an unresolvable artifact is **rejected** | negative test |
| 5 | A changed artifact marks dependent proofs STALE | mutation test |
| 6 | A verification provider is added **without core modification** | out-of-tree package |

**Rung target: R3.**

---

## 10. Anti-Requirements

- Do not treat tests as satisfying a user-facing criterion.
- Do not assert a rung; compute it.
- Do not retain a stale proof silently.
- Do not allow a proof without a citation.
- Do not special-case verification providers in core.
- Do not let the Proof Engine write to the ledger's past — append only.
