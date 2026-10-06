# ZYLCODE AGENT EVALUATION MODEL

**Date:** 2026-10-06
**Status:** **DESIGNED. No evaluation harness exists.** §5 records the one exception.
**Covers directive phases:** 11 (software-engineering evaluation), 13 (security engineering).
**Substrate it would extend:** `first_mission.rs` (one end-to-end scenario), the test battery,
`claim`/`evidence_graph` (where results would be recorded).

---

## 0. Honesty statement

There is **no evaluation framework in this repository**. There is one end-to-end scenario (the First
Mission) and a large test battery. Neither is an evaluation framework:

* the test battery tests **the code**;
* the First Mission proves **one workflow works**;
* an evaluation framework measures **how well the agent behaves across many scenarios**, with
  repeatable fixtures and scored outcomes.

The directive's question — *"Can it diagnose a failing test?"* — is not answered by a passing test
suite. It is answered by a fixture that fails in a known way and a scored attempt to fix it.

**A critical prerequisite, stated up front:** most of the listed evaluations require a **live
model**, which is `BLOCKED` (AGENT-01). An evaluation framework can be *built* now, but its
model-dependent cases cannot be *scored* until a provider answers. Building the harness and
publishing empty scores would be a fabricated result — the exact failure this project exists to
avoid.

---

## 1. Why evaluation is first-class, not a QA afterthought

Every other capability in this factory produces evidence. Without evaluation, **the agent itself
produces none** — its competence is asserted, never measured. That is the largest remaining
epistemic hole in the product, because an agent that cannot be measured cannot be trusted with
autonomy, and autonomy is the whole point.

> *Acting ≠ having acted.* The same rule applies to competence: **a claim of capability that was not
> measured is a defect.**

---

## 2. The fixture model

An evaluation case is a **fixture plus an oracle**:

```jsonc
{
  "id": "EVAL-014",
  "capability": "diagnose_failing_test",
  "fixture": {
    "repo": "fixtures/statslib",
    "setup": ["git checkout base"],
    "injected_fault": { "file": "core.py", "replace": "…", "with": "…" }
  },
  "task": "The suite fails. Find and fix the cause.",
  "oracle": {
    "must_pass": ["pytest -q"],
    "must_not_touch": ["docs/**", "unrelated/**"],
    "must_cite": ["the failing assertion's file:line"],
    "budget": { "max_files_changed": 3, "max_steps": 30 }
  },
  "requires_provider": true
}
```

**Design rules:**

1. **The oracle is deterministic.** A scored outcome is decided by exit codes and file sets, never
   by a model judging a model.
2. **`must_not_touch` is load-bearing.** "Did it change the right file?" is only half the question;
   "did it leave everything else alone?" is the half that protects the owner's work.
3. **`requires_provider` is explicit.** A case that needs a model is marked, so the suite can report
   `SKIPPED — BLOCKED_PROVIDER` rather than silently passing or silently failing.

---

## 3. The evaluation set (directive Phase 11)

Each row states what is needed and what is available today.

| # | Question | Fixture needs | Provider? | Status |
|---|---|---|---|---|
| 1 | Can it diagnose a failing test? | fault-injected repo + failing suite | yes | **DESIGNED** |
| 2 | Can it modify the correct file? | oracle file set | yes | **DESIGNED** |
| 3 | Can it avoid unrelated files? | `must_not_touch` set | yes | **DESIGNED** |
| 4 | Can it detect insufficient evidence? | a task whose evidence is withheld | **no** | **DESIGNED — runnable now** |
| 5 | Can it refuse fabricated success? | a task whose only "success" is an assertion | **no** | **DESIGNED — runnable now** |
| 6 | Can it preserve dirty owner work? | a repo with pre-existing uncommitted edits | **no** | **DESIGNED — runnable now** |
| 7 | Can it recover after restart? | kill mid-task, resume | **no** | **DESIGNED — runnable now** |
| 8 | Can it follow an ADR? | an accepted ADR + a task that tempts a contradiction | yes | **DESIGNED** (needs decision memory) |
| 9 | Can it explain why a build failed? | a broken build + captured output | yes | **DESIGNED** |
| 10 | Can it identify a regression? | a repo with a known-good baseline | yes | **DESIGNED** |
| 11 | Can it generate a patch that passes tests? | the Best-of-N path | yes | **DESIGNED** |

**Five of eleven cases are runnable without a provider.** They are the ones that test *honesty*
rather than *intelligence* — and honesty is the property this product cannot ship without. **They
should be built first**, both because they are possible today and because they are the cases whose
failure would be most damaging.

---

## 4. Security engineering (Phase 13)

### 4.1 Status: no scanner exists

Verified: there is **no** secret scanner, SAST integration, dependency-vulnerability audit or
licence checker in `crates/`. The only automated check in CI is the retracted-claim guard, which is
a documentation check.

**Do not describe ZylCode as having security analysis.** It has a permission gate and path
containment, which are enforcement, not analysis.

### 4.2 The designed finding object

The directive requires each finding to carry severity, location, evidence, status, remediation and
verification:

```jsonc
{
  "id": "SEC-0007",
  "rule": "command_injection",
  "severity": "high",
  "location": { "file": "src/tools/shell.rs", "line": 118 },
  "evidence": [{ "kind": "tool_evidence", "id": "…" }],
  "status": "open",              // open | remediated | accepted_risk | false_positive
  "remediation": "…",
  "verification": { "state": "not_run" }
}
```

**The important field is `verification`.** A finding marked `remediated` without a re-scan is an
assertion, not a fact. The same fail-closed posture as `Claim::promote_verified` applies: a finding
reaches `remediated` only when a deterministic re-scan produced evidence.

### 4.3 The designed scan set

| Analysis | Approach | Status |
|---|---|---|
| secret detection | entropy + known-pattern scan over the workspace | **DESIGNED** |
| dependency vulnerability | lockfile + advisory source | **DESIGNED** |
| SAST | language-specific rules; tree-sitter is already a dependency | **DESIGNED** |
| dangerous API detection | AST query for `unsafe`, shell construction, `unwrap` on IO | **DESIGNED** |
| injection / traversal | already **enforced** at the tool boundary (`resolve_within`) — analysis would *audit* it | **PARTIAL** |
| supply-chain / licence | manifest + licence metadata | **DESIGNED** |

### 4.4 The gate rule

> *Do not automatically modify security-sensitive code without the correct risk gate.*

Architecturally: a security remediation is a factory task whose risk class is at least
`Destructive`-adjacent, so it exceeds any default ceiling and **parks for human approval**. The
`SECURITY_REVIEWER` role can *find*; only a human can *authorise the fix*.

---

## 5. What exists today that resembles evaluation

| Piece | What it actually is | What it is not |
|---|---|---|
| `first_mission_e2e.rs` | one end-to-end scenario with a real failure, repair, kill/resume | not a scored framework |
| `first_mission_kill_resume.rs` | a real `abort()` + resume | not a general recovery evaluation |
| the test battery (575 tests, 2026-09-28) | tests of the code | not tests of agent behaviour |
| `repo_intelligence_benchmark.rs` | a retrieval benchmark with known answers | a genuine, narrow, deterministic evaluation |
| `decision_proptest.rs` | property tests of the decision protocol | not behavioural scoring |

The retrieval benchmark is worth noting as the **template**: known-answer fixtures, a computed
metric, a stated threshold, and a determinism assertion. The evaluation framework should look like
that, generalised from retrieval to agent behaviour.

---

## 6. The honest conclusion

The directive asks for evaluation "as a first-class system". It is not one, and it cannot become one
overnight:

* The **harness** can be built now — fixtures, oracles, scoring, and honest `SKIPPED — BLOCKED_PROVIDER`
  reporting.
* The **five provider-free cases** (§3, rows 4–7) can be built and scored now.
* The **six provider-dependent cases** cannot be scored until `AGENT-01` closes.

**The correct first move is the five provider-free cases.** They measure the property that matters
most and that no provider can supply: *does the agent refuse to claim what it did not do?*
