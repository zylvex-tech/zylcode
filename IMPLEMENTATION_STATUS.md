# ZylCode Implementation Status

> **Last updated:** 2026-10-23 · `main` @ `6469bae`
>
> For the complete project status including all 16 phases, see
> [`docs/PROJECT_MASTER_STATUS.md`](docs/PROJECT_MASTER_STATUS.md).

---

## Phase Status at a Glance

| Epoch | Phases | Status |
|---|---|---|
| **I — Trust & Understanding** | 1A, 1B, 1C, 1D, 2A, 2B | 4 ACCEPTED · **2A RE-OPENED** · 2B BLOCKED |
| **II — Project & Agent OS** | 3A, 3B, 4 | All BLOCKED on 2A |
| **III — Extensibility** | 5 | BLOCKED on 3A |
| **IV — See What You Build** | 6A, 6B, 7A–7D | All NOT STARTED |
| **V — Vision Studio** | 8A, 8B, 8C, 9 | All NOT STARTED |
| **VI — Devices, Delivery, Ecosystem** | 10–16 | All NOT STARTED |

**Critical path:** Phase 2A re-acceptance → 2B → 3A → everything else.

---

## Accepted Foundation (Phases 1A–1D)

These phases form the trusted kernel of ZylCode. They are implemented, tested,
and accepted.

### Real Tool Runtime (1A) ✅
- Real `FileSystemTool`, `ShellTool`, `GitTool`, `SearchTool`
- Evidence recording on every execution
- Risk classification: Read → Write → Execute → Destructive

### Agent Execution Loop (1B) ✅
- Iterative state machine: Analyzing → Planning → Executing → Verifying
- Observation feedback to model
- Bounded execution with step limits and timeouts

### Model-Driven Agent (1C) ✅ (with conditions)
- Formal `AgentDecision` protocol
- Approval enforcement for destructive operations
- Completion requires evidence

### Durable Memory & Recovery (1D) ✅ (with conditions)
- Append-first evidence ledger with SQLite persistence
- Crash recovery across 3 critical windows
- Hash-chain integrity (SHA-256)
- Fail-closed verification

---

## Blocked: Phase 2A — Repository Intelligence

**Status: NOT ACCEPTED (re-opened since 2026-09-16)**

The 7,224-line intelligence library exists and works as a library, but the
independent audit found:
- Metrics initially counted build artifacts
- Benchmark failed on re-execution
- No product surface (library only)

**Remediation:** P0 fixes applied (path exclusion, rebaseline, determinism).
**P1 items NOT STARTED:** integration into AgentLoop, quality threshold raises,
failing test fix.

This blocks Phase 2B and everything downstream.

See:
- [`docs/governance/PHASE2A_INDEPENDENT_AUDIT.md`](docs/governance/PHASE2A_INDEPENDENT_AUDIT.md)
- [`docs/governance/PHASE2A_REMEDIATION_ORDER.md`](docs/governance/PHASE2A_REMEDIATION_ORDER.md)
- [`docs/PHASE2A_COMPLETION_REPORT.md`](docs/PHASE2A_COMPLETION_REPORT.md)

---

## Recent Changes (2026-10)

### PR #28 — `serde-saphyr` migration (merged 2026-10-23)
Eliminated `serde_yaml` / `unsafe-libyaml` from the dependency graph. Replaced
with `serde-saphyr` 2.0.0 (`granit-parser`, pure Rust, `#![forbid(unsafe_code)]`).
- Preserved strict boolean semantics (`strict_booleans: true`)
- Preserved empty-document rejection
- Rewrote `manifest.rs` from untyped `Value` to typed struct
- Added 14 regression tests
- Bumped MSRV 1.77 → 1.89
- Resolved RUSTSEC-2024-0449

### PR #29 — `CODEOWNERS` (merged 2026-10-23)
Added repository ownership governance file.

### PR #24 — LRU advisory fix (merged)
Moved `lru` off 0.12.5 (two unsound advisories) and `tokio` off yanked release.

### PR #23 — Router egress spy fix (merged)
Scoped test-only egress spy per thread, not per process.

---

## Capability Registry Summary

From `docs/capability-registry.json` (v1.4.1-audited):

| Status | Count |
|---|---|
| GREEN | 22 |
| DISPUTED | 12 (all Phase 2A intelligence capabilities) |
| PARTIAL | 2 |
| EXTERNAL_BLOCKER | 1 (remote CI — GitHub billing) |

**Total:** 37 capabilities tracked.

---

## What Works Today

A user can:
1. **Install from source** (`git clone`, `pnpm install`, `cargo build`)
2. **Run the desktop app** (Tauri v2 + React)
3. **Configure providers** (BYOK: OpenAI, Anthropic, OpenRouter, Ollama)
4. **Ask the agent to do something** — real iterative loop with tool execution
5. **See real tool use** — filesystem, shell, git operations with evidence
6. **Approve destructive actions** — permission gate is real
7. **Save and resume sessions** — evidence ledger persists to SQLite

A user **cannot** (honest gaps):
- Browse repository intelligence from the UI (library exists, not wired)
- Run computer-use automation (R0 simulation only)
- Download a signed installer (release pipeline configured, not producing)
- Use multi-agent orchestration (not built)
- Access the design canvas (not built)

---

## Next Step

**Phase 2A P1 remediation.** See [`docs/PROJECT_MASTER_STATUS.md`](docs/PROJECT_MASTER_STATUS.md) §8.

1. Verify clean worktree compiles at `main` tip
2. Fix dirty-tree legacy (review/commit/discard 53 modified files)
3. Fix failing router test
4. Wire intelligence into AgentLoop context assembly
5. Raise benchmark thresholds
6. Request Phase 2A re-audit

---

## Verification

```bash
# Full workspace check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets

# Frontend
pnpm --filter zylcode-desktop build
pnpm --filter zylcode-desktop test

# Security
cargo audit                # 0 vulns, 2 accepted warnings
pnpm audit --prod          # clean

# Retracted-claim guard
python scripts/check_retracted_claims.py
```

**MSRV:** 1.89 (required by `serde-saphyr` 2.0.0)

---

> **Built with evidence. Verified with honesty.**
