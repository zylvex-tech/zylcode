# TRACK A — TESTER 001 RELEASE & REMEDIATION RECORD

**Date:** 2026-10-09
**Branch:** `competition/natlas-2026`
**Scope:** Process the first external tester's feedback, remediate confirmed
defects, and record honestly what was and was not verified.
**Author:** Zylvex Technologies Limited (engineering)

---

## 1. Evidence discipline for this document

This record separates three things that are easy to blur:

1. **Tester-reported** — what tester 001 said they saw. Attributed to the tester,
   not independently verified by us.
2. **Independently verified** — what we reproduced or proved with a command whose
   output is quoted below.
3. **Not run** — anything we did not execute. Stated as such; never marked passed.

No test is described as passing unless its output is quoted. No capability is
described as working because the code exists.

---

## 2. Tester 001 — profile and reported outcome

| Field | Value |
|---|---|
| Tester id | 001 |
| Name | Abdul |
| Browser | Brave |
| Languages exercised | English, Hausa, Yoruba |
| Self-reported score | 7 / 10 |
| Date | 2026-10-09 |

### What tester 001 reported as working (tester-reported, not independently re-run by us)

- The bridge reached the live endpoint and returned responses in the tested
  languages.
- The structured-intent shape was recognisable in the replies.

> These are the tester's observations. We have **not** re-executed the tester's
> exact session, so they remain *tester-reported* rather than *verified by us*.
> Where our own tests independently confirm adjacent behaviour, that is stated
> separately in §5.

### What tester 001 reported as defective

| Id | Severity | Reported symptom |
|---|---|---|
| NAT-A-001 | P0 | ZeroGPU quota exhaustion — the endpoint became unusable and the cause was not clear |
| NAT-A-002 | P1 | A processing state that gave no clear signal of what was happening |
| NAT-A-003 | P1 | Error messages were not human-readable |
| NAT-A-004 | P1 | Multilingual regression coverage was missing (Igbo in particular) |

---

## 3. Confirmation of each defect against the code

Each defect was checked against the actual source **before** any change was made.

| Id | Confirmed? | Evidence |
|---|---|---|
| NAT-A-001 | ✅ Confirmed | `NatlasResilienceState::from_http` mapped a `503` by checking load/cold-start words only; a ZeroGPU quota `503` ("You have exceeded your GPU quota") contained none of them and was therefore misclassified `Warming`. |
| NAT-A-002 | ✅ Confirmed | The playground showed a bare `RUNNING…` badge with no elapsed time and no ceiling; the Rust client had no retry policy and no attempt counter. A slow cold start was indistinguishable from a hang. |
| NAT-A-003 | ✅ Confirmed | `NatlasError` had a `Display` impl but no human-facing guidance; the playground surfaced raw codes (`FAILED — HTTP_STATUS`). |
| NAT-A-004 | ✅ Confirmed | `NatlasEvidence` (Rust) had **no `language` field**; the SDK's `NatlasEvidence` interface had none either; Igbo appeared nowhere in the Rust tree. The multilingual matrix nonetheless claimed the harness recorded language. |

---

## 4. Remediation — what changed

### NAT-A-001 — ZeroGPU quota exhaustion (P0)

- `NatlasResilienceState::from_http` now checks for a **quota** body (`quota`,
  `exceeded`, `rate limit`, `too many requests`) **before** the cold-start words,
  and maps `429`/`402` to `Quota`. A quota `503` can no longer be mislabelled
  `Warming`.
- `is_retryable()` excludes `Quota`: a spent budget is not retried, because
  retrying cannot create budget and would leave the surface looking stuck.
- The TypeScript SDK (`classifyHttp`) and the browser playground implement the
  identical ordering.
- A documented fallback exists: `docs/competition/natlas/LOCAL_RUNTIME_FALLBACK.md`,
  referenced by the quota guidance string itself.

### NAT-A-002 — processing state (P1)

- Added a bounded `RetryPolicy` to `NatlasClient`: `max_attempts` **and** a
  wall-clock `budget_ms`, with exponential backoff. A run therefore always
  terminates with a stated outcome.
- Added `NatlasInvocation.attempts` so a reader can tell "worked first time"
  from "recovered after a cold start".
- Default policy is `RetryPolicy::NONE` (exactly one attempt), preserving the
  previous single-shot semantics for every existing caller.
- The playground now shows a **live elapsed-time indicator** with the ceiling
  stated (`RUNNING… 12s / 60s`), so the processing state is never an ambiguous
  spinner.

### NAT-A-003 — human-readable errors (P1)

- `NatlasError::human_message()` returns two lines: what happened, then what to
  do about it. Guidance is static and never contains a secret.
- `NatlasResilienceState::guidance()` provides an actionable sentence for every
  state.
- The SDK exposes `guidance()`, `humanMessage()` and `resilienceStateOf()`, and
  `Invocation` now carries a `state`.
- The playground renders the state **and** the guidance on every failure.

### NAT-A-004 — multilingual regression coverage, incl. Igbo (P1)

- Added `language: Option<String>` to `NatlasRequest` and
  `NatlasEvidence` (Rust), with `with_language()`. It is recorded, **never
  inferred**, and never sent on the wire.
- Added `language?: string` to the SDK's `NatlasRequest` and `NatlasEvidence`,
  and set it in `invoke()`.
- Added a dedicated Rust regression suite, `tests/natlas_multilingual.rs`,
  covering all four competition targets plus the non-target Pidgin bonus, and
  asserting the tag is recorded on both the success and failure paths and is
  absent when unstated.
- Added SDK regression tests for language propagation and omission.
- Corrected the inaccurate harness-readiness claim in
  `MULTILINGUAL_VALIDATION_MATRIX.md` and recorded the correction.

---

## 5. Verification — commands run and their output

All commands were run on the remediation working tree at branch
`competition/natlas-2026`. Output is quoted verbatim.

```
$ cargo test -p zylcode-core --lib competition::natlas::intent
test result: ok. 18 passed; 0 failed; 0 ignored

$ cargo test -p zylcode-core --test natlas_boundary
test result: ok. 17 passed; 0 failed; 0 ignored

$ cargo test -p zylcode-core --test natlas_runtime
test result: ok. 12 passed; 0 failed; 0 ignored

$ cargo test -p zylcode-core --test natlas_bridge
test result: ok. 14 passed; 0 failed; 0 ignored

$ cargo test -p zylcode-core --test natlas_multilingual   # NEW (NAT-A-004)
test result: ok. 7 passed; 0 failed; 0 ignored
```

**Competition total: 68 / 68 passing** (18 + 17 + 12 + 14 + 7).

Regression checks specific to the defects:

```
$ cargo test -p zylcode-core --lib competition::natlas::client
  a_transient_failure_is_retried_and_recovers ... ok
  a_spent_quota_is_not_retried ... ok
  retries_stop_at_the_attempt_cap ... ok
  the_default_policy_makes_exactly_one_attempt ... ok
  backoff_delays_grow_then_cap ... ok

$ cargo test -p zylcode-core --lib competition::natlas::types
  a_zero_gpu_quota_503_is_quota_not_warming ... ok
  quota_is_reported_but_never_retryable ... ok
  every_state_has_a_non_empty_human_guidance ... ok
  a_quota_error_carries_human_readable_guidance_and_is_not_retryable ... ok

$ cargo test -p zylcode-core --lib competition::natlas::evidence
  language_is_absent_until_the_caller_states_it ... ok
  language_is_recorded_when_stated_and_blank_is_ignored ... ok
  language_survives_the_jsonl_round_trip ... ok
```

JavaScript SDK:

```
$ cd apps/natlas-sdk && npx --no-install tsc --noEmit
(no output — typecheck clean)

$ node --experimental-strip-types test/smoke.mjs
16 passed, 0 failed
```

SDK tests added for this remediation:

```
ok   resilience: a ZeroGPU quota 503 is QUOTA, not warming
ok   resilience: a spent quota is not retryable; a cold start is
ok   resilience: every state has non-empty, actionable guidance
ok   resilience: an HTTP_STATUS error carries a human message, not a bare code
ok   invoke: a 503 quota body yields state=quota, not a fabricated success
ok   invoke: a stated language is recorded in evidence; an unstated one is omitted
```

The playground's inline script was syntax-checked:

```
$ node --check <extracted inline script>
PLAYGROUND SCRIPT: SYNTAX OK
```

### CI hygiene (uncovered while verifying)

The repository CI runs `cargo clippy --workspace --all-targets -- -D warnings`.
Clippy flagged one pre-existing unused import in `tests/natlas_live.rs`
(`use std::path::Path;`), which would have failed CI. It was removed. This is a
one-line hygiene fix, not a behavioural change.

The repository's own guard was also run:

```
$ python scripts/check_retracted_claims.py
OK: no NEW retracted claims (11 pre-existing baselined occurrence(s), 11 baseline line(s)).
```

---

## 6. What was NOT run (stated plainly)

| Item | State |
|---|---|
| A live call to `zylvex-natlas-zylcode-bridge.hf.space` after the fix | **NOT RUN** — the live Space was not exercised during this remediation; no network call to the live endpoint was made. |
| The `LOCAL_RUNTIME_FALLBACK.md` procedure end-to-end | **NOT RUN** — documented from the transports' configuration surface, not executed. |
| Tester 001's exact session replayed | **NOT RUN** — their observations remain tester-reported. |
| The playground exercised in a real Brave browser after the fix | **NOT RUN** — its logic mirrors the SDK, which *is* tested; the browser itself was not driven. |
| A real N-ATLAS model call proving Igbo output quality | **NOT RUN** — language *quality* is evidenced separately (EV-015); this remediation covers only the ZylCode-side plumbing. |

No item in this table is claimed as passed.

---

## 7. Honest summary

- **Four confirmed defects** were found in the code, not merely in the report,
  and each was remediated with a regression test that fails without the fix.
- **68 / 68** competition tests pass; the SDK typechecks and passes 16 / 16.
- The fix for NAT-A-001 addresses the **classification and retry** of a quota
  exhaustion. It does **not** create GPU budget; when the shared quota is spent,
  the honest answer is still "quota exhausted" — now with a clear cause, a clear
  next step, and a documented fallback.
- The tester's reported successes are recorded as **tester-reported**, not
  upgraded to verified outcomes.
- Nothing that was not executed is described as passing.

---

## 8. Follow-up (not part of this release)

1. Re-run the live endpoint once the ZeroGPU quota resets, and capture the
   output as fresh evidence (the classification change should now surface
   `QUOTA` instead of `WARMING`).
2. Execute the `LOCAL_RUNTIME_FALLBACK.md` procedure once, and record the result.
3. Drive the playground in a real browser (including Brave) and capture a
   screenshot as evidence.
4. Recruit the second external tester (PS1 requires ≥2).
