# P15 — Streaming Repository Context (Wave Report)

**Date:** 2026-10-19
**Status:** W1–W4 **SHIPPED** — builder evidence. Acceptance remains the
auditor's verdict (Phase 2A stays RE-OPENED; nothing here claims otherwise).

**One-sentence outcome:** the agent's for-turn context is no longer a
once-per-`run()` snapshot — it re-ranks after the agent's own mutations,
carries typed repository facts with explicit absences, and every surface can
observe whether the index is answering for the tree as it stands
(`fresh` / `stale` / `reindexing`) at changed-file cost.

Work order: [`P15_STREAMING_CONTEXT_WORK_ORDER.md`](P15_STREAMING_CONTEXT_WORK_ORDER.md).

---

## 1. What landed, per wave

### W1 — Change-increment fingerprint (stat memo) — SHIPPED 2026-10-15

Commit `f14a0e5`. Schema-v2 cache carries a per-file stat memo
`(len, mtime, content_hash)`; measurement walks stat facts only and re-hashes
only drifted files. See §1.W1 of the work order for its evidence list
(zero-read warm build, same-size invalidation, mtime no-op, v1 migration,
corruption discard).

### W2 — Mutation-driven invalidation at turn boundaries — SHIPPED 2026-10-19

Commit `8d4ccc6`. A successful mutating tool op (`fs.write`, `fs.delete`,
`fs.rename`, `fs.copy`, `fs.mkdir`) marks the session's dirty set; at the
Executing turn boundary the context builder re-ranks *before the next model
decision*, the citation block is **replaced** (never stacked), mutated file
contents are re-read, and the set is drained. An unreadable mutated path
(e.g. deleted) is stated as an absence, never re-invented from cache.

Evidence: `an_agent_edit_reranks_context_at_the_next_turn_boundary`,
`a_mutated_path_that_vanishes_is_declared_not_reread`,
`ledger_hash_chain_is_intact_after_a_run`.

### W3 — Typed payload ingestion into agent context — SHIPPED 2026-10-19

Commit `53d9026`. `agent_typed_block` renders a bounded block (5 files,
8 symbols and 8 dependency edges per package, 2 packages) from the same
parsed index `repo_query_payload` uses, ingested on both the cold gather and
the W2 re-gather. Absences are explicit lines ("no symbol declarations
recognized", "no internal dependency edges"); an **unbuildable index
announces its own absence before selection facts**, so a repository we could
not read never masquerades as "no files selected".

Evidence: `gathered_context_carries_typed_repository_facts`,
`an_unbuildable_index_states_its_absence_in_the_gather_too`, renderer tests
in `intelligence::api` (facts, absences, unavailable path, bound counts).

### W4 — Streaming observation surface — SHIPPED 2026-10-19

Commit `56d24b7`.

* **Core** — `intelligence::freshness`: a read-only probe comparing the
  stat-memo-measured fingerprint against the stored index, and a per-root
  reindex registry: every `build_repo_query` call holds a `ReindexGuard`, so
  `reindexing` is true *exactly* while a scan runs in this process and never
  after — observed, not inferred, including when a scan fails (guard drop).
* **Fingerprint integrity fix** — `should_exclude` now excludes `.zylcode/`
  (the index's own cache and product state) from scan and stat inventory,
  asserted by `exclude_zylcode_state`. Before this, a repository without a
  `.gitignore` entry for `.zylcode/` indexed the cache written under it and
  re-indexed itself forever.
* **serve-intel** — `GET /api/repo-context` (one-shot JSON) and
  `GET /api/repo-context-stream` (SSE: an initial `state` event, then an
  event **only on a real state or fingerprint transition**; a failing probe
  becomes one `unavailable` event per distinct failure, never a fake state).
  The startup banner now prints the *bound* port, so `--port 0` is usable
  and truthful.
* **Desktop** — `repoContext.ts` maps one payload shape onto two transports
  (EventSource in the browser; polled `repo_context_status` Tauri command on
  desktop); `RepoQueryPanel` renders the freshness line bound to that signal
  (`index fresh · <hash>` / `index stale · N file(s) changed — run a query to
  re-index` / `index reindexing…` / `unavailable — <reason>`); vite proxy
  route added.

Evidence: integration test
`crates/zylcode-cli/tests/serve_intel_context_stream.rs` drives the **real
`zylcode` binary** over real HTTP through fresh → stale → fresh (a real
content change, counted; a real query performing the rebuild), then reads the
SSE stream for the initial event, the `stale` transition with a changed
fingerprint, and a **silence window** proving no-repeat emission. Unit tests:
8 in `intelligence::freshness` (probe states, guard lifecycle, mtime touch,
cross-root isolation, missing-root error), panel vitest for every signal
state + unmount cleanup, `signalFromPayload` mapper vitest.

---

## 2. Acceptance criteria → evidence

| # | Criterion | Evidence |
|---|-----------|----------|
| A1 | A mutation changes the ranked context used by the **next** turn | `an_agent_edit_reranks_context_at_the_next_turn_boundary`, `a_mutated_path_that_vanishes_is_declared_not_reread` (W2) |
| A2 | Fingerprint cost ∝ changed files | `a_warm_second_build_reads_no_file_contents` (unix-gated), `a_same_size_content_change_invalidates`, stat-memo path (W1) |
| A3 | mtime-only touches do not invalidate | `an_mtime_touch_does_not_invalidate` (memo path) + `an_mtime_only_touch_is_not_a_change` (probe, W4) |
| A4 | Typed payloads with correct attribution | `intelligence::api` renderer tests + `gathered_context_carries_typed_repository_facts` (W3) |
| A5 | Desktop/serve-intel reflect stale→fresh | `serve_intel_context_stream` (real binary, HTTP + SSE), freshness vitest ×8, panel vitest ×6 (W4) |
| A6 | Full gates green | §3 below |

## 3. Full gates (A6) — all green, 2026-10-19

| Gate | Result |
|------|--------|
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean (includes src-tauri) |
| `cargo test -p zylcode-core --lib` | **526 passed, 0 failed** |
| `cargo test -p zylcode-core --tests` (factory e2e, agent-loop e2e, crash recovery, natlas, proptests…) | all passed; live suites `#[ignore]`d by design |
| `cargo test -p zylcode-cli --test serve_intel_context_stream` | 1 passed (real binary, 7.6s) |
| `cargo bench -p zylcode-core` (criterion, 2 benches: `router_cache`, `parse`) | exit 0 — 44 measurements |
| `tsc --noEmit` (desktop) | clean |
| `vitest run` (desktop) | **126 passed / 126** (18 files) |

## 4. Known limitations (stated, not smoothed)

* `reindexing` is visible only to observers **in the process running the
  scan**. A rebuild triggered in another process shows as stale → fresh,
  which is still truthful — the state just is not shared cross-process.
* The stream samples at the probe tick (600 ms): a transition shorter than a
  tick between two probes is not promised.
* A probe re-reads stat-drifted files (bounded: changed files × 512 KB cap)
  until the next index write persists the new memo.
* The W1 zero-read proof remains unix-gated (chmod semantics); Windows is
  covered by the shared hash core + mtime/corruption/migration tests.
* Desktop freshness is a 1.5 s poll, not a subscription — the engine runs in
  the Tauri process while the panel is the webview.
* Builder evidence throughout; independent acceptance outstanding.

## 5. Registry re-baseline

`docs/capability-registry.json` — `p15_update` block: new entry
`repo_context_freshness_stream` (GREEN/R3, builder evidence), re-baselines on
`incremental_indexing` (stays PARTIAL — symbol-level reindex still future
work), `agent_context_gathering`, `repository_scanner`. Summary now 39
entries (30 GREEN, 4 PARTIAL, 4 DISPUTED, 1 EXTERNAL_BLOCKER). Dispute
history preserved; no acceptance claimed.
