# TECHNICAL DOCUMENTATION — SOURCE STRUCTURE

**Status:** **STRUCTURE ONLY.** No final document has been generated, because it must be grounded
in runtime evidence that does not yet exist.

> **Every quantitative result in the final document must come from actual evidence.** A section
> with no evidence stays marked `NOT YET MEASURED` rather than being filled with an estimate.

---

## Sections

| # | Section | Evidence required | Current state |
|---|---|---|---|
| 1 | **Executive Summary** | — | writable now |
| 2 | **Problem** | — | writable now |
| 3 | **Why Developer Infrastructure** | — | writable now |
| 4 | **ZylCode × N-ATLAS Developer Bridge** | architecture | writable now |
| 5 | **Architecture** | `ARCHITECTURE.md`, source | writable now |
| 6 | **Genuine N-ATLAS Integration** | **a captured invocation** | ⛔ blocked |
| 7 | **Developer Toolkit** | SDK + playground | writable now |
| 8 | **First-Time Developer Journey** | `QUICKSTART.md`, a real walkthrough | writable now |
| 9 | **Multilingual Support** | measured results per language | ⛔ blocked |
| 10 | **Safety / Human Approval** | approval gate, permission model | writable now |
| 11 | **Evidence & Observability** | evidence schema, real records | partially blocked |
| 12 | **Testing** | test counts, commands, results | writable now |
| 13 | **External Beta Validation** | **real tester records** | ⛔ zero testers |
| 14 | **Limitations** | honest limits | writable now |
| 15 | **Reproducibility** | exact commands + environment | writable now |
| 16 | **Deployment** | — | writable now |
| 17 | **Open-Source Repository** | licence, contributing | writable now |
| 18 | **Future Work** | — | writable now |

---

## Sections that must not be written yet

| Section | Blocked on |
|---|---|
| 6 — Genuine N-ATLAS Integration | model access gate (`hf auth login` + terms) |
| 9 — Multilingual Support | genuine invocations in each language |
| 13 — External Beta Validation | two real external testers |
| 11 — Evidence & Observability (partially) | at least one real record |

**Do not draft these from expectation.** They are the sections a judge will scrutinise, and an
unsupported claim there is worse than an honest gap.

---

## Evidence sources the final document will cite

| Claim type | Source |
|---|---|
| test results | `cargo test` output, captured |
| model identity | server-reported, in evidence JSONL |
| latency | measured per invocation |
| licence | `NATLAS_RUNTIME_FEASIBILITY_2026-10-06.md` §3.5 |
| hardware | measured (`nvidia-smi`, `df`, CPU count) |
| beta validation | `BETA_TEST_EVIDENCE_TEMPLATE.md` rows |

---

## House style

- Distinguish **IMPLEMENTED / TESTED / RUNTIME_VERIFIED / BLOCKED / PROPOSED** everywhere.
- Never round a status up.
- State the boundary of every claim ("tested against a stub", "not yet measured").
- Include a reproduction block: commands, environment, observed output.
