# P15 — Streaming Repository Context (Work Order)

**Status:** PLANNED, not started. Builder-authored work order; execution still
owed its own wave evidence. Nothing in this document claims implementation.

**One-sentence goal:** the perfect for-turn context rank the agent loop already
has stops being a once-per-`run()` snapshot — it must refresh against real file
mutations between turns, at changed-file cost rather than repository cost, and
carry the typed query payloads (symbols, packages, dependencies) on the same
honesty rules as the ranked file list.

---

## 0. What exists today (verified seams, P14 wave)

* `AgentLoop::gather_context` (`agent.rs`) runs inside `run()`, ranks files
  through `ContextBuilder` → `PersistedIndex::build()`, keeps
  `navigation_citations` on the session, and `with_repository_context`
  prepends them to every provider request.
* `PersistedIndex::build()` (`intelligence/persisted.rs`) measures a
  content-hash fingerprint over the whole tree, serves a cache hit on match,
  rebuilds on mismatch; mtime-only touches are correctly ignored.
* `repo_query_payload` exposes exactly 12 query methods with
  relevance/reason/provenance on every result.

**The gaps this wave closes:**

1. `gather_context` runs once per `run()`. The agent's own executed
   `fs.write`/`fs.delete` mutations during a task do not force a re-rank at the
   next turn boundary — stale ranked context and stale file contents can be
   fed to the provider for the rest of the task.
2. Freshness is correct but paid as **O(repository) hashing on every
   gather**. There is no change-increment path.
3. Only the ranked file list is ingested. The typed payloads (symbols,
   packages, dependency neighbors, hot files) never reach the agent context,
   so the Desktop `RepoQueryPanel` can answer things the agent itself is not
   told.

## 1. Work items

### W1 — Change-increment fingerprint (cost fix)

Add a stat-memo layer to `PersistedIndex`: the cache file stores, beside the
fingerprint, per-file `(size, mtime, content_hash)` memos. `build()`
re-hashes only files whose `(size, mtime)` differ from the memo; files
unchanged since the last measurement reuse the memoized hash. Deletes and
adds reap the memo map.

* Correctness invariant to keep, tested already and again: a content change
  invalidates, an mtime-only touch must **not** flip the fingerprint.
* Honesty invariant: any memo mismatch falls back to full hashing for that
  file — never trusts an mtime as "unchanged content".
* Migration: existing caches that lack memos degrade gracefully to today's
  full-hash behavior (the "foreign-schema cache is discarded" rule covers
  it).

### W2 — Mutation-driven invalidation at turn boundaries

* `ContextBuilder::mark_dirty(file)` (or a session-level dirty set) is called
  on every successful tool execution that mutates the tree.
* At each turn boundary, if the dirty set is non-empty, the next
  `gather_context` re-ranks **before** the next model decision — not at the
  end of `run()`.
* The re-gather replaces (not appends) evidence for the current goal, per the
  existing G3 rule; contents of files re-read into system messages are
  refreshed, not stacked.

### W3 — Typed payload ingestion into agent context

`gather_context` additionally renders a bounded block from
`repo_query_payload` for the files already selected: dependency neighbors and
the symbols those files declare, plus hot files when the query supports it.
Every line carries the existing attribution discipline: resolved facts
(derived from the parsed index) are quoted and citable; anything outside that
block is labelled interpretation. Empty results are explicit, never silent.

### W4 — Streaming observation surface

* serve-intel: `GET /api/repo-context-stream` (or an event in the existing
  event channel) exposing `stale` / `reindexing` / `fresh` + fingerprint
  short-hash transitions.
* Desktop: the mounted repo surface shows live state; `RepoQueryPanel` gains
  a stale/refreshing state bound to the same signal.
* No new claims in the capability registry until the wave is done; the
  registry update is part of the wave's re-baseline, not this work order.

## 2. Acceptance criteria (each checkable by command)

| # | Criterion | Evidence |
|---|-----------|----------|
| A1 | An agent-executed file mutation changes the ranked context used by the **next** turn | integration test on `AgentLoop` tool-op → gather ordering |
| A2 | Fingerprint cost is proportional to changed files, not repository size | test on synthetic tree asserting memoized hashes are reused (behavioral, not timing) |
| A3 | mtime-only touches still do not invalidate | existing `an_mtime_touch_does_not_invalidate` test extended to the memo path |
| A4 | Typed payloads appear in provider context with correct attribution discipline | unit tests on the context string builder |
| A5 | Desktop/serve-intel surfaces reflect stale→fresh transitions | vitest on the panel state; serve-intel handler test |
| A6 | Full gates stay green: `cargo fmt --check`, clippy (`-D warnings`), core lib tests, factory e2e, benchmark suite, `tsc --noEmit`, `vitest run` | wave report |

Out of scope for P15: retrieval-algorithm changes, provider/model calls,
installer signing, registry status flips (those need independent acceptance).

## 3. Risks and mitigations

| Risk | Mitigation |
|------|------------|
| Stat-memo poisoned by filesystems with coarse mtime granularity | A2-class test plus the fallback-to-rehash rule for any mismatch |
| Dirty-set races with concurrent tool executions | dirty set owned by the runner thread; single writer, snapshot-read by the gather path |
| Bigger context blocks crowd the prompt budget | bounded line caps with explicit "… N more omitted" markers, mirroring the ranked-list cap |
| Cache schema bump breaks old caches silently | versioned cache schema; mismatched version = discard + full rebuild (already the persisted-index policy) |

## 4. Order of execution

W1 → W2 (W2 builds on W1's cheap freshness probe) — then W3 — then W4.
Each item lands with its own tests; no item lands with a red gate.
