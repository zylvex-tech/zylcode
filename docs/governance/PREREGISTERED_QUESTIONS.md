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
