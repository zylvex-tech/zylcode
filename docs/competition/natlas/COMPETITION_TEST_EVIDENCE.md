# COMPETITION TEST EVIDENCE

**Date:** 2026-10-10
**Branch:** `competition/natlas-2026` · **Commit:** `8216738`
**Evidence log:** `evidence/EV-027-competition-verification-2026-10-10.log`

---

## 1. Rule applied

> **These results were produced by a fresh run on 2026-10-10. No historical result was
> reused as current verification.** Every figure below is quoted from a command executed
> today, in this working tree, at this commit.

Environment:

```
rustc 1.97.1 (8bab26f4f 2026-07-14) | cargo 1.97.1 (c980f4866 2026-06-30)
node v22.22.2
Host: MINGW64_NT-10.0-26200 (Windows) — x86_64
```

---

## 2. Rust gates

| # | Command | Result |
|---|---|---|
| 1 | `cargo test -p zylcode-core --lib` | **498 passed; 0 failed; 0 ignored** |
| 2 | `cargo test -p zylcode-core --lib competition::natlas::intent` | **18 passed; 0 failed; 480 filtered out** |
| 3 | `cargo test -p zylcode-core --test natlas_multilingual` | **7 passed; 0 failed** |
| 4 | `cargo test -p zylcode-core --test natlas_boundary` | **17 passed; 0 failed** |
| 5 | `cargo test -p zylcode-core --test natlas_runtime` | **12 passed; 0 failed** |
| 6 | `cargo test -p zylcode-core --test natlas_bridge` | **14 passed; 0 failed** |
| 7 | `cargo clippy --workspace --all-targets -- -D warnings` | **exit 0** (no code warnings) |

### Competition-relevant total

```
intent        18
boundary      17
runtime       12
bridge        14
multilingual   7
-------------------
TOTAL         68   ->  68 / 68 passing, 0 failed
```

Verbatim from the log:

```
### GATE 1  cargo test -p zylcode-core --lib
running 498 tests
test result: ok. 498 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 119.70s

### GATE 2  cargo test -p zylcode-core --lib competition::natlas::intent
running 18 tests
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 480 filtered out; finished in 0.00s

### GATE 3  cargo test -p zylcode-core --test natlas_multilingual
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

### GATE 4  cargo test -p zylcode-core --test natlas_boundary
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.68s

### GATE 5  cargo test -p zylcode-core --test natlas_runtime
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.07s

### GATE 6  cargo test -p zylcode-core --test natlas_bridge
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.63s
```

---

## 3. SDK gates

| # | Command | Result |
|---|---|---|
| 8 | `npx tsc --noEmit` (in `apps/natlas-sdk`) | **exit 0** — no type errors |
| 9 | `node --experimental-strip-types test/smoke.mjs` | **16 passed, 0 failed** |

The 16 SDK tests include the four resilience tests added for NAT-A-001/003 and the
language-evidence test added for NAT-A-004:

```
ok   resilience: a ZeroGPU quota 503 is QUOTA, not warming
ok   resilience: a spent quota is not retryable; a cold start is
ok   resilience: every state has non-empty, actionable guidance
ok   resilience: an HTTP_STATUS error carries a human message, not a bare code
ok   invoke: a 503 quota body yields state=quota, not a fabricated success
ok   invoke: a stated language is recorded in evidence; an unstated one is omitted
16 passed, 0 failed
```

---

## 4. Repository guard gate

| # | Command | Result |
|---|---|---|
| 10 | `python scripts/check_retracted_claims.py` | `OK: no NEW retracted claims (11 pre-existing baselined occurrence(s), 11 baseline line(s)).` |

This guard prevents a claim that was previously measured, found unsupported, and removed
from silently re-entering the repository. It is wired into `.github/workflows/ci.yml`.

---

## 5. What these gates prove — and what they do not

| The gates prove | The gates do **not** prove |
|---|---|
| The Rust and TypeScript code compiles and its tests pass at `8216738`. | That the live endpoint returns multilingual output (see EV-028 — quota-blocked). |
| Quota is classified as `QUOTA` and is non-retryable. | That ZeroGPU has budget. Classification is not capacity. |
| A `language` tag round-trips from request to evidence. | That the model understands or generates that language. |
| The strict-lint and retracted-claims guards hold. | That any capability is production-approved. |

---

## 6. Not run

| Gate | Status |
|---|---|
| GitHub Actions CI on the pushed commit | **NOT RUN** — the workflow exists; it runs on push. |
| Cross-platform (Linux) test execution | **NOT RUN** — host is Windows only. |
| Live `/v1/chat/completions` with a valid key | **NOT RUN** — `NATLAS_API_KEY` unavailable. |
| ASR routes | **NOT RUN** — `asr_models_loaded: []`. |

*End of evidence document.*
