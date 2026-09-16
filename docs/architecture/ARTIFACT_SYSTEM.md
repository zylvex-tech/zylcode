# ARTIFACT_SYSTEM.md — ZylCode Artifact System

**Status: PROPOSED**
**Rung: R0 (CLAIMED)**
**Phase: 6A**
**Governing docs:** `ZYLCODE_PRODUCT_CONSTITUTION_V2.md` §3 · `ZYLCODE_ARCHITECTURE_V2.md` §4.3

> ⚠️ **This system does not exist.** There is no canonical artifact model.

---

## 1. Responsibility

Provide the canonical home for everything a human or the Proof Engine needs to **inspect**.

> An artifact is a **persistent Project object** — not a temporary file, not a log line, not a
> chat attachment.

---

## 2. Why Artifacts Precede Browser Automation

Phase 6A comes **before** Phase 6B (live preview) and Phase 7 (browser control). The reason is
ordering, not importance:

- Browser automation **produces** screenshots, traces, DOM dumps and network logs.
- Live preview **produces** renderable previews.
- Visual intelligence **produces** diffs.

If artifacts have no home when those phases start producing them, each phase invents its own
storage and its own lifetime rules — and the Proof Engine then cannot cite any of them uniformly.

**Define the container before the contents.**

---

## 3. Artifact Kinds

```
WEB_PREVIEW         COMPONENT_PREVIEW     DESIGN
DOCUMENT            DIAGRAM               IMAGE
DIFF                TEST_REPORT           BUILD_REPORT
SCREENSHOT          VIDEO                 DEVICE_SCREEN
DATABASE_VIEW       API_RESPONSE
```

Each kind has: a content type, a renderer, a lifetime policy, and a citation format.

---

## 4. Artifact Model

```
Artifact
├── id                  stable, unique
├── kind                one of §3
├── project_id          owning Project
├── mission_id?         producing Mission (if any)
├── step_id?            producing step (attribution)
├── producer            { kind: agent|runtime|user|engine, id }
├── created_at
├── updated_at
├── revision            monotonic; updates create revisions
├── content_ref         reference to bytes (never inline for large content)
├── content_type        MIME
├── size
├── provenance          Observed | Parsed | Inferred | Derived
├── citations[]         ledger entries this artifact supports
└── lifetime            ephemeral | mission | project | release
```

### 4.1 Attribution is mandatory

Every artifact records **which mission and step produced it**. An artifact with no producer is
not accepted — an unattributed artifact cannot be cited, and an uncitable artifact cannot be
evidence.

### 4.2 Revision, not mutation

Updating an artifact creates a **new revision**. Prior revisions remain. This is what allows a
"before / after" comparison in visual repair (Phase 9) and makes a proof citation stable even
after the artifact changes.

---

## 5. Four Actors

| Actor | May |
|---|---|
| **Agent** | create artifacts |
| **Runtime** | update artifacts (e.g. a preview refreshing) |
| **User** | inspect artifacts |
| **Proof Engine** | cite artifacts |

All four are first-class. A design that only lets agents create artifacts and only lets humans
read them will not support live previews.

---

## 6. Lifetime Policies

| Lifetime | Retained until | Examples |
|---|---|---|
| `ephemeral` | session end | transient DOM dumps |
| `mission` | mission archived | test reports, intermediate screenshots |
| `project` | project deleted | design documents, key diagrams |
| `release` | forever | release evidence bundles |

**Release artifacts are never garbage-collected.** A release whose evidence can disappear is not
auditable.

---

## 7. Storage

```
Artifact Store
├── metadata        (indexed, queryable — in the Project Store)
└── content         (content-addressed blob store, outside the Project Store)
```

**Content-addressed** so that identical content is stored once and a revision is a pointer swap.

The Project Store holds **metadata and references**, never bytes. This keeps Project state small
and migration fast.

---

## 8. Interfaces

```
zylcode artifact list [--kind K] [--mission M]
zylcode artifact open <id>
zylcode artifact export <id> --out <path>
zylcode artifact revisions <id>
```

Plus the UI Artifacts panel.

**Internal API**

```
ArtifactStore::create(kind, producer, content) -> ArtifactId
ArtifactStore::revise(id, content) -> Revision
ArtifactStore::cite(id, ledger_entry)
ArtifactStore::query(filter) -> Vec<Artifact>
```

---

## 9. Proof Integration

An artifact is citable **only if**:

1. it has a producer;
2. it resolves to a ledger entry;
3. its content hash is recorded.

```
Proof → cites → Artifact → resolves to → Ledger entry → contains → content hash
```

**A proof citing an artifact that does not resolve is invalid.** This is the mechanism that
prevents a completion report from asserting a result it never produced.

---

## 10. Benchmark / Acceptance

| # | Criterion | Method |
|---|---|---|
| 1 | An agent creates an artifact | transcript |
| 2 | A runtime updates it; a new revision exists and the old one is intact | transcript + revision list |
| 3 | The Proof Engine cites it; the citation resolves to a ledger entry | transcript |
| 4 | A citation of a non-existent artifact is rejected | negative test |
| 5 | Project export includes artifact metadata; bytes travel or are reported unavailable | export transcript |
| 6 | Release artifacts are never collected | GC test with output |

**Rung target: R3.**

---

## 11. Anti-Requirements

- Do not store artifact bytes in the Project Store.
- Do not mutate artifacts in place — revise.
- Do not accept an artifact without a producer.
- Do not allow a proof to cite an unresolvable artifact.
- Do not garbage-collect release artifacts.
- Do not let each phase invent its own storage.
