# ZylCode Agent Operating Protocol

> **MANDATORY READING FOR EVERY IMPLEMENTATION AGENT.**
>
> This document exists because of a repeatedly observed failure: **agents forget what product
> they are building while they implement an individual task.**

**Status:** GOVERNING
**Applies to:** DeepSeek, Codex, ZCode, Claude, Gemini, any human contributor, any future agent

---

## 1. The Mandatory Preamble

**Every prompt issued to an implementation agent must begin with the following text, verbatim:**

---

> **Read the ZylCode Constitution, Architecture, Master Execution Plan, Capability Model and
> Proof Graph, plus the current phase specification. They govern implementation.**
>
> **Where implementation conflicts with documentation, investigate the discrepancy rather than
> silently choosing one.**

---

Documents referenced:

```
docs/governance/ZYLCODE_PRODUCT_CONSTITUTION_V2.md
docs/governance/ZYLCODE_ARCHITECTURE_V2.md
docs/governance/ZYLCODE_MASTER_EXECUTION_PLAN.md
docs/governance/ZYLCODE_CAPABILITY_MODEL.md
docs/governance/ZYLCODE_PROOF_GRAPH.md
docs/governance/ZYLCODE_AGENT_OPERATING_PROTOCOL.md   (this document)
docs/roadmap/ZYLCODE_ROADMAP_V2.md
docs/architecture/<RELEVANT_SYSTEM>.md
```

This preamble is **not optional** and **not paraphrased**. It is the mechanism by which a
task-scoped agent stays aligned with the product.

---

## 2. The Three Laws of the Program

### Law 1 — The loop is the product

ZylCode exists to close the loop from intent to verified deliverable. Before starting any task,
ask: **which arc of the loop does this strengthen?** If the answer is "none", stop and raise it.

### Law 2 — Evidence or it did not happen

> A claim is not a fact until it is independently reproducible.

Every report carries a reproduction block. Every metric names and scopes what it counts. Every
capability carries a rung. See `ZYLCODE_PROOF_GRAPH.md`.

### Law 3 — Builders do not certify their own work

You implement. An independent auditor accepts. You may state what you did and what you observed.
You may not declare a phase complete.

---

## 3. Pre-Flight Checklist

Before writing any code:

- [ ] Read the six governance documents listed in §1.
- [ ] Read the specification for the system you are touching (`docs/architecture/`).
- [ ] Confirm the phase you are working on is **not blocked** in `ZYLCODE_MASTER_EXECUTION_PLAN.md`.
- [ ] Confirm the phase's dependencies are **ACCEPTED**, not merely reported complete.
- [ ] Confirm the capability you intend to add is described in the Constitution's eight-system model.
- [ ] Confirm you are not about to build something a later phase owns.

If any of these fail, **stop and report** rather than proceeding.

---

## 4. During Implementation

### 4.1 The discrepancy rule

> Where implementation conflicts with documentation, **investigate** — do not silently choose.

Silently choosing either direction destroys the value of having a governing document.

| Situation | Wrong response | Right response |
|---|---|---|
| Code does X, spec says Y | Change one to match the other quietly | Report the discrepancy, propose a resolution, record it |
| Spec describes an unbuilt system | Assume it exists | Confirm it is marked PROPOSED; implement only if it is your phase |
| A test contradicts the architecture | Delete the test | Report it; the test may be encoding the old contract |
| A metric looks wrong | Publish it anyway | Investigate scope before publishing (see Phase 2A) |

### 4.2 Marking rules

- A specification for an unbuilt system **must** say **PROPOSED** at the top and at each
  unimplemented section.
- **Documentation must never imply that a system exists because it is described.**
- Describing a system is not building it. Marking is mandatory and is itself auditable.

### 4.3 Scope discipline

- Implement your phase. Do not opportunistically implement later phases.
- If you discover work a later phase owns, **record it** and leave it.
- If your phase requires something a **blocked** phase owns, raise it — do not route around it.

### 4.4 The reachability rule

A capability is not delivered until a **user or an agent can invoke it through a named entry
point**.

- Add the entry point (CLI command, UI action, agent tool).
- Capture a transcript of it working.
- If you cannot add an entry point, the capability is **PARTIAL / R2** and must be recorded as
  such. It is not GREEN.

### 4.5 The measurement rule

Before publishing any number:

1. State its **scope** — exactly what is being counted.
2. Sanity-check the scope against something independent (e.g. `git ls-files`, a manual count).
3. Record the **command** that produced it.
4. **Assert it in a test** if it is a headline metric. An unasserted metric will regress silently.

> "Files indexed: 14,973" was not wrong arithmetic. It was wrong *scope*. The repository has 254
> files; the rest was `target/` build output. The number was never asserted by any test, so
> nothing caught it.

### 4.6 The cross-platform rule

ZylCode targets **Windows first**, then macOS and Linux. Any path, separator, encoding or
case-sensitivity assumption must be tested on Windows specifically.

- Normalise paths at every boundary.
- Do not hand-roll path or gitignore matching; use a maintained crate (`ignore`, `globset`).
- **Test against the real repository**, not a synthetic fixture. A fixture that differs from
  reality produces green tests over broken behaviour — this has already happened once.

---

## 5. Completion Report Format

Every completion report **must** contain all of the following sections. A report missing any of
them is a **draft**, and must not be submitted for audit.

```markdown
# Phase <ID> Completion Report — <title>

## 1. Status
DRAFT | READY FOR AUDIT
(You may not write "COMPLETE" or "ACCEPTED". That is the auditor's word.)

## 2. Scope delivered
What was built, and explicitly what was NOT built.

## 3. Capabilities added / changed
| capability | status | rung | entry point |

## 4. Reproduction block
### Environment
<OS, toolchain versions, commit SHA>
### Commands
<exact commands, copy-pasteable>
### Raw output
<verbatim captured output — not a summary, not an expectation>

## 5. Test results
<verbatim output including the summary line; failures shown, not hidden>

## 6. Metrics
| metric | scope | value | command that produced it |

## 7. Discrepancies found
<anything where implementation and documentation disagreed, and what you did about it>

## 8. Limitations
<mandatory, must be non-empty>

## 9. Known failures
<anything failing, including pre-existing failures. Do not omit.>

## 10. Requested audit
What specifically you want the auditor to try to falsify.
```

### 5.1 Forbidden in a completion report

- ❌ An "Expected Output" section presented where evidence belongs.
- ❌ Any number without its scope and its command.
- ❌ "All tests pass" without verbatim output.
- ❌ The words COMPLETE / ACCEPTED / DONE as a self-assessment.
- ❌ `Commit SHA: (Pending)` — fill it in before committing the report, or omit the field.
- ❌ A benchmark result that has not been re-run immediately before the report.

### 5.2 The lesson that produced these rules

The Phase 2A report contained an **"Acceptance Demonstration"** section headed *"Expected
Output"* — a hand-written narrative of what the system *would* answer, in the position where
captured evidence belongs. It also reported a benchmark green that fails on re-execution, and a
"337 tests all pass" that is contradicted by a failing test.

None of this required bad faith. It required only that the author did not re-run their own
benchmark before reporting it. That is the normal human failure this protocol is designed to
make impossible.

---

## 6. What the Auditor Will Do

Assume the following, because it will happen:

1. Your benchmark will be **re-run** on a different machine.
2. Your metrics will be **recomputed** from their stated scope.
3. Your entry points will be **invoked**.
4. Your tests will be **run from a clean checkout**.
5. Your claims will be **actively falsified**, not confirmed.
6. Your `target/` directory will be **counted** if you report a file count.

Write your report so that it survives all six.

---

## 7. Escalation

Stop and escalate — do not guess — when:

- the phase you are working on is **blocked**;
- a dependency is not **ACCEPTED**;
- implementation and documentation conflict in a way you cannot resolve;
- you cannot produce a reproduction block;
- you cannot reach the claimed rung;
- you would have to modify governance documents to make your work correct.

Escalating is a **correct** outcome. Silently proceeding is not.

---

## 8. Anti-Patterns (observed in this project)

| Anti-pattern | Why it is fatal | Correct behaviour |
|---|---|---|
| Reporting a benchmark green without re-running it | Produces a false PASS that propagates downstream | Re-run immediately before reporting |
| Testing against a fixture that differs from reality | Green tests over broken behaviour | Test the real input |
| Publishing an unasserted headline metric | Regresses silently | Assert it in a test |
| Building a module with no entry point | Library, not capability | Add the surface, or report PARTIAL/R2 |
| Writing an "Expected Output" where evidence belongs | Substitutes intention for fact | Capture raw output |
| Committing a report with `(Pending)` fields | A certificate that predates what it certifies | Fill in before committing |
| Leaving a large dirty tree | Makes attribution impossible | Commit or discard |
| Marking unbuilt systems as existing | Agents build the wrong product | Mark PROPOSED |
| Hand-rolling path/gitignore matching | Breaks on the primary platform | Use maintained crates |
| Self-certifying | The exact failure this program exists to prevent | Hand off to audit |

---

## 9. Summary

1. **Read the governance documents first.** Every time. They govern.
2. **Investigate discrepancies.** Never silently choose.
3. **Mark PROPOSED** for what is not built.
4. **Reach the user or agent.** No entry point, no capability.
5. **Scope your metrics.** Name and count the right thing.
6. **Reproduce before reporting.** Re-run it yourself.
7. **Report honestly**, including failures and limitations.
8. **Do not certify your own work.**

The product's core promise is that it can prove what it did. That promise is only credible if
the team building it holds itself to the same standard.
