# PUBLIC REPOSITORY READINESS AUDIT — Phase C1

- **Date:** 2026-10-06
- **Branch:** `competition/natlas-2026`
- **Status:** audit complete; **the repository has NOT been made public and has NOT been pushed.**
- **Gate:** PUBLIC-1 (content audit) ✅ · PUBLIC-2 (licence) ✅ · PUBLIC-3 (contributing guide) ✅

> The competition requires a **public repository with an open-source licence and a contribution
> guide**. This audit establishes readiness. Publishing is an owner action and was **not**
> performed.

---

## 1. Required public-facing files

| File | Present | Notes |
|---|---|---|
| `README.md` | ✅ | 664 lines, exists at root |
| `LICENSE` | ✅ | **Apache-2.0** — already in place |
| `CONTRIBUTING.md` | ✅ | 134 lines |
| `SECURITY.md` | ✅ | 41 lines |

**GATE PUBLIC-2 (licence): already resolved.** ZylCode's source is Apache-2.0. No licence choice
is required from the owner, so no licence gate is triggered.

> **Important distinction.** The **N-ATLAS model weights are a separate work** under the N-ATLaS
> "Open-Source Research and Innovation License". They are *not* Apache-2.0 and must never be
> committed here or relicensed under Apache-2.0. See
> `NATLAS_RUNTIME_FEASIBILITY_2026-10-06.md` §7.

---

## 2. Secret and credential audit

Method: pattern scan across all **tracked** files.

```
git grep -nIE "(sk-[A-Za-z0-9]{20,}|hf_[A-Za-z0-9]{20,}|ghp_[A-Za-z0-9]{20,}|AKIA[0-9A-Z]{16}|-----BEGIN [A-Z ]*PRIVATE KEY-----)"
```

| Finding | Assessment |
|---|---|
| `apps/zylcode-desktop/src/lib/forge.test.ts:107` — a `sk-abcdef…` string | **Not a credential.** It is a fixture inside a test named *"rejects manifests containing secret-shaped strings"* — the test exists to prove the system rejects such strings. Deliberate, synthetic, and correct to keep. |
| `apps/zylcode-desktop/src/components/TokenMetricsWidget.tsx` | **False positive** — the word "token" in a UI component filename, not a credential. |

**Result: no credentials, keys, tokens or private keys are present in tracked files.**

---

## 3. Large-file and weight audit

```
git ls-files | grep -iE '\.(gguf|safetensors|bin|pt|onnx|pth)$'   ->  (none)
```

**No model weights are tracked.** The ignore rules below make that structural rather than
accidental.

---

## 4. Ignore rules added this phase

Appended to `.gitignore`:

| Rule | Why |
|---|---|
| `*.gguf`, `*.safetensors`, `*.pt`, `*.pth`, `*.onnx`, `*.bin` | model weights must never be committed |
| `/models/`, `/n-atlas/` | conventional local weight directories |
| `Modelfile`, `Modelfile.*` | may embed absolute local paths |
| `.hf/`, `.huggingface/` | Hugging Face cache and **credentials** |
| `.natlas/` | local runtime scratch |
| `docs/competition/natlas/evidence/` | runtime evidence may embed repository content |

> Scoping note: the evidence rule is deliberately narrowed to the competition evidence directory,
> because `docs/governance/evidence/` contains **tracked** files that must not be affected.

---

## 5. Personal-path and proprietary-content audit

| Check | Result |
|---|---|
| Absolute user paths in tracked source | none found in the competition work |
| Proprietary/third-party binaries | none added |
| Internal-only documents in the competition directory | none |
| Evidence containing sensitive data | none committed; the ignore rule prevents it |

> **Known pre-existing working-tree items (NOT committed, NOT this phase's work):**
> `.git-msg.txt`, `COMPLETE_AUDIT_REPORT.html`, `run_tests.bat`, `vc_inspect.py`, `tasks/`,
> `docs/governance/RECONCILIATION_FORENSIC_REPORT.md`,
> `crates/zylcode-core/src/project/filesystem.rs.backup`, plus 7 modified tracked files. These are
> **foreign, pre-existing** entries. **They should be reviewed before the repository is made
> public** — they are not part of the competition work and were deliberately left untouched.

---

## 6. Attribution requirements for a public repository

Because the toolkit references N-ATLAS, a public README should carry:

> Uses **N-ATLaS**, developed by NCAIR (National Centre for Artificial Intelligence and Robotics)
> and Awarri Technologies with the Federal Ministry of Communications, Innovation and Digital
> Economy of Nigeria, under the N-ATLaS Open-Source Research and Innovation License.

This is a licence obligation, not a courtesy.

---

## 7. Verdict

| Gate | State |
|---|---|
| PUBLIC-1 — content audit clean | ✅ for the competition work; ⚠️ owner should review the pre-existing untracked files listed in §5 before publishing |
| PUBLIC-2 — open-source licence resolved | ✅ Apache-2.0 already present |
| PUBLIC-3 — `CONTRIBUTING.md` exists | ✅ |

**The branch is publishable after the owner reviews §5.** No push was performed.
