# KNOWN LIMITATIONS — N-ATLAS × ZylCode Developer Bridge

**Date:** 2026-10-10
**Branch:** `competition/natlas-2026` · **Commit:** `8216738`

This document lists what the submission **cannot** do, does **not** prove, or has
**not** tested. It is written to be read by a sceptical reviewer. Where a limitation is
environmental rather than a defect, that is stated.

---

## 1. Live-endpoint limitations

### L-1 · ZeroGPU quota is shared, metered, and can be exhausted at any time
The endpoint runs on Hugging Face **ZeroGPU**, a shared pool. When the daily budget is
spent, generation fails. This is **not retryable** — retrying cannot create GPU budget.

- **Observed on 2026-10-10 (EV-028):** the first live call succeeded
  (*"The sum of 2 and 2 is 4."*); every subsequent call returned `event: error`.
- The client classifies this as `QUOTA` and **refuses to retry**. That is correct
  behaviour, but it does not restore capacity.
- **Consequence:** a reviewer who arrives after the budget is spent will see errors.
  See `LOCAL_RUNTIME_FALLBACK.md`.

### L-2 · `/healthz` reports `healthy` even when generation is failing
During the 2026-10-10 quota failure, `GET /healthz` continued to return
`{"status":"healthy", ...}`. **The health endpoint does not reflect GPU quota state.**
A reviewer who checks health, sees `healthy`, and then receives an error may reasonably
be confused. This is a real limitation of the deployed Space, recorded rather than hidden.

### L-3 · `/v1/*` requires `NATLAS_API_KEY`, which is not published
All API routes (`/v1/models`, `/v1/chat/completions`, `/v1/completions`,
`/v1/audio/transcriptions`) return `401` without the key. The key is a Space secret and
is **not** in this repository or this environment. API-level verification therefore
could not be performed here; the deployed **frontend** path was used instead (EV-028).

### L-4 · One GPU replica
The Space runs a single replica (`replicas.current: 1`). There is no horizontal scale,
and a cold start is not masked by a warm second instance.

---

## 2. Multilingual limitations

### L-5 · Comprehension is evidenced; native-speaker sign-off is not obtained
EV-015 recorded genuine in-language replies for Yoruba, Hausa and Igbo, but the **prompts
were authored by the team, not by native speakers**. Per the matrix's own Rule 1, that is
weaker evidence than a native-speaker-authored prompt. Recorded as
**validated-comprehension with native-speaker sign-off recommended**.

### L-6 · Pidgin generation is NOT claimed
EV-016 shows the model **understood** Nigerian Pidgin but **replied in standard English**.
Pidgin is a **non-target bonus**; Pidgin *generation* is untested and must not be claimed.

### L-7 · Multilingual claims could not be re-confirmed live on 2026-10-10
The live re-check (EV-028) was blocked by quota after the first call. **This neither
upgrades nor downgrades any language claim** — a quota error is evidence about the GPU
pool, not about a language.

### L-8 · Claim A ≠ Claim B ≠ Claim C
A passing plumbing test (language metadata round-trips) says **nothing** about whether the
model understands or generates a language. See `MULTILINGUAL_VALIDATION_MATRIX.md`
§ "Claim separation".

---

## 3. Model / parsing limitations

### L-9 · LLM-in-JSON fidelity gap persists
The model still emits Python triple-quoted strings with raw newlines where valid JSON is
required (EV-017). ZylCode recovers with a **bounded, deterministic repair layer**
(`NatlasEngineeringIntent::parse_with_repair`) that never invents content and records
whether it normalised. The underlying model behaviour is **not fixed**; it is *tolerated*.

### L-10 · Dialectal and accent bias
The N-ATLaS card documents dialectal/accent bias and limited code-switching. Not
independently measured here.

---

## 4. Verification limitations

### L-11 · No tester session was independently reproduced
Two external testers submitted responses (`EXTERNAL_BETA_TEST_REPORT.md`). We did **not**
re-run their sessions. Their ratings and observations are **tester-reported**. The one
checkable claim (18 passed / 0 failed at commit `5599392`) *was* reproduced.

### L-12 · The testers validated the pre-remediation commit
The only commit-citing response (Sheet B) cites `5599392`, which is the **direct parent**
of the current `HEAD` (`8216738`). The testers therefore exercised the code **before** the
NAT-A remediation. The remediation is newer than any external validation.

### L-13 · Track classification is not fully established
Sheet A's `TESTING TRACK` column is **empty** for both responses; the only declared track
is Sheet B's self-declared **`C`**. No track is inferred from a spreadsheet title.

### L-14 · GitHub Actions CI has not executed on the pushed commit
`.github/workflows/ci.yml` exists and mirrors the local gates, but its run on the pushed
commit is **NOT RUN** at the time of writing.

### L-15 · Windows-only execution
All gates were run on Windows (`MINGW64_NT-10.0-26200`). **Linux execution is NOT RUN.**

---

## 5. Scope limitations

### L-16 · Repository is PUBLIC
`github.com/zylvex-tech/zylcode` is **public** (`isPrivate: false`), which pre-dates this
release. Per instruction, **visibility was not changed**. Flagged for the owner.

### L-17 · `Cargo.lock` is not tracked
`Cargo.lock` is listed in `.gitignore` (line 4) and is **not committed**. For an
application (as opposed to a library) this weakens byte-for-byte build reproducibility.
Recorded as a finding; **not silently changed**.

### L-18 · No licence selected for the ZylCode repository
The Space is Apache-2.0 (a fork of a third-party Space). The ZylCode repository itself has
no licence file selected by the owner. (Note: this is the *ZylCode* repo; the separate
ZUMP project's licensing deferral is unrelated.)

### L-19 · Nothing is production-approved
Every module remains at its stated maturity. Passing tests do not confer production
approval.

---

## 6. Summary table

| Id | Limitation | Severity | Mitigation |
|---|---|---|---|
| L-1 | Shared quota can be exhausted | High (environmental) | Documented fallback; no-retry classification |
| L-2 | `/healthz` healthy during quota failure | Medium | Documented; client does not trust it for quota |
| L-3 | API key not published | Medium | Frontend path used for live proof |
| L-4 | One replica | Low | — |
| L-5 | No native-speaker prompt sign-off | Medium | Recommended before final sign-off |
| L-6 | Pidgin generation unclaimed | Low (scope) | Explicitly not claimed |
| L-7 | Live multilingual re-check blocked | Medium | Recorded as BLOCKED, not failed |
| L-9 | Model JSON fidelity gap | Medium | Bounded repair layer |
| L-11 | Tester sessions not reproduced | Medium | Labelled tester-reported |
| L-12 | Testers used pre-remediation commit | Medium | Stated plainly |
| L-13 | Track partly undeclared | Low | Not inferred from titles |
| L-14 | CI not yet run on push | Low | Workflow present |
| L-15 | Windows-only | Medium | Stated |
| L-16 | Repo public | Owner decision | Unchanged |
| L-17 | `Cargo.lock` untracked | Low–Medium | Finding reported |
| L-18 | No repo licence | Owner decision | Reported |

*End of document.*
