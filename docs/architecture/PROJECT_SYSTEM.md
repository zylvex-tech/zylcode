# PROJECT_SYSTEM.md — ZylCode Project System

**Status: PROPOSED**
**Rung: R0 (CLAIMED)**
**Phase: 3A**
**Governing docs:** `ZYLCODE_PRODUCT_CONSTITUTION_V2.md` §4 · `ZYLCODE_ARCHITECTURE_V2.md` §3.1

> ⚠️ **This system does not exist.** It is specified here so that it can be built correctly.
> Per Constitution §9.3, describing a system is not building it.
> Today, "Project" exists only as a UI-level notion with no persistent identity.

---

## 1. The Critical Principle

> **A Project is not a directory.**
>
> A Project is ZylCode's persistent representation of a software product.

A directory is a filesystem location that can be moved, copied, renamed, or lost.
A Project is a durable identity that survives:

- application restart
- machine migration
- agent turnover
- model changes
- repository relocation
- team membership changes

Everything ZylCode knows about a product hangs off the Project.

---

## 2. Why This System Comes First

The Project System precedes every other stateful system. If it is built late, these must all be
rebuilt:

| System | What it would have to redo |
|---|---|
| Mission Engine | missions have no stable owner |
| Model Platform | routing history has no scope |
| Extension Platform | package state has nowhere to live |
| Artifact System | artifacts have no home |
| Vision Studio | design intent has nowhere to persist |
| Multi-Agent | agents have no shared world state |

This is the exact failure the Master Execution Plan exists to prevent: *reaching Stage 10 before
discovering the Project model was wrong.*

---

## 3. Creation Flow

```
New Project

Name                 ──►  stable identity (immutable slug + display name)
Repositories         ──►  one or many, with role (primary / dependency / reference)
Project type         ──►  web · mobile · desktop · library · service · multi-target
Targets              ──►  windows · macos · linux · web · android · ios
Design system        ──►  reference to a design library (Phase 8A)
Models               ──►  preferred providers + routing policy (Phase 4)
Runtime environments ──►  local · container · remote worker (Phase 11)
Deployment targets   ──►  destinations (Phase 13)
Secrets              ──►  references only — never values stored in the Project
Team                 ──►  members and roles
```

**Secrets rule:** the Project stores **references** to secrets, never secret material. Secret
storage is delegated to the platform keychain. This is a security invariant, not a preference.

---

## 4. Persisted State

| Domain | Contents |
|---|---|
| **Requirements** | intent, acceptance criteria, constraints |
| **Repositories** | paths, remotes, roles, index references |
| **Architecture** | records, decisions (ADRs), diagrams |
| **Design** | `DesignDocument` references (Phase 8A) |
| **Conversations** | threads, scoped to project |
| **Missions** | missions, states, criteria, proofs (Phase 3B) |
| **Agents** | agent definitions, runs, budgets |
| **Decisions** | ADRs, routing decisions, permission grants |
| **Evidence** | references into the Evidence Ledger |
| **Artifacts** | artifact records (Phase 6A) |
| **Runtimes** | environments, capability advertisements |
| **Model config** | providers, policies, measured performance |
| **Build config** | targets, commands, outputs |
| **Releases** | release graph, evidence bundles |
| **Deployment history** | what shipped, where, when, with what proof |

---

## 5. Architecture

```
┌──────────────────────────────────────────────────────────┐
│                     PROJECT STORE                        │
│                                                          │
│  ┌────────────┐  ┌────────────┐  ┌───────────────────┐   │
│  │ Identity   │  │ Graph      │  │ Knowledge Graph   │   │
│  │ (stable)   │  │ (structure)│  │ (meaning)         │   │
│  └────────────┘  └────────────┘  └───────────────────┘   │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │  References (never copies)                         │  │
│  │  → repositories  → artifacts  → evidence           │  │
│  │  → designs       → missions   → releases           │  │
│  └────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────┘
          │                                   ▲
          │  read / write                     │  append
          ▼                                   │
  ┌──────────────────┐            ┌───────────────────────┐
  │ Engines          │            │ EVIDENCE LEDGER       │
  │ (Agent, Vision,  │            │ (append-only, separate│
  │  Execution,      │            │  from Project state)  │
  │  Proof, Delivery)│            └───────────────────────┘
  └──────────────────┘
```

### 5.1 Ownership rule

**One writer per fact.** The Project Store owns project identity and structure. It does **not**
own:

- repository contents (filesystem)
- repository structure (Intelligence Graph)
- proof (Evidence Ledger)
- artifacts' bytes (Artifact Bus)

It owns **references** to all of those. Duplicated truth is the most expensive mistake available.

---

## 6. Requirements

### R1 — Durable identity

A Project has an identity independent of any filesystem path. Moving a repository does not
create a new Project.

### R2 — Restart survival

Close the application, reopen it: the Project is intact, including in-flight Mission state.

### R3 — Machine migration

Export a Project on machine A, import on machine B, and the Project is intact:

- structure and identity
- missions and their states
- knowledge graph
- **references** re-resolve to local paths (with a resolution step and explicit reporting of
  anything that cannot be resolved)

Evidence and artifact **bytes** travel in the export bundle or are reported as unavailable —
never silently omitted.

### R4 — Concurrency

Project state mutations are serialised per project. Concurrent Missions in one Project must not
corrupt shared state.

### R5 — Versioned schema

The Project store carries a schema version. Migration is explicit, forward-only, and reversible
via backup. An unreadable future version fails closed with a clear message.

---

## 7. Interfaces

### 7.1 Entry points (required for R3)

```
zylcode project create
zylcode project open <id>
zylcode project list
zylcode project export <id> --out <path>
zylcode project import <path>
zylcode project status <id>
```

Plus the UI Project surface (creation flow, project switcher, project settings).

### 7.2 Internal API

```
ProjectStore::create(spec) -> ProjectId
ProjectStore::load(id) -> Project
ProjectStore::mutate(id, mutation) -> Result
ProjectStore::export(id) -> Bundle
ProjectStore::import(bundle) -> ProjectId
Project::missions() / artifacts() / evidence_refs() / knowledge()
```

Every mutation is recorded as a Project **Decision** entry so the project's history is
inspectable.

---

## 8. Benchmark / Acceptance

| # | Criterion | Method |
|---|---|---|
| 1 | Create a project through the UI | captured transcript |
| 2 | Restart the application; project intact | transcript before/after |
| 3 | Export on machine A, import on machine B | two-machine transcript |
| 4 | Unresolvable references reported explicitly | transcript showing the report |
| 5 | Two concurrent missions do not corrupt state | concurrency test with output |
| 6 | Schema version mismatch fails closed | test with a future-versioned store |
| 7 | Knowledge graph survives migration | graph dump before/after |

**Rung target: R3.** All seven require captured evidence.

---

## 9. Anti-Requirements

The Project System must **not**:

- execute code
- reason or plan
- render UI
- store secret values
- duplicate repository structure (that is the Intelligence Graph)
- duplicate proof (that is the Ledger)
- store artifact bytes (that is the Artifact Bus)

Every one of these is a tempting shortcut and every one of them creates a second source of truth.

---

## 10. Related Documents

- `PROJECT_KNOWLEDGE_GRAPH.md` — the meaning layer built on this system
- `AGENT_KERNEL.md` — the primary reader/writer
- `ARTIFACT_SYSTEM.md` — artifact records referenced here
- `ZYLCODE_PROOF_GRAPH.md` — proof references
