# PHASE NUMBERING RECONCILIATION

**Issued:** 2026-09-16
**Issued by:** Architecture Owner (independent audit role)
**Trigger:** Architecture v2.1 reconciliation (`4844fcb`) surfaced conflicting phase-numbering
schemes coexisting in the repository.
**Verdict:** **THREE competing schemes. Two are historical and must be relabelled. One is
governing.**

---

## 1. The Problem

The repository contains documents that assign **the same phase numbers to different work**.

This is the single most dangerous class of documentation defect, because a task-scoped
implementation agent reading "implement Phase 12" has no way to know which Phase 12 is meant
without resolving the scheme. The Phase 2A failure was preceded by exactly this: the README
defined an R0–R5 ladder with different meanings from the governance package, and a green test
certified the wrong thing.

Phase numbers are worse than the ladder case, because the ladder was at least *called* R0–R5 in
both places. Here, two documents both say "Phase 12" and mean unrelated work.

---

## 2. The Three Schemes

### Scheme A — Governance roadmap (GOVERNING)

`docs/roadmap/ZYLCODE_ROADMAP_V2.md`, `docs/governance/ZYLCODE_MASTER_EXECUTION_PLAN.md`.
16 numbered phases in 6 epochs. Phase 7 splits into 7A–7D.

**Status: AUTHORITATIVE.** Ratified in `1af2072`, extended in `4844fcb`.

### Scheme B — Provider / frontend milestones (HISTORICAL — shipped)

`docs/API.md`, `docs/ARCHITECTURE.md`, `docs/PROVIDERS.md`, plus **source comments** in
`apps/zylcode-desktop/src-tauri/src/main.rs` and `crates/zylcode-core/src/router.rs`.

Covers multi-provider routing and failover: Phase 7.2 (multi-provider routing) and Phase 7.3
(provider configuration IPC + `ProviderSettings.tsx`).

**Status: HISTORICAL.** This work is **complete and shipped** — verified:

```
apps/zylcode-desktop/src/components/ProviderSettings.tsx   10,466 bytes   (exists)
crates/zylcode-core/src/router.rs:108  pub struct ProviderConfig { … }    (live)
apps/zylcode-desktop/src-tauri/src/main.rs:286  async fn get_provider_configs( … )
apps/zylcode-desktop/src-tauri/src/main.rs:770  get_provider_configs,      (registered)
```

It is not a live plan. The labels are stale markers on finished work.

### Scheme C — Strategic Plan (HISTORICAL — superseded)

`docs/STRATEGIC_PLAN.md`. Self-declared **"ACTIVE — single source of truth for all strategic
decisions"**, and linked from the README.

Its Section 6 roadmap uses Phases 9–14 for work unrelated to the governance roadmap, and
Appendix A explicitly documents a renumbering:

> | Old Phase | New Phase | What |
> | Phase 0 | Phase 9 | Plan Commit + README Fix |
> | Phase 1 | Phase 10 | Rungs 1–2 |
> | Phase 2 | Phase 11 | Rung 3 |
> | Phase 3 | Phase 12 | Rung 4 |
> | Phase 4 | Phase 13 | Offline Mode |
> | Phase 5 | Phase 14 | Cost/Rung UI + Benchmark |
>
> "Existing completed work (Phases 3.3–8.3) is not renumbered."

**Status: SUPERSEDED.** Its content is largely absorbed into the governance package:

| Strategic Plan topic | Now lives in |
|---|---|
| Verification rungs | `ZYLCODE_PROOF_GRAPH.md` (R0–R5, more rigorous) |
| Permission gate | Trust foundation §5.2; Phase 7B prerequisite |
| Global phase mapping | `ZYLCODE_MASTER_EXECUTION_PLAN.md` |
| Monetization / tiers | **Not yet a governance document** — see §5 |
| Offline / air-gapped mode | `docs/AIR_GAPPED.md` (separate) |

---

## 3. The Collision, Numbered

Phases 9–14 collide in **six of six cases**:

| # | Governance roadmap (A) | Strategic Plan (C) |
|---|---|---|
| **9** | Visual Intelligence & Self-Repair | Plan Commit + README Fix |
| **10** | Android Device Lab | Rungs 1–2 (First Shippable Verification) |
| **11** | macOS/iOS Worker | Rung 3 (Permission Gate) |
| **12** | Proof Engine v2 | Rung 4 (Z3/Dafny Proofs) |
| **13** | Delivery Engine | Offline Mode |
| **14** | Multi-Agent Engineering | Cost/Rung UI + Benchmark |

Additionally, `docs/ARCHITECTURE.md` uses **`Phase 8.1` / `Phase 8.2`** for
`ContextCompressor` and `VectorCacheStore`, colliding with the governance roadmap's
**Phase 8A/8B/8C** (Design Foundation, Canvas, Design↔Code).

And Scheme B's `Phase 7.2` / `Phase 7.3` sit inside the governance roadmap's **Phase 7**
(Browser Runtime / Computer Use) — a reader would reasonably conclude provider failover is
part of the browser phase.

---

## 4. Why This Happened

Not carelessness. These are **three different axes** wearing one label:

- **Scheme A** sequences *product capability* — what ZylCode can do.
- **Scheme B** sequenced *a feature's internal milestones* — steps within multi-provider routing.
- **Scheme C** sequenced *a remediation plan* — verification rungs and monetization, written at a
  time when the product roadmap was not settled.

Each is internally coherent. The defect is that all three used the token `Phase N` with no
scheme qualifier and no cross-reference, and two of them claim repository-wide authority.

---

## 5. Recommendation

**Relabel Schemes B and C. Do not renumber Scheme A.** The governance roadmap is ratified,
referenced by the Constitution, and depended on by the roadmap's gates. Renumbering it would
invalidate the audit trail.

### 5.1 Scheme B — rename, do not renumber

Replace milestone labels with names that cannot collide. Suggested mapping:

| Old | New |
|---|---|
| `Phase 7.2` | `MULTIPROVIDER-1` (multi-provider routing) |
| `Phase 7.3` | `MULTIPROVIDER-2` (provider configuration IPC) |
| `Phase 8.1` | `CTX-COMPRESSION` (ContextCompressor) |
| `Phase 8.2` | `VECTOR-CACHE` (VectorCacheStore) |

Touch: `docs/API.md:23`, `docs/ARCHITECTURE.md:14,20,21`, `docs/PROVIDERS.md:1,72`,
`apps/zylcode-desktop/src-tauri/src/main.rs:281`, `crates/zylcode-core/src/router.rs:63,205`.

**The source comments matter and are not cosmetic.** A Rust comment reading
`// Provider kind for Phase 7.2 multi-provider routing` sits inside a crate that will be extended
by the actual Phase 7 (Browser Runtime / Computer Use) work. It is a live trap.

### 5.2 Scheme C — mark superseded, preserve content

`docs/STRATEGIC_PLAN.md` declares itself the single source of truth and is linked from the
README. It must not keep that claim. Recommended:

1. Replace the status header with a **SUPERSEDED** banner naming the governing documents.
2. Add a prominent mapping table: each Strategic Plan phase → the governance phase that now owns
   that work (or "absorbed into Trust Foundation" / "no current owner").
3. **Preserve the file.** It contains monetization and pricing reasoning with no governance
   equivalent. Deleting it would destroy the only record of those decisions.
4. Update the README link to present it as history.

Do **not** move it to `docs/governance/superseded/` — that directory is reserved for
never-committed drafts, and this file *was* committed and *did* drive work. A distinct banner is
more honest than filing it with documents that never existed.

### 5.3 Scheme A — hold numbering stable

No change. Phase 7 remains 7A–7D; 8–16 unrenumbered. Consistent with the standing instruction.

### 5.4 The rule to record

Add to `ZYLCODE_AGENT_OPERATING_PROTOCOL.md`:

> **A phase number belongs to the governance roadmap and to nothing else.** Internal milestone
> tracking uses a named track with its own prefix, never `Phase N`. Where a document needs to
> reference work outside the roadmap, it names the track, not a number.

---

## 6. Unresolved — Needs Owner Decision

**Monetization and pricing have no governance home.** `STRATEGIC_PLAN.md` Sections 5–6 define
tiers, BYOK cost positioning, and a pricing gate, and these were **never absorbed** into the
governance package — the Constitution, Architecture and Master Execution Plan are silent on
revenue.

This is a genuine gap, not a numbering issue. Options:

1. **Absorb** — add a monetization section to the Constitution (it is a product-defining question).
2. **Defer** — record explicitly that monetization is deliberately out of scope until Phase 16
   (Public Commissioning), so its absence is a decision rather than an oversight.
3. **Separate governing doc** — a `ZYLCODE_COMMERCIAL_MODEL.md` referenced from the index.

Recommendation: **option 2 now, option 1 before Phase 16.** Recording the deferral closes the gap
cheaply and prevents another agent from re-deriving pricing strategy from a stale plan.

---

## 6.1 Resolution (2026-09-16) — options 2 and 3 combined

**Resolved.** A separate governing document was created and the deferral recorded in it:

- **`ZYLCODE_COMMERCIAL_MODEL.md`** — governs what is sold, the BYOK stance (no model markup,
  no paywalling a user's own evidence), the tier table, and the pricing gate.
- **Constitution Amendment 1** — registers it in §9.1 and records the change append-only per §13.
- **Gate restated as a capability state.** The former gate ("no pricing page until Phase 10
  ships") referenced an obsolete phase number. It is now: *no pricing until a capability exists
  at **R3** in the area a tier claims to sell.* Capability states do not drift when phases are
  renumbered — this is the general rule, and it is the same defect class as §2 of this document.
- **Tier ladder corrected.** The original tiers referenced a four-rung ladder from the superseded
  plan; they are restated against the authoritative **R0–R5** ladder.

Option 1 (absorbing pricing into the Constitution) was **rejected**: pricing changes on a business
cadence while the Constitution should not. The Constitution now *references* the commercial model
rather than containing it.

`STRATEGIC_PLAN.md` §5 carries a pointer to the governing document; its reasoning is retained as
the historical origin.

---

## 7. Reproduction

```bash
# The three schemes
grep -rn "Phase 7\.[0-9]" --include=*.md --include=*.rs . | grep -v target
grep -n "^### Phase" docs/STRATEGIC_PLAN.md
grep -n "^## Phase " docs/roadmap/ZYLCODE_ROADMAP_V2.md

# Scheme C's self-declared authority
sed -n '1,6p' docs/STRATEGIC_PLAN.md
sed -n '276,292p' docs/STRATEGIC_PLAN.md    # Appendix A renumbering table

# Scheme B is shipped, not planned
ls -la apps/zylcode-desktop/src/components/ProviderSettings.tsx
grep -n "pub struct ProviderConfig" -A 12 crates/zylcode-core/src/router.rs
grep -n "get_provider_configs" apps/zylcode-desktop/src-tauri/src/main.rs

# Authority inversion: two docs claim source-of-truth status
grep -rn "single source of truth" docs/
```
