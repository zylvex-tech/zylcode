# ZylCode Governance

> **The governing documents. They govern implementation.**

**Ratified:** 2026-09-16
**Supersedes:** the six-engine / 14-stage drafts, archived in `superseded/`

---

## Start Here

| If you are… | Read, in order |
|---|---|
| **An implementation agent** | `ZYLCODE_AGENT_OPERATING_PROTOCOL.md` → `ZYLCODE_PRODUCT_CONSTITUTION_V2.md` → your phase spec |
| **New to the project** | `ZYLCODE_PRODUCT_CONSTITUTION_V2.md` → `ZYLCODE_ARCHITECTURE_V2.md` → `ZYLCODE_MASTER_EXECUTION_PLAN.md` |
| **Auditing a phase** | `PHASE2A_INDEPENDENT_AUDIT.md` (method) → `ZYLCODE_PROOF_GRAPH.md` → `ZYLCODE_CAPABILITY_MODEL.md` |
| **Planning work** | `ZYLCODE_MASTER_EXECUTION_PLAN.md` → `../roadmap/ZYLCODE_ROADMAP_V2.md` |

---

## The Mandatory Preamble

Every prompt to DeepSeek, Codex, ZCode, or any other implementation agent **must** begin with:

> Read the ZylCode Constitution, Architecture, Master Execution Plan, Capability Model and Proof
> Graph, plus the current phase specification. They govern implementation.
> Where implementation conflicts with documentation, **investigate the discrepancy rather than
> silently choosing one**.

---

## Document Map

### Governance (`docs/governance/`)

| Document | Governs |
|---|---|
| **`ZYLCODE_PRODUCT_CONSTITUTION_V2.md`** | The supreme specification. **Eight systems.** Evidence principle. Product definition. |
| **`ZYLCODE_ARCHITECTURE_V2.md`** | **v2.1.** System topology, contracts, data ownership, **honest implementation status**. |
| **`ZYLCODE_MASTER_EXECUTION_PLAN.md`** | The 16-phase / 6-epoch program. Gates. Dependency order. Phase 7 splits into 7A–7D. |
| **`ZYLCODE_CAPABILITY_MODEL.md`** | What a capability is, how status is computed, how the registry is audited. |
| **`ZYLCODE_PROOF_GRAPH.md`** | **R0–R5.** The definition of "proven". Governs every capability claim. |
| **`ZYLCODE_AGENT_OPERATING_PROTOCOL.md`** | How agents work here. Pre-flight, rules, report format, anti-patterns. |
| **`DEEPSEEK_MASTER_PROMPT_V21.md`** | The two-job work order: Phase 2A remediation, then Architecture v2.1 reconciliation. |
| **`PHASE2A_INDEPENDENT_AUDIT.md`** | The audit that re-opened Phase 2A. Also the worked method for future audits. |
| **`PHASE2A_REMEDIATION_ORDER.md`** | The operational work order for the Phase 2A fix. |

### Roadmap (`docs/roadmap/`)

| Document | Governs |
|---|---|
| **`ZYLCODE_ROADMAP_V2.md`** | Per-phase objective, deliverables, entry points, gates, benchmarks. |

### Architecture (`docs/architecture/`)

| Document | Status | Phase |
|---|---|---|
| `PROJECT_SYSTEM.md` | **PROPOSED** | 3A |
| `PROJECT_KNOWLEDGE_GRAPH.md` | **PROPOSED** | 3A |
| `AGENT_KERNEL.md` | **PARTIAL** (R3) | 1A–1D done |
| `INTELLIGENCE_GRAPH.md` | **PARTIAL** (R2) | 2A re-opened |
| `EXTENSION_PLATFORM.md` | **PROPOSED** | 5 |
| `ARTIFACT_SYSTEM.md` | **PROPOSED** | 6A |
| `VISION_STUDIO.md` | **PROPOSED** | 8A–9 |
| `EXECUTION_ENGINE.md` | **PARTIAL** (R3 shell / R0 devices) | 7A, 10, 11 |
| `COMPUTER_USE_ENGINE.md` | **PROPOSED — non-functional skeleton present** | 7B–7D |
| `PROOF_ENGINE.md` | **PROPOSED** | 12 |
| `DELIVERY_ENGINE.md` | **PARTIAL** (R1) | 13 |

> **PROPOSED means it does not exist.** Per Constitution §9.3, describing a system is not
> building it. Documentation must never imply otherwise.
>
> **"Non-functional skeleton present"** means code exists in the tree but performs no real work —
> it is labelled explicitly because an unlabelled skeleton in a status table is more dangerous
> than an absent feature. The Computer-Use Engine is the current case: see
> `../architecture/COMPUTER_USE_ENGINE.md`.

---

## The Five Laws

1. **The loop is the product.** Every capability is justified by an arc of
   *Imagine → Specify → Design → Build → Run → See → Test → Repair → Verify → Ship*.
2. **Evidence or it did not happen.** A claim is not a fact until it is independently reproducible.
3. **Builders do not certify their own work.**
4. **A Project is not a directory.** It is the persistent representation of a software product.
5. **Where implementation conflicts with documentation, investigate — never silently choose.**

---

## Current Status (2026-09-16)

| | |
|---|---|
| **Phase 1A–1D** | ✅ Accepted (1C, 1D with conditions) |
| **Phase 2A** | ❌ **NOT ACCEPTED — RE-OPENED** |
| **Phase 2B** | 🚫 **BLOCKED** on 2A |
| **Phases 3A–16** | ⏸ Not started |
| **Architecture** | v2.1 — eight systems; Phase 7 split into 7A–7D; 8–16 unrenumbered |

**Phase 2A was rejected** because: its headline metric counted `target/` build output (14,900 of
15,064 files; the repository is 254 files); its benchmark **fails** on independent re-execution;
it has **zero** product integration; and its "Acceptance Demonstration" was written as an
expectation rather than captured from a run.

See `PHASE2A_INDEPENDENT_AUDIT.md`.

**Separate finding, surfaced during v2.1 preparation:** the **Computer-Use Engine** exists in the
tree as a **simulation facade** — 31 simulation sites, all-zero capture buffers, `sleep`-based
input, hardcoded OCR text, and fabricated confidence values — with tests that pass because the
facade cannot fail. It is **not** a capability and must not be treated as one. See
`../architecture/COMPUTER_USE_ENGINE.md` and `ZYLCODE_ARCHITECTURE_V2.md` §9.

**Next action:** DeepSeek executes `DEEPSEEK_MASTER_PROMPT_V21.md` — Job 1 (Phase 2A remediation)
first, then Job 2 (Architecture v2.1 reconciliation) as a separate commit.

---

## Superseded

`superseded/` holds the earlier six-engine / 14-stage drafts. They were never committed and
contradicted the approved architecture. They are retained for history only.

**Do not implement from anything in `superseded/`.**
