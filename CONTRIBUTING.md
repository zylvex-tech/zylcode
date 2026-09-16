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

- **Rust** toolchain current stable (see `rust-toolchain*` if present).
- **Node + npm** for the desktop frontend (`apps/zylcode-desktop`).
- **Tauri v2** for the desktop shell.
- Build: `cargo build` (core/cli/mcp) and `npm run tauri dev` (desktop).

Do **not** add a dependency unless the dependency order in the architecture requires it. WASM, async
runtimes, and serialization choices are governed, not conventional.

---

## 2. Formatting

- Rust: `cargo fmt` (stable style). Run before every commit.
- TypeScript/React: the repo's formatter (Prettier-config-per-package). Run before commit.
- Keep commits small and single-purpose. A commit that mixes formatting with behaviour is harder to
  audit and will be asked to be split.

---

## 3. Testing

- Rust unit/integration: `cargo test`.
- Frontend: the repo's test runner.
- **Tests must assert effect, not success.** A test that asserts `is_ok()` on an action it claims to
  verify is not a test of that action (see the Computer-Use Engine spec, §4.1). A test that proves a
  behaviour against a fixture that differs from reality certifies a broken implementation — this has
  happened; do not repeat it.
- Do not weaken a failing test to make a run green. Report the failure.

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
