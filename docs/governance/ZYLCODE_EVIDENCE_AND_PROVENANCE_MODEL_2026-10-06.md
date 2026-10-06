# ZYLCODE EVIDENCE AND PROVENANCE MODEL

**Date:** 2026-10-06
**Status:** GOVERNING FOR THIS WAVE. Subordinate to `ZYLCODE_PROOF_GRAPH.md` (the capability rung
ladder) and `ZYLCODE_PRODUCT_CONSTITUTION_V2.md` (the evidence principle).
**Covers directive phases:** 8 (evidence engine), 10 (agent memory), and the provenance half of 7.
**Implemented substrate:** `crates/zylcode-core/src/{claim,evidence_graph,failure,proof_engine}.rs`,
`crates/zylcode-mcp/src/{evidence,actor}.rs`.

---

## 0. The one sentence this document exists to defend

> **A claim is not a fact until a deterministic component observed it and the observation is
> reproducible.**

Everything below is machinery for making that sentence true in code rather than in prose.

---

## 1. The three ladders — name yours, always

This repository has accumulated three distinct "R" concepts. Conflating them is a documented defect
(`ENGINEERING_TRUTH.md` §2.4.5). Every artifact must name its ladder.

| Ladder | Range | Answers | Where |
|---|---|---|---|
| **Capability rung** | `R0–R5` | "How mature is this *capability*?" | `ZYLCODE_PROOF_GRAPH.md` |
| **Claim ladder** | `R0–R7` | "How strongly is this *statement* established?" | `claim::VerificationLevel` |
| **Evidence rung** (tool catalogue) | `R0–R5` | "How far has this *tool* been exercised?" | `tool_catalogue::EvidenceRung` |

The claim ladder, in the order the fail-closed gate enforces:

```text
R0 model output → R1 structural validity → R2 build verified → R3 test verified
→ R4 integration verified → R5 environment verified → R6 production observed
→ R7 formally verified
```

`promote_verified()` refuses below **R2**. A model's output alone is **R0** and can never be
promoted on its own.

---

## 2. The Claim object (Phase 8)

### 2.1 Epistemic status

| Status | Requires | Meaning |
|---|---|---|
| `OBSERVED` | ≥1 evidence ref | A deterministic component saw it happen. |
| `DERIVED` | ≥1 evidence ref **and** a stated note | Follows from cited premises. |
| `VERIFIED` | ≥1 evidence ref **and** level ≥ R2 | Established, fail-closed. |
| `HYPOTHESIS` | — | A reasoned prediction with no confirming evidence. |
| `UNVERIFIED` | — | Asserted with no evidence either way. **The default.** |
| `CONTRADICTED` | ≥1 evidence ref | Cited evidence conflicts with the statement. |
| `UNKNOWN` | — | Nothing has been established. |

### 2.2 The fail-closed rules, in code

```text
Claim::new(...)                      → always starts UNVERIFIED
promote_verified()  with no evidence → REFUSED, status unchanged
promote_verified()  below R2         → REFUSED, status unchanged
promote_verified()  if CONTRADICTED  → REFUSED
validate()          before persist   → refuses a hand-built VERIFIED without evidence
```

**Why `Claim::new` cannot start at `VERIFIED`:** the ordering of the *word* and the *evidence* is the
entire product thesis. A constructor that could mint a verified claim would make the gate advisory.

**Confidence is never evidence.** `set_confidence` clamps to `0.0..=1.0` and cannot unlock
promotion. A confidence value that was not measured is a defect, not a placeholder.

### 2.3 Durable, hash-chained

`ClaimStore` persists to `.zylcode/claims.json` with an atomic write and a **hash chain**
(`prev_hash` → `entry_hash`). `verify_chain()` recomputes every hash and linkage; a tampered or
corrupted file returns `false`, and a **corrupt file is an error, not a silent empty state**.

---

## 3. The Evidence Graph (Phase 8)

### 3.1 The chain

```text
INTENT → SPECIFICATION → PLAN → DECISION → ACTION → TOOL_CALL → RESULT
       → ARTIFACT → VERIFICATION → CLAIM
```

### 3.2 Node provenance (directive Phase 8 fields)

Every node carries, where applicable: `actor`, `model`, `tool`, `inputs`, `outputs`,
`files_changed`, `environment`, `commit`, `verification`, `conclusion`, **`unknowns`**, and
`evidence_refs`.

**`unknowns` is a first-class field.** A graph that can only record what is known cannot be trusted
about what is not. Recording ignorance is part of the model, not a courtesy.

### 3.3 The graph links, it does not duplicate

`evidence_refs` point into the stores that already own the data:

| `EvidenceKind` | Points at |
|---|---|
| `LedgerEntry` | a hash-chained execution-ledger entry |
| `ProofRecord` | a deterministic proof record |
| `ToolEvidence` | a line in the MCP tool-evidence JSONL sink |
| `Artifact` | an Artifact Bus object |
| `Claim` | another claim (a derivation parent) |

This is deliberate: one fact, one home. The graph is the *traversal*, not a second copy that can
drift.

### 3.4 Integrity and traversal

* `add_node` refuses an empty summary or actor — "who initiated this?" is mandatory provenance.
* `add_edge` refuses unknown endpoints, self-loops and duplicates. **No dangling claims.**
* `verify_integrity()` recomputes the node hash chain and confirms every edge endpoint resolves.
* `ancestry(id)` walks back to the originating `INTENT`, with cycle detection.

**Provenance is only worth something if it is traversable.** An unlinked pile of records answers
"what happened?" but not "why, and on whose authority?".

### 3.5 Per-job scoping (this wave)

Each factory job writes its **own** evidence graph at
`.zylcode/factory/<job_id>.graph.json`. This was chosen over one global graph because a shared graph
would let a resume in one job mis-anchor its spine on another job's last node. The cost is that a
project-level roll-up is **DESIGNED, not implemented** — and the census says so.

---

## 4. The Failure object (Phase 8 / Phase 20)

Evidence that only records success cannot answer "what was attempted?".

| Field | Purpose |
|---|---|
| `operation` | what failed |
| `status` | `FAILED · BLOCKED · TIMED_OUT · CANCELLED · UNKNOWN · PARTIALLY_COMPLETED · RECOVERED` |
| `cause` | parsed from real output, or the stated refusal reason |
| `evidence_refs` | backing records |
| `affected_artifacts` | what was touched |
| `diagnosis` | root cause, recorded **before** recovery |
| `recovery_strategy` / `retry_count` | what was attempted, how many times |
| `resolution` | required for `RECOVERED` and `BLOCKED` |

**`RECOVERED` is fail-closed** and requires a diagnosis **and** ≥1 recovery attempt **and** cited
evidence **and** a resolution. A failure cannot even be *constructed* as `RECOVERED`.

---

## 5. Memory (Phase 10)

### 5.1 The six scopes the directive requires

| Scope | What it holds | Status |
|---|---|---|
| SESSION | execution ledger, checkpoints | **IMPLEMENTED** (`ledger.rs`, `sqlite_ledger.rs`) |
| PROJECT | project identity and history | **PARTIAL** (`project_store.rs`) |
| REPOSITORY | index, symbols, git graph | **IMPLEMENTED** (`intelligence/*`) |
| DECISION | ADRs an agent must consult | **NOT_IMPLEMENTED** |
| RESEARCH | research workspace state | **NOT_IMPLEMENTED** |
| ORG POLICY | allowed providers/tools, residency | **NOT_IMPLEMENTED** |

### 5.2 Decision memory — the designed model

The directive's example, made concrete:

```jsonc
{
  "id": "ADR-0042",
  "decision": "Use SQLite for local project state.",
  "status": "accepted",              // proposed | accepted | superseded | rejected
  "reason": "…",
  "evidence": [{ "kind": "proof_record", "id": "…" }],
  "approved_by": "…",
  "supersedes": null,
  "created_at": "…",
  "scope": { "project_id": "…" }
}
```

**Two rules that make it useful rather than decorative:**

1. A decision is **attributable** — an unattributed ADR is an opinion.
2. An agent proposing architecture that contradicts an `accepted` ADR must **cite the ADR and
   justify the contradiction**, or the proposal is incomplete. This is a *review* rule; enforcing it
   mechanically is **DESIGNED**.

**Not implemented.** Stated plainly so no reader infers otherwise.

### 5.3 What memory must never be

A memory store that silently upgrades a remembered statement into a fact. Memory entries are
**inputs to reasoning**, never evidence. A remembered claim is `UNVERIFIED` until re-evidenced.

---

## 6. Actor attribution

Every tool invocation is attributed (`ToolEvidence::actor`) or explicitly unidentified, and
**unidentified is treated as least-privilege**. A refusal is recorded as evidence whether or not the
tool ran — "a trail that only keeps successes cannot answer *what was attempted?*".

---

## 7. The truth surface — claim vs evidence, visibly

The product's identity claim is that it **distinguishes CLAIM from EVIDENCE**. The intended surface
groups statements by epistemic status (`PROVEN / DERIVED / UNKNOWN / HYPOTHESIS / BLOCKED`).

**Status: PARTIAL.** `surfaces::evidence_payload()` feeds the Evidence Center, but the grouping is
not implemented because the grouping it needs (the Claim object) only arrived recently. The data now
exists; the surface is **DESIGNED**.

---

## 8. What this wave added to the evidence model

**Nothing new was invented.** The wave *used* the existing model from the factory runner:

| Existing primitive | How the factory uses it |
|---|---|
| `EvidenceGraph::add_node` | one node per dispatched tool call, verification and result |
| `EvidenceGraph::add_edge` | links each node to the spine; the spine resumes across a restart |
| `Claim::promote_verified` | promotes a test claim **only** when a real exit code matched |
| `Claim::contradict` | records a failed expectation instead of dropping it |
| `Failure::new` | records every tool failure with a real cause |

The value added is **integration**: before this wave, a tool dispatched by the factory path wrote to
the JSONL sink but produced no traversable graph node and no claim. It now does both.

---

## 9. Non-negotiable prohibitions

Restating the directive's list, because each is easy to violate under delivery pressure:

* Do **not** silently use Ollama to declare `AGENT-01` passed.
* Do **not** fabricate model responses.
* Do **not** simulate agent success.
* Do **not** silently fall back between providers.
* Do **not** weaken provider acceptance criteria.
* Do **not** claim cloud commissioning without real evidence.
* Do **not** let an AI-generated statement become a research result by default (see
  `RESEARCH_MODE_ARCHITECTURE` §3).
