# Architecture Contributions

This guide governs changes to the **architecture** — anything that alters system topology, contracts,
data ownership, the dependency order, or the trust foundation described in
`docs/governance/ZYLCODE_ARCHITECTURE_V2.md` and the Constitution.

## The rule

> **Architecture changes require explicit rationale and cannot silently violate the Constitution.**

The Constitution (`ZYLCODE_PRODUCT_CONSTITUTION_V2.md`) is the supreme spec. An architecture change
that contradicts it without an amendment is a **defect**, to be recorded as such — not quietly
implemented or quietly reverted.

## Before you change the architecture

1. Read the Constitution sections the change touches.
2. Read the relevant per-system spec under `docs/architecture/`.
3. Ask: does this change a **rung**, a **dependency**, a **data-ownership** boundary, or a
   **trust-foundation** behaviour? If yes, it is an architecture change — proceed with a proposal, not
   a direct PR.
4. Confirm the change does not break the **rung ceiling** (A7) or the **reachability** principle (A10).

## How to propose

- Open a GitHub Discussion under **Architecture** (or an issue using `feature_request`) describing:
  - the current contract;
  - the proposed change;
  - the rationale (the *why*, with evidence);
  - the rung/dependency/data-ownership impact;
  - the migration or compatibility story.
- If the change alters the Constitution, it requires an **amendment** recorded in the Constitution's
  §13 log (append-only: date, author, changed clause, rationale, evidence). Amendments touching §1–§3
  require independent review.

## What is not an architecture change

- Fixing a bug within a defined contract.
- Adding a capability *within* an existing system's stated scope.
- Documentation that describes reality more accurately (that is a correction, not a change).

## Anti-patterns

- "I'll just move this module; it's cleaner." — if it changes a dependency order, it's architectural.
- Renumbering phases to make room. — the roadmap is stable; split or insert subphases instead.
- Adding a capability to the roadmap as a phase when it is a **named track** (rule 4.7).
- Describing an unimplemented system as shipping. — mark PROPOSED; if a non-functional skeleton
  exists, say so explicitly.

## After acceptance

Implementation follows the normal contribution flow, in its own bounded commit, with the architecture
decision referenced. The auditor verifies the implementation matches the accepted architecture, not
just that it compiles.
