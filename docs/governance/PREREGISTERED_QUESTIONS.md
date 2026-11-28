# Pre-registered benchmark question sets (condition C4)

Phase 2A's verdict accepts the retrieval capability with conditions, and C4 names
the hardest one: the benchmark questions were written by the same person who
tuned the retriever, and eight were added late to lift the average. A score
against a set that can move with the implementation measures the author, not the
software.

C4 cannot be satisfied by the builder alone. What the builder **can** do is make
the process auditable, so an independent author's set cannot be quietly
reinterpreted after scoring. That is what this document and
`scripts/freeze_question_set.py` provide.

## The procedure

1. **Publish the format first.** The scenario schema in
   [`docs/architecture/IMPACT_ANALYSIS.md`](../architecture/IMPACT_ANALYSIS.md) §7
   is the contract. It exists before the engine, so an author can write questions
   without seeing the implementation.
2. **Author the set outside the implementation.** Owner, auditor, or a held-out
   set written independently. Builder-authored questions are allowed only as a
   separately labelled development set and never count toward acceptance.
3. **Freeze it before scoring:**

   ```console
   $ python scripts/freeze_question_set.py \
       --set docs/benchmarks/impact_scenarios.json \
       --reason "authored by <name>, date, before implementation"
   OK: froze docs/benchmarks/impact_scenarios.json at sha256:…
   ```

   The lock records the hash, the question count, the timestamp and the reason.

4. **Score, publish, and keep the lock.** CI runs
   `python scripts/freeze_question_set.py --set <path> --check`, which fails if
   the set changed since it was frozen. Scores therefore always name the exact
   bytes they describe.

5. **Growing the set is allowed and visible.** A re-freeze requires `--force` and
   a new `--reason`, and the diff shows the lock changing. Scores from before the
   re-freeze are not comparable, and the lock's reason is where that is recorded.

## What this does not solve

- It does not make the author independent; it makes the set's identity
  checkable. Whether the author had access to the implementation is a governance
  statement, not a hash.
- It cannot stop a builder from tuning against a published set. Pre-registration
  narrows that window to "before the freeze"; a held-out set with a private
  portion is the stronger control, and is the owner's call.
- A frozen set can still be a bad set. Freezing guarantees that a number
  describes fixed questions, not that the questions are good.

## Current state

No acceptance question set has been frozen yet. The Phase 2B acceptance corpus is
the first intended use, and it is blocked on an external author — the honest
statement is that C4 is **open**, with a mechanism that is ready for the author
rather than a claim that it has been satisfied.

## The checkable scope (added 2026-11-18)

C4's real requirement — questions written by someone who did not tune the retriever
— cannot be automated, and pretending otherwise would be the same hand-assertion
the capability model forbids. What **can** be automated is the property that makes
the requirement meaningful: a set that can be edited after scoring measures the
author.

So C4 now has a scope a script can check, and three places enforce it:

| Where                                      | What it asserts                                                                 |
| ------------------------------------------ | ------------------------------------------------------------------------------- |
| `docs/benchmarks/`                         | the only location an acceptance set may live                                    |
| `freeze_question_set.py --check-all <dir>` | every `*.json` there has a matching `*.lock.json`; an empty directory passes    |
| `scripts/check_conditions.py` (C4)         | the same rule, plus: an `OPEN` condition may not name a `satisfied_by` artefact |

CI runs `--check-all docs/benchmarks`. Today the directory does not exist, so the
step passes and C4 stays **OPEN** — a mechanism ready for an author, not a claim
that one appeared. The condition closes only when the register's `satisfied_by`
names a frozen set whose lock's `--reason` records who wrote it and when.
