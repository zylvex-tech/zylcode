# ZylCode Capability Model

> How capabilities are named, classified, claimed, and proven.

**Status:** GOVERNING
**Companions:** `ZYLCODE_PROOF_GRAPH.md`, `docs/capability-registry.json`

---

## 1. Purpose

The capability registry is a **public artifact**. It is what the product claims to be able to do.
It must therefore be held to the same standard as the code.

This document defines:

1. what a capability is;
2. how capabilities are named and scoped;
3. what the status vocabulary means;
4. how a capability's status is computed;
5. how the registry is audited.

---

## 2. What a Capability Is

> **A capability is something a user or an agent can do, through a named entry point, whose
> result can be evidenced.**

This definition has three parts, all required:

| Part | Meaning | Failure mode if absent |
|---|---|---|
| **Actor** | A user or agent | A library nobody calls |
| **Entry point** | Named, committed, reachable | An unreachable module |
| **Evidence** | Reproducible artifact | An unverifiable claim |

**A module is not a capability.** `crates/zylcode-core/src/intelligence/` is a module. "An agent
can ask which files must change to alter crash recovery, and get a ranked answer with reasons"
is a capability — and it does not currently exist, because nothing invokes the module.

---

## 3. Naming and Scoping Rules

### 3.1 Names describe an outcome, not an implementation

| ❌ Bad | ✅ Good |
|---|---|
| `repository_scanner` | `repository_indexing` |
| `rust_symbol_index` | `symbol_lookup` |
| `context_retrieval` | `task_context_assembly` |

The name should survive a rewrite of the implementation.

### 3.2 Every metric names and scopes what it counts

> This rule exists because "Files indexed: 14,973" counted build artifacts and was published as
> evidence of repository intelligence.

- A count must state its **scope**: "files indexed (repository source, excluding build output
  and VCS internals)".
- A count that is dominated by something other than what its name implies **must not be
  published as evidence**.
- Where a number is reported, the command that produced it must be recorded alongside it.

### 3.3 One capability, one entry point minimum

A capability with multiple surfaces (CLI + UI + agent tool) lists them all. A capability with
zero surfaces is not a capability.

---

## 4. Status Vocabulary

Status and rung are **independent axes**. Status describes *how complete*; rung describes *how
proven*. Both are required.

### 4.1 Status (completeness)

| Status | Meaning |
|---|---|
| **ABSENT** | Not implemented. |
| **STUB** | Surface exists; behaviour is mocked or returns a placeholder. |
| **PARTIAL** | Some real behaviour; significant scope missing or unreachable. |
| **FUNCTIONAL** | Works for its stated scope; may be unproven. |
| **GREEN** | Complete for its stated scope **and** at rung **R3 or above**. |
| **EXTERNAL_BLOCKER** | Complete locally; blocked by an external system (e.g. CI billing). |

### 4.2 The GREEN rule

> **GREEN requires R3 or above. No exceptions.**

`GREEN` is the only status that implies "this works and you can use it". It may not be granted on
the basis of:

- passing unit tests (R2);
- a module existing;
- a report asserting it;
- a benchmark that does not reproduce.

This rule alone would have prevented the Phase 2A mis-classification.

### 4.3 EXTERNAL_BLOCKER is not a free pass

`EXTERNAL_BLOCKER` means "we cannot verify this through the blocked channel". It does **not**
mean "unverified but assumed fine".

When CI is unavailable, **local reproduction becomes the only evidence** — so the R3/R4 bar
applies with *more* force, not less. Phase 2A used `REMOTE_CI = BLOCKED_EXTERNAL` in a report
that also contained a non-reproducing benchmark. That combination is exactly what this rule
prohibits.

---

## 5. Registry Schema

Each entry:

```json
{
  "<capability_id>": {
    "status": "GREEN | FUNCTIONAL | PARTIAL | STUB | ABSENT | EXTERNAL_BLOCKER",
    "rung": "R0 | R1 | R2 | R3 | R4 | R5",
    "entry_points": ["zylcode repo context <task>", "AgentLoop::assemble_context"],
    "scope": "what this does and explicitly does not do",
    "evidence": "what was measured, with the command that measured it",
    "evidence_commit": "<sha>",
    "verified_at": "<iso-8601>",
    "limitations": ["..."],
    "depends_on": ["<capability_id>"]
  }
}
```

### 5.1 Required fields and why

| Field | Why it exists |
|---|---|
| `rung` | Makes the proof claim explicit and auditable |
| `entry_points` | Enforces the reachability test (R3) |
| `scope` | Prevents a narrow capability being read as a broad one |
| `evidence_commit` | A claim is about a specific revision, not "the project" |
| `verified_at` | Proofs go stale; this makes staleness detectable |
| `limitations` | Honest boundary. An entry with no limitations is suspicious. |

### 5.2 Deprecations

Removed capabilities are marked `"status": "REMOVED"` with a `superseded_by` field, never deleted.
The registry has history.

---

## 6. Computing a Status

```
1. Does a named entry point exist and is it committed?      no → ABSENT or STUB
2. Is it reachable by a user or agent?                      no → PARTIAL (max rung R2)
3. Does it work for its stated scope?                       no → PARTIAL
4. Does it have a reproduction block at R3+?                no → FUNCTIONAL (max rung R2)
5. Has it been independently audited (R5)?                  no → GREEN (rung R3/R4)
                                                            yes → GREEN (rung R5)
```

**Status is computed from evidence, never asserted by hand.** A hand-asserted status is R0.

---

## 7. Capability Domains

Capabilities are grouped by the eight core systems plus the trust foundation.

| Domain | Examples |
|---|---|
| **Trust foundation** | evidence ledger, permission enforcement, crash recovery, audit |
| **Agent Kernel** | tool runtime, reasoning loop, approval, bounded execution, memory |
| **Intelligence Graph** | indexing, symbol lookup, dependency graph, change graph, task context |
| **Project System** | project identity, persistence, migration, project knowledge graph |
| **Vision Studio** | design model, canvas, design↔code, visual diff |
| **Execution Engine** | shell, browser, desktop, container, android, ios-worker |
| **Proof Engine** | build/test/runtime/visual/security verification, proof graph |
| **Delivery Engine** | git, CI, packaging, deployment, store publishing |
| **Model Platform** | provider config, capability routing, measured performance |
| **Extension Platform** | package ABI, permissions, contribution points |
| **Computer-Use Engine** | screen/window perception, element grounding, input synthesis, risk levels, flight recorder |

---

## 8. Audit Procedure

At every gate G3:

1. **Sample** capabilities — at minimum, every capability claimed GREEN, and every capability
   touched by the phase under audit.
2. **Attempt reproduction** of each `evidence` claim from its recorded command.
3. **Verify reachability** — invoke each listed entry point.
4. **Verify scope** — check the claim is not broader than the behaviour.
5. **Attempt rung downgrades** — actively try to falsify the rung. A rung that survives a
   genuine falsification attempt is worth something.
6. **Recompute status** using §6.
7. **Record the audit** — including capabilities that passed. Audits have history too.

### 8.1 Falsification is the default stance

The auditor's job is not to confirm. It is to try to break the claim. Phase 2A's benchmark was
accepted because it was never re-run; the audit's first action was to re-run it, and it failed.

---

## 9. Registry Maintenance Rules

1. **Update the registry in the same commit as the code it describes.** A registry that lags the
   code is worse than no registry.
2. **Never upgrade a status without evidence.** Downgrades need evidence too, but they are
   always safe.
3. **Corrections are visible.** When a published number was wrong, the correction is recorded as
   a correction (Constitution §3.2 rule 5).
4. **`evidence` must name its scope and its command.**
5. **`limitations` is mandatory and must be non-empty** for any capability that is not fully
   general.

---

## 10. Current State

`docs/capability-registry.json` currently claims **35 capabilities: 33 GREEN, 1 PARTIAL,
1 EXTERNAL_BLOCKER**.

That summary is **not accepted**. Specifically:

- The 12 capabilities added in Phase 2A are recorded GREEN on the basis of unit tests and a
  benchmark that does not reproduce. Under §4.2 they are **at most PARTIAL / R2**.
- `repository_scanner`'s evidence string embeds the mis-scoped "14,973 files" figure.

**Required action:** the registry is corrected as part of Phase 2A remediation (P0.2 / P1.6),
with the corrections visible rather than silent.

The registry will be re-based on the `rung` field, so that every future status is computed
rather than asserted.

---

## 11. Summary

| Concept | Rule |
|---|---|
| Capability | actor + entry point + evidence |
| Module | not a capability |
| GREEN | requires R3+ |
| Rung | computed, never asserted |
| Metric | names and scopes what it counts |
| EXTERNAL_BLOCKER | raises the local bar, does not lower it |
| Audit | falsify first, confirm second |
| Correction | visible, never silent |
