# ZylCode × N-ATLAS — Competition Workspace (Phase C0)

**Competition:** N-ATLAS / National AI Innovation Challenge
**Track:** Innovation & Enterprise
**Problem area:** Developer Infrastructure
**Branch:** `competition/natlas-2026`
**Status of this phase:** scaffold and discovery complete; **runtime integration is blocked**.

---

## Read this first

> **N-ATLAS has never been contacted.** No endpoint, credential, model identifier or response
> shape is known to this project. Every "success" in the test suite comes from a **test double** and
> is *not* competition evidence. Runtime integration status is
> **`BLOCKED_NATLAS_ACCESS`** and stays that way until the real interface is available.

If a reader takes one thing from this directory, it should be that sentence.

---

## What this directory contains

| Document | What it is |
|---|---|
| `NATLAS_INTEGRATION_DISCOVERY_2026-10-06.md` | Where N-ATLAS plugs in, why that seam, and what is unknown |
| `NATLAS_COMPETITION_ARCHITECTURE_2026-10-06.md` | The competition architecture and its honest status |
| `NATLAS_VALIDATION_PROTOCOL_2026-10-06.md` | How a claim of "it works" will be validated |
| `NATLAS_EVIDENCE_CHECKLIST_2026-10-06.md` | The evidence a submission must carry |
| `BETA_TEST_EVIDENCE_TEMPLATE.md` | Empty template for real external testers (**not populated**) |

---

## Status vocabulary

| Word | Means |
|---|---|
| `IMPLEMENTED` | Code exists on this branch. |
| `TESTED` | Exercised by a committed test that ran. |
| `RUNTIME_VERIFIED` | Exercised against the **real** N-ATLAS service, with the outcome captured. |
| `BLOCKED` | Cannot be reached for a stated reason. Never a synonym for "not done". |
| `PROPOSED` | Specified well enough to build. Nothing has been built. |

**`TESTED` is not `RUNTIME_VERIFIED`.** No artefact in this directory claims the latter, because the
service has never been reached.

---

## Phase C0 at a glance

| Element | State |
|---|---|
| Isolated branch `competition/natlas-2026` | `IMPLEMENTED` |
| Integration discovery | `IMPLEMENTED` (this set of documents) |
| Configuration (env-only, no invented defaults) | `IMPLEMENTED` · `TESTED` |
| `NatlasTransport` boundary trait | `IMPLEMENTED` |
| Client: invoke → parse → evidence | `IMPLEMENTED` · `TESTED` (test double) |
| Evidence record + secret redaction | `IMPLEMENTED` · `TESTED` |
| Intent → factory task-graph handoff | `IMPLEMENTED` · `TESTED` |
| **Real N-ATLAS transport** | **`BLOCKED_NATLAS_ACCESS`** |
| End-to-end competition slice | `PROPOSED` (Phase C1) |
| External beta validation | `PROPOSED` — no testers yet |

---

## The competition proposition, stated honestly

> Use Nigeria's N-ATLAS AI capability as a genuine intelligence component inside an AI-native
> software-engineering workflow.

The workflow around N-ATLAS is already real: repository intelligence, a durable task graph, gated
tool execution, tests, and an evidence graph that traces every step back to the intent. What does
not exist yet is the N-ATLAS call itself. The whole competition risk therefore sits in one known
place — the transport — rather than being spread across the architecture.

---

## Configuration (for Phase C1, once access exists)

All values are supplied externally. There are **no defaults**, because every one of them is an
N-ATLAS interface detail this project does not know.

| Variable | Required | Notes |
|---|---|---|
| `NATLAS_BASE_URL` | yes | host of the service |
| `NATLAS_REQUEST_PATH` | yes | deliberately not defaulted |
| `NATLAS_MODEL` | yes | never guessed |
| `NATLAS_API_KEY` | yes | never logged, never committed |
| `NATLAS_TIMEOUT_MS` | no | our own wait bound (default 60 000) |

Missing any required variable yields an explicit **not-configured** state, not a fallback.

---

## Hard rules for anyone continuing this work

1. Do not invent endpoints, auth formats, model ids or response shapes.
2. Do not route N-ATLAS through the product router's synthetic-degradation path
   (see discovery §1.3).
3. Do not present a mock-driven run as N-ATLAS evidence.
4. Do not push, merge to `main`, or touch other repositories.
5. Do not populate the beta-test template with fabricated testers.
