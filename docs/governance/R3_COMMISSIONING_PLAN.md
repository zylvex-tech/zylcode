# R3 COMMISSIONING PLAN — initial real-tool slice

**Status: PLAN ONLY. Nothing in this document is commissioned.**
`r3_verified_count` is **0**, and `tool_catalogue::tests::no_capability_claims_r3_yet`
fails if that ever changes without this plan being executed.

Authority: owner decision, Option C — controlled consolidation, units 7 and 8.

---

## 1. Why R3 is a separate gate

Passing unit and integration tests makes a capability **R2 (EXECUTED)**. It does
not make it R3.

| Rung | Meaning | Evidence required |
|---|---|---|
| R0 | Claimed | a statement |
| R1 | Observed | read in source |
| R2 | Executed | a committed test ran it |
| **R3** | **Verified** | **reachable through a named product surface, with captured evidence** |
| R4 | Reproducible | a documented procedure reproduces it |
| R5 | Commissioned | in production use |

R2 proves the executor works. R3 proves a *user* can reach it and that what
happened is recorded. Those are different claims, and conflating them is how a
library gets mistaken for a product.

---

## 2. The initial slice

| id | risk | why it is in the first slice |
|---|---|---|
| `fs.read` | Read | already R2; the simplest end-to-end path |
| `fs.write` | Write | the first mutating capability; proves the evidence trail records a state change |
| `search.find` | Read | exercises path traversal without mutation |
| `search.grep` | Read | exercises content search |
| `git.status` | Read | proves git binding through a product surface |
| `git.diff` | Read | proves the second git binding |
| `shell.execute` | Execute | the escape hatch; proves the *permission gate* is what constrains it |

**Deliberately excluded: `git.commit`.**

It is the only mutating git capability, it is `RiskLevel::GitWrite`, and
`product_reachable` is currently `false` for it. It should be commissioned only
after the read/write slice has demonstrated that the evidence record is complete
enough to reconstruct what a commit did. A commit is the first irreversible
action; it should not be the first thing through the gate.

---

## 3. The evidence record — required for every candidate

For each capability, capture all eleven fields. A missing field means the
capability stays at R2.

| # | Field | Notes |
|---|---|---|
| 1 | **actor** | who or what initiated it — human user id, or agent session id |
| 2 | **named entry point** | the exact UI control, CLI invocation, or API route. "The app" is not an entry point |
| 3 | **tool id** | the catalogue id, e.g. `fs.read` |
| 4 | **input** | the exact parameters passed |
| 5 | **permission decision** | the risk class, and the allow/deny outcome with its reason |
| 6 | **actual operation** | what was really executed — for `git.status`, the literal command line |
| 7 | **actual output** | the real result, not a summary |
| 8 | **state change** | files created/modified/deleted, with before and after. "none" is a valid and required answer |
| 9 | **evidence record** | the persisted `ToolEvidence` — invocation id, timings, exit status, stdout/stderr digest |
| 10 | **reproduction procedure** | the exact steps a second person follows to get the same result |
| 11 | **timestamp + commit SHA** | the build the evidence was captured against |

### 3.1 Where the record comes from

`ToolEvidence` carries most of fields 3, 6, 7, 9 and 11 (`real_tools.rs`):
`invocation_id`, `tool_id`, `session_id`, `actor`, `bound_operation`, `risk`,
`approval_decision`, `arguments`, `working_directory`, `start_time`, `end_time`,
`exit_status`, `stdout`, `stderr`, `changed_files`, `timeout`.

### 3.2 Gaps — updated after evidence persistence landed

| Gap | Status |
|---|---|
| A permission gate exists and writes a decision | **CLOSED.** `crates/zylcode-mcp/src/permission.rs`. Every dispatch resolves, then gates, then executes. `PermissionDecision::{Allow, Deny, RequireApproval}` is recorded into `ToolEvidence::approval_decision` **whether or not the tool ran** — a refusal is an event worth keeping. |
| `approval_decision` is populated | **CLOSED.** Field 5 can now be captured. |
| An actor is modelled | **CLOSED as a field, OPEN in practice.** `ToolContext::actor` and `ToolEvidence::actor` exist and are recorded, but every production construction site passes `None`. `DynamicTool::call` and `BuiltinTool::call` do not know who called them. Field 1 cannot be captured truthfully until the callers supply an identity. |
| Evidence is persisted | **CLOSED as a mechanism, OPEN in production.** `crates/zylcode-mcp/src/evidence.rs`. `JsonlEvidenceSink` writes every dispatch outcome to `./.zylcode/evidence.jsonl` (or `$ZYLCODE_EVIDENCE_LOG`). Refusals are recorded as well as successes. Field 9 can now be captured. The default sink is local to the process; a production deployment would replace it with a shared audit log or database. |
| A named product entry point | **OPEN.** Fields 2 and 10 need a UI or CLI path with a reproduction procedure. |

All three original prerequisites are met. R3 still cannot be claimed because the
only entry points are library calls and tests — there is no named product surface
a user can reach. The `actor` field is also `None` in practice because no caller
supplies an identity. Claiming R3 without a reachable product path would be the
same unverifiable assertion this Gate has been removing.

### 3.3 The gate's default posture

`PermissionPolicy::default()` allows `Read` and requires explicit approval for
everything above it. This is deliberately the restrictive default: a permissive
default would mean every caller who forgot to configure a policy silently got one
that permits arbitrary execution.

Consequences a commissioner must know:

* `fs.read`, `fs.list`, `search.*`, `git.status`, `git.diff` run unapproved.
* `fs.write`, `shell.*`, `npm.run`, `cargo.test`, `git.commit` **require** an
  allow-list entry or `ToolContext::approval_required = true`.
* `shell.execute` — the one unbound escape hatch — is gated the same way as
  `git.commit`. That is the point of it being unbound: it is not special-cased
  into permission.
* Tests that exercise a mutating executor must use
  `PermissionGate::permissive()`. The name is the point: a reader can see the
  choice, and `the_default_is_the_restrictive_one` fails if anyone "fixes" a test
  by loosening the default instead.

---

## 4. Procedure per capability

1. **Prepare.** Start the desktop app (or the CLI) at a recorded commit SHA.
2. **Act.** Perform the operation through the named entry point — not through a
   test, not through a library call.
3. **Capture.** Record all eleven fields. Save the `ToolEvidence` JSON.
4. **Falsify.** Repeat the operation with a permission that should be denied and
   record that the denial occurred *and* that no operation ran. A gate that
   cannot deny is not a gate.
5. **Reproduce.** Hand fields 10 and 11 to a second person with no prior context.
   They must reach the same output and the same state change.
6. **Record.** Append the capture to `docs/governance/R3_COMMISSIONING_EVIDENCE.md`
   with the SHA.
7. **Promote.** Only then set the entry's rung to `R3Verified` in
   `EXECUTABLE_SPECS` and remove the guard in
   `tool_catalogue::tests::no_capability_claims_r3_yet`.

---

## 5. What R3 is not

* **Not** "the tests pass." That is R2.
* **Not** "the executor works." That is also R2.
* **Not** "the tool is registered." Registration is a precondition, not evidence.
* **Not** achievable for the 21 definition-only entries. They have no executor;
  they cannot reach R1, let alone R3.

---

## 6. Relationship to the "ZylCode moment"

The owner's framing: *a user gives ZylCode a repository task; ZylCode actually
reads files, searches the repository, edits something, runs the build/test suite,
observes a failure, repairs it, reruns verification, shows the diff, survives
interruption, and produces an evidence trail proving what happened.*

That end-to-end loop is a **composition** of R3 capabilities. It cannot be
demonstrated while the individual steps are R2. This slice — seven capabilities,
all read-only except `fs.write` and `shell.execute` — is the smallest set that
can carry that loop honestly, and `git.commit` is deliberately held back until
the loop's evidence trail is proven.

The honest milestone is twelve tools that work, not one hundred and fifty names
attached to a simulator.
