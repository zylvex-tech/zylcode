# Website Truth Audit — PUBLIC-FOUNDATION-04

**Track:** `PUBLIC-FOUNDATION-04`
**Status:** Repo-evidence portion complete; live-site crawl is an **owner action** (see below).
**Output:** `PUBLIC_CLAIM_MATRIX.yaml` (the claim of record).

---

## What this audit is for

Before any ZylForge.com or ZylCode public copy is written, every material capability claim must be
reconciled against the implemented rung. This audit produces the **Public Claim Matrix** that the
website, README, and roadmap read from. It prevents a website from promoting a PROPOSED capability as
shipping — the exact failure mode that triggered the governance package.

---

## Limitation — live-site crawl from the sandbox

> Both `zylforge.com` and `zylvex.tech` return **HTTP 200** (reachable), but the response **body does not
> transfer** through the build sandbox on this attempt (the fetch errored on body write). The live-page
> text could therefore **not** be extracted or audited from here.

This is the same limitation encountered with GitHub metadata in PF-01: a capability I cannot observe, I
do not assert. The live crawl is the **owner's action**, with the checklist below. The matrix below is
built from the **authoritative repo evidence** (architecture rung table + capability registry), which is
the correct source anyway — the matrix is defined as derived from those, not from marketing copy.

---

## Repo-evidence findings (what we CAN establish)

1. **Phase 2A ("Repository Intelligence Foundation") is RE-OPENED, not accepted.** The capability
   registry marks its 11 sub-capabilities `DISPUTED`. A public claim of "production-ready repository
   intelligence" is therefore a **prohibited claim** until remediated and re-audited. Matrix entry:
   `repository_intelligence` → `IN_DEVELOPMENT`, `R2`, `disputed: true`.

2. **The capability registry's `GREEN` count is not a permit to advertise.** `summary.green: 20` counts
   non-disputed GREEN, but most entries carry `rung: null` and are internal Phase 1A–1D building blocks,
   not public product capabilities. The registry's own audit section records that 11 previously-GREEN
   entries were disputed. Public claims use the **matrix**, not the raw registry `GREEN`.

3. **Computer-Use Engine is a simulation facade (R0).** The code in `crates/zylcode-core/src/computer_use/`
   returns fabricated data and never acts; tests assert `is_ok()` on a facade that cannot fail. Public
   claim: `PROPOSED` ("planned"). Prohibited: "can operate your computer."

4. **What IS genuinely available (R3, reachable):** agent kernel tool loop, evidence ledger, crash
   recovery, shell execution. These may be described as `AVAILABLE` per the matrix.

5. **Service-delivered engineering capabilities** (job packs, fabrication docs) are marked
   `SERVICE_DELIVERED` but each carries `owner_verify_required: true` — the owner must confirm Zylvex/
   ZylForge can actually deliver before any paid claim.

---

## Owner crawl checklist (run against the live sites)

For each capability claim found on `zylforge.com` / `zylvex.tech` / the README:

- [ ] Locate the claim's text and the page it appears on.
- [ ] Find the matching `capability` entry in `PUBLIC_CLAIM_MATRIX.yaml`.
- [ ] Does the live `capability_state`/`public_label` match or **exceed** the matrix entry?
      - Exceeds → **prohibited claim**. Record it in the table below and correct before publication.
      - Matches → acceptable.
- [ ] Does the claim use a word from the canonical vocabulary, or an unapproved term (e.g. `BETA`,
      `best`, `fastest`, `production-grade`, `formally verified`)?
- [ ] For service claims: is `owner_verify_required` satisfied (real delivery capability confirmed)?
- [ ] For any screenshot/mockup: is it labelled `CONCEPT` / `PROPOSED` / `IN DEVELOPMENT` on the image itself?

### Discrepancy log (fill during crawl)

| Site/page | Claim found | Matrix entry | Verdict (match / exceed) | Action |
|---|---|---|---|---|
| | | | | |

---

## How discrepancies flow back

A live claim that exceeds the matrix is corrected at the source (the site), **not** by inflating the
matrix. If the matrix is wrong (the capability actually advanced), the matrix entry is updated via the
normal governance change — with evidence — and the architecture rung table updated first. Per
Constitution §11, a corrected claim stays visible; it is not silently rewritten.

---

## Deliverable status

- [x] `PUBLIC_CLAIM_MATRIX.yaml` — written from repo evidence, canonical two-axis vocabulary.
- [x] Repo-evidence findings recorded above.
- [ ] Live-site crawl — **owner action**, using the checklist.
- [ ] Reconcile live claims → matrix; correct any over-statement before PF-08/PF-09 (website design/impl).
