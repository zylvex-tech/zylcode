# ZylCode Proof Graph

> The canonical definition of what it means for anything in ZylCode to be *proven*.

**Status:** GOVERNING
**Companion to:** `ZYLCODE_PRODUCT_CONSTITUTION_V2.md` §3
**Supersedes:** the build-centric verification ladder in `README.md` (for governance purposes)

---

## 1. Why the old ladder was wrong

The existing ladder was build-centric:

```
Rung 0 — unverified
Rung 1 — static analysis
Rung 2 — formal proof
Rung 3 — formally proven and reviewed
Rung 4 — proven, reviewed, audited
```

It applies only to *source code artifacts*. But ZylCode must make provable claims about things
that are not source code:

- "The Android build launches on the emulator."
- "The design round-trips to Jetpack Compose."
- "The scanner excludes secrets."
- "This model is better at this task class."
- "The extension ABI accepts an out-of-tree package."

None of these is "static analysis" or "formal proof". Under the old ladder they had no rung at
all — which is precisely how a scanner indexing `target/` could be recorded as **GREEN**.

> The Phase 2A audit is the case study: a capability that passed every existing test, was
> reported complete, and was recorded GREEN, while being unusable and mis-measured.

---

## 2. The Proof Ladder

**This definition applies to any capability, not merely source code.**

| Rung | Name | Definition | What exists |
|---|---|---|---|
| **R0** | **CLAIMED** | Asserted in documentation, a report, or the registry. | A sentence. Nothing else. |
| **R1** | **OBSERVED** | Seen to work once, informally. | A human memory. No procedure. |
| **R2** | **EXECUTED** | Runs under automated test or a committed script. | A test. Not reachable by a user or agent. |
| **R3** | **VERIFIED** | Reachable and usable through a product surface, with captured evidence. | A user or agent can invoke it; output is captured. |
| **R4** | **REPRODUCIBLE** | A third party re-runs a committed procedure and obtains the same result. | A reproduction block that works on another machine. |
| **R5** | **COMMISSIONED** | Independently audited by a party that did not build it, and accepted. | An audit record. |

### 2.1 The critical distinction: R2 vs R3

> **R2 is a library. R3 is a capability.**

A module with 79 passing unit tests that no user or agent can reach is **R2**. This is the most
important line in this document, because it is the line that was crossed incorrectly in Phase 2A.

**Reachability test.** For a capability to be R3, all of the following must be true:

1. There is a **named entry point** a user or agent can invoke (CLI command, UI action, tool,
   API method exposed to the agent).
2. There is a **captured transcript** of that entry point producing the claimed behaviour.
3. The entry point is **committed** and reachable from a tagged build.

If any of the three is missing, the capability is R2 regardless of test coverage.

### 2.2 The rung ceiling

> **A system's rung is the minimum rung of the critical-path components it depends on.**

```
Intelligence Graph = min(scanner, symbol index, dependency graph, query API, integration)
```

A system with four R3 components and one R2 component is **R2**. Optimism is not additive.

---

## 3. Evidence Requirements Per Rung

| Rung | Required evidence artifact |
|---|---|
| R0 | the claim itself, in a document |
| R1 | a dated note: what was done, on what machine, what was seen |
| R2 | a committed test/script, plus its captured output |
| R3 | **reproduction block**: command · environment · raw stdout · commit SHA · the product surface used |
| R4 | R3 block that has been executed **by a second party** on a **different machine**, with their output attached |
| R5 | an audit record naming the auditor, the commit, the method, and the verdict |

**A completion report without a reproduction block is a draft, not a report.**

---

## 4. The Proof Graph

Proofs form a directed graph, not a flat list. A proof may depend on other proofs.

```
                    Mission: "Password reset works"
                              (acceptance criteria)
                                        │
        ┌───────────────┬───────────────┼───────────────┬───────────────┐
        ▼               ▼               ▼               ▼               ▼
   Request form    Email flow     Token expiry   Validation     Browser test
      (proof)        (proof)         (proof)       (proof)         (proof)
        │               │               │               │               │
        └───────────────┴───────┬───────┴───────────────┴───────────────┘
                                ▼
                        Artifacts (citable)
                                │
                                ▼
                        Evidence Ledger (append-only)
```

### 4.1 Node types

| Node | Meaning |
|---|---|
| **Claim** | Something asserted to be true |
| **Criterion** | An acceptance condition attached to a Mission |
| **Proof** | An evaluated claim with a rung and a citation |
| **Artifact** | A citable, inspectable output (report, screenshot, diff, log, trace) |
| **Evidence** | An immutable ledger entry recording what happened |
| **Decision** | A recorded choice (including model routing and permission grants) |

### 4.2 Rules

1. **Every Proof cites at least one Artifact.** A proof without a citation is a Claim.
2. **Every Artifact resolves to a Ledger entry.** Artifacts cannot be fabricated post-hoc.
3. **A Criterion is satisfied only by a Proof at R3 or above.** Tests alone do not satisfy a
   user-facing acceptance criterion.
4. **A Mission cannot reach `COMPLETE` while any Criterion is unsatisfied.** Fail closed.
5. **Downgrades are recorded, not deleted.** If a capability regresses, a new Proof supersedes
   the old one and the change is visible.
6. **An index is not a ledger.** The Intelligence Graph may be rebuilt; the Ledger may not.

### 4.3 Proof invalidation

A Proof is invalidated when any of its inputs change:

- the artifact it cites changes;
- a dependency's proof rung drops;
- the environment in which it was produced is no longer reproducible.

Invalidated proofs must be re-established or explicitly marked **STALE**. Silently keeping a
stale proof is a defect.

---

## 5. Worked Examples

### 5.1 The Phase 2A scanner — how this would have caught it

| Aspect | Under the old ladder | Under the Proof Graph |
|---|---|---|
| Unit tests pass | recorded GREEN | **R2** (not reachable) |
| Headline metric | published as evidence | **R0** — the number was never asserted by any test |
| "Benchmark PASS" | recorded as PASS | **fails R2** — does not reproduce |
| Reachability | not assessed | **fails R3** — no entry point exists |
| Real `.gitignore` | not tested | **fails R2** — fixture differed from reality |

**Result under the Proof Graph: R2, not GREEN.** The mis-classification is prevented by
construction, not by vigilance.

### 5.2 "Secret files are excluded from indexing"

- **R0:** the docs say so.
- **R1:** someone saw `.env` absent from a listing.
- **R2:** `scan_excludes_secret_files` passes — *but only against a synthetic fixture.*
- **R3:** a CLI command indexes the real repository, and a listed secret file is demonstrably absent.
- **R4:** a second party runs that command on their machine and gets the same result.
- **R5:** an auditor confirms it, including the negative case.

**Note the trap:** R2 was green here while the *related* exclusion control was broken. Passing
tests on a fixture that does not match the real input is the specific failure this ladder exists
to prevent.

### 5.3 "Android build launches on the emulator"

- **R0:** the roadmap says Phase 10 will do this.
- **R3:** a captured `adb` session: install, launch, screenshot of the running app.
- **R5:** an auditor reproduces it on a clean machine.

"Compiles successfully" is **R2**. *Using the application* is R3. The distinction is the whole
point of Phase 10.

### 5.4 "This model is better at this task class" (Model Democracy)

- **R0:** a dropdown preference.
- **R2:** a routing test with mocked responses.
- **R3:** routing decisions recorded against measured outcomes on real tasks.
- **R5:** an auditor confirms the measurement methodology.

This is why Model Democracy is **PARTIAL** until routing is based on measured results.

---

## 6. Rung Assignment Procedure

1. **Identify the capability** in the registry by name.
2. **List its critical-path components.** (For a system, this includes its integration.)
3. **Assign each component a rung** using §2 and §3.
4. **The capability's rung is the minimum.** (§2.2)
5. **Attach the evidence artifact** for that rung.
6. **Record the rung with a date and a commit SHA.**
7. **Re-audit at G3** before any dependent phase begins.

**Rungs are computed, not asserted.** If a rung cannot be justified by an attached artifact, it
is R0 by definition.

---

## 7. Current Rung Assignments

| Capability / System | Rung | Evidence gap |
|---|---|---|
| Real tool runtime (shell, fs, git, search) | **R3** | — |
| Agent execution loop | **R3** | — |
| Model-driven agent decisions | **R3** | live provider commissioning outstanding |
| Durable memory / crash recovery | **R3** | generic verification execution partial |
| Evidence ledger | **R3** | — |
| Permissions gate | **R3** | — |
| Provider configuration / real HTTP dispatch | **R3** | — |
| **Repository intelligence (Phase 2A code)** | **R3 (code) / not accepted (governance)** | committed entry points shipped (`serve-intel` routes, Tauri commands, IDE surfaces) with hash-chained evidence; **Phase 2A acceptance still awaits independent re-audit** — the original audit's three findings (metric mis-scoped, benchmark non-reproducible, zero integration) are remediated in code but rung acceptance is G3/G4's verdict, not a self-claim |
| Release installers | **R1** | CI blocked externally |
| Model routing (Model Democracy) | **R1** | not measurement-based (read-only chain/metrics views are R3) |
| Everything in Epochs II–VI | **R0** | PROPOSED, not implemented |

---

## 8. Governance Interaction

- **Gate G2** (self-evidence) requires the R3 reproduction block.
- **Gate G3** (independent audit) requires R4 attempt by the auditor.
- **Gate G4** (acceptance) requires the auditor's verdict.
- **Truth-in-Advertising** (Constitution §11): nothing is described as working below its rung.
- **A capability claim above its actual rung is a defect**, and is recorded as one — not quietly
  corrected.

---

## 9. Summary

> **R0 claimed. R1 observed. R2 executed. R3 verified. R4 reproducible. R5 commissioned.**

The ladder is deliberately hard to climb. That is the point. ZylCode's differentiator is that
it can *prove* what it did — and it cannot honestly claim that for its users while being
generous about it for itself.
