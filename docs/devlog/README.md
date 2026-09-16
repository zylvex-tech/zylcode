# ZylCode Development Log

**Track:** `PUBLIC-FOUNDATION-03` — Development Log (build-in-public infrastructure)
**Status:** Infrastructure established. Posts are added per the editorial queue below.

This is the engineering journal for ZylCode and the ZylForge product family. Its purpose is specific
and narrow: **to show how this project is actually built, including the failures**, so that public
claims about the product remain tied to evidence.

This is **not** a marketing channel. Every entry follows a fixed structure that forces failure and
limitation into the open.

---

## Post structure (mandatory)

```
PROBLEM
WHAT WE ATTEMPTED
WHAT FAILED
WHAT WE LEARNED
WHAT CHANGED
CURRENT LIMITATIONS
EVIDENCE
NEXT STEP
```

`WHAT FAILED` and `CURRENT LIMITATIONS` are **not optional**. An entry that omits them is a promotion,
not a log. Publishing failure is the mechanism by which this project earns the right to publish success.

---

## Editorial standard

- **Real only.** No hypothetical scenarios, no aspirational "we will" dressed as history.
- **Evidence-linked.** Every claim cites a commit, a test run, or an artifact. No evidence → no claim.
- **No security-sensitive internals.** Do not publish unpublished vulnerability details, secret
  architecture, or anything that would aid an attacker.
- **Label maturity.** If a described system is PROPOSED or IN DEVELOPMENT, say so. The public status
  vocabulary (`ZYLCODE_PUBLIC_COMMUNICATION_POLICY.md` §3) applies here too.
- **No fabricated screenshots or counts.** A log entry about a benchmark cites the run; it does not
  invent one.

---

## Naming

`YYYY-MM-DD-<slug>.md`. One file per post. Dated by the day it is published, not drafted.

---

## Editorial queue

| # | Subject | Status | Evidence anchor |
|---|---|---|---|
| 1 | Why coding agents shouldn't certify their own work | **SEEDED** | Phase 2A re-opening (`0ecea8e` → audit) |
| 2 | We caught our own false benchmark | PLANNED | Phase 2A benchmark 276s vs claimed 12.7s |
| 3 | How ZylCode survives an agent crash | PLANNED | Recovery subsystem (Phase 1D) |
| 4 | R2 is a library; R3 is a capability | PLANNED | Proof Graph rung definitions |
| 5 | Why ZylCode uses an Evidence Ledger | PLANNED | Ledger design (Phase 1D) |
| 6 | Building repository intelligence without lying to ourselves | PLANNED | Phase 2A audit + remediation |
| 7 | Why a model dropdown is not Model Democracy | PLANNED | Model Platform status |
| 8 | Building ZylCode in public | PLANNED | This log + Public Foundation track |

New subjects are added here first, then written, then linked. The queue is the contract with readers
that we publish substance, not a schedule we pretend to keep.

---

## Relationship to the governance

The development log is the **public face** of the same discipline the internal governance enforces:
evidence over assertion, independent verification over self-certification, and honest limitation over
optimistic summary. Where the two would conflict, the governance wins and the log says so.
