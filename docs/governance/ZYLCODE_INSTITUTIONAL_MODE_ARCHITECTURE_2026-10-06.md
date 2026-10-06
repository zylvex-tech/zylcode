# ZYLCODE INSTITUTIONAL MODE ARCHITECTURE

**Date:** 2026-10-06
**Status:** **DESIGNED. Nothing in this document is implemented.** See §0.
**Covers directive phases:** 15 (institutional / lab mode), 16 (education mode),
14 (private / local / sovereign mode).
**Substrate it would extend:** `zylcode_mcp::permission` (the policy chokepoint),
`router` (provider registry), `claim`/`evidence_graph` (the evidence model).

---

## 0. Honesty statement

There is **no institutional, education or provider-policy code in this repository**. Verified by
`grep -ri 'institution|education|classroom' crates/**/*.rs` → nothing.

What exists and is relevant:

| Existing piece | Relevance |
|---|---|
| `PermissionPolicy` (tool-scoped, per-runtime) | the *shape* an institutional policy would extend |
| Provider registry with `requires_api_key` | the list a provider policy would filter |
| `ToolEvidence::actor` | the attribution a multi-user mode would need to bind to a request |
| `claim` / `evidence_graph` | the evidence a supervisor review would inspect |

**The directive's own warning is taken literally:** *"Do not build enterprise theatre. Define the
actual needs first."* This document defines needs and stops.

---

## 1. The institutional requirement, stated as a design test

The directive supplies a test, not a customer:

> *Could ZylCode eventually support: a university software laboratory; a research group; an
> engineering software project; an AI/robotics research team; indigenous software development;
> student software engineering; faculty-led research; reproducible computational research;
> secure/private development; collaborative human/AI engineering; research-to-product translation?*

Mapped to architecture:

| Requirement | Needs | Status |
|---|---|---|
| university software laboratory | org/role model, shared projects | **NOT_IMPLEMENTED** |
| research group | research workspace | **NOT_IMPLEMENTED** |
| engineering software project | factory job | **IMPLEMENTED** |
| AI/robotics research team | reproducible research | **NOT_IMPLEMENTED** |
| indigenous software development | offline/local mode, low bandwidth | **PARTIAL** |
| student software engineering | education mode | **NOT_IMPLEMENTED** |
| faculty-led research | supervisor review | **NOT_IMPLEMENTED** |
| reproducible computational research | reproduction capture | **NOT_IMPLEMENTED** |
| secure/private development | provider policy, offline | **NOT_IMPLEMENTED** |
| collaborative human/AI engineering | HITL gates | **IMPLEMENTED** (single-user) |
| research-to-product translation | provenance linking | **NOT_IMPLEMENTED** |

**Three of eleven are implemented.** The institutional mode is genuinely early.

---

## 2. Institutional mode (Phase 15)

### 2.1 The object model

```text
Organisation
  ├─ Laboratories        (a lab is a scoped working environment)
  ├─ Courses             (a course is a lab with a curriculum and a cohort)
  ├─ ResearchGroups      (a group owns research projects)
  ├─ Projects            (engineering or research)
  ├─ Members → Roles     (student · researcher · supervisor · admin)
  ├─ Policies            (see §4)
  └─ ResourceLimits      (compute, tokens, storage, concurrency)
```

### 2.2 The four policy sentences the directive asks for

These are the acceptance criteria for institutional mode. Each must be expressible as **data**,
not prose:

| Institutional sentence | Policy object |
|---|---|
| "Students may use these providers and tools." | provider allow-list + tool allow-list, scoped to a role |
| "Research Group A must operate locally." | provider policy with `offline: true` / `residency: local` |
| "Production deployment requires supervisor approval." | risk gate: `Destructive`/`Release` ⇒ `RequireApproval`, approver role = supervisor |
| "This project must retain reproducibility evidence." | retention policy on the evidence graph + artifact bus |

**None of these exist as data.** The design is that they become fields on a policy object evaluated
at the **existing** permission chokepoint — the institutional layer must not introduce a second
enforcement point, because two enforcement points is the same as none.

### 2.3 What institutional mode must NOT become

* A dashboard of vanity metrics.
* A per-seat licence gate pretending to be a policy engine.
* A surveillance system (§3.4).

---

## 3. Education mode (Phase 16)

### 3.1 Teach, do not merely generate

The directive's modes, as a policy-controlled **assistance level**:

| Mode | Agent behaviour | Evidence produced |
|---|---|---|
| `EXPLAIN` | explains; produces no solution code | explanation artifact |
| `GUIDE` | asks leading questions; produces no solution code | dialogue + hints |
| `PAIR` | co-edits; the student drives | diff with co-author attribution |
| `ASSESS` | evaluates the student's attempt; produces no code | assessment record |
| `REVIEW` | critiques the student's work | review notes |
| `AUTONOMOUS` | solves; the default for non-education use | full factory evidence |

### 3.2 The rule that makes it education rather than cheating

> *A student should not automatically receive a finished solution when the institution wants guided
> learning.*

Architecturally this is a **capability restriction**, enforced at the same chokepoint as the
permission gate: in `EXPLAIN`/`GUIDE`/`ASSESS`, the `fs.write` executor is **refused for
solution paths**, and the refusal is recorded as evidence. The agent is not asked politely to
refrain; it is unable to comply with a request to write the answer.

**Why enforcement rather than instruction:** a prompt-level instruction is a suggestion. A refusal
at the tool boundary is a fact, and it is auditable.

### 3.3 Learning evidence

The directive's list, as data: `student_attempt`, `assistance_level`, `tests`, `feedback`,
`revision_history`, `learning_outcome`. Each is an artifact with an actor — which is why
`ToolEvidence::actor` matters: the record must distinguish what the **student** produced from what
the **agent** produced.

### 3.4 The prohibition

> *Do not implement surveillance-oriented behaviour.*

Concretely, this rules out: keystroke capture, always-on screen recording, idle-time scoring,
"productivity" ranking of students. The learning evidence above is **submitted work plus its
provenance**, which is what a supervisor would review anyway. Nothing beyond that.

---

## 4. Private / local / sovereign mode (Phase 14)

### 4.1 Four operating modes

```text
CLOUD AI  ·  LOCAL AI  ·  HYBRID AI  ·  NO-AI DETERMINISTIC TOOLING
```

The fourth is the important one and the most often forgotten: **ZylCode is useful with no model at
all.** The deterministic half — repository intelligence, the factory task graph, real tools,
evidence, build and test — runs without any provider. That is not a degraded mode; it is a
first-class mode, and it is the only mode that is fully working today.

### 4.2 The rule the directive states in capitals

> **LOCAL MODEL AVAILABILITY MUST NOT BE MISREPRESENTED AS CLOUD-GATE COMPLIANCE.**

> *Provider capability and governance are separate concerns.*

This is enforced in the architecture by keeping two things apart:

| Concept | Answers | Where |
|---|---|---|
| **Capability** | can this model do the task? | `model_capabilities.rs` |
| **Governance** | is this model *permitted* here? | a policy object (**not implemented**) |

A local model may be capable and permitted; it is still **not** evidence that a cloud provider was
commissioned. `AGENT-01` requires a real cloud provider turn; a local model can never close it.
This is stated in the evidence model and again here, because it is the single most likely place for
the project to accidentally lie to itself.

### 4.3 Provider policy (designed)

```jsonc
{
  "allowed_providers": ["local-ollama", "org-gateway"],
  "prohibited_providers": ["*"],
  "data_residency": "on-premises",
  "network_policy": "offline",
  "offline_mode": true,
  "model_capability_requirements": { "min_context": 32000, "tools": true },
  "logging_policy": "local-only",
  "retention_policy": { "evidence_days": 3650 }
}
```

Evaluated at provider selection and at the tool chokepoint. **Not implemented.**

---

## 5. Multi-user — the honest gap

Institutional mode implies multiple humans. Today ZylCode is a **single-user desktop application**:

* no authentication, no user model, no server-side tenancy;
* `ToolEvidence::actor` is a per-process task-local value, adequate for attributing an agent run,
  inadequate for attributing a request from one of forty students.

**Therefore institutional mode is blocked on a service architecture that does not exist.** Building
roles and policies on a single-user application would produce theatre — exactly what the directive
forbids. The correct order is: service identity → request-scoped actor → then roles and policies.

---

## 6. Status summary

| Capability | Status |
|---|---|
| Organisation / lab / course / research-group model | **NOT_IMPLEMENTED** |
| Members, roles, templates | **NOT_IMPLEMENTED** |
| Institutional policy objects (4 sentences above) | **NOT_IMPLEMENTED** |
| Resource limits | **NOT_IMPLEMENTED** |
| Supervisor review surface | **NOT_IMPLEMENTED** |
| Education assistance levels | **NOT_IMPLEMENTED** |
| Solution-write refusal in guided modes | **NOT_IMPLEMENTED** |
| Learning-evidence record | **NOT_IMPLEMENTED** |
| Provider policy (residency/offline/retention) | **NOT_IMPLEMENTED** |
| Offline / no-AI deterministic mode | **PARTIAL** — the deterministic half works; no policy layer |
| Multi-user identity | **NOT_IMPLEMENTED** |

**Recommended sequencing, when funded:** (1) request-scoped actor identity; (2) provider policy
object; (3) the four institutional policy sentences as data; (4) education assistance levels. Steps
1–2 are prerequisites for everything else and are each independently small.
