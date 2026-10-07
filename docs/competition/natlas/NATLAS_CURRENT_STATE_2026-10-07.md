# N-ATLAS COMPETITION — AUTHORITATIVE CURRENT STATE (Phase 0)

- **Date:** 2026-10-07
- **Author:** ZylForge (audit pass — read-only reconnaissance, then bounded implementation)
- **Branch:** `competition/natlas-2026`
- **HEAD:** `2a6035edce96e815af7d5c9b6833b3071bfbc01e` (`2a6035e`)
- **Deadline:** 2026-10-12 23:59 WAT
- **Predecessor document:** `PHASE_C1_OWNER_REPORT_2026-10-06.md`

This document records the factual baseline measured on 2026-10-07, **before** any code
was modified in this pass. It supersedes nothing; it appends a new measurement.

---

## 1. Authoritative Git state

| Item | Value |
|---|---|
| Repository root | `C:/Projects/zylcode` |
| Current branch | `competition/natlas-2026` |
| HEAD | `2a6035e` — `docs(competition): Phase C1 owner report` |
| Upstream configured | **No** (`fatal: no upstream configured`) |
| `main` | `b4d13ab5e279524867d91e8e8c07a30b42a8f884` |
| `origin/main` | `aae6b4e` (2026-10-03) |
| Ahead / behind `main` | **15 ahead, 0 behind** |
| Merge-base with `main` | `b4d13ab` (== `main` tip) |
| Worktrees | `C:/Projects/zylcode` (this branch); `C:/Projects/zylcode/.wt/verify-build` (detached `018622e`) |
| Stashes | 0 |

### Local branches

| Branch | Tip | Date | Subject |
|---|---|---|---|
| `competition/natlas-2026` | `2a6035e` | 2026-10-06 | docs(competition): Phase C1 owner report |
| `main` | `b4d13ab` | 2026-10-06 | docs(gate): final owner gate report for the factory foundation wave |
| `local/granular-integration` | `e540ad8` | 2026-09-18 | fix(router): drop provider prefix from hermeticity fixture |
| `track/p12-2a-reacceptance` | `e540ad8` | 2026-09-18 | (same tip) |
| `origin/main` | `aae6b4e` | 2026-10-03 | docs(governance): mark TOOL-01 closed |

### Competition isolation — CONFIRMED

`git merge-base main HEAD == b4d13ab` and `git rev-list --left-right --count main...HEAD`
returns `0  15`. **The competition branch is a strict descendant of `main`.** It does not
rewrite, revert or interfere with the main ZylCode line. Nothing has been pushed.

---

## 2. Working tree — 14 foreign entries (unchanged)

```
 M .github/workflows/ci.yml            M .github/workflows/release.yml
 M crates/zylcode-core/src/agent.rs    M crates/zylcode-core/src/cli.rs
 M crates/zylcode-core/src/lib.rs      M crates/zylcode-core/src/router.rs
 M crates/zylcode-core/src/terminal.rs
?? .git-msg.txt                        ?? COMPLETE_AUDIT_REPORT.html
?? crates/zylcode-core/src/project/filesystem.rs.backup
?? docs/governance/RECONCILIATION_FORENSIC_REPORT.md
?? run_tests.bat                       ?? tasks/            ?? vc_inspect.py
```

These are **owner / other-line artefacts**, not competition work. They are preserved
untouched. `crates/zylcode-core/src/lib.rs` in particular carries a **foreign uncommitted
hunk** and must never be staged wholesale (see `stage-partial-file-hunk` discipline).

---

## 3. Existing N-ATLAS work recovered (15 commits)

Unique to this branch, oldest first:

| SHA | Subject |
|---|---|
| `dfa47a1` | feat(competition): isolated N-ATLAS integration boundary |
| `f79c142` | test(competition): deterministic N-ATLAS boundary tests with a named test double |
| `9d6a78b` | docs(competition): N-ATLAS discovery, architecture, validation and evidence |
| `f4724b9` | docs(competition): record the pre-existing router test flake exposed by this branch |
| `21420d2` | docs(competition): Phase C0 final report |
| `8d896ed` | docs(competition): assess official N-ATLAS runtime options |
| `f998419` | feat(competition): implement genuine N-ATLAS runtime transport |
| `8dcc6d0` | test(competition): verify transport and anti-fallback boundary |
| `8ed94c1` | feat(competition): add N-ATLAS developer SDK |
| `b54d0bc` | feat(competition): add interactive N-ATLAS developer playground |
| `f8aa9f7` | docs(competition): add first-time developer documentation set |
| `36cc178` | test(competition): add multilingual validation harness |
| `4538349` | docs(competition): prepare external beta validation package |
| `67a19cf` | docs(competition): prepare public submission evidence package |
| `2a6035e` | docs(competition): Phase C1 owner report |

### Code inventory (measured by line count)

| Path | Lines |
|---|---|
| `crates/zylcode-core/src/competition/mod.rs` | 40 |
| `.../natlas/{client,config,evidence,intent,local,mod,remote,transport,types}.rs` | 2 104 |
| `crates/zylcode-core/tests/natlas_boundary.rs` | 386 |
| `crates/zylcode-core/tests/natlas_runtime.rs` | 305 |
| `apps/natlas-sdk/src/index.ts` | 634 |
| `apps/natlas-sdk/test/smoke.mjs` | 207 |
| `apps/natlas-sdk/playground/index.html` | 553 |
| `docs/competition/natlas/**` | 23 documents |

---

## 4. Environment re-probe (2026-10-07)

| Item | Measured |
|---|---|
| `cargo` / `rustc` | 1.97.1 / 1.97.1 |
| `node` | v22.22.2 |
| Ollama client | **0.32.4 present, server NOT running** |
| Ollama models present | `qwen3:0.6b`, `JOSIEFIED-Qwen3:8b-fp16`, `Qwen3.8-27B-…` — **no N-ATLAS** |
| Free disk (C:) | **99 GB** |
| `HTTP_PROXY` / `HTTPS_PROXY` | `http://127.0.0.1:61562` · `NO_PROXY` empty |

> Loopback traffic must use `.no_proxy()` — this is already load-bearing in
> `LocalNatlasTransport` and is guarded by a test.

---

## 5. External access re-probe — **THE MATERIAL CHANGE**

The single blocker recorded by Phase C1 was: *"Authenticate to Hugging Face and accept the
N-ATLaS terms."* **That gate is now open.**

| Probe | Result | Meaning |
|---|---|---|
| `~/.cache/huggingface/token` | **non-empty** (created 2026-10-06 12:23) | owner authenticated |
| `GET /api/whoami-v2` | **HTTP 200**, user `zylvex` | token valid |
| `GET NCAIR1/N-ATLaS/resolve/main/config.json` | **HTTP 200** (was **401** at C1) | **gated access GRANTED** |

Verified model facts (unchanged, re-confirmed):

- Official repo `NCAIR1/N-ATLaS`, revision `e294476928aca9030e924ca27bb8e085e8581273`
- Official weights: **15.06 GB** across 4 safetensors shards
- Mirror `QuantFactory/N-ATLaS-GGUF` (ungated): `Q4_K_M` = **4.92 GB**

### Measured download throughput — the new practical blocker

| Method | Throughput |
|---|---|
| Single connection via proxy | ~144 KB/s |
| 6 parallel range requests | ~238 KB/s aggregate |
| Direct (no proxy) | ~45 KB/s |

Implied download time: **official ≈ 17.6 h · mirror Q4_K_M ≈ 5.7 h.** Not achievable inside a
working session. This is a **bandwidth** blocker, not an access blocker.

### Endpoints discovered and probed (2026-10-07)

| Endpoint | Status | Notes |
|---|---|---|
| `https://natlas-engine.publicaai.com/chat/{lang}` | **DOWN — HTTP 502 on every path** | Official demo engine (contract recovered from the shipped client bundle) |
| `https://samuelolubukun-natlas-sovereign-engine.hf.space/healthz` | **LIVE — HTTP 200** | Reports `model: NCAIR1/N-ATLaS`, ZeroGPU, official FMCIDE/Awarri attribution |
| `…/v1/chat/completions` (same host) | **401 — API key required** | OpenAI-compatible; key unknown to us |
| `https://calvaryyy-n-atlas-chat.hf.space/gradio_api/call/generate` | **LIVE — public** | Loads `NCAIR1/N-ATLaS` in fp16; ZeroGPU quota-limited |

### Genuine invocation — ACHIEVED

On 2026-10-07 a real request was sent to a live N-ATLAS instance and a real model response was
received. Evidence: `docs/competition/natlas/evidence/EV-000…EV-003`. See
`NATLAS_EVIDENCE_INDEX.md`.

**Proof level: L2 → L3** for the specific claim *"N-ATLAS is reachable and returns genuine model
output"*. Sustained invocation is limited by the ZeroGPU anonymous quota; a controlled deployment
is required for repeatable evidence.

---

## 6. What changed since Phase C1

1. **Access gate: CLOSED → OPEN.** Terms accepted; token valid; `config.json` returns 200.
2. **Genuine invocation: NEVER → CAPTURED.** One real N-ATLAS round trip recorded.
3. **Contract knowledge: UNKNOWN → PARTIALLY VERIFIED.** Three real interfaces now documented
   from primary sources (see `NATLAS_CONTRACT_VERIFICATION_2026-10-07.md`).
4. **Blocker moved.** From *access* to *bandwidth + credentials + service availability*.

## 7. Blockers as of 2026-10-07

| # | Blocker | Owner action |
|---|---|---|
| B1 | No controlled N-ATLAS endpoint with credentials | Deploy our own engine Space **or** obtain a key |
| B2 | Official demo engine down (502) | None — third-party; monitor |
| B3 | Local model download impractical (≈6–18 h) | Run overnight, or accept a hosted route |
| B4 | Public Space quota-limited | Controlled deployment resolves this |

## 8. Constraints honoured this pass

No `reset --hard`, no `clean`, no force checkout, no rebase, no amend, no push, no merge, no
branch deletion, no overwrite of owner files. Read-only reconnaissance preceded every write.
