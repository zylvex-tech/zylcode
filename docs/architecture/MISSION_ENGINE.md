# Mission Engine (Phase 3B)

**Status:** implemented engine core (durable, 11 states, cited-proof completion) — see
[§7 Implementation status](#7-implementation-status). This document is the contract, not a
completion report.

---

## 1. The rule that defines the feature

> `COMPLETE` requires every acceptance criterion to be satisfied by a **cited** proof.

Everything else in this document exists to make that sentence true under real conditions: a process
that can die at any moment, an operator who might want the answer to be yes, and a future reader who
will ask "how do you know?".

A mission therefore cannot be created without at least one acceptance criterion. An objective with no
criteria has no definition of done, and a mission engine that accepts one is a chat window with
paperwork.

---

## 2. States

```text
CREATED -> UNDERSTANDING -> PLANNING -> AWAITING_APPROVAL -> EXECUTING -> VERIFYING -> COMPLETE
                                  \-> BLOCKED                |             |
                                                             v             v
                                                          REPAIRING -> EXECUTING
                                                             |
Any non-terminal state -> FAILED / CANCELLED
```

| State               | Meaning                                                                |
| ------------------- | ---------------------------------------------------------------------- |
| `CREATED`           | Objective and criteria are recorded; nothing has been decided yet.     |
| `UNDERSTANDING`     | The objective and its criteria are being read.                         |
| `PLANNING`          | Steps are being derived from the criteria.                             |
| `AWAITING_APPROVAL` | A plan exists and no work runs until a human passes the gate.          |
| `EXECUTING`         | Steps are completed one at a time; each completion is durable.         |
| `VERIFYING`         | Every step is done; criteria are waiting for cited evidence.           |
| `REPAIRING`         | Verification found work still to do; a repair re-enters execution.     |
| `BLOCKED`           | Cannot proceed, with a recorded reason. Resumable, not failed.         |
| `COMPLETE`          | Terminal. Every criterion has a cited proof.                           |
| `FAILED`            | Terminal. Cannot be reached merely because verification is incomplete. |
| `CANCELLED`         | Terminal. Requires a reason.                                           |

`COMPLETE`, `FAILED` and `CANCELLED` are final. Reopening any of them would rewrite history, which is
the one thing an evidence system must never do. `BLOCKED` is deliberately _not_ terminal: a block is
a claim about the world ("the approval service is unreachable") that can stop being true.

The legal edges live in `Phase::can_transition_to`, and a same-phase record is allowed only as
in-phase progress (a completed step), never as a shortcut.

---

## 3. Durability: the journal is the truth

State lives in `<workdir>/.zylcode/`:

| File                    | Role                                                               |
| ----------------------- | ------------------------------------------------------------------ |
| `mission.journal.jsonl` | Append-only, one complete mission state per line. Source of truth. |
| `mission.json`          | Derived snapshot for humans and other tools. Never authoritative.  |

Every accepted change appends one line containing the **whole** mission after the change and calls
`fsync`. This makes the crash window precise:

- A crash during append can only leave an incomplete **final** line. Load discards it, reports
  `discarded_partial_tail`, and resumes from the previous complete record. The engine drops that torn
  tail before appending again, so one crash does not poison every later load.
- A corrupt line anywhere else, a sequence gap, a journal that starts at a phase other than
  `CREATED`, or an illegal transition inside the journal is **corruption**: load returns an error.
  Guessing a plausible past is worse than refusing to answer.
- A missing or stale snapshot is repaired on load and reported, because the journal already holds
  the answer.

Steps are the resumable unit: each completion is its own journal record, so a mission killed at step
2 of 5 knows exactly that.

### What "kill" means here

The CLI's `--crash-after-steps N` completes N steps, persists them, and exits with code 9. That is a
genuine process boundary — the next invocation is a new process with no memory — and it is how the
roadmap benchmark is exercised in
`crates/zylcode-cli/tests/mission_engine_cli.rs::kill_mid_executing_then_resume_completes_with_cited_evidence`.

---

## 4. Evidence and completion

A citation is `{criterion_id <- (kind, evidence_id, summary, cited_at)}`. `kind` is one of `test`,
`ledger`, `file`, `command`, `artifact`, `human`. `human` is allowed and deliberately visible as
unverified by machine.

Rules:

1. Evidence may only be cited while the mission is `VERIFYING`. Pre-baking proof before the work that
   produces it is exactly the failure mode this feature exists to prevent.
2. An empty or blank evidence id is rejected; it cannot satisfy anything.
3. Citing the same evidence id twice for a criterion is idempotent — no duplicate record is written.
4. `COMPLETE` is refused while any criterion lacks a citation, and the error names each one.
5. `VERIFYING` with unmet criteria is not a failure and not a success: the CLI exits 2 and says
   "not COMPLETE", naming the criteria still owed a proof.

The check lives in the transition itself (`append_transition`), not only in the CLI, so no caller can
walk around it.

---

## 5. Entry points

`zylcode missions create | status | resume | cite | cancel`

```console
$ zylcode missions create --workdir . --objective "ship the engine" \
    --criterion "tests pass" --criterion "docs updated" \
    --step "write code" --step "run tests"
$ zylcode missions resume --workdir .               # stops at AWAITING_APPROVAL
$ zylcode missions resume --workdir . --approve     # executes, then waits for evidence
$ zylcode missions cite --workdir . --criterion c1 --evidence test:mission_engine --kind test
$ zylcode missions status --workdir .
$ zylcode missions cancel --workdir . --reason "owner withdrew the objective"
```

Exit codes: `0` moved forward or already terminal, `2` verification is waiting on citations (not
complete), `9` execution paused deliberately (`--crash-after-steps`), `1` error.

**Naming note (deliberate):** `zylcode mission` (singular) is the existing First Mission runner and
keeps its flags. The engine is `zylcode missions` (plural) until the runner is folded into the engine;
the roadmap's `zylcode mission create|status|resume` is served by that plural command today. Stating
this here is cheaper than pretending the two are already one command.

---

## 6. Non-goals for this increment

- The engine does not decide _how_ to execute a step. It records durable progress and enforces the
  state contract; the executor (agent loop, Best-of-N, tools) is a caller.
- No model invocation, no tool permissions, no UI. The desktop Mission surface is not wired to this
  engine.
- Evidence is cited by id; this module does not hash or verify the artifact it names. Ledger
  verification remains the ledger's job — the citation points at it.
- No cross-process lock. Concurrent writers on the same workdir can interleave; the journal would
  record both, and the second write's transition may be refused as illegal. Serialising writers is
  outstanding.

---

## 7. Implementation status

| Piece                                                     | State                                                             |
| --------------------------------------------------------- | ----------------------------------------------------------------- |
| 11-state machine with legal-edge enforcement              | Implemented (`mission_engine.rs`, unit-tested)                    |
| Cited-proof completion, refused with named unmet criteria | Implemented                                                       |
| Durable journal with `fsync`, torn-tail recovery          | Implemented                                                       |
| Interior corruption / sequence gap fails closed           | Implemented                                                       |
| Kill-resume across real processes                         | Verified by CLI integration test (exit 9, then resume)            |
| `create` / `status` / `resume` / `cite` / `cancel` CLI    | Implemented                                                       |
| Desktop Mission surface using the engine                  | Not implemented                                                   |
| Mission persistence inside the Project System (Phase 3A)  | Not implemented                                                   |
| Cross-process writer lock                                 | Not implemented                                                   |
| Automatic evidence verification (hashes, ledger replay)   | Not implemented; citations are pointers, not proofs by themselves |

The engine is therefore **not** the whole of Phase 3B, and Phase 3B is not accepted. What is claimed
is narrow and checkable: the states, the completion rule, and durability across a kill are real and
tested. What is not claimed is equally explicit, above.
