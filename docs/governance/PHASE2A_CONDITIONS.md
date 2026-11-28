# Phase 2A audit conditions — register

Phase 2A was accepted **with conditions** by the independent audit of 2026-11-16
([verdict](PHASE2A_AUDIT_VERDICT_2026-11-16.md)). Six conditions were attached.
This register is the per-condition state, and the file the gate reads:
`docs/governance/phase2a-conditions.json`.

**The rule this register exists to enforce:** a condition may be marked `CLOSED`
only when a predicate in the JSON keeps it closed. A closure backed by a paragraph
of prose is a memory; the next PR can undo it silently. `scripts/check_conditions.py`
evaluates every predicate on each gate run, so a closure that stops holding is a
red build.

| ID  | Condition                                                                | State                     | Enforced by                                                       |
| --- | ------------------------------------------------------------------------ | ------------------------- | ----------------------------------------------------------------- |
| C1  | `typescript_symbol_index` stays R2 until wired into `build_repo_query()` | **CLOSED** 2026-11-17     | registry state must be GREEN/R3 (three probes, one a CLI surface) |
| C2  | `repository_benchmark` and `documentation_accuracy` remain R2/DISPUTED   | **CLOSED** 2026-11-18     | both registry entries must stay DISPUTED/R2                       |
| C3  | the three GREEN/R2 conflicts are resolved                                | **CLOSED** 2026-11-17     | none of the three may be GREEN                                    |
| C4  | the question set must become independently authored                      | **OPEN**                  | any set under `docs/benchmarks/` must carry a freeze lock         |
| C5  | throughput has a collapse floor, not an acceptance budget                | **ACCEPTED AS EXCEPTION** | the floor and the word "collapse" must stay in the benchmark      |
| C6  | symbol resolution is regex-based (`DEFINITION_INDEXED`)                  | **OPEN**                  | `impact.rs` must keep stating the unanswered call-graph question  |

## What closing C2 means, and what it does not

C2 said the two entries "must remain R2/DISPUTED". An instruction to keep
something unchanged has no natural completion date — it is a standing constraint,
and leaving it phrased as an open to-do invites exactly the confusion it was
meant to prevent: an open condition reads as work pending, while the requirement
is that no work happen.

So C2 is closed on 2026-11-18 by converting it into the invariant the audit asked
for:

- `scripts/check_conditions.py` fails if either entry's status is not `DISPUTED`
  or its rung is not `R2`;
- the two entries' `rung_basis` strings are unchanged and still name the auditor's
  ruling;
- promoting either one now requires editing this register, which is a visible
  change with a reason, not a silent registry edit.

**Nothing was promoted by closing C2.** `repository_benchmark` is still a test
harness with no user surface, and `documentation_accuracy` still has no defined
scope — the two findings that produced C2 in the first place.

## C4's checkable scope

C4 is the one condition the builder cannot close alone: it needs questions
written by someone who did not tune the retriever. What the builder can do is make
the requirement testable rather than aspirational, so "independently authored" is
not a word that can be applied retroactively:

1. **A committed set must be frozen.** `docs/benchmarks/` is the only place an
   acceptance set may live. `scripts/check_conditions.py` fails if any `*.json`
   there has no matching `*.lock.json`, so a set cannot be scored and then edited.
2. **The lock records who and why.** `scripts/freeze_question_set.py` writes the
   SHA-256, the question count, the timestamp and a required `--reason`. A
   re-freeze requires `--force` and a new reason, so growing the set is visible.
3. **`satisfied_by` must point at a real file.** C4 cannot be marked CLOSED while
   the register names no artefact, and the register refuses an `OPEN` condition
   that names one.

Since no such set exists, C4 stays **OPEN** and the check passes trivially. That
is the honest state: a mechanism ready for an author, not a claim that an author
appeared.

## Reproduce

```console
$ python scripts/check_conditions.py
OK   C1: ... [CLOSED]
...
6 condition(s): ACCEPTED_AS_EXCEPTION=1, CLOSED=3, OPEN=2

$ python scripts/check_conditions.py --self-test
self-test: 0 failure(s)
```
