# Beta Tester Quickstart — N-ATLAS × ZylCode Developer Bridge

**Version:** 2026-10-08  
**Branch:** `competition/natlas-2026`  
**Target tester:** External developer with basic Rust and Git experience  
**Estimated time:** 15–20 minutes  

---

## 1. What You Are Testing

You are validating the **N-ATLAS Developer Bridge**: a pipeline that takes a programming instruction in a Nigerian language (Yoruba, Hausa, Igbo, or English) and routes it through Nigeria's sovereign `NCAIR1/N-ATLaS` AI model to produce a real, human-approved code change with captured evidence.

**What makes this different from a chatbot:**
- The model does not just reply — it emits a **structured engineering intent** (what file to write, what test to run).
- Every side effect (file write, command execution) requires **human approval** before it happens.
- Every step is recorded in a **tamper-evident evidence ledger**.
- N-ATLAS is isolated behind a dedicated transport boundary — it **cannot silently fall back** to another foundation model.

---

## 2. What You Need

| Requirement | Details |
|---|---|
| **Rust toolchain** | `cargo` 1.70+ (`rustup.rs`) |
| **Git** | Any recent version |
| **Python 3** | For the live endpoint verification script |
| **Internet access** | To reach the controlled N-ATLAS endpoint |
| **No API key needed for hermetic tests** | The competition test suites run offline |
| **Optional: `NATLAS_API_KEY`** | Only if you want to run the live journey against the real endpoint |

---

## 3. Setup (3 minutes)

```bash
# 1. Clone the competition branch
git clone --branch competition/natlas-2026 https://github.com/zylvex-tech/zylcode.git
cd zylcode

# 2. Verify the branch
git log --oneline -3
# Expected: top commit is challenge-specific README + C3 verification report

# 3. Check the endpoint is reachable (no key needed for health check)
curl -s https://zylvex-natlas-zylcode-bridge.hf.space/healthz | python3 -m json.tool
# Expected: "model": "NCAIR1/N-ATLaS", "status": "healthy"
```

---

## 4. Hermetic Test Journey (10 minutes)

These tests prove the ZylCode-side mechanics **without** requiring a live N-ATLAS endpoint or any API credentials. They run deterministically on every machine.

```bash
# 4a. Intent parsing + bounded JSON repair layer
cargo test -p zylcode-core --lib competition::natlas::intent
# Expected: 18 passed; 0 failed; 0 ignored

# 4b. Boundary contract tests
cargo test -p zylcode-core --test natlas_boundary
# Expected: 17 passed; 0 failed; 0 ignored

# 4c. Runtime resilience tests
cargo test -p zylcode-core --test natlas_runtime
# Expected: 12 passed; 0 failed; 0 ignored

# 4d. Engineering bridge — intent → factory execution with approval gate
cargo test -p zylcode-core --test natlas_bridge
# Expected: 14 passed; 0 failed; 0 ignored
```

**Total competition hermetic tests: 61 / 61 expected passing.**

### What these tests prove

- `natlas_boundary` (17 tests): The transport contract is enforced — malformed responses are rejected, secrets are redacted, auth failures are stated honestly, no synthetic substitution.
- `natlas_runtime` (12 tests): The client handles HTTP errors, timeouts, and unreachable endpoints without fabricating success.
- `natlas_bridge` (14 tests): A structured intent from N-ATLAS is correctly converted into an approval-gated task graph, and a file is only written **after** human approval is recorded.
- `intent` (18 tests): The JSON repair layer handles triple-quoted strings, raw newlines, and trailing commas deterministically; unrecoverable defects fail closed.

---

## 5. Live Endpoint Verification (Optional, 2 minutes)

If you want to verify the **genuine N-ATLAS endpoint** is serving the real model:

```bash
# No API key needed for the public health endpoint
curl -s https://zylvex-natlas-zylcode-bridge.hf.space/healthz
```

**Expected response (excerpt):**
```json
{
  "status": "healthy",
  "service": "natlas-engine",
  "model": "NCAIR1/N-ATLaS",
  "attribution": "N-ATLaS is an initiative of the Federal Ministry of Communications, Innovation and Digital Economy, and powered by Awarri Technologies."
}
```

**This proves:** The endpoint is live, serving the genuine `NCAIR1/N-ATLaS` weights, and attribution to Awarri + FMCIDE is preserved.

---

## 6. Human-Approval Behaviour

The bridge is **fail-closed**:

1. The model returns a structured intent (e.g., "write `helloworld.py` with `print('Hello')`").
2. ZylCode converts this into a task graph where every side-effect node (WriteFile, RunCommand) depends on an `approve` node.
3. **Before approval:** The runner status is `AwaitingApproval`. No file is written. No command runs.
4. **After approval:** The runner executes the side effects in dependency order.
5. **If approval is denied:** The mutation step stays `Pending` forever. No file is written.

This is tested hermetically in `natlas_bridge.rs`:
- `approval_gate_parks_the_run_and_no_file_is_written_before_approval`
- `approved_chain_runs_write_then_verify_and_produces_a_verified_claim`

---

## 7. Failure & Troubleshooting

| Symptom | Likely Cause | Fix |
|---|---|---|
| `cargo test` compile errors | Rust version < 1.70 | Run `rustup update` |
| Tests timeout | Full workspace compilation | First run compiles dependencies; subsequent runs are fast |
| `curl` to healthz fails | Network / proxy issue | Check internet; the endpoint is public |
| `natlas_live` tests fail | No `NATLAS_API_KEY` set | These are optional; set env var or skip |
| Endpoint returns 503 | ZeroGPU quota exhausted | Wait and retry; the endpoint is shared ZeroGPU |
| Endpoint returns 401 | Wrong `NATLAS_API_KEY` | Key is read from local secret file; contact owner for access |

**ZeroGPU quota note:** The endpoint runs on Hugging Face shared ZeroGPU. Sustained calls may hit quota limits. The hermetic tests do **not** require the live endpoint and will always pass.

---

## 8. What to Capture & Submit

Please capture the following and send back:

1. **Screenshot of hermetic test results** — all 4 suites green.
2. **Screenshot of healthz response** — showing `model: NCAIR1/N-ATLaS`.
3. **Your operating system + Rust version** (`rustc --version`).
4. **Any compilation warnings or errors** you encountered.
5. **Time taken** for the first `cargo test` run (compilation + test).

Optional but valuable:
6. **Live test result** — if you have a `NATLAS_API_KEY`, run `cargo test -p zylcode-core --test natlas_live -- --ignored` and share the output.

---

## 9. Feedback Questions

1. Were the setup instructions clear? If not, what was confusing?
2. Did all 61 hermetic tests pass on your machine?
3. How long did the first compilation take?
4. Is the human-approval gating behaviour clearly documented and testable?
5. Does the README answer "what challenge, what was built, how to verify" in the first screen?
6. Any concerns about licence, attribution, or usage limits?

---

## 10. Privacy & Safety Warning

**Do NOT submit:**
- Your `NATLAS_API_KEY` or any other API credential
- Confidential repository contents or proprietary code
- Personal data, names, or identifying information of others
- Sensitive production credentials or secrets

**Redaction:** If you accidentally include a secret in a screenshot, note it and the evidence system will redact it. The `redact_secrets` function is tested and counts every scrub.

**Safe scope:** The hermetic tests run in a temporary directory and delete their fixtures after completion. No changes are made to your actual filesystem outside the test workspace.

---

## 11. Quick Reference

```bash
# One-liner: all hermetic competition tests
cargo test -p zylcode-core --lib competition::natlas::intent && \
cargo test -p zylcode-core --test natlas_boundary && \
cargo test -p zylcode-core --test natlas_runtime && \
cargo test -p zylcode-core --test natlas_bridge

# Health check (no credentials)
curl -s https://zylvex-natlas-zylcode-bridge.hf.space/healthz | python3 -m json.tool
```

---

**Questions?** Open an issue on the repository or contact the ZylCode team.

**Last updated:** 2026-10-08  
**Branch:** `competition/natlas-2026`
