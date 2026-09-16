# Capability Evidence Guide

Every capability ZylCode claims must be backed by evidence at a defined Proof Graph rung. This guide
explains how to **add or change a capability entry** in `docs/capability-registry.json` without
fabricating status.

## The ladder (authoritative)

| Rung | Name | What it means |
|---|---|---|
| **R0** | CLAIMED | Claimed only. No evidence. A hand-asserted status is R0 by definition. |
| **R1** | OBSERVED | Runs; one-off observation. |
| **R2** | EXECUTED | Unit/integration tested; reachable behaviour. |
| **R3** | VERIFIED | Reachable through a named product surface, with captured evidence. |
| **R4** | REPRODUCIBLE | Independently reproduced by another party. |
| **R5** | COMMISSIONED | Publicly shipped and accepted. |

Full definitions: `docs/governance/ZYLCODE_PROOF_GRAPH.md`.

## Adding a capability — checklist

1. **Choose the rung from evidence, not optimism.** If it is tested and reachable, R2. If a user can
   invoke it from a named surface and you captured the transcript, R3. If you merely wrote the code, it
   is at most R2 (and only if tested).
2. **Record the entry point.** Name the command, CLI subcommand, or UI surface that reaches it. An
   unreachable subsystem is not a capability regardless of test coverage (Constitution A10).
3. **Attach evidence.** `evidence_commit` (the SHA where it was demonstrated), `verified_at`, and a short
   `reproduction` string (commands + expected output).
4. **Record limitations.** What does not work, what is faked, what is PROPOSED. The Computer-Use Engine
   is the standing example: R0 with a non-functional skeleton present — say so.
5. **Update the architecture status table** in `ZYLCODE_ARCHITECTURE_V2.md` §9 to match.

## The rung ceiling

A capability's rung cannot exceed the weakest link in its critical path. Reaching R3 requires the
trust-foundation pieces it depends on to be at R3. If Permissions is below R3, a system depending on it
is blocked from R3 — record the block, do not claim around it.

## Prohibited

- **Fabricated evidence** — runtime output, test results, benchmarks, screenshots, counts. Never.
- **Expected-as-observed** — a written expectation is not a captured run.
- **Self-certification** — the builder does not mark their own work GREEN. An independent auditor does.
- **`(Pending)` SHAs** — include a real SHA or omit the field.
- **Over-claiming scope** — selling one R3 capability does not make the whole product R3.

## Example entry (illustrative)

```json
{
  "id": "repository_intelligence",
  "product": "zylcode",
  "rung": "R2",
  "status": "PARTIAL",
  "entry_points": [],
  "scope": "Index, symbol lookup, dependency graph implemented and unit-tested.",
  "limitations": "Not integrated into any product surface; benchmark fails on re-run.",
  "evidence_commit": "0ecea8e",
  "verified_at": null,
  "disputed": true
}
```

`disputed: true` is how a re-audit is recorded — not by silently overwriting the old value.
