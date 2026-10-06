# ZYLCODE × N-ATLAS 2026 — PHASE C1 OWNER REPORT

- **Date:** 2026-10-06
- **Branch:** `competition/natlas-2026`
- **Phase status:** **COMPLETE for all work not blocked on model access — STOPPED AT ONE OWNER GATE**
- **N-ATLAS integration status:** **`BLOCKED_NATLAS_ACCESS`** (runtime) · **`BLOCKED_NATLAS_API_ACCESS`** (remote API)
- **Pushed:** **NO — nothing was pushed.**
- **Proof level:** **L2** (deterministic tests with named test doubles)

---

## 1. Branch and starting HEAD

- Branch: `competition/natlas-2026`
- Starting HEAD: **`21420d21bbd29e8398f8aac1dc28e5b5a4fb0e7c`** (the C0 final report)
- Starting state matched the C0 close exactly: `main` = `b4d13ab`, 14 foreign working-tree entries,
  0 stashes, nothing staged, **no owner work after C0**.

**Correction to the C0 report.** C0 item 17 stated `origin/main` was `1338d0b`. The measured value
is **`aae6b4e4ffd939cd09abb6d7345ae1a762f7ba03`**. The C0 figure was stale; this report records the
correct one.

## 2. Current HEAD

- Engineering tip after this phase's nine commits: **`67a19cf`**
- This report is added as the closing commit and becomes the new tip.

## 3. Files created (20)

**Rust (3)**
1. `crates/zylcode-core/src/competition/natlas/local.rs`
2. `crates/zylcode-core/src/competition/natlas/remote.rs`
3. `crates/zylcode-core/tests/natlas_runtime.rs`

**TypeScript SDK (5)** — `apps/natlas-sdk/`
4. `package.json` · 5. `tsconfig.json` · 6. `README.md`
7. `src/index.ts`
8. `test/smoke.mjs`

**Playground (1)**
9. `apps/natlas-sdk/playground/index.html`

**Documentation (11)**
10. `docs/competition/natlas/NATLAS_RUNTIME_FEASIBILITY_2026-10-06.md`
11. `docs/competition/natlas/PUBLIC_REPO_READINESS_2026-10-06.md`
12. `docs/competition/natlas/WORKING_ARTEFACT_EVIDENCE_PLAN.md`
13. `docs/competition/natlas/TECHNICAL_DOCUMENTATION_STRUCTURE.md`
14. `docs/competition/natlas/VIDEO_DEMO_PLAN.md`
15. `docs/competition/natlas/MULTILINGUAL_VALIDATION_MATRIX.md`
16–24. `docs/competition/natlas/developer/{QUICKSTART, INSTALLATION, NATLAS_SETUP, FIRST_WORKFLOW,
   ARCHITECTURE, SECURITY_AND_APPROVALS, EVIDENCE_AND_VALIDATION, TROUBLESHOOTING, BETA_TEST_GUIDE}.md`

## 4. Files modified (3)

1. `crates/zylcode-core/src/competition/natlas/mod.rs` — declares `local`/`remote`, adds the
   distinct `BLOCKED_NATLAS_API_ACCESS` reason, re-exports, adds 2 tests.
2. `docs/competition/natlas/BETA_TEST_EVIDENCE_TEMPLATE.md` — expanded to capture external/core
   status, environment, language, timings, consent (still **0 rows**).
3. `.gitignore` — 29 lines of model-weight / credential / evidence ignore rules.

> **`crates/zylcode-core/src/lib.rs` was NOT modified this phase.** The foreign hunk it carries
> remains untouched and uncommitted.

## 5. Commits created (9, narrow)

| SHA | Message |
|---|---|
| `8d896ed` | `docs(competition): assess official N-ATLAS runtime options` |
| `f998419` | `feat(competition): implement genuine N-ATLAS runtime transport` |
| `8dcc6d0` | `test(competition): verify transport and anti-fallback boundary` |
| `8ed94c1` | `feat(competition): add N-ATLAS developer SDK` |
| `b54d0bc` | `feat(competition): add interactive N-ATLAS developer playground` |
| `f8aa9f7` | `docs(competition): add first-time developer documentation set` |
| `36cc178` | `test(competition): add multilingual validation harness` |
| `4538349` | `docs(competition): prepare external beta validation package` |
| `67a19cf` | `docs(competition): prepare public submission evidence package` |

## 6. Tests added (33)

| Suite | Added | Total |
|---|---|---|
| `competition::` unit tests (in-crate) | 11 | **29** (was 18) |
| `tests/natlas_runtime.rs` (new) | 12 | **12** |
| TypeScript SDK smoke tests | 10 | **10** |

## 7. Exact tests run

```
cargo test -p zylcode-core --lib competition::
cargo test -p zylcode-core --test natlas_boundary
cargo test -p zylcode-core --test natlas_runtime
cargo clippy -p zylcode-core --all-targets -- -D warnings
cd apps/natlas-sdk && node --experimental-strip-types test/smoke.mjs
cd apps/natlas-sdk && ../../node_modules/.bin/tsc --noEmit
```

## 8. Exact pass / fail / ignored totals

| Suite | Passed | Failed | Ignored |
|---|---|---|---|
| `--lib competition::` | **29** | 0 | 0 |
| `--test natlas_boundary` (C0, preserved) | **17** | 0 | 0 |
| `--test natlas_runtime` (new) | **12** | 0 | 0 |
| **Rust competition total** | **58** | **0** | **0** |
| SDK smoke (`test/smoke.mjs`) | **10** | 0 | 0 |
| SDK typecheck (`tsc --noEmit`) | clean | — | — |
| Clippy `-D warnings` | clean | — | — |

**All existing C0 tests were preserved and still pass.**

## 9. N-ATLAS authoritative source discovered

**Yes.** `NCAIR1/N-ATLaS` on Hugging Face — official, published by NCAIR (National Centre for AI
and Robotics, NITDA, Nigeria) with Awarri Technologies.

- Base model: **Llama-3 8B**, supervised fine-tune
- Revision observed: `e294476928aca9030e924ca27bb8e085e8581273` (last modified 2025-09-23)
- Languages: Yoruba, Hausa, Igbo, Nigerian-accented English
- Documented context: 8,092 tokens

**GATE C1-A: satisfied.**

## 10. Runtime options investigated

| Route | Outcome |
|---|---|
| **A — official inference API** | no documented public API found → `BLOCKED_NATLAS_API_ACCESS` |
| **B — official model, local execution** | **viable**; owner selected this route |
| B-alt — community GGUF mirror | available and ungated; **not selected**, recorded as fallback |

## 11. Local-model feasibility

**Feasible.** An 8B model at Q4 needs ~7–8 GB RAM at runtime; the machine has 32 GB. Measured
environment: 12-core Intel, Quadro P520 with 4 GB VRAM, 67 GB free disk, Ollama 0.32.4 installed.
Ollama imports safetensors directly, so **no llama.cpp conversion is required**.

## 12. API feasibility

**Not feasible as a documented integration.** No endpoint, auth scheme, schema, model id or API
key process is published. **Nothing was invented.**

## 13. Licence / access findings

- Licence: **N-ATLaS "Open-Source Research and Innovation License"** (Sept 2025, Nigerian law).
- Obligations: **attribution** to Awarri Technologies + FMCIDE; derivatives under the **same
  licence**; renamed derivatives carry **"Powered by Awarri"**; **≤ 1,000 active end-users**.
- Permitted: research, accessibility, language preservation, civic tech, education. **This
  prototype is inside the permitted set.**
- **Access is gated** (`gated: "auto"`): unauthenticated `config.json` → **HTTP 401**. The model
  card is public (200).

**GATE C1-B: satisfied.**

## 14. Model / download / RAM / VRAM requirements

| Item | Value |
|---|---|
| Official format | safetensors, 4 shards |
| Official download | **15,333 MB (~15.0 GiB)** |
| Community Q4_K_M (fallback) | 4,692.8 MB (~4.6 GiB) |
| Runtime RAM (FP16) | ~18–20 GB |
| Runtime RAM (Q4) | ~7–8 GB |
| VRAM | 4 GB → partial offload only |
| Free disk | 67 GB (sufficient) |

## 15. Whether any model was downloaded

**NO.** No weights were downloaded. The download gate was reached and reported rather than assumed.

## 16. Whether any genuine N-ATLAS call occurred

**NO.** No request has ever been sent to N-ATLAS. Nothing has been presented as a genuine call.

## 17. Genuine call details

**Not applicable** — no genuine call occurred. No timestamp, runtime, model identity, latency or
evidence id exists to report.

## 18. Current proof-ladder level

**L2.** All success paths are exercised by deterministic tests using explicitly named test doubles
(`StubServer`, `MockNatlasTransport`). **L3 requires a genuine invocation, which has not happened.**

## 19. End-to-end workflow status

| Stage | Status |
|---|---|
| Configuration (env-only, no defaults) | `TESTED` |
| Boundary trait | `IMPLEMENTED` |
| Local transport (real HTTP round trip) | `TESTED` (against a stub server) |
| Runtime probe | `TESTED` |
| Client → interpret → evidence | `TESTED` |
| Intent → validated `TaskGraph` | `TESTED` |
| Approval gate | `IMPLEMENTED` (a real gate in the graph) |
| **Genuine invocation** | **`BLOCKED_NATLAS_ACCESS`** |
| Approved-write execution | `PROPOSED` — needs the concrete patch/test command |

## 20. Multilingual matrix status

| Language | State |
|---|---|
| Yoruba | ⛔ not tested — blocked |
| Hausa | ⛔ not tested — blocked |
| Igbo | ⛔ not tested — blocked |
| English / Nigerian-English | ⛔ not tested — blocked |

Gates ML-1 … ML-4 are **all blocked on the same dependency**. The harness is ready; the model is
not. The three Nigerian-language prompts are **deliberately left for a native speaker to author** —
machine-translating them would invalidate the direct-language claim.

## 21. Toolkit components actually implemented

| Component | State | Honest description |
|---|---|---|
| **Developer SDK (TypeScript)** | ✅ real | `apps/natlas-sdk` — configure, `checkStatus`, `invoke`, `submitIntent`, evidence. Typechecks; 10 smoke tests pass. |
| **Interactive playground** | ✅ real | self-contained HTML; status, model identity, language, context, invocation, intent, approval gate, evidence. Never claims success without a real call. |
| **Developer documentation** | ✅ real | 9 documents under `docs/competition/natlas/developer/`. |
| Fine-tuning starter kit | 🚫 not built | out of scope, as instructed |

Portal options claimable: **"More than one"** (SDK + playground + docs) — and the SDK is genuinely
a Python/JS SDK. **The Python SDK option is NOT claimed** (only TypeScript was built).

## 22. First-time developer journey status

**Documented and ready; not yet validated by a real first-time developer.**
`QUICKSTART.md` answers all eleven required questions (what, need, install, obtain/configure,
verify connection, open repo, submit task, approval gate, verify result, find evidence, report a
problem).

## 23. Beta-test package status

**Prepared, unpopulated.** The template captures tester identity + consent, external/core status,
background, environment, repository, language, exact task, start/end timestamps, measured elapsed
time, evidence id, outcome, verification result, rating, verbatim feedback, issues, resolution.
`BETA_TEST_GUIDE.md` defines TESTS A–J.

## 24. Number of REAL external beta testers

**ZERO.** None have been fabricated. The owner and Ibrahim Abdulrahman are core team and **must
not** be counted as external.

## 25. Public GitHub readiness

Audited. `README.md`, `LICENSE`, `CONTRIBUTING.md`, `SECURITY.md` all present. No credentials in
tracked files (the one `sk-…` hit is a **test fixture** proving the system rejects secret-shaped
strings). No weights tracked. Ignore rules added.

⚠️ **Owner should review the pre-existing untracked files before publishing** (see item 32).

## 26. Licence status

**Resolved.** `LICENSE` is **Apache-2.0** and already present. No owner licence choice is required.
The **N-ATLAS weights are a separate work** under the N-ATLaS licence and must never be committed
or relicensed here.

## 27. Working artefact evidence readiness

**Plan complete, artefact absent.** Everything except evidence of a genuine invocation is ready.
`WORKING_ARTEFACT_EVIDENCE_PLAN.md` names the intended URLs and what each proves.

## 28. Technical-document readiness

**Structure complete (18 sections), document not written.** Sections 6, 9, 11, 13 must not be
drafted until real evidence exists.

## 29. Video-plan readiness

**Plan complete, not recorded.** 3–5 minute structure with per-section timings and explicit
non-negotiables (never show the `TEST DOUBLE` badge as if genuine).

## 30. Regressions / new defects

**None.** All C0 tests preserved and passing. Two defects were found **in my own new code during
this phase and fixed in the same turn**:

1. **Loopback proxy bypass** — `LocalNatlasTransport` initially honoured the ambient `HTTP_PROXY`,
   which would route `127.0.0.1` traffic to a third party. Fixed with `.no_proxy()`; a test now
   guards it.
2. **Test-setup errors** (empty API key in fixtures; a proxy-mediated 502 mistaken for a connection
   refusal). Fixed; assertions tightened to "never a fabricated success".

## 31. Existing router-flake status

**Unchanged, still pre-existing, still documented.** `router::tests::d1_vector_cache_cross_prompt_
contamination_guard` remains an intermittent parallel-execution flake caused by a process-global
counter at `router.rs:1088`. **Not caused by this work and not fixed** — the fix belongs in
`router.rs`, which this phase must not modify. Recorded in `NATLAS_VALIDATION_PROTOCOL` §6.1.
Run canonical suites with `--test-threads=1` to avoid it.

## 32. Working-tree status

Exactly the **14 foreign entries** (7 modified + 7 untracked), unchanged:

```
 M .github/workflows/ci.yml            M .github/workflows/release.yml
 M crates/zylcode-core/src/agent.rs    M crates/zylcode-core/src/cli.rs
 M crates/zylcode-core/src/lib.rs      M crates/zylcode-core/src/router.rs
 M crates/zylcode-core/src/terminal.rs
?? .git-msg.txt                       ?? COMPLETE_AUDIT_REPORT.html
?? crates/zylcode-core/src/project/filesystem.rs.backup
?? docs/governance/RECONCILIATION_FORENSIC_REPORT.md
?? run_tests.bat                      ?? tasks/            ?? vc_inspect.py
```

None created, discarded, overwritten, stashed or committed by this phase. Stashes: **0**.

## 33. Ahead / behind

`git rev-list --left-right --count origin/main...HEAD` → **`0  20`** (0 behind, 20 ahead).
`origin/main` = `aae6b4e`.

## 34. Confirmation that nothing was pushed

**Confirmed.** No push, no PR, no merge, no rebase, no force operation, no tag, no public
deployment. All work exists only as local commits on `competition/natlas-2026`.

## 35. Exact blockers requiring owner action

**ONE blocker gates almost everything:**

> **Authenticate to Hugging Face and accept the N-ATLaS terms**, then provide a read token to this
> machine via the standard mechanism:
>
> ```bash
> hf auth login      # paste the token at the prompt — never into source code or chat
> ```
>
> Official weights: `https://huggingface.co/NCAIR1/N-ATLaS` (accept the terms first).

*Alternative:* authorise the ungated community GGUF mirror (`QuantFactory/N-ATLaS-GGUF`,
Q4_K_M ≈ 4.6 GB), which needs no Hugging Face login.

**Secondary blockers (owner decisions, not yet taken):**
- authorising a public deployment / hosted playground;
- making the repository public;
- recruiting two real external beta testers;
- reviewing the pre-existing untracked files before publication.

## 36. Recommended next phase

**Phase C2 — First Genuine Invocation and Multilingual Evidence.**

1. Owner completes the Hugging Face gate (item 35).
2. Download the official weights; `ollama create n-atlas`; confirm with `curl /api/tags`.
3. Capture the **first genuine invocation** → close **GATE C1-C**, **C1-D**, **C1-E**; proof level
   rises to **L3**.
4. Run the bounded slice to the approval gate; capture the evidence chain → **C2-A … C2-G**.
5. Run the multilingual matrix across Yoruba, Hausa, Igbo and Nigerian English → **ML-1 … ML-4**.
6. Only then recruit two external testers → **BETA-1 … BETA-3**.
7. Only then write the technical document, record the video, and prepare the evidence bundle.

**Do not begin Phase C2 until the owner has reviewed this report.**
