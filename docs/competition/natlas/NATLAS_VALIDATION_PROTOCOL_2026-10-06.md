# N-ATLAS VALIDATION PROTOCOL — 2026-10-06

**Phase:** C0
**Purpose:** define, *before* any N-ATLAS call is made, what will count as proof that the integration
works — so the standard cannot be lowered after the fact to fit whatever happens.

---

## 1. Why this document exists before the code works

A validation protocol written after a result is known tends to describe that result. Writing it
first, while the outcome is still unknown, is what makes it a test rather than a narrative.

It also pre-commits to the uncomfortable outcomes. If N-ATLAS access never arrives, this document
says what the honest status is, so "blocked" cannot quietly become "demonstrated".

---

## 2. The proof ladder

Adapted from ZylCode's existing evidence model. A statement's rung is the highest one it can actually
cite — never the one it would like to claim.

| Rung | Requires | Applies to |
|---|---|---|
| **L0 — Asserted** | nothing | prose, intent, design |
| **L1 — Parsed** | a deterministic parser accepted the value | response parsing, intent schema |
| **L2 — Unit-tested** | a committed test that ran, using a **test double** | boundary behaviour |
| **L3 — Integrated** | the real N-ATLAS transport returned a real response | the integration itself |
| **L4 — Verified end-to-end** | L3 **and** a bounded action **and** a test whose real exit code was checked | the competition slice |
| **L5 — Externally validated** | L4 **and** an independent human tester recorded in the beta log | real-world validation |

**The rule that matters:** a test-double result can reach **L2 and no further**. L3 requires a real
call. No amount of mock coverage substitutes for it.

---

## 3. What each competition claim will require

| Claim | Minimum rung | Evidence required |
|---|---|---|
| "ZylCode can parse an N-ATLAS response" | L1 | parser tests |
| "The boundary handles failure, timeout and malformed input correctly" | L2 | boundary tests (test double) |
| "ZylCode is integrated with N-ATLAS" | **L3** | a captured real response with provider/model identity and a timestamp |
| "An N-ATLAS response drove a real engineering action" | **L4** | L3 evidence + tool evidence + a verification claim whose exit code matched |
| "The integration helped real developers" | **L5** | L4 evidence + ≥1 real tester record |

Today the project can honestly claim **L2** for the boundary. It can claim nothing at L3 or above,
because N-ATLAS has never been contacted.

---

## 4. Anti-fabrication rules

These are not stylistic preferences. Each one has a specific failure it prevents.

1. **A test double is named.** It appears as `Mock*` / `TEST DOUBLE` in code and comments, and its
   results are labelled as such wherever they are reported.
2. **No response is ever synthesised and presented as N-ATLAS.** The product router's synthetic
   degradation path is explicitly *not* used for N-ATLAS (discovery §1.3).
3. **No endpoint, auth scheme, model id or response shape is invented.** Where one is unknown, the
   state is `BLOCKED`, not a plausible guess.
4. **A blocked result is never recorded as a failure of N-ATLAS.** "We could not reach it" and "it
   failed" are different facts.
5. **Secrets never appear in evidence**, and redaction is counted so its occurrence is visible.
6. **Timestamps and latencies are captured, not estimated.**
7. **A missing measurement is recorded as missing.** No confidence value, latency or token count is
   invented to fill a field.

---

## 5. Validation procedure (executed when access exists)

For the first real invocation, and for each subsequent one used as evidence:

1. **Capture the environment.** Record which configuration variables were set — names and presence,
   never values.
2. **Invoke once**, through the real transport.
3. **Preserve the raw outcome** as `NatlasEvidence` (status, latency, provider/model identity,
   request id, redacted error if any).
4. **Confirm the classification** of the request.
5. **Run the handoff.** Parse the response as an intent; record whether the schema held.
6. **Execute the bounded slice.** Side-effect-free steps, then the approval gate.
7. **Verify with a real command.** Promote a claim only if the exit code matched the expectation.
8. **Link the records** into the evidence graph so the chain from intent to verification is
   traversable.
9. **Re-run the workspace battery** to show no regression was introduced.
10. **Record what is still unknown.** Every validation run ends with an explicit unknowns list.

A run that cannot complete step 7 is reported as **incomplete**, not as success.

---

## 6. Regression discipline

The competition branch must not weaken the product.

* The existing test suite must pass unchanged. A new failure is a stop condition, not a detail.
* Any test the competition work touches must be *added to*, never relaxed.
* The branch must remain separable: removing `competition/` and one `lib.rs` line must leave the
  product byte-identical to its pre-competition state.

### 6.1 A pre-existing flake was exposed, and it is recorded here rather than smoothed over

On the first full workspace battery of this branch, one **pre-existing** test failed:

```
router::tests::d1_vector_cache_cross_prompt_contamination_guard
  assertion `left == right` failed: the D1 guard test must not perform network egress
  left: 6   right: 5
```

It is a **test-isolation defect, not a regression from the competition work.** The evidence:

| Check | Result |
|---|---|
| Does the competition module reference the router or the counter? | No — zero references. It cannot increment `NETWORK_EGRESS_COUNT`. |
| `NETWORK_EGRESS_COUNT` | a **process-global** `static AtomicU64`, incremented only inside `call_provider` (`router.rs:1088`) |
| The test alone | passes (1 passed / 0 failed) |
| Full lib suite, single-threaded | passes (459 / 0) |
| Full lib suite, parallel re-run, same configuration that failed | **passes** (459 / 0) |
| Full lib suite, parallel, competition tests skipped | passes (441 / 0) |

The test reads the global counter before and after its own scenario and asserts the two are equal.
Any sibling test that dispatches to a real provider concurrently increments the same counter, so the
assertion is unsound under parallel execution. It is a latent race that this branch's added tests
made slightly more likely to surface; it did not introduce it.

**It was not fixed here**, deliberately: the fix belongs in `router.rs`, which this phase must not
modify, and serialising tests that share process-global state is a product-side change outside the
competition's scope. It is reported to the owner as a finding.

**Consequence for the competition:** a submission must not present a battery run as green evidence
unless it actually was. If the battery is red, re-run and record the true outcome — including this
known flake, named as such.

---

## 7. External beta validation (protocol only — no testers yet)

The Developer Infrastructure problem area expects real-world validation. The protocol:

1. A tester is a **real person**, named with consent, with a stated role and background.
2. The tester attempts **one bounded task** on **one named repository**.
3. The N-ATLAS invocation evidence for that task is preserved (non-secret fields only).
4. Success or failure is recorded **as observed**, including partial success and failure.
5. Elapsed time is measured, not estimated.
6. The tester's rating, observations and written feedback are recorded verbatim.
7. Any issue discovered is recorded with its resolution, or left explicitly unresolved.
8. **No tester is ever fabricated, and no result is ever back-filled.** An empty beta log is an
   honest artefact; a populated fake one is a fraud.

The template is `BETA_TEST_EVIDENCE_TEMPLATE.md`, and it is **deliberately empty**.

---

## 8. What would falsify the current claims

Stated plainly, so the claims are testable rather than rhetorical:

* If the real N-ATLAS response does not match the ZylCode intent contract, the handoff claim is
  false and the schema must change — the *contract*, not the evidence.
* If the real wire format differs from every assumption, the transport is rewritten; the boundary
  above it is unaffected, which is the point of the design.
* If the bounded slice cannot be verified with a real exit code, the end-to-end claim (L4) fails and
  must be withdrawn rather than softened.
