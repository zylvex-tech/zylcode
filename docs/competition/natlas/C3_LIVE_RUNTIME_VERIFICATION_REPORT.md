# C3 LIVE RUNTIME VERIFICATION REPORT

**Branch:** `competition/natlas-2026` · **Commit:** `dadb5d9` · **Date:** 2026-10-07
**Status: C3 LIVE RUNTIME VERIFIED — READY FOR OWNER DEMO RECORDING**

---

## What this report proves

A **genuine, uninterrupted live journey** from a target-language (Yoruba) human instruction,
through the real controlled N-ATLAS endpoint, through the ZylCode engineering bridge, to a
real file mutation and a verified test — all captured as reproducible evidence.

No simulated model response. No hermetic test double. No synthetic fallback.

---

## The chain (every link verified)

| Step | What happened | Evidence |
|---|---|---|
| 1. User Instruction | Yoruba: *"Jọwọ, ṣẹda faili Python kékeré tí yoo sọ 'Hello ZylCode'…"* | EV-024 |
| 2. Endpoint Health | `GET /healthz` → 200, model `NCAIR1/N-ATLaS` | EV-024 |
| 3. Real N-ATLAS Inference | `POST /v1/chat/completions` → HTTP 200, real model output | EV-024 |
| 4. Bounded Repair | Triple-quoted string (`"""..."""`) with raw newlines → escaped JSON | EV-023, EV-024 |
| 5. Strict Parse | `NatlasEngineeringIntent::parse_with_repair` → valid intent, `was_repaired=true` | EV-024 |
| 6. TaskGraph | `to_task_graph()` → approval-gated DAG with WriteFile + Verify | Hermetic tests |
| 7. Pre-Approval Check | File does NOT exist; runner status `AwaitingApproval` | EV-025 |
| 8. Human Approval | `runner.approve("step-01", "owner")` → gate opens | EV-024 |
| 9. File Mutation | `helloworld.py` written with `print('Hello ZylCode')` | EV-024 |
| 10. Test Execution | `python helloworld.py` → exit 0, stdout `Hello ZylCode` | EV-024 |
| 11. Verified Claim | Claim status `Verified`, statement "`python helloworld.py` exited 0" | EV-024 |
| 12. Provenance | INTENT node → approval → mutation → test → claim, all traceable | EV-026 |

---

## Root cause of EV-017 (and why EV-024 succeeded)

**EV-017 failure:** The model emitted a Python triple-quoted string with raw newlines as a JSON
string value. Strict JSON parse failed.

**Fix applied:**
1. **Strengthened SYSTEM preamble (rev 2):** Explicit JSON serialization rules, compact valid
   example, 6 numbered CRITICAL RULES including "NEVER use triple-quoted strings" and
   "Embedded newlines MUST be written as `\n`".
2. **Bounded repair layer (`repair_json`):** Deterministic, narrowly scoped to triple-quoted
   strings, raw newlines inside strings, and trailing commas. Never invents content. Records
   whether normalization occurred. Fail-closed on unrecoverable defects.

**Result:** The model still emits triple-quoted strings (the LLM-in-JSON fidelity gap persists),
but the repair layer recovers deterministically. EV-024 proves the full chain succeeds with repair.

---

## Security invariants (all verified)

| Invariant | Status | Verification |
|---|---|---|
| Path traversal rejected | ✅ | `fs.write` test: `../escaped.txt` refused |
| Writes confined to workspace | ✅ | `fs.write` containment check before I/O |
| Command execution governed by policy | ✅ | Permission gate + FactoryRunner policy |
| Approval required before every side effect | ✅ | Structural DAG gating, order-independent |
| Model cannot self-approve | ✅ | Approval requires human actor |
| Approval for one task cannot authorize another | ✅ | `approved_by` set on specific node only |
| Secrets removed from evidence | ✅ | `redact_secrets` tested + integration-tested |
| Endpoint failure never invokes another provider | ✅ | `NatlasResilienceState` has no fallback state |

---

## Regression counts (exact, not combined)

```
cargo test -p zylcode-core --lib competition::natlas::intent    -> 18 passed; 0 failed; 0 ignored
cargo test -p zylcode-core --test natlas_boundary                -> 17 passed; 0 failed; 0 ignored
cargo test -p zylcode-core --test natlas_runtime                 -> 12 passed; 0 failed; 0 ignored
cargo test -p zylcode-core --test natlas_bridge                  -> 14 passed; 0 failed; 0 ignored
```

`natlas_live.rs` contains 2 ignored tests (gated behind environment variables); they are
harnesses, not CI assertions.

---

## Evidence IDs

| ID | Description | Status |
|---|---|---|
| EV-023 | Production structured-output contract + bounded JSON repair layer | PASS |
| EV-024 | Genuine live acceptance journey (Yoruba → N-ATLAS → mutation → test) | **PASS (L3)** |
| EV-025 | Truthful failure path — denied approval blocks mutation | PASS |
| EV-026 | End-to-end provenance chain | PASS |

---

## Remaining submission blockers

| Blocker | Status | Owner action required |
|---|---|---|
| ≥2 external beta testers | ⛔ | Owner must recruit |
| Team profile | ⛔ | Owner must supply |
| CAC certificate (Track B) | ⛔ | Owner must supply |
| Final video screen-capture | 🔧 | Owner recording step (script provided) |
| Technical documentation | 🔧 | `ARCHITECTURE_BRIDGE.md` write-up pending |

**C3 classification:** `LIVE RUNTIME VERIFIED`

**Submission readiness:** NOT YET CLOSED (human blockers remain)

---

## Honesty ledger

- 0 fabricated tester rows. 0 fabricated benchmark numbers. 0 simulated-success claims.
- EV-024 required repair (`was_repaired=true`) — recorded honestly, not hidden.
- The model still emits triple-quoted strings despite the strengthened preamble — documented.
- Zero external testers recruited — stated explicitly.
- No push performed — owner authorization still required.
