# Foundation Wave Reconciliation Record

**Date:** 2026-09-26
**Scope:** ZylCode foundation wave per the ecosystem master plan (§5):
Phase 2A stabilization, Project System, Artifact Bus, Proof Engine, Mission
Console strengthening, ZylForge integration contract, model capability
metadata. This record distinguishes what is implemented, tested, reachable,
and still unverified — per `ZYLCODE_PROOF_GRAPH.md` states.

**Baseline:** `main` @ `c0b2e7f`, dirty tree preserved (prior agent's Phase 2A
scanner remediation + provider scorecard work were kept, verified, and built
upon — nothing overwritten).

---

## 1. Phase 2A — repository intelligence (wave item 1)

| Aspect | State |
|---|---|
| Gitignore boundary | **IMPLEMENTED + TESTED** — scanner now uses the `ignore` crate (ripgrep's implementation) instead of an approximate parser; anchored paths, `**`, negation all honored (`scan_respects_gitignore_glob_patterns`) |
| Intelligence suite | **TESTED** — 88/88 `intelligence` tests green |
| Reachability | **REACHABLE** — `/api/search`, `/api/files`, `/api/file-content`, `/api/evidence` served and consumed by IDE surfaces |
| Runtime verification | **RUNTIME_VERIFIED** for served routes (live curl checks in this session) |
| Still open | Stale-index *invalidation UX* (index refresh is timestamp-based; no explicit UI surface), symbol extraction depth |

## 2. Project System (wave item 2)

| Aspect | State |
|---|---|
| Schema | **IMPLEMENTED** — `ProjectRecord` v1 envelope in `.zylcode/projects.json` (`PROJECTS_PATH` overridable) |
| Persistence | **IMPLEMENTED + TESTED** — survives restart (`register_persists_and_survives_reopen`); atomic tmp+rename writes |
| Versioning/migration | **IMPLEMENTED + TESTED** — forward-only migration v0→v1 stamped and persisted; future versions refused, not guessed |
| Recovery | **IMPLEMENTED + TESTED** — corrupt store refuses to open, bytes preserved; empty file treated as recoverable-empty |
| Import/export | **IMPLEMENTED + TESTED** — JSON roundtrip with id/root dedup |
| Reachability | **REACHABLE** — `GET /api/projects` (+ Foundation tab in the IDE); registration UI not yet built |
| Windows correctness | **TESTED** — verbatim `\\?\` prefixes normalized; 8.3 short-name aliases resolve to the same directory |

## 3. Artifact Bus (wave item 3)

| Aspect | State |
|---|---|
| Contract | **IMPLEMENTED** — `ARTIFACT_SCHEMA_VERSION=1`; id, kind, hash, lineage, provenance, permissions, retention, lifecycle, full transition history |
| Lifecycle | **IMPLEMENTED + TESTED** — forward-only `proposed → generated → validated → reviewed → accepted → delivered`; regeneration refused; no state skipping |
| Immutability | **IMPLEMENTED + TESTED** — SHA-256 pinned at generation; tampering flagged `Tampered` and blocks advancement |
| Retention | **IMPLEMENTED + TESTED** — expiry releases bytes, keeps the record with a history note |
| ZylForge boundary | **TESTED** — `.zyl` packages stored verbatim, never reinterpreted |
| Reachability | **REACHABLE** — `GET /api/artifacts` (+ Foundation tab); artifact *producers* wired next (missions currently link via `attach_artifacts`) |

## 4. Proof Engine (wave item 4)

| Aspect | State |
|---|---|
| Records | **IMPLEMENTED** — command, commit, tree-clean flag, expected/actual, exit status, duration, environment, artifact refs, logs, reviewer, acceptance, limitations |
| Honest states | **IMPLEMENTED + TESTED** — `Passed / NotRun / Blocked / Failed / RuntimeNotReached / EvidenceMissing`; "not run" can never be accepted |
| Acceptance gate | **IMPLEMENTED + TESTED** — only a `Passed` **runtime-command** proof can be accepted; simulated/static evidence refused |
| Immutability | **IMPLEMENTED + TESTED** — hash chain over outcome fields; history mutation detected (`chain_detects_history_mutation`) |
| Staleness | **IMPLEMENTED + TESTED** — stale-vs-HEAD reported per query, never rewritten |
| Recovery | **IMPLEMENTED + TESTED** — interrupted write leaves last committed state intact |
| Reachability | **REACHABLE** — `GET /api/proofs` (chain status + staleness) |
| Still open | Automatic proof construction from mission runs (wiring `record()` into the mission runner) |

## 5. Mission Console (wave item 5)

| Aspect | State |
|---|---|
| States | **IMPLEMENTED + TESTED** — added `WaitingApproval` (visible gate), `Blocked` (asserts reason, is neither queued nor failed), `Verified` (only from `Done`, with proof note) to the existing honest Queued/Running/Done/Failed set |
| Approvals | **IMPLEMENTED + TESTED** — `request_approval` / `approve` (refuses non-waiting missions); resume requeues |
| Artifacts | **IMPLEMENTED + TESTED** — `attach_artifacts` (dedup) links Artifact Bus records |
| Existing behavior | Preserved — 11/11 missions tests green including all prior queue semantics |
| Reachability | Already reachable via `/api/missions`; UI already renders the queue. Approval/verify endpoints are core-level; HTTP exposure is the next slice |

## 6. ZylForge integration contract (wave item 6)

| Aspect | State |
|---|---|
| Contract document | **SPECIFIED** — `docs/governance/ZYLFORGE_INTEGRATION_CONTRACT.md` (capability refs, `.zyl` rules, MCP boundary, evidence exchange) |
| Adapter | **IMPLEMENTED + TESTED** — `zylforge_bridge.rs`: descriptors carried verbatim, missing states fail closed to `UNVERIFIED`, foreign products refused, unparseable descriptors recorded not discarded |
| Connectivity | **UNVERIFIED** — no live ZylForge round-trip has occurred; every output carries `connectivity: unverified` |
| `.zyl` writes | **NONE EXIST** — contract v1 forbids; no code path writes `.zyl` content |

## 7. Model capability metadata (wave item 12 subset)

| Aspect | State |
|---|---|
| Registry | **IMPLEMENTED + TESTED** — `model_capabilities.rs`: text/image/file input, vision interpretation, structured output, tool calling, streaming, local/provider availability, auth, quota, runtime verification |
| Vision honesty | **TESTED** — `image_input` is transport-only; every unproven model is `VISION_UNVERIFIED`; no default model claims proven vision |
| Determinism | **TESTED** — registry serialization is byte-identical across calls |
| Reachability | **REACHABLE** — `GET /api/models` |

## 8. Verification battery (exact commands, this session)

| Command | Result |
|---|---|
| `cargo fmt --check` | clean (after `cargo fmt`) |
| `cargo clippy --workspace --all-targets --offline` | **0 warnings, 0 errors** |
| `cargo test --workspace --offline` | **541 passed, 0 failed** |
| `npx tsc --noEmit` (apps/zylcode-desktop) | clean |
| `npx vitest run` (apps/zylcode-desktop) | **54 passed (8 files), 0 failed** |

New test counts this wave: project_store 9, artifact_bus 8, proof_engine 8,
missions +4 (11 total), zylforge_bridge 8, model_capabilities 7,
provider_scorecard 5, router scorecard-integration 1.

## 9. Explicitly rejected claims

- "Proof Engine complete" — foundation only; automatic proof construction from
  mission runs is not wired yet.
- "Project System complete" — store + service route done; project
  registration/switching UI not built.
- "ZylForge integration works" — the adapter is tested against payloads;
  no live ZylForge instance has been contacted. `UNVERIFIED`.
- "Vision models understand images" — no image-understanding test exists;
  all such models are `VISION_UNVERIFIED`.
- "Measurement-based routing proven in production" — the mechanism is tested
  and live; it needs real dispatch samples before any performance claim.

## 10. Next engineering wave

1. Wire Proof Engine into the mission runner (auto-record proof per
   best-of-N verification) and expose mission approval/verify over HTTP.
2. Project registration/switching UI on the Project System store.
3. Artifact producers: mission plans, patches, and test reports become
   registered artifacts automatically.
4. Runtime commissioning record for image understanding (separate from this
   wave's metadata contract).
5. CI runners for macOS/Linux (external dependency: GitHub Actions billing).
