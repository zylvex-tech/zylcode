# ZylCode Strategic Plan v2.0

**Status:** ACTIVE — single source of truth for all strategic decisions
**Version:** 2.0
**Date:** September 2026

---

## Section 0: Three Blocking Questions — RESOLVED

Three decisions that were blocking forward progress are now resolved. This section exists so we never reconstruct them from conversation summaries.

### Question 1: Where does the plan live?

**Decision:** In the repository. `docs/STRATEGIC_PLAN.md`. Every strategic update goes through `git commit`. The plan is always the committed version in `main`, never a conversation artifact.

**Rationale:** Conversation context gets compressed, summaries lose nuance, and critical decisions disappear. The plan must be durable and reviewable. If the plan isn't in the repo, it doesn't exist.

### Question 2: What is the Phase 3 verification target?

**Decision:** MCP tool-permission gate.

**Specifically:** Prove soundness (if the gate says `Allow`, there exists a valid permission chain) and completeness (if there exists a valid permission chain, the gate says `Allow`) of the authorization decision.

**Chosen over:**
- Generic config validator (too broad, hard to define "valid")
- Tool-call replay (replay ≠ proof, doesn't cover unseen paths)

**Architecture implication:** The permission-check function must be a pure, side-effect-free function:

```
fn(agent_id, tool_id, session_context) -> Decision
```

where `Decision` is a typed enum (not a boolean). This shape must be established in Phase 10 (Section 6.2, action #2) before Phase 11 begins.

**What this does NOT cover (by design):**
- Whether the agent *should* call the tool (safety/reasoning — that's Rung 4 territory)
- Whether the tool *exists* (registry concern, orthogonal)
- Whether the tool *succeeds* (execution concern, orthogonal)

### Question 3: How do we handle CI billing lock?

**Decision:** Proceed with code work. Every commit message includes the line: `CI: pending (account billing lock)`. No release is tagged "verified" until CI is actually green. Local `cargo test --workspace` output is pasted in full as a substitute.

**Rationale:** The billing lock is a binary external dependency. We don't know when it unlocks. We don't stop shipping because of it. But we don't pretend CI is green when it isn't.

---

## Section 1: The Problem

ZylCode is an open-source AI coding agent. The market is crowded. Every tool claims to be "fast," "accurate," and "secure." Users have no way to verify these claims. Trust is based on marketing, not evidence.

The fundamental problem: **there is no objective, auditable proof that an AI coding agent does what it says it does.**

### What "verified" actually means (and what it doesn't)

**"Verified" in our context means:**
- A specific claim has a formal proof or reproducible benchmark behind it
- The proof/benchmark is versioned alongside the code it validates
- Any user can re-run the proof/benchmark and get the same result
- The claim is scoped — it says exactly what is proven and what is not

**"Verified" does NOT mean:**
- "We tested it and it seems to work" (that's QA, not verification)
- "It passed our internal benchmarks" (that's marketing without public evidence)
- "It's formally verified" (we use this precisely and only when it's literally true)

**The gap we're filling:** Not "is this code correct?" (that's software testing), but "can we prove to a third party that this agent's behavior matches its claims?" (that's verification-as-a-product).

---

## Section 2: Our Advantage (Honest Assessment)

**What we actually have that others don't:**

1. **Existing 3.3–8.3 implementation** — provider routing, failover, caching, compression, vector search, fault injection, telemetry, config hot-reload, benchmarks. This is real, tested, shippable infrastructure. Not a prototype.

2. **Rungs 1–2 are trivially buildable** — we already have the pieces. Structured logs + attestation manifest + a CLI command to produce a verification artifact. This is glue work, not research.

3. **The permission-check function shape is known** — we know what the pure function looks like (Section 0.2). We can implement it as a typed enum now and prove properties over it later.

4. **BYOK at all tiers** — this is a real cost advantage. Provider routing already handles multiple backends. We don't need to become a model provider.

**What we don't have (honest gaps):**

1. **No CI/CD that works right now** — billing lock. We have the workflow; we can't run it.
2. **No Z3/Dafny proofs yet** — Rung 4 is research, not engineering. It may not ship for months.
3. **No formal verification of the Rust codebase** — we're not a verified-Rust project. We're a project that produces verification artifacts *about* its behavior.
4. **No user trust yet** — trust is earned by shipping. Every phase that completes is evidence.

---

## Section 3: The Verification Ladder (Rungs 1–4)

The core product narrative: ZylCode doesn't just *claim* to be verified — it *produces evidence* that you can inspect.

### Rung 1: Structured Logs (Hours of Work)
**What:** JSON-structured, append-only logs with provenance metadata (timestamp, agent version, tool ID, session ID, decision outcome).

**Proof:** Every log entry is self-describing and parseable. No hidden state. No opaque binaries.

**Output:** `zylcode audit --format=json` produces a machine-readable log of every tool permission decision.

**Status:** Not started. Implementation is straightforward given existing telemetry infrastructure.

### Rung 2: Attestation Manifest (Days of Work)
**What:** A signed manifest that lists every claim the agent makes about its behavior, the evidence behind each claim, and the version of the code that produced the evidence.

**Proof:** The manifest is a DAG — each claim references evidence, each evidence references a code version, each code version references a commit. Tampering with any node breaks the chain.

**Output:** `zylcode verify --output=manifest.json` produces a self-contained attestation file.

**Status:** Not started. Requires the `telemetry:verification_rung` event type (Section 6.4).

### Rung 3: Permission Gate (Weeks of Work)
**What:** A pure function that makes tool-permission decisions. Sound: if it says `Allow`, there's a valid permission chain. Complete: if there's a valid chain, it says `Allow`.

**Proof:** The function is side-effect-free. Its behavior is fully determined by its inputs. Any deviation is a bug, not an emergent property.

**Output:** The function exists in code. Properties over it are tested. The proof is that the implementation matches the specification.

**Status:** COMPLETE (Phase 11, commit `98ed76c`). Pure permission-check function, property tests, CLI subcommands, and kind-correctness enforcement all shipped. 158 tests passing.

### Rung 4: Z3/Dafny Proofs (Months of Work)
**What:** Machine-checkable proofs that specific properties hold across all possible inputs.

**Proof:** A trusted checker (Z3, Dafny) confirms the proof. Not a human reading code — a machine reading math.

**Output:** Proof files that can be re-checked by anyone with the toolchain.

**Status:** Not started. This is research territory. May not ship for a long time.

---

## Section 4: Immediate Technical Additions (NOT Gated on Phases)

These go into the codebase now, near-zero cost, because they set up the infrastructure for later phases.

### 4.1: `telemetry:verification_rung` Event Type
Add to the existing telemetry system. A new event type that records which rung of verification produced a given piece of evidence. This is just a new enum variant — no behavioral change.

### 4.2: Security-Scan MCP Tool
A tool that runs `cargo audit` and `cargo deny` against the current workspace and returns structured results. This is a thin wrapper around existing Cargo subcommands.

### 4.3: CLI Headless Entry Point
A `zylcode verify --headless` mode that produces verification artifacts without a TUI. For CI pipelines and scripting.

### 4.4: Fault-Injection Results Publication
Package the existing 16 fault-injection tests + 19 telemetry-audit tests as a standalone benchmark report. Publish as a CI artifact when CI works again.

---

## Section 5: Monetization Model

**Principle:** Monetization is gated on real shipping progress. No pricing page until Phase 10 ships. No revenue projections until Phase 11.

### Tiers

| Tier | Price | What's Included | Target |
|------|-------|-----------------|--------|
| **Free** | $0 | Rungs 1–2, BYOK, community support | Individual developers, OSS contributors |
| **Pro** | $9/mo flat | Rung 3, managed API key, priority support | Solo developers, small teams evaluating |
| **Team** | $19/seat | Audit trail, governance controls, SSO | Teams with compliance requirements |
| **Enterprise** | Custom | Self-hosted, contractual verification SLA, dedicated support | Regulated industries |

### Cost Advantage: BYOK at All Tiers

ZylCode routes to your API key (OpenAI, Anthropic, local Ollama, anything). We don't markup your model costs. You pay the provider directly. Our subscription is for the verification infrastructure, not the tokens.

This is a structural advantage over tools that bundle API access with a markup. We never touch your tokens.

### Pricing Gate

**Do not publish pricing until Phase 10 ships.** Phase 10 produces the first verification artifact (Rungs 1–2). That's the minimum viable proof that we can deliver what we claim.

---

## Section 6: Roadmap

### Phase 9: Plan Commit + README Fix

**Gate:** Plan documents in `main`. README reflects real state of the project.

**Actions:**
1. Commit this strategic plan to `docs/STRATEGIC_PLAN.md` ← **YOU ARE HERE**
2. Implement the `Decision` enum and pure permission-check function shape (Section 0.2)
3. Add `telemetry:verification_rung` event type (Section 4.1)
4. Rewrite README to reflect real Phase 3.3–8.3 progress with specifics
5. Fix CI badge to show actual status (not misleading "passing" when CI is locked)
6. Remove all remaining overclaims ("formally verified" → "structured verification")

**Exit criteria:** Plan in repo. README honest. Permission-check function shape established. CI badge accurate.

### Phase 10: Rungs 1–2 (First Shippable Verification)

**Gate:** First verification artifact produced and documented.

**Actions:**
1. Structured audit log (Rung 1): JSON logs with provenance metadata
2. Attestation manifest (Rung 2): signed DAG of claims → evidence → versions
3. `zylcode audit --format=json` CLI command
4. `zylcode verify --output=manifest.json` CLI command
5. Documentation: "How to read a verification artifact"
6. Blog post: "What 'verified' actually means"

**Exit criteria:** A user can run `zylcode verify` and get a machine-readable attestation file.

### Phase 11: Rung 3 (Permission Gate)

**Gate:** Pure permission-check function with property tests.

**Actions:**
1. Implement the pure permission-check function (Section 0.2)
2. Property tests: soundness (Allow → valid chain), completeness (valid chain → Allow)
3. `zylcode security-scan` MCP tool (Section 4.2)
4. `zylcode verify --headless` mode (Section 4.3)
5. Publish fault-injection results as benchmark (Section 4.4)

**Exit criteria:** `cargo test --workspace` passes. Property tests demonstrate soundness and completeness. Security scan produces structured output.

**Status:** COMPLETE (commit `98ed76c`). All 5 actions shipped: `check_permission` with property tests (9 proptests), `security-scan` CLI subcommand, `verify --headless` CLI subcommand, fault-injection benchmark docs, and `validate_artifact_kind` kind-correctness enforcement. 158 tests passing across all crates.

### Phase 12: Rung 4 (Z3/Dafny Proofs)

**Gate:** Machine-checkable proof for at least one property.

**Actions:**
1. Identify the most valuable property to prove formally (likely: "the permission gate never grants access to an unregistered tool")
2. Write the proof in Z3 or Dafny
3. CI check that re-runs the proof on every commit (when CI works)
4. Documentation: "How to re-verify the proofs"

**Exit criteria:** A `.smt2` or `.dfy` file exists. `z3 --smt2 proof.smt2` returns `unsat` (proven). Anyone with Z3 can reproduce it.

**Note:** This phase may take months. It is research, not engineering. We do not block other work on it.

### Phase 13: Offline Mode

**Gate:** Agent runs without network access, using local models.

**Actions:**
1. Ollama integration (already partially exists via BYOK routing)
2. Offline-capable tool execution (no cloud dependencies)
3. Local verification artifact generation (no external signing service)
4. Documentation: "Running ZylCode in air-gapped environments"

**Exit criteria:** `zylcode verify --offline` produces a valid attestation using only local resources.

### Phase 14: Cost/Rung UI + Benchmark

**Gate:** Users can see verification rung status and cost breakdown in the UI.

**Actions:**
1. Desktop UI shows current verification rung (1–4) and what it means
2. Cost breakdown: tokens used, provider, per-session cost
3. Benchmark suite: reproducible performance numbers
4. Public benchmark page: `zylcode benchmark --output=report.json`

**Exit criteria:** Desktop app shows verification status and cost. Benchmark results are reproducible.

---

## Section 7: Decision Log

| Date | Decision | Rationale | Reversible? |
|------|----------|-----------|-------------|
| Sep 2026 | Plan lives in repo, not conversation | Durability, reviewability, no lost context | Yes (move it) |
| Sep 2026 | Phase 3 target = permission gate | Concrete, provable, narrow scope | Yes (change target) |
| Sep 2026 | CI lock = proceed, commit with note | Don't stop shipping for external dependency | Yes (revert policy) |
| Sep 2026 | Pricing gate = Phase 10 ships | Don't sell promises; sell shipped code | Yes (change gate) |
| Sep 2026 | BYOK at all tiers | Structural cost advantage, no token markup | Yes (add markup later) |

---

## Appendix A: Phase Mapping

Old numbering (conversation) → New numbering (this plan):

| Old Phase | New Phase | What |
|-----------|-----------|------|
| Phase 0 | Phase 9 | Plan Commit + README Fix |
| Phase 1 | Phase 10 | Rungs 1–2 |
| Phase 2 | Phase 11 | Rung 3 |
| Phase 3 | Phase 12 | Rung 4 |
| Phase 4 | Phase 13 | Offline Mode |
| Phase 5 | Phase 14 | Cost/Rung UI + Benchmark |

Existing completed work (Phases 3.3–8.3) is not renumbered. It's done. It's in `main`. It's tested. It ships.

---

## Appendix B: What "CI: pending" Looks Like

Every commit from now until CI is unlocked includes this footer:

```
CI: pending (account billing lock)
```

This is not a workaround. It's transparency. When CI is unlocked, we remove the footer and the badge goes green. No pretense.
