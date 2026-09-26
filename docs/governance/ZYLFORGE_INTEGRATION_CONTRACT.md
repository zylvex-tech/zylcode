# ZylForge ↔ ZylCode Integration Contract

**Status:** SPECIFIED (contract v1) — the ZylCode-side adapter is IMPLEMENTED and
TESTED (`crates/zylcode-core/src/zylforge_bridge.rs`); live ZylForge connectivity
is UNVERIFIED until a real ZylForge instance responds to it.

**Date:** 2026-09-26
**Authority:** This document governs every ZylCode↔ZylForge interaction. Where
other documents disagree, this contract wins for cross-product matters.

---

## 1. Product boundary

| Domain | Authoritative product |
|---|---|
| CAD geometry, OCCT operations, geometry measurements | **ZylForge** |
| Engineering validity of `.zyl` contents | **ZylForge** |
| Software projects, missions, evidence, artifacts, delivery | **ZylCode** |
| Cross-product orchestration and handoff | **ZylCode** |
| `.zyl` artifact *consumption* and provenance recording | **ZylCode** (read-only) |

ZylCode is not a second geometry kernel. It never executes, simulates, or
reinterprets OCCT operations.

---

## 2. Capability references

A ZylForge capability is referenced by ZylCode as a **capability descriptor**:

| Field | Meaning |
|---|---|
| `capability_id` | Stable ID (`zylforge.<workbench>.<operation>`) |
| `product` | Always `zylforge` |
| `workbench` | ZylForge workbench name |
| `operation` | Operation name |
| `state` | ZylForge's own state (`PROPOSED`/`IMPLEMENTED`/`RUNTIME_VERIFIED`/…) |
| `evidence_ref` | ZylForge evidence reference, carried verbatim |
| `version` | ZylForge build/version string |
| `limitations` | ZylForge's recorded limitations, carried verbatim |

**Rules**

1. ZylCode records the descriptor exactly as ZylForge publishes it. It does
   not upgrade a capability's state, trim its limitations, or interpret its
   evidence.
2. A ZylForge capability in `PROPOSED` or `SPECIFIED` state stays that state
   inside ZylCode. Converting it to `IMPLEMENTED` anywhere in ZylCode surfaces
   is a governance violation.
3. Unknown/missing descriptors are recorded with state `UNVERIFIED` — absence
   of evidence is recorded as absence of evidence.

---

## 3. `.zyl` artifacts

| Field | Meaning |
|---|---|
| `artifact_type` | `zyl_package` (see Artifact Bus) |
| `schema_version` | The `.zyl` package's own schema version, carried verbatim |
| `project_id` | ZylCode project the package is associated with |
| `geometry_provenance` | ZylForge operation history that produced the geometry |
| `feature_tree_provenance` | ZylForge feature tree reference |
| `telemetry_ref` | ZylForge telemetry record reference |
| `verification_ref` | ZylForge verification summary reference |
| `source_build` | ZylForge build identity |
| `hash` | SHA-256 of the package bytes (pinned by the Artifact Bus) |

**Rules**

1. ZylCode consumes `.zyl` packages read-only: it hashes, stores, lists, and
   attaches them. It does not regenerate, reinterpret, or rewrite geometry.
2. Any rewrite of `.zyl` content requires a future, versioned, explicitly
   governed contract — none exists at contract v1, so no ZylCode code path may
   write `.zyl` content.
3. A placeholder artifact must never be presented as a production `.zyl`.

---

## 4. MCP boundary

ZylCode **may**:

- Inspect ZylForge capability metadata (descriptors above).
- Read approved artifacts (`.zyl`, Job Pack references).
- Request a plan from ZylForge.
- Request a verification summary from ZylForge.
- Request a Job Pack handoff.
- Record provenance for all of the above in its own ledger and Artifact Bus.

ZylCode **must not**:

- Pretend to execute OCCT geometry.
- Claim geometry success without ZylForge evidence.
- Bypass human approval for any cross-product action.
- Rewrite `.zyl` content (no such contract exists).
- Convert proposed ZylForge capability into implemented status.

---

## 5. Evidence exchange

ZylCode records, in its evidence ledger / Proof Engine, the following
cross-product events — each with explicit states:

| Event | Recorded as |
|---|---|
| ZylForge operation proposal received | `PROPOSED` + descriptor ref |
| User approval granted/denied | `ACCEPTED`/`REJECTED` + reviewer |
| Kernel result reported by ZylForge | carried verbatim; state = ZylForge's state |
| Verification result reported | `RuntimeCommand` proof only if ZylForge attests runtime execution; otherwise `UNVERIFIED` |
| Artifact hash pinned | Artifact Bus `Generated` |
| Runtime capture | linked artifact refs |
| Blocked journey | `BLOCKED` + reason |
| Unsupported feature | `UNVERIFIED`/`BLOCKED` + descriptor ref |

**Rule:** ZylCode never upgrades a reported result. "ZylForge says the kernel
succeeded" is recorded as *ZylForge-attested*, and marked `RUNTIME_VERIFIED`
only when ZylForge's own evidence says runtime verification happened.

---

## 6. Adapter obligations (ZylCode side)

`zylforge_bridge.rs` is the only module allowed to know about ZylForge. It:

1. Parses capability descriptors; preserves unknown fields verbatim.
2. Records state transitions into the Project System's store of record
   (Artifact Bus + Proof Engine) with provenance strings naming ZylForge.
3. Fails closed: a descriptor whose state field is missing/unknown is
   `UNVERIFIED`, never assumed good.
4. Exposes read-only queries; nothing in the public API writes `.zyl` bytes.

Until live connectivity exists, all bridge outputs carry
`connectivity: UNVERIFIED` and no ZylCode surface may display ZylForge state
without that qualifier.
