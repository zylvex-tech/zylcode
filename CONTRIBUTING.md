# Contributing to ZylCode

Thank you for your interest in contributing. ZylCode is an evidence-first software creation OS; its
governing principle is that **claims must be reproducible and builders do not certify their own work**.
That principle applies to contributions as much as to the product.

---

## 0. Read First (required before any change)

| Document | Why |
|---|---|
| `docs/governance/ZYLCODE_PRODUCT_CONSTITUTION_V2.md` | The supreme spec. Read before proposing anything architectural. |
| `docs/governance/ZYLCODE_ARCHITECTURE_V2.md` | System topology and contracts. |
| `docs/governance/ZYLCODE_MASTER_EXECUTION_PLAN.md` | The 16-phase / 6-epoch program and gates. |
| `docs/governance/ZYLCODE_CAPABILITY_MODEL.md` | What a capability is and how status is computed. |
| `docs/governance/ZYLCODE_PROOF_GRAPH.md` | R0–R5. The definition of "proven". |
| `docs/governance/ZYLCODE_AGENT_OPERATING_PROTOCOL.md` | How agents work here; the pre-flight, marking and reporting rules. |
| `docs/governance/ZYLCODE_PUBLIC_COMMUNICATION_POLICY.md` | What may be said publicly (status vocabulary, screenshot rule, claim matrix). |
| `docs/roadmap/ZYLCODE_ROADMAP_V2.md` | Per-phase detail and entry points. |

A change that contradicts a governing document is a **defect to be recorded**, not silently resolved
in either direction. Investigate the discrepancy first; propose an amendment if the document is wrong.

---

## 1. Development Environment

| Tool | Version | Source of the requirement |
|---|---|---|
| Rust | stable; MSRV **1.90** | `Cargo.toml` `rust-version = "1.90"`, `edition = "2021"`. No `rust-toolchain*` file, so stable is implied. |
| Node | **22** (floor is 18) | `package.json` `engines.node = ">=18.0.0"`, but `apps/natlas-sdk/test/smoke.mjs` is TypeScript run directly via `node --experimental-strip-types`, which needs **≥ 22.6**. CI pins 22. |
| pnpm | **9** (9.12.0) | `package.json` `packageManager = "pnpm@9.12.0"`. CI pins 9. |
| Tauri | v2 | desktop shell (`apps/zylcode-desktop`). |

```bash
git clone https://github.com/zylvex-tech/zylcode.git && cd zylcode
pnpm install --frozen-lockfile      # the lockfile is committed; a failure means it drifted
cp .env.example .env                # then edit — see below
```

**Environment variables.** Start from `.env.example`. Every entry in it was validated against the
source that reads it, and each carries the exact `file:line` read site plus the default the code
falls back to when the variable is unset. **Nothing in `.env.example` is required** — ZylCode runs
fully offline by default, and with no provider credential configured the router short-circuits to
its deterministic synthetic provider.

Two things worth knowing before you set a provider key:

- Provider credentials are **not** test-neutral. `RouterConfig::default()` sets
  `fallback_provider: LocalOllama`, which makes the router's no-keys fast path inapplicable, so a
  dispatch with no configured credential still attempts the primary provider and then its Ollama
  fallback. With `OPENROUTER_API_KEY` (or `ANTHROPIC_API_KEY` / `DEEPSEEK_API_KEY`) set,
  `cargo test` performs real HTTP requests. **Leave them unset for a hermetic offline run.**
- `.env` is git-ignored (`.gitignore` lines 35-37). `.env.example` is the committed template and
  must never contain a real value.

Do **not** add a dependency unless the dependency order in the architecture requires it. WASM, async
runtimes, and serialization choices are governed, not conventional.

---

## 2. Formatting

Run before every commit. Both checks are enforced in CI on every push and pull request.

```bash
cargo fmt --all -- --check                                  # Rust — hard failure on all 3 OSes
pnpm --filter zylcode-desktop format:check                  # frontend (Prettier, read-only)
```

To apply rather than check: `cargo fmt --all` and
`pnpm --filter zylcode-desktop format`. Prettier's config lives in
`apps/zylcode-desktop/.prettierrc.json`.

Keep commits small and single-purpose. A commit that mixes formatting with behaviour is harder to
audit and will be asked to be split.

---

## 3. Testing

```bash
# Rust
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings

# Coverage (the CI gate: measured 80.63% lines against a 78% floor, 2026-10-21)
cargo llvm-cov --workspace --summary-only --fail-under-lines 78

# Frontend (apps/zylcode-desktop)
pnpm --filter zylcode-desktop lint          # ESLint 9 flat config; exits non-zero on errors only
pnpm --filter zylcode-desktop typecheck
pnpm --filter zylcode-desktop test          # vitest
pnpm --filter zylcode-desktop build

# N-ATLAS SDK
pnpm --filter @zylvex/natlas-bridge typecheck
node --experimental-strip-types apps/natlas-sdk/test/smoke.mjs   # needs Node >= 22.6
```

**Offline CLI verification.** `repo-context` runs the real Repository Intelligence pipeline with no
network access and is the fastest end-to-end check that the workspace indexes and ranks correctly:

```bash
cargo run -p zylcode-cli -- repo-context "which files would likely need inspection to modify crash recovery"
```

(If the toolchain cannot link in your environment, an already-built
`target/debug/zylcode.exe repo-context "…"` runs the identical code path.)

Expected shape: `indexed: <n> files, <n> symbols, <n> packages, <n> entry points`, then a ranked list
whose entries cite their evidence — including `changes together with '<path>' in git history` for
co-change-backed results. **Those citations require full git history**; a shallow clone
(`--depth 1`) silently drops them and changes the ranking.

- Rust unit/integration: `cargo test`.
- **Tests must assert effect, not success.** A test that asserts `is_ok()` on an action it claims to
  verify is not a test of that action (see the Computer-Use Engine spec, §4.1). A test that proves a
  behaviour against a fixture that differs from reality certifies a broken implementation — this has
  happened; do not repeat it.
- Do not weaken a failing test to make a run green. Report the failure.
- **Do not assert on process-global state from a parallel test.** `cargo test` runs tests on
  multiple threads in one process. A counter or environment variable shared across tests will be
  attributed to the wrong test — `NETWORK_EGRESS_COUNT` in `crates/zylcode-core/src/router.rs` was
  exactly this bug, and is now thread-local for that reason. Tests that mutate the environment must
  hold `DISPATCH_MUTEX`, which is private to that module.

---

## 4. Evidence Requirements

Any capability claim must follow the Proof Graph:

- **R2 (EXECUTED)** needs a tested, reachable behaviour.
- **R3 (VERIFIED)** needs a named, committed entry point plus captured evidence (a transcript or
  artifact). Reachability is part of correctness — an unreachable subsystem is not a capability.
- **Never fabricate:** runtime evidence, test results, benchmark results, installation success,
  release availability, product capabilities, user counts, performance measurements, security/compliance
  claims, customer testimonials, screenshots, pricing availability, or GitHub/community statistics.

A claim without a reproduction block (commands, environment, raw output, SHA) is not a fact.

---

## 5. Capability Registry

Capabilities are registered in `docs/capability-registry.json`. Adding one requires:
- a `rung` derived from evidence (never asserted by hand);
- `entry_points`, `scope`, `evidence_commit`, `verified_at`, `limitations`;
- a matching entry in the architecture status table.

See `docs/contributing/CAPABILITY_EVIDENCE_GUIDE.md`.

---

## 6. Prohibited in a Contribution

- **No fabricated completion reports.** State what was and was not done.
- **No unrelated refactors.** Stay within the scope of the issue/phase. Drive-by cleanups are a separate PR.
- **No claims above the implemented rung.** If it is R2, say R2.
- **No phase-number reuse** for non-roadmap work. The roadmap owns the numbers; use named tracks
  (`PUBLIC-FOUNDATION-NN`, `DESIGN-SYSTEM-NN`, `CLOUD-PLATFORM`). See Agent Protocol §4.7.
- **No lifting statements from superseded/historical documents** as current capability evidence
  (Agent Protocol §4.8). `docs/STRATEGIC_PLAN.md` is quarantined.
- **No secrets.** Never commit API keys, MCP tokens, cloud credentials, or signing keys.

---

## 7. Commit & PR Expectations

- One logical change per commit; one concern per PR.
- PR description includes: objective, what changed, files changed, tests/checks run, **raw results**,
  known limitations, capability status/rung if applicable, and the commit SHA.
- Link the phase or named-track milestone the PR advances.
- A PR that cannot be reproduced from its own instructions is not mergeable.

---

## 8. Security Reporting

See `SECURITY.md`. Do **not** file public issues for suspected vulnerabilities.

---

## 9. Documentation Expectations

- New systems get a spec under `docs/architecture/` marked **PROPOSED** until implemented.
- Public-facing copy reads from the **Public Claim Matrix**; it never independently asserts a
  capability state.
- Design concepts published anywhere carry a visible `CONCEPT` / `PROPOSED` / `IN DEVELOPMENT` label.

---

## 10. Architecture Changes

Architecture changes require explicit rationale and cannot silently violate the Constitution. See
`docs/contributing/ARCHITECTURE_CONTRIBUTIONS.md`.

---

## Questions

Open a GitHub Discussion under **Architecture** or **Ideas** rather than a code PR for anything that
changes governing documents, the phase program, or the capability model.
