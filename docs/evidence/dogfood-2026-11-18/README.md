# Dogfood evidence packet — 2026-11-18

ZylCode run against **itself**. Not a fixture, not a synthetic corpus: the binary
built from this working tree, indexing this repository, answering questions about
this repository's own code.

**Raw transcript:** [`transcript_2026-11-18.txt`](transcript_2026-11-18.txt) —
verbatim stdout, one block per command, with exit codes.

## Environment

| Field        | Value                                                                |
| ------------ | -------------------------------------------------------------------- |
| Date         | 2026-11-18                                                           |
| Host         | Windows 11, Git Bash                                                 |
| Binary       | `target/debug/zylcode.exe`, built from this working tree             |
| Corpus       | this checkout: 801 files, 3,967 symbols, 8 packages, 41 entry points |
| Index build  | 36–38 s per invocation (cold persisted cache)                        |
| Reproduction | `./target/debug/zylcode.exe <command>` from the repository root      |

## Commands run, and what they showed

1. `repo-context "where is the retrieval ranking implemented"`
2. `repo-context "where is the impact analysis implemented"`
3. `repo-context "which files would likely need inspection to modify crash recovery"`
4. `impact --file crates/zylcode-core/src/intelligence/impact.rs --depth 2`
5. `impact --symbol LedgerStore --depth 1`
6. `repo-symbols ContextRetriever`

All six exited 0. Every block below is quoted from the transcript, not from memory.

## What works

**Retrieval on an unambiguous question is right at rank 1.** Command 2 returns
`crates/zylcode-core/src/intelligence/impact.rs` as the top file (2.74) with the
reason chain that produced it — path token, anchor word, defined symbol, recency,
and a co-change partner. That is the answer a human would give.

**Co-change evidence carries a task it could not have matched lexically.**
Command 3 puts `crates/zylcode-core/src/ledger.rs` and `memory_ledger.rs` in the
top ten on the strength of "changes together with `crash_recovery.rs`", even though
neither file contains the words _crash_ or _recovery_. This is the mechanism that
Q8 in the benchmark depends on.

**Impact analysis states its own limits.** Command 4 ends with four explicit
`not determined` lines: unresolved call graph, unresolved dynamic
imports/re-exports/path aliases, 364 unresolved import specifiers, and the depth
bound. Command 5 finds six importers of `LedgerStore` and one test at risk, each
with the `use` statement quoted.

**Symbol lookup crosses language boundaries.** Command 6 returns the TypeScript-fed
`repo-symbols` surface working on a Rust symbol, and resolves `ContextRetriever`,
its impl block, and `retrieve` to file and line.

## What does not work, found by running it

**1. The known top-1 miss is real through the product, not only in the benchmark.**
Command 1 returns `provider_scorecard::ProviderScorecard::ranking` at 1.59 and
`provider_scorecard.rs` at 0.96; `context.rs` — where `ContextRetriever` lives —
does not appear in the top nine. A user asking this question gets a wrong answer
first, and the ranking has no way to know that "retrieval ranking" names the
retriever rather than a scorecard method called `ranking`. This is the same case
recorded in `docs/evidence/phase2a-2026-11-16/README.md`; it is measured now by
`top1_points_at_the_implementing_file`, which scores 9/10.

**2. `impact --file` claims a TypeScript basis for a pure-Rust file.** Command 4
reports `confidence: High — every traversed edge is a relative
TypeScript/JavaScript import resolved to a file`, while listing **zero** transitively
affected files and 364 unresolved specifiers. No TypeScript edge was traversed, so
the sentence describes work that did not happen. The confidence value itself (High
with an empty closure) is also stronger than the evidence supports.

**3. A Rust `use ...::{self}` edge is missed.** `crates/zylcode-cli/src/main.rs`
imports `zylcode_core::intelligence::impact::{self, ChangeSpec}` and calls
`impact::analyze(...)`, yet command 4 reports no importer at all. Module-path
matching keys on the `use` path ending in a file, and a `{self, X}` group does not
resolve to `impact.rs`. Recorded, not fixed: fixing it is a Phase 2B import-graph
slice, and inventing an edge would be worse than the gap.

## Why findings 2 and 3 are written down instead of fixed here

Both are engine defects, and this packet is evidence rather than a repair. They are
listed because the alternative — a dogfood report that only quotes the good
output — is the failure mode every acceptance report in this repository's history
was written to correct. They are candidates for the next 2B slice, and finding 2 in
particular is the kind of defect that makes a report wrong in a way a passing test
suite cannot see.

## Scope of this evidence

One platform, one commit, one machine. It is not a benchmark run and does not
replace one: the numbers are from `repo-context`'s own scoring, which is not
comparable across heads because the corpus is the working tree. It is also not an
independent audit — the same builder ran the tool on their own tree, which is
exactly the limitation C4 names.
