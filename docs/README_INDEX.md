# Repository Documentation Index — read this before any other `.md`

**Status:** GOVERNING MAP
**Last swept:** 2026-09-16
**Governed by:** `ZYLCODE_AGENT_OPERATING_PROTOCOL.md` §4.7 (phase numbers) and §4.8 (superseded documents)

---

## 1. The problem this file solves

This repository contains **63 Markdown files at its root**. **15 are named `*_COMPLETE*` or
`*_SUMMARY*`**, and **23 assert a completion claim** in their body. They use a **third
phase-numbering scheme** that the governance set has already reconciled and retired, and **not one of
them carries a superseded banner**.

An agent that reads `PROJECT_COMPLETE.md` before reading `docs/governance/` will conclude the project
is finished and that Phases 3 and 4 passed. Both conclusions are false. The authoritative state is:

> **Phase 2A is NOT ACCEPTED — RE-OPENED. Phase 2B is BLOCKED on 2A. Phases 3A–16 are NOT STARTED.**

The root files are not lies about the past — they are **provenance**. They record what was believed
and attempted during an earlier, differently-numbered program. Under Protocol §4.8 they may be read
for rationale and never used as current capability evidence.

**This file exists so that the reading order is explicit rather than accidental.**

---

## 2. Reading order (mandatory)

```
1. docs/governance/README.md                        ← the governance index
2. docs/governance/ZYLCODE_PRODUCT_CONSTITUTION_V2.md
3. docs/governance/ZYLCODE_ARCHITECTURE_V2.md        ← §9 is the authoritative rung table
4. docs/roadmap/ZYLCODE_ROADMAP_V2.md                ← §2 is the authoritative status board
5. docs/capability-registry.json                     ← machine-readable, 12 entries DISPUTED
6. Everything else
```

**If any root-level `.md` disagrees with items 1–5, the governance document wins.** This is stated
in `README.md` and is restated here because the root files outnumber the governance set.

---

## 3. Root-level document classes

| Class | Count | May be used as evidence? | Action |
|---|---|---|---|
| `README.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`, `SUPPORT.md`, `LICENSE` | 6 | Yes — these are maintained | Keep |
| **Legacy status reports** (`*_COMPLETE.md`, `*_SUMMARY.md`, `PHASE*_*.md`, `PROJECT_COMPLETE.md`, `IMPLEMENTATION_STATUS.md`, `FINAL_*`, `INSTALLATION_*`) | ~20 | **No** — historical | Quarantined by §4 below |
| **Legacy design/research notes** (`*-spec.md`, `*-implementation.md`, `*-plan.md`, `research-*.md`, `findings*.md`, `blueprint.md`, `architecture.md`) | ~30 | **No** — provenance only | Quarantined by §4 below |
| **Shell/batch installers** (`*.bat`, `*.ps1`) | ~8 | Not documentation | Owner to decide (see §6) |

**Counts are approximate by class (they overlap) but exact where it matters:** 63 root `.md` files
total; 15 with `*_COMPLETE*`/`*_SUMMARY*` names; 23 asserting a completion claim in their body;
**0 carrying a superseded banner.**

---

## 4. Quarantine

The files below are **HISTORICAL**. They are not to be deleted — deleting them would falsify the
record — and they are not to be cited as current capability evidence.

### 4.1 Legacy status reports (assert completion under a retired numbering scheme)

```
PROJECT_COMPLETE.md                    PHASE1_CLOSURE_REPORT.md
IMPLEMENTATION_STATUS.md               PHASE1_COMPLETE.md
COMPLETE_PROJECT_BLUEPRINT.md          PHASE1_IMPLEMENTATION_COMPLETE.md
FINAL_SUMMARY.md                       PHASE2_COMPLETE.md
FINAL_WORKING_APPLICATION.md           PHASE2_IMPLEMENTATION_COMPLETE.md
FINAL_WORKING_SUMMARY.md               PHASE2_SUMMARY.md
INSTALLATION_GUIDE.md                  PHASE3_COMPLETE.md
INSTALLATION_SUCCESS.md                PHASE3_SUMMARY.md
INSTALLATION_SUMMARY.md                PHASE4_COMPLETE.md
DEVELOPER_GUIDE.md                     PHASE4_PROGRESS.md
USER_GUIDE.md                          PHASE4_SUMMARY.md
NAVIGATION_GUIDE.md                    QUICK_REFERENCE.md
```

**Why quarantined:** each asserts completion of a program whose phase numbers are not the governing
ones. `PROJECT_COMPLETE.md` claims the project is complete; `PHASE4_COMPLETE.md` claims "All Phase 4
Requirements Met". Neither is true under the governing roadmap.

### 4.2 Legacy design and research notes

```
ai-input-implementation.md   ai-input-spec.md          architecture.md
blueprint.md                 competitive-analysis.md   computer-use-implementation.md
computer-use-spec.md         deepseek-research.md      enhanced-mcp-bridge.md
file-upload-implementation.md file-upload-spec.md      findings-audit.md
findings.md                  implementation-checklist.md implementation-roadmap.md
mcp-bridge-spec.md           openai_codex_research_document.md
openai_codex_summary.md      phase1-summary.md         phase2-implementation-plan.md
phase2-plan.md               phase3-implementation-plan.md
phase4-implementation-plan.md plugin-marketplace-implementation.md
plugin-marketplace-spec.md   production-deployment-plan.md
project-summary.md           research-driven-strategy.md research-findings.md
skills-system-implementation.md skills-system-spec.md  strategic-plan.md
task_plan.md                 performance-optimization-plan.md
```

**Why quarantined:** these are the source of the "non-functional skeleton" hazards. For example
`computer-use-implementation.md` and `computer-use-spec.md` describe a Computer-Use Engine; the
implementation is a **simulation facade** (`ZYLCODE_ARCHITECTURE_V2.md` §9, R0). Reading the spec as
evidence of capability would repeat the exact error the Phase 2A audit caught.

**Note on `strategic-plan.md` (lowercase):** this is a **different file** from the quarantined
`docs/STRATEGIC_PLAN.md`, which already carries a SUPERSEDED banner. The lowercase root copy has no
banner and makes market-leadership claims. Both are historical; the governance set supersedes both.

---

## 5. The phase-numbering hazard, restated

Three schemes existed. Two are retired.

| Scheme | Where it appears | Status |
|---|---|---|
| **A — governing** | `docs/governance/`, `docs/roadmap/`, `README.md` | **AUTHORITATIVE** — 16 phases / 6 epochs |
| **B — root legacy reports** | `PHASE1_*.md` … `PHASE4_*.md`, `IMPLEMENTATION_STATUS.md` | **RETIRED** — do not use |
| **C — older source comments** | relabelled in a prior pass | **RETIRED** — do not use |

`PHASE_NUMBERING_RECONCILIATION.md` records the collision analysis. **A phase number belongs to the
governing roadmap and to nothing else** (Protocol §4.7). When a root file says "Phase 3", it means
something from scheme B — most likely part of what the governing roadmap calls Phase 1 or the
unstarted Phase 3A. Do not translate between them; read the governing document instead.

---

## 6. Open items for the owner

These are **decisions**, not defects I should resolve unilaterally:

1. **Root-file relocation.** The 63 root `.md` files should arguably move to `docs/history/` with a
   banner, leaving the root readable. That is a large, mechanical, history-altering change — an owner
   decision. Until then, this index is the mitigation.
2. **Installer scripts.** `install.bat`, `launch.bat`, `start-dev.bat`, `build-and-run.bat`,
   `diagnose.ps1`, `simple-diagnose.ps1`, `test-installation.ps1`, `test-simple.ps1`,
   `fix_clippy_defaults.ps1` are untracked. Undecided whether they are deliverables or scratch.
3. **Stray artifacts.** `vector_cache.db.bak` is untracked and appears to be a runtime backup; it
   should not be committed. `.dsh-vision-toolkit/` is untracked and of unknown provenance.
4. **`.gitignore` coverage.** `target/` and `node_modules/` are ignored for git but **not** for the
   repository-intelligence scanner — see the Phase 2A remediation item P0-1.

---

## 7. Maintenance

When a root-level file gains a proper home in `docs/`, or a new legacy file appears, update §3 and §4
in the same commit. This index is only useful if it is true.
