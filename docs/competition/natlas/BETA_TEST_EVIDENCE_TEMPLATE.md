# N-ATLAS EXTERNAL BETA TEST — EVIDENCE LOG (TEMPLATE)

**Status:** **EMPTY. No testers have run this. No row below is populated.**

> **Do not fabricate testers.** This file is deliberately empty. An empty beta log is an honest
> artefact; a populated one that did not happen is fraud, and it would also destroy the only claim
> the competition actually rewards — that the system was validated by real people.
>
> Real testers will be added here, one block each, after they have actually run the workflow. Until
> then the tester count is zero and any statement of external validation is false.

---

## How to record a test

1. A tester is a **real person** who has consented to being named. Use their name or an agreed
   identifier.
2. Record whether the tester is **external** or a **core-team member**. Core-team members (the
   owner and Ibrahim Abdulrahman) **must never be counted as external testers**.
3. The tester works through the ten tests (A–J) in
   [developer/BETA_TEST_GUIDE.md](developer/BETA_TEST_GUIDE.md).
4. The N-ATLAS invocation evidence for each attempt is preserved using **non-secret fields only**
   (`NatlasEvidence`: timestamp, provider, model, request id, classification, status, latency,
   success, redacted error). **Never record an API key or secret.**
5. Record the outcome **as observed** — including partial success and failure. A failed task
   reported honestly is worth more than a success that is not.
6. Time is **measured**, not estimated.
7. Feedback is recorded **verbatim**, not summarised into something more flattering.
8. An issue is recorded with its resolution, or explicitly left unresolved.

---

## Tester record

_Copy this block once per tester. **Leave empty until a real tester runs it.**_

### Tester `T-___`

| Field | Value |
|---|---|
| Name or agreed identifier | |
| Consent to be named / anonymised use of evidence | |
| **External or core team** | |
| Technical background | |
| Location / timezone | |
| Operating system | |
| CPU / system RAM | |
| GPU / VRAM | |
| Runtime (Ollama version, model tag) | |
| Repository used | |
| Repository size | |
| Language(s) covered | |
| Session start (ISO 8601) | |
| Session end (ISO 8601) | |
| Total elapsed (measured) | |

#### Test results

| Test | What it checks | Outcome | Elapsed | N-ATLAS evidence id | Verification result | Notes |
|---|---|---|---|---|---|---|
| **A** | Installation / onboarding | | | | | |
| **B** | Connection — is the runtime genuinely N-ATLAS? | | | | | |
| **C** | Repository workflow | | | | | |
| **D** | Developer task | | | | | |
| **E** | N-ATLAS interpretation → structured intent | | | | | |
| **F** | Approval gate understood and exercised | | | | | |
| **G** | Controlled execution | | | | | |
| **H** | Verification — actual result exposed | | | | | |
| **I** | Evidence record located | | | | | |
| **J** | Usability — explain success/failure/confusion | | | | | |

#### Exact task and prompt

| Field | Value |
|---|---|
| Exact instruction given (verbatim) | |
| Language of the instruction | |
| Repository context sent (paths) | |
| Raw model response location | |
| Parsed intent result | |
| Was the intent contract satisfied? | |
| Operation proposed | |
| Operation approved / rejected | |
| Observed language quality | |
| Failure / limitations observed | |

#### Feedback

| Field | Value |
|---|---|
| Rating (1–5) | |
| Observations | |
| Verbatim feedback | |
| Issues discovered | |
| Resolution | |

---

## Log index

| # | Tester | External? | Date | Languages | Tests attempted | Tests passed | Evidence ids |
|---|---|---|---|---|---|---|---|
| — | — | — | — | — | — | — | — |

**Tester blocks recorded: 0**

---

## Summary (to be completed only when real tester blocks exist)

| Metric | Value |
|---|---|
| External testers | **0** |
| Core-team members counted as external | **0** (must remain 0) |
| Tests attempted (A–J) | 0 |
| Tests succeeded | 0 |
| Tests failed | 0 |
| Median elapsed time | not measured |
| Mean rating | not measured |
| Languages validated with genuine N-ATLAS | none |
| Issues discovered | 0 |
| Issues resolved | 0 |

**Do not fill this summary until the log above contains real tester blocks.**
