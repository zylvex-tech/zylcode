# ZYLCODE RESEARCH MODE ARCHITECTURE

**Date:** 2026-10-06
**Status:** **DESIGNED. Nothing in this document is implemented.** See §0.
**Covers directive phases:** 6 (research workspace), 7 (reproducible computational research),
17 (research-to-product pipeline).
**Substrate it would extend:** `claim.rs`, `evidence_graph.rs`, `artifact_bus.rs`,
`first_mission.rs` (the reproducibility pattern).

---

## 0. Honesty statement

There is **no research code in this repository**. Verified by
`grep -ri 'research' crates/**/*.rs` → no module. There is no `ResearchProject` type, no dataset
model, no seed capture, no citation model.

This document is an **architecture**, not a status report. It is written because the directive asks
for the architectural foundation, and because a design that names exactly what does not exist is
more useful than a vague aspiration.

**What does exist and is relevant:** the *reproducibility pattern* proven by the First Mission
(`first_mission.rs`) — environment capture, spec hash, effect hashes, a resume that reconciles rather
than repeats, and a reproducibility package. Research mode should **extend that pattern**, not invent
a parallel one.

---

## 1. What a research project is (Phase 6)

A research project is a **first-class durable object**, not a folder with notes. It is a Project
(Constitution §4) with a different schema.

```jsonc
{
  "id": "research-…",
  "schema_version": 1,
  "project_id": "…",                  // links to the Project System
  "question": "…",                    // the research question, verbatim
  "hypothesis": "…",
  "background": "…",
  "sources": [ Source ],
  "literature_notes": [ Note ],
  "datasets": [ Dataset ],
  "experiments": [ Experiment ],
  "parameters": { … },
  "environment": { … },               // captured, not described
  "results": [ Result ],
  "figures": [ ArtifactRef ],
  "observations": [ Observation ],
  "limitations": [ String ],
  "citations": [ Citation ],
  "decisions": [ AdrRef ],
  "artifacts": [ ArtifactRef ],       // via artifact_bus
  "reproduction": { … },              // see §4
  "provenance": [ EvidenceRef ]       // into the evidence graph
}
```

### 1.1 Why it must be typed, not free text

A markdown file cannot answer "which of these sentences is a source, and which is my inference?".
The research value of the object is precisely that **every statement has a type**. Prose cannot carry
that; a schema can.

---

## 2. Reproducible computational research (Phase 7)

### 2.1 The capture set

For every experiment, capture — where technically feasible:

| Field | Why | Available from existing code? |
|---|---|---|
| `git_sha` | exact source revision | yes — `gitops`, `first_mission` |
| `dependency_lockfiles` | exact dependency set | yes — `Cargo.lock`, `pnpm-lock.yaml` are on disk |
| `runtime_versions` | interpreter/compiler versions | yes — `first_mission::capture_environment` |
| `os_platform` | platform | yes — same |
| `environment_metadata` | selected env vars, **never secrets** | yes — `evidence_graph` node `environment` |
| `dataset_hash` | which data | **no — new** |
| `input_hash` | which inputs | **partial** — `sha256_file` exists |
| `command` | exactly what ran | yes — `CmdOutcome` / `ToolEvidence` |
| `parameters` | with what settings | **no — new** |
| `random_seed` | determinism | **no — new** |
| `output_hash` | what came out | **partial** — `sha256_hex` exists |
| `test_result` | did verification pass | yes — claims + exit codes |
| `generated_artifacts` | the figures/data | yes — `artifact_bus` |

**Nine of thirteen already have machinery.** The four gaps (dataset hash, parameters, seed, output
hash) are small, well-understood additions — but they are **not implemented**, and until they are,
ZylCode must not claim reproducibility support.

### 2.2 "Reproduce Experiment 14"

The target behaviour: the phrase resolves to a **deterministic re-execution** with the captured
environment, and the result is compared by hash.

**The honesty constraint, taken verbatim from the directive:**

> *Do not pretend reproducibility exists until execution proves it.*

Therefore the designed feature has three outcomes, not two:

| Outcome | Meaning |
|---|---|
| `REPRODUCED` | every captured hash matched |
| `REPRODUCED_WITH_DIFFERENCES` | ran, but an output hash differs — **and the difference is recorded, not hidden** |
| `NOT_REPRODUCIBLE` | the environment or inputs could not be reconstructed — with the specific missing piece named |

A binary pass/fail would force the middle case to be reported as a failure or a success. Both would
be false.

---

## 3. Epistemic typing — the rule that protects research integrity (Phase 6)

> **Never allow an AI-generated statement to silently become a research result.**

The designed taxonomy, mapping onto the existing `Claim` model:

| Research type | Maps to `ClaimStatus` | Promotion rule |
|---|---|---|
| `SOURCE` | — (a citation, not a claim) | must carry an identifier; not promotable |
| `OBSERVATION` | `OBSERVED` | requires a captured measurement |
| `INFERENCE` | `DERIVED` | requires cited premises + stated reasoning |
| `HYPOTHESIS` | `HYPOTHESIS` | **never** promotable without new evidence |
| `RESULT` | `VERIFIED` | requires evidence at level ≥ R2 **and** a reproduction record |
| `DECISION` | — (an ADR) | requires attribution |

**The single most important line in this document:** an AI-generated statement enters as
`HYPOTHESIS`, and the *only* path to `RESULT` runs through a deterministic measurement plus a
reproduction record. There is no "the model said so" path. This is the existing `claim.rs` gate
applied to research, which is why the claim model is the correct substrate rather than a new one.

### 3.1 Provenance is in the data model, not the prose

Every research statement carries `provenance: [EvidenceRef]`. A statement whose provenance is empty
is displayable but **cannot be exported as a result**. The export gate is the enforcement point.

---

## 4. Research-to-product pipeline (Phase 17)

### 4.1 The pathway

```text
RESEARCH QUESTION → EXPERIMENT → PROOF OF CONCEPT → PROTOTYPE
→ ENGINEERING PROJECT → VALIDATION → PILOT → PRODUCT → RELEASE
```

### 4.2 The differentiator: provenance survives the transition

The strategic value is not the pathway — many tools model a lifecycle. It is that **the provenance
chain is not severed when research becomes engineering**.

```text
research claim (RESULT, with reproduction record)
   └─ derived_from → experiment
        └─ produced_by → prototype artifact
             └─ superseded_by → engineering project
                  └─ verified_by → factory job evidence
                       └─ released_as → release artifact
```

Because both halves already speak the same language — `EvidenceRef` into one graph, `Claim` for
epistemic status, `ArtifactRecord` for outputs — the chain is a matter of **linking**, not
translation. That is the architectural bet: **one evidence model, two workflows.**

### 4.3 The hard part, stated honestly

The hard part is not the data model. It is that research claims are often **superseded by
engineering reality** ("the prototype's assumption did not survive contact with production"). The
designed answer is that a superseded claim is marked `CONTRADICTED` with the contradicting evidence
attached — it is **not deleted**. A provenance chain that can be edited to remove inconvenient
history is not a provenance chain.

---

## 5. What is NOT designed here

* **Institutional deployment** of research mode (see `INSTITUTIONAL_MODE_ARCHITECTURE`).
* **Literature search / external source retrieval** — a connector problem, not a core-architecture
  problem, and out of scope for this wave.
* **Peer review workflows.**
* **Statistical analysis engines.** Research mode stores and reproduces; it does not (yet) compute.

---

## 6. Status summary

| Capability | Status |
|---|---|
| Research project object | **NOT_IMPLEMENTED** |
| Epistemic typing for research statements | **NOT_IMPLEMENTED** (claim model is the substrate) |
| Reproduction capture (4 missing fields) | **NOT_IMPLEMENTED** |
| "Reproduce Experiment N" | **NOT_IMPLEMENTED** |
| Research-to-product provenance linking | **NOT_IMPLEMENTED** |
| The reproducibility *pattern* it would extend | **IMPLEMENTED** (`first_mission.rs`) |
| Evidence substrate it would use | **IMPLEMENTED** (`claim`, `evidence_graph`, `artifact_bus`) |

**Recommended first vertical slice, when this is funded:** take the First Mission's reproducibility
package, add the four missing capture fields (dataset hash, parameters, seed, output hash), and make
one real experiment reproducible end to end. That is small, provable, and it validates the entire
design before any research UI exists.
