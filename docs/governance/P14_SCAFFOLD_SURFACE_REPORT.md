# P14 — Scaffold Surface Wave Report (2026-10-14)

**Scope:** the five-item public-readiness push requested on `main` — context
retrieval verification, the honesty pass, the 12-query command surface,
Factory × Agent unification, and the distribution workstream. Builder report;
acceptance of the disputed Phase 2A items remains the independent auditor's
call, and every claim below is tied to a command someone can run.

---

## 1. Context retrieval into the agent loop — verified as already wired

The audit's "library without a product surface / not connected to the loop"
finding no longer describes the tree. The seam was verified end to end:

* `ContextBuilder::build` ranks files through the **real Repository
  Intelligence pipeline** (`context_builder.rs`, P1.2 integration) with the
  legacy keyword path only as fallback;
* deterministic `zylcode-nav` graph citations are gathered per goal
  (`nav_api::agent_citation_block`) and stored on the session;
* `AgentLoop::gather_context` runs inside `run()` and the citations are
  prepended to every provider request (`with_repository_context`);
* called out honestly for the model: unresolved structure must be labelled as
  interpretation, not inference.

Command: `cargo test -p zylcode-core --lib context_builder` (4 tests, green).

## 2. Honesty pass (Phase A items executed this wave)

* **computer-use facade disclosed**: `SIMULATION_MODE` const +
  `ComputerUseSystem::simulation_notice()`, `ComputerUseStats.simulation_mode:
  true`, and the CLI prints the banner before any computer-use output. No
  public claim of real GUI automation exists anywhere in the docs.
* **capability registry re-baselined** (v1.5.0-builder): the twelve DISPUTED
  library-only entries were re-baselined to their true, test-derived state —
  seven intelligence entries to GREEN/PARTIAL **on the basis of new product
  surfaces** (this wave), `incremental_indexing` to PARTIAL naming the
  persisted-index implementation that now exists. Two entries stay DISPUTED
  on their own merits (`documentation_accuracy`, `repository_benchmark`);
  dispute history preserved verbatim everywhere. A `computer_use_automation`
  entry records R0. Summary counts recomputed from statuses.
* **not-connected widgets recorded**: ResourceGraph, TelemetryDashboard,
  AuditViewer, BridgeMonitor are unmounted presentation components; recorded
  in the registry (`p14_not_collectable_surfaces`) so they can never be
  counted as product surfaces. Mounted panels were each checked to have a
  live data path (lib adapters / Tauri / serve-intel).
* **provider-chain claim retired**: the router now walks the entire configured
  chain before degrading offline (committed separately this session), so the
  registry's provider entries no longer rest on "stops after one fallback".

## 3. The 12 repository query methods — now a command surface

`zylcode_core::intelligence::api::repo_query_payload(root, method, arg)`
dispatches exactly the `RepoQuery` members and rejects unknown methods or
missing selectors with an explicit error (never an empty answer). Product
surfaces:

| Surface | Entry point |
|---|---|
| serve-intel | `GET /api/repo-query?method=…&arg=…` |
| Tauri desktop | `repo_query` command (`method`, `arg`) |
| desktop UI | `RepoQueryPanel` (mounted in the Intel surface) |

Commands: `cargo test -p zylcode-core --lib intelligence::api` (payload +
rejection tests); frontend `typecheck`/`vitest` for the panel.

## 4. Factory × Agent unification — the seam, shipped

* `factory/agent_bridge.rs`:
  * `agent_intent_to_job` — a validated structured engineering intent
    (the N-ATLAS contract) becomes a durable `FactoryJob` whose graph is the
    already-tested, approval-gated `TaskGraph`;
  * `default_tool_runtime` — the restrictive gate (`allow_up_to(Execute)`)
    and JSONL evidence sink, so the HTTP surface cannot become a more
    permissive runtime;
* serve-intel: `POST /api/factory/run` (intent → durable job → runner until
  gate/completion), `POST /api/factory/approve` (human gate + re-run, with
  the approver recorded), `GET /api/factory/job` (durable job document).
* **Approval stays human**: approve steps map to real `Approval` actions and
  the runner parks in `AwaitingApproval`; `ModelTask` steps block with
  `BLOCKED_PROVIDER` and are never simulated.

The Mission-Queue/Factory semantic collision is resolved at the surface level
by spelling both out: a mission is what the operator asked for; the factory
job is how decomposed work is proven. The seam makes "agent plans, factory
executes" a real path rather than two silos.

Commands: `cargo test -p zylcode-core --lib factory::agent_bridge`;
e2e slice: `cargo test -p zylcode-core --test factory_job_e2e`.

## 5. Distribution (Phase D) — configured, honestly incomplete

* `tauri.conf.json`: `createUpdaterArtifacts: true`, NSIS per-user install,
  DMG / DEB / RPM targets already declared.
* `docs/RELEASE_INSTALLERS.md`: the build/sign/updater procedure, plus the two
  **stated blockers** (no signing key/certificate; GitHub Actions billing lock
  prevents release.yml from producing artifacts).
* README now carries a dated, honest "What actually works today" section
  listing working product-reachable capabilities and non-working ones.

## Evidence commands (one block, reproducible)

```bash
cargo test -p zylcode-core --lib            # 508 unit tests
cargo test -p zylcode-core --test factory_job_e2e --test natlas_runtime \
  --test natlas_boundary --test natlas_bridge
cd apps/zylcode-desktop && npx tsc --noEmit && npx vitest run
node scripts/registry_p14_rebaseline.mjs    # idempotent re-baseline (already applied)
```

## Limitations stated plainly

* This is **builder evidence**. GREEN/PARTIAL statuses added by this wave
  follow the P12 precedent and still await independent acceptance.
* No signed installer exists yet; distribution remains a documented procedure.
* Clippy did not run in the original wave gate. **Amended 2026-10-15:** the
  completion verification pass ran clippy over core, cli and desktop — clean
  (zero warnings, `-D warnings` effective policy). The frontend lint gap
  stands: `package.json` declares a `lint` script but no eslint dependency is
  installed; recorded as a gap, not skipped silently.
* Amended 2026-10-15: the interim verification pass committed with this wave
  also ran `cargo fmt --check` (clean), the factory e2e suite (7 tests) and
  the repository-intelligence benchmark suite (2 tests), the full vitest run
  (112 tests, 17 files) and the desktop TypeScript type-check — all green.
