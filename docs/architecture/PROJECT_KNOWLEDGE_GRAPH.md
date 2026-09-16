# PROJECT_KNOWLEDGE_GRAPH.md — ZylCode Project Knowledge Graph

**Status: PROPOSED**
**Rung: R0 (CLAIMED)**
**Phase: 3A (delivered alongside the Project System)**
**Governing docs:** `ZYLCODE_PRODUCT_CONSTITUTION_V2.md` §4 · `PROJECT_SYSTEM.md` · `INTELLIGENCE_GRAPH.md`

> ⚠️ **This system does not exist.**
> It is specified because it is a long-term differentiator and because the Project System is the
> natural place to build it — not because any part of it has been implemented.

---

## 1. The Idea

There is a capability gap between ordinary memory and repository intelligence:

| Layer | Knows | Blind to |
|---|---|---|
| **Engineering memory** | what happened in a session | why the code is the way it is |
| **Repository intelligence** | what the code *says* | what humans *decided*, what agents *learned*, what runtime *proved* |
| **Project Knowledge Graph** | **all four** | — |

Every Project develops an evolving knowledge graph in which a single statement is connected to
its decision, its implementation, its tests, its origin commit, and its verification:

```
"Authentication uses JWT"
        │
        ├── decided in      ADR-004
        ├── implemented by  auth.rs
        ├── tested by       auth_test.rs
        ├── introduced in   commit abc123
        └── verified by     run E-9237
```

It combines four normally-separate sources:

> **what the code says + what humans decided + what agents learned + what runtime evidence proved**

---

## 2. Why This Is a Differentiator

Every serious code-intelligence product answers *"where is X defined?"*

Very few answer *"why is X like this, who decided, what depends on that decision, and was it ever
proven to work?"*

The second question is the one that matters when:

- an agent must decide whether changing X is safe;
- a new engineer must understand a design choice made two years ago;
- a system must explain *why* it is about to modify something;
- a decision must be revisited because its original justification no longer holds.

**The four-source join is the differentiator.** Each source alone is a commodity.

---

## 3. The Four Sources

| Source | Origin | Provenance |
|---|---|---|
| **Code facts** | Intelligence Graph | `Parsed` / `Observed` |
| **Human decisions** | ADRs, PR discussions, mission approvals | `Decided` |
| **Agent learnings** | mission outcomes, repair histories, failures | `Learned` |
| **Runtime evidence** | Evidence Ledger | `Verified` |

**Every knowledge node records which source it came from.** A statement sourced from code is not
the same kind of claim as a statement sourced from a decision, and conflating them is how systems
become confidently wrong.

---

## 4. Node and Edge Model

### 4.1 Nodes

```
KnowledgeNode
├── id
├── statement        human-readable claim, e.g. "Authentication uses JWT"
├── subject          what the claim is about (symbol / package / file / concept)
├── provenance       Parsed | Decided | Learned | Verified
├── confidence       derived from provenance + corroboration
├── created_at
├── source_ref       link to the originating record
└── supersedes?      prior node if this revises one
```

### 4.2 Edges

| Edge | Meaning |
|---|---|
| `decided_in` | links to an ADR or approval |
| `implemented_by` | links to a symbol/file |
| `tested_by` | links to a test |
| `introduced_in` | links to a commit |
| `verified_by` | links to a ledger entry |
| `supersedes` | this node replaces an earlier one |
| `contradicts` | two nodes disagree — **must be surfaced, not hidden** |
| `depends_on` | knowledge-level dependency |

### 4.3 The `contradicts` edge is essential

When the code says one thing and a decision record says another, that is exactly the signal a
human needs. The graph must **surface contradictions**, not resolve them silently.

> A knowledge graph that hides contradictions is a more confident version of an unreliable one.

---

## 5. Construction

Knowledge is accumulated, never bulk-imported.

| Trigger | What is learned |
|---|---|
| Mission completes | the objective, the acceptance criteria, the proofs |
| ADR recorded | the decision, its rationale, the alternatives rejected |
| Repair loop succeeds | the failure signature and its fix |
| Verification runs | what was proven, and at which rung |
| Human correction | an explicit override, highest confidence |
| Contradiction detected | a `contradicts` edge, surfaced for review |

**Corrections are append-only.** A superseded node is retained with `supersedes`/`superseded_by`
links. The graph has history, like the Ledger.

---

## 6. Consumption

### 6.1 By agents

Before changing code, an agent can ask:

```
knowledge.about(symbol)      → what is known, from all four sources
knowledge.why(symbol)        → the decisions behind it
knowledge.risks(symbol)      → contradictions and unverified claims
knowledge.unverified(scope)  → claims that were never proven
```

This is what turns context assembly from *"here are the relevant files"* into *"here is the
relevant context and here is why it is relevant"* — extending the `ContextResult.reason` pattern
already present in the Intelligence Graph.

### 6.2 By humans

- A symbol's history: decisions, changes, tests, proofs.
- A decision's reach: what it affected, what depends on it.
- Unverified claims: what the project believes but never proved.
- Contradictions: where the code and the record disagree.

### 6.3 By the Proof Engine

`unverified(scope)` is a direct input to proof planning — it names what still needs proving.

---

## 7. Interfaces

```
zylcode knowledge about <subject>
zylcode knowledge why <subject>
zylcode knowledge contradictions
zylcode knowledge unverified [--scope <scope>]
zylcode knowledge graph <subject> --format dot|json
```

Plus the UI Knowledge surface (a symbol's knowledge panel, and a project-wide contradiction view).

---

## 8. Benchmark / Acceptance

| # | Criterion | Method |
|---|---|---|
| 1 | A decision, its implementation, its test, its commit and its proof are joined in one query | transcript |
| 2 | Every node records its source | graph dump |
| 3 | A contradiction between code and a decision is **surfaced** | seeded contradiction test |
| 4 | Superseded nodes are retained with links | mutation + history test |
| 5 | `unverified(scope)` names claims never proven | transcript |
| 6 | The graph survives project export/import | two-machine transcript |
| 7 | An agent uses the graph to justify a change | captured agent run |

**Rung target: R3.**

---

## 9. Dependencies

| Needs | From |
|---|---|
| Project identity and persistence | Phase 3A |
| Code facts with provenance | Phase 2A/2B |
| Decisions and approvals | Phase 3A/3B |
| Ledger entries | Phase 1D |
| Proof nodes | Phase 12 |

**Do not attempt this before Phase 3A.** The graph has nowhere to live and nothing to join.

---

## 10. Anti-Requirements

- Do not hide contradictions; surface them.
- Do not conflate provenance — a code fact is not a decision.
- Do not bulk-import knowledge; accumulate it from real events.
- Do not delete superseded knowledge; link it.
- Do not present an unverified claim as established.
- Do not build this before the Project System exists.

---

## 11. Naming Reconciliation (v2.1)

Architecture v2.1 discussion referred to this system as the **Engineering Knowledge Graph**.

**These are the same system, and the canonical name is `Project Knowledge Graph`.**

The reconciliation is recorded rather than the rename performed, for a specific reason: the graph
is scoped to a **Project** — it joins facts about one product, its decisions, its agents and its
runtime evidence. "Engineering" describes a discipline; "Project" describes the ownership
boundary, which is what the scoping rule actually depends on. Renaming to the discipline would
weaken the one word in the name that carries architectural meaning.

If the term "Engineering Knowledge Graph" appears in a future proposal, it means this document.

**Do not create a second file under the other name.** Two documents describing one system is how
this package acquires the drift it exists to prevent.
