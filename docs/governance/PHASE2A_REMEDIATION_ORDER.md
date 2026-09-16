# Phase 2A Remediation Order

**Issued:** 2026-09-16
**Issued to:** Implementation agent (DeepSeek / Codex / ZCode)
**Phase:** 2A — Repository Intelligence Foundation
**Status of phase:** ❌ **NOT ACCEPTED — RE-OPENED**
**Audit:** `docs/governance/PHASE2A_INDEPENDENT_AUDIT.md`

> **This document is a complete, self-contained work order.**
> Copy §1 and §2 into the agent's prompt. The agent does not need this conversation.

---

## 1. Mandatory Preamble (paste verbatim, first)

> Read the ZylCode Constitution, Architecture, Master Execution Plan, Capability Model and Proof
> Graph, plus the current phase specification. They govern implementation. Where implementation
> conflicts with documentation, investigate the discrepancy rather than silently choosing one.
>
> Read `docs/governance/PHASE2A_INDEPENDENT_AUDIT.md` in full. It defines this work order.

Documents:

```
docs/governance/ZYLCODE_PRODUCT_CONSTITUTION_V2.md
docs/governance/ZYLCODE_ARCHITECTURE_V2.md
docs/governance/ZYLCODE_MASTER_EXECUTION_PLAN.md
docs/governance/ZYLCODE_CAPABILITY_MODEL.md
docs/governance/ZYLCODE_PROOF_GRAPH.md
docs/governance/ZYLCODE_AGENT_OPERATING_PROTOCOL.md
docs/governance/PHASE2A_INDEPENDENT_AUDIT.md   ← the audit you are remediating
docs/roadmap/ZYLCODE_ROADMAP_V2.md
docs/architecture/INTELLIGENCE_GRAPH.md
```

---

## 2. The Work Order

### 2.1 What happened

Phase 2A (`0ecea8e`) delivered a real, well-structured `intelligence` module with 79 passing unit
tests. An independent audit then re-ran its benchmark and inspected its evidence.

**The audit rejected it.** The module is not integrated, its headline metric measured build
output, and its benchmark fails on re-execution.

**The code is not being thrown away.** The audit explicitly preserves the module design, the
canonical types, and the `Provenance` / `ContextResult.reason` modelling. This is a repair, not
a rewrite.

### 2.2 Why this matters

Phase 2B builds reasoning on top of this index. Reasoning over an index that is 98.9% build
artifacts produces **confidently wrong** answers — which is worse than no answers. Phase 2B is
blocked until this is fixed.

---

## 3. P0 — Blocking. Phase 2A cannot be re-submitted without these.

### P0.1 — Fix path exclusion

**Defect.** `should_exclude()` in `crates/zylcode-core/src/intelligence/classifier.rs`:

```rust
let excluded = ["/target/", "/node_modules/", "/dist/", "/build/", "/.git/", ...];
let path_str = path.to_string_lossy();
for exc in &excluded {
    if path_str.contains(exc) { return true; }
}
```

On Windows — the **primary target platform** — `to_string_lossy()` yields
`C:\Projects\zylcode\target\debug\...`. None of the forward-slash patterns can ever match. The
exclusion list is **dead code on Windows**.

The `.gitignore` fallback does not rescue it: the real `.gitignore` contains `/target` and
`**/target`, neither of which `matches_gitignore_pattern()` handles (leading `/` and `**/`).

**Required fix.**

1. Normalise paths before matching, or match on `Path::components()` — never on raw string
   containment.
2. Replace the hand-rolled gitignore matcher with the **`ignore`** crate (same author as
   `walkdir`, used by ripgrep). Do not hand-roll this again.
3. **Test against the real repository `.gitignore`**, not a synthetic fixture.

**The specific trap to avoid.** `scanner.rs::scan_excludes_target` passes today because its
fixture writes a `.gitignore` containing `target/` — a bare directory pattern the naive matcher
*does* handle. The real repository uses `/target` and `**/target`. **The fixture differed from
reality, so a green suite certified a broken scanner.** Add a test that reads the actual
`.gitignore` from the repository root.

**Verification.**

```bash
# The decisive test: index the real repo and assert scope
# No indexed path may contain target/, node_modules/, or .git/
```

### P0.2 — Re-baseline every metric, visibly

**Defect.** Reported "Files indexed: 14,973". Measured 15,064, of which **14,900 are under
`target/`** (98.9%). The repository is **254 files** (`git ls-files` = 193).

**Required.**

1. Re-run the benchmark against a correctly-scoped index.
2. Report the new `files_indexed`, precision and recall. **Expected genuine index size: ~254 files.**
3. Correct every figure in `docs/PHASE2A_COMPLETION_REPORT.md` and
   `docs/capability-registry.json`.
4. **Corrections must be visible as corrections** — a correction note, not a silent edit
   (Constitution §3.2 rule 5).

### P0.3 — Make the benchmark deterministic and self-enforcing

**Defect.** The suite asserts only `file_count() > 50`, `symbol_count() > 20`,
`package_count() >= 3`. A scan indexing **51 files** would print "PASS". None of the headline
numbers are asserted, so none are protected against regression. Results also vary between runs
(reported P@10 0.30 / R@10 0.53 vs. observed 0.31 / 0.58; Q6 differed entirely).

**Required.**

1. Assert `files_indexed` within a tolerance band of the real count.
2. Assert symbol count.
3. **Assert index scope explicitly**: no indexed path may contain `target/`, `node_modules/`, or `.git/`.
4. **Split the suites**: performance and retrieval quality must be separate tests, so a slow
   machine does not red-flag correctness.
5. Make the retrieval benchmark deterministic (fixed inputs, stable ordering, no environment
   dependence).

### P0.4 — Machine-independent timing budget

**Defect.** Reported 12.7s. Measured **276.1s** — 22× slower — and the assertion
`scan_duration.as_secs() < 120` **fails**.

**Required.**

1. Express the budget in **files/second**, not wall-clock seconds.
2. Parallelise the scan. Serially reading and SHA-256 hashing 15K files is the bottleneck. With
   the exclusion fixed the working set drops to ~254 files, but parallel scanning is still the
   right design.
3. Do not ship a threshold that passes on the author's machine and fails on a reviewer's.

### P0.5 — Produce a real acceptance demonstration

**Defect.** The completion report's "Acceptance Demonstration" is headed **"Expected Output"** and
contains a hand-written narrative of what the system *would* answer. There is no command, no
captured stdout, no artifact. That is a specification of intended behaviour in the position where
evidence belongs.

**Required.** A committed transcript containing:

- the exact command(s)
- environment (OS, toolchain versions)
- **raw captured output** (verbatim, not summarised)
- the commit SHA

A third party must be able to replay it verbatim and obtain the same result.

### P0.6 — Correct the capability registry

**Defect.** `docs/capability-registry.json` records 12 Phase 2A capabilities as **GREEN**, on the
basis of unit tests and a benchmark that does not reproduce. `repository_scanner`'s evidence
string embeds the mis-scoped "14,973 files" figure.

**Required.** Either raise the capabilities to genuine **R3** (P1.7), or downgrade them to
**PARTIAL / R2**. Both are acceptable. **Claiming GREEN without either is not.**

Add a `rung` field to every entry (schema in `ZYLCODE_CAPABILITY_MODEL.md` §5).

---

## 4. P1 — Required for the capability to be honest.

### P1.7 — Integrate, or downgrade

**Defect.** Zero product integration:

```
grep -rn "intelligence::" --include=*.rs crates/ apps/ \
  | grep -v "src/intelligence/" \
  | grep -v "tests/repo_intelligence_benchmark.rs"
→ (no output)
```

`crates/zylcode-core/src/lib.rs:14` declares `pub mod intelligence;` and nothing else in the
workspace consumes it. `agent.rs` still uses the pre-existing `context_builder::ContextBuilder`
(`agent.rs:14, 223, 315, 666–687`). There is no CLI command, no UI surface, no agent tool.

By the reachability test (`ZYLCODE_PROOF_GRAPH.md` §2.1), this is **R2 — a library, not a
capability.**

**Required — pick one, and say which:**

- **(a) Integrate.** Wire repository intelligence into `AgentLoop`'s context assembly and expose
  at least one CLI entry point (e.g. `zylcode repo context <task>`). Then capture a transcript of
  it working.
- **(b) Downgrade.** Record the 12 capabilities as `PARTIAL` / `R2` in the registry and state
  plainly that Phase 2A delivered a tested library, not a user-reachable capability.

### P1.8 — Raise the acceptance thresholds

With a clean index, `Precision@10 ≥ 0.25` is not a meaningful bar — a keyword search could pass it.

**Required.**

1. Fix Q5 (test file discovery), Q6 (reverse dependency) and Q10 (build command discovery), which
   currently score **0.00**.
2. Require `Precision@10 ≥ 0.60`.
3. Expand to **≥ 20** known-answer questions.
4. The bar must be one that **`grep` would fail**. Otherwise the graph adds nothing over `grep`.

### P1.9 — Resolve the failing test

**Defect.** `cargo test --workspace` → `225 passed; 1 failed`:

```
router::tests::synthetic_offline_dispatch_returns_parseable_payload
  panicked at crates\zylcode-core\src\router.rs:1030:9:
  assertion failed: text.contains("<zylcode-response>")
```

The synthetic offline provider does not emit the `<zylcode-response>` envelope the router's own
test requires. This breaks the **offline / air-gapped** path (`docs/AIR_GAPPED.md`) — a path that
matters for a local-first product.

The audit could not determine whether this predates `0ecea8e`, because the working tree was dirty
(see P1.10). Fix it, and state plainly whether it was pre-existing.

### P1.10 — Commit or discard the working tree

**Defect.** 53 modified tracked files, 36 untracked. This is not a reviewable state — it is why
the audit could not attribute the failing test above.

**Required.** Commit or discard. Leave the tree clean before submitting for re-audit.

### P1.11 — Fix the completion report's self-consistency

**Defect.** The report was committed containing:

```
## Commit SHA
(Pending — to be committed after final verification)

## Push Verification
(Pending — to be pushed after commit)
```

A certificate that predates the thing it certifies is void.

**Required.** Fill these in **before** committing the report, or drop the fields.

---

## 5. P2 — Process.

### P2.12 — Reproduction block in every report

From this point on, every completion report carries a reproduction block
(`ZYLCODE_AGENT_OPERATING_PROTOCOL.md` §5). A report without one is a draft.

---

## 6. Constraints — what NOT to do

| # | Constraint |
|---|---|
| 1 | **Do not begin Phase 2B.** It is blocked until Phase 2A is re-accepted. |
| 2 | **Do not modify the governance documents** in `docs/governance/` to make your work correct. If you believe a governance document is wrong, **report the discrepancy** (Constitution §9.2). |
| 3 | **Do not delete or rewrite the audit.** It is a record. Corrections go in new documents. |
| 4 | **Do not silently edit published numbers.** Corrections are visible. |
| 5 | **Do not rewrite the intelligence module from scratch.** The design is sound; the audit preserves it. |
| 6 | **Do not weaken or delete a failing test to make the suite green.** |
| 7 | **Do not report COMPLETE / ACCEPTED / DONE.** That is the auditor's word. |
| 8 | **Do not commit an "Expected Output" section** where evidence belongs. |

---

## 7. Required Report Format

Submit as `docs/PHASE2A_COMPLETION_REPORT.md` (replacing the rejected version, which is preserved
in git history).

```markdown
# Phase 2A Completion Report (Re-submission) — Repository Intelligence Foundation

## 1. Status
READY FOR AUDIT
(Not "COMPLETE". Not "ACCEPTED".)

## 2. What changed since the rejected report
Point-by-point against P0.1–P0.6, P1.7–P1.11.

## 3. Corrections to previously published figures
| figure | previously published | corrected | why it was wrong |

## 4. Reproduction block
### Environment
<OS, cargo/rustc versions, commit SHA>
### Commands
<exact, copy-pasteable>
### Raw output
<verbatim captured output — not a summary, not an expectation>

## 5. Index scope evidence
### Indexed file count: <N>
### Proof of scope
<output showing no indexed path contains target/, node_modules/, or .git/>

## 6. Test results
<verbatim output including the summary line>
<the previously failing test's status, and whether it was pre-existing>

## 7. Benchmark results
### Retrieval quality
<Precision@K, Recall@K, per-question, with the assertion thresholds shown>
### Performance
<files/second, with the machine-independence rationale>

## 8. Integration
<either: the entry points added + a transcript; or: the downgrade applied to the registry>

## 9. Capability registry changes
<per-capability status and rung, with the evidence for each>

## 10. Discrepancies found
<anywhere implementation and documentation disagreed, and what you did about it>

## 11. Limitations
<mandatory, non-empty>

## 12. Known failures
<including anything pre-existing. Do not omit.>

## 13. Requested audit
Specifically: try to falsify the index scope, the benchmark thresholds, and the entry points.
```

---

## 8. Definition of Done

Phase 2A is re-submittable when **all** of the following are true:

- [ ] `should_exclude()` correct on Windows, verified against the **real** `.gitignore`
- [ ] Indexed file count is in the low hundreds, not fifteen thousand
- [ ] Index scope is **asserted in a test**
- [ ] Benchmark is deterministic and fails when a metric regresses
- [ ] Benchmark passes on a machine other than the author's, with a files/second budget
- [ ] Reproduction block present, with raw output and a filled-in SHA
- [ ] Every previously published figure either corrected or re-verified, visibly
- [ ] Capability registry consistent with `ZYLCODE_CAPABILITY_MODEL.md` §4.2 (GREEN ⇒ R3)
- [ ] Every listed capability has a named, committed entry point — **or** is recorded PARTIAL/R2
- [ ] `cargo test --workspace` fully green
- [ ] Working tree clean
- [ ] No governance document modified
- [ ] Phase 2B not started

---

## 9. What Happens Next

1. Agent submits the re-submission report.
2. Independent auditor re-runs the benchmark, recomputes the metrics, invokes the entry points,
   and attempts to falsify the rung (`ZYLCODE_CAPABILITY_MODEL.md` §8).
3. Verdict: **ACCEPTED** / **ACCEPTED WITH CONDITIONS** / **NOT ACCEPTED (re-opened)**.
4. **Only on ACCEPTED** does Phase 2B begin.

The audit's stance is falsification, not confirmation. Assume the benchmark will be re-run, the
scope recomputed, and the entry points invoked.
