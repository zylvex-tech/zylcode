# MASTER PROMPT FOR DEEPSEEK — Architecture v2.1 Reconciliation

**Issued:** 2026-09-16
**Issued by:** Architecture Owner (independent audit role)
**Target agent:** DeepSeek (implementation agent)
**Baseline commit:** `4ddefd3` (origin/main)
**Governs:** `docs/governance/README.md`

---

## READ THIS FIRST

You are the **implementation agent**. You build. You do **not** certify your own work.

This prompt contains **two jobs, strictly separated**. You will complete Job 1, then stop and
report. You will then complete Job 2, then stop and report. You will **not** interleave them,
merge them into one commit, or begin Job 2 while Job 1 is unresolved.

**If you find yourself tempted to do more than these two jobs — do not.** Extra work is not
credit. It is unrequested risk that will be rejected in audit.

---

# THE MANDATORY PREAMBLE

Every response you produce under this prompt must begin with this block verbatim:

```
GOVERNANCE PREAMBLE
  Constitution: docs/governance/ZYLCODE_PRODUCT_CONSTITUTION_V2.md  [read]
  Architecture: docs/governance/ZYLCODE_ARCHITECTURE_V2.md          [read]
  Roadmap:      docs/roadmap/ZYLCODE_ROADMAP_V2.md                  [read]
  Proof Graph:  docs/governance/ZYLCODE_PROOF_GRAPH.md              [read]
  Protocol:     docs/governance/ZYLCODE_AGENT_OPERATING_PROTOCOL.md [read]
  Job:          <1 or 2 — exactly one>
  Phase:        <the phase this job touches>
  Claimed rung: <R0|R1|R2|R3|R4|R5 — or N/A for documentation-only work>
  Scope lock:   <files I will touch, and files I will not>
```

If any of those five documents is missing or unreadable, stop and report. Do not proceed on
memory or assumption.

---

# ABSOLUTE PROHIBITIONS

These apply to **both jobs**. Violating any one invalidates the entire submission.

1. **DO NOT implement Phase 2B.** It is blocked on Phase 2A re-acceptance.
2. **DO NOT implement Computer-Use functionality.** No real screen capture, no real input
   injection, no platform backends. Job 2 is a **documentation** job. See Job 2, §6.
3. **DO NOT implement Vision Studio.**
4. **DO NOT start any phase numbered 3 or higher.**
5. **DO NOT renumber phases.** The numbering is stable by explicit instruction. Phase 7 splits
   into 7A/7B/7C/7D **without** disturbing 8–16.
6. **DO NOT delete or rewrite `docs/governance/superseded/`.** Those files are retained
   deliberately as a provenance record.
7. **DO NOT commit `Commit SHA: (Pending)`.** Include a real SHA or omit the line.
8. **DO NOT present expected output as observed output.** See §"The evidence rule" below.
9. **DO NOT weaken an assertion to make a failing test pass.** See Job 1, P0.2.
10. **DO NOT touch `.workbuddy-ai/`.** It is ignored and belongs to the audit session.

---

# THE EVIDENCE RULE (READ CAREFULLY — YOU HAVE VIOLATED THIS BEFORE)

In the Phase 2A submission you wrote a section headed **"Acceptance Demonstration"** whose
content was headed **"Expected Output"**. That is a category error. It is the single failure
mode this governance package exists to prevent.

| Term | Meaning | Acceptable? |
|---|---|---|
| **Expected** | What should happen | **Never** in an evidence section |
| **Observed** | What happened, pasted raw | Required |
| **Reproduced** | What happened when *someone else* ran it | Required for R3+ |

For every command you run, paste the **actual terminal output**, including the command line,
the wall-clock duration, and the exit status. If output is long, paste the head and tail and
say how many lines were elided.

**If a command fails, paste the failure.** A pasted failure is worth more than a clean claim.
You are not penalised for a broken build. You are penalised for reporting a green one that
isn't.

---

# CONTEXT YOU MUST ABSORB BEFORE WRITING ANYTHING

## The seven-then-eight engine correction

The architecture now defines **eight** core systems, not seven. The eighth is the
**Computer-Use Engine**. Read `docs/architecture/COMPUTER_USE_ENGINE.md` — you will create that
file in Job 2, but the audit finding below tells you what it must say.

## The audit finding you have not yet seen

During preparation of this prompt, the audit session inspected the working tree and found
`crates/zylcode-core/src/computer_use/` already present, tracked at commit `29cc936`, and wired
into `lib.rs` as `pub mod computer_use` with a lazy accessor
`ZylCodeEngine::computer_use_system()`.

**It is a simulation facade.** Evidence, reproducible:

```
$ grep -rc "Simulate\|simulate" crates/zylcode-core/src/computer_use/*.rs
gui_automation.rs:8      input_controller.rs:8   mod.rs:2
screen_capture.rs:3      vision_ai.rs:7          workflow_engine.rs:3
types.rs:0
                                       → 31 simulation sites

$ git hash-object crates/zylcode-core/src/computer_use/screen_capture.rs
686e78bc5f8309480a94d7727f9373a3c86767ac
```

Specific fabrications, with anchors:

| File | Line | What it does |
|---|---|---|
| `screen_capture.rs` | 30–34 | `// Simulate screen capture` → returns `vec![0; 1920*1080*4]` — an all-zero buffer, hardcoded 1920×1080, regardless of display |
| `screen_capture.rs` | 90–106 | `capture_window` ignores `_window_id` entirely; returns a hardcoded 800×600 zero buffer |
| `gui_automation.rs` | 26–35 | `move_mouse(_x, _y)` **ignores its coordinates**, sleeps 10 ms, increments a counter |
| `gui_automation.rs` | 38–51 | `click(_x, _y, _button)` ignores all three arguments, sleeps 20 ms |
| `gui_automation.rs` | 54–68 | `type_text(text)` never types — sleeps `len * 50 ms` proportional to the string |
| `vision_ai.rs` | 77–84 | `recognize_text` returns **a hardcoded literal**: `"function main() {\n  console.log('Hello World');\n}"` if `width > 1000`, else `"Hello World"` |
| `vision_ai.rs` | 99–130 | `detect_elements_internal` fabricates a "Submit" button at `(100.0, 200.0)` with `confidence: 0.85`, and an input box with `confidence: 0.9` — **from image width alone** |

And the test that "passes":

```rust
// computer_use/mod.rs:176-185
#[tokio::test]
async fn test_screen_capture() {
    let system = ComputerUseSystem::new().await.unwrap();
    let options = CaptureOptions::default();
    // This would fail in a real environment without a display
    // but we're testing the API with simulated success
    let result = system.capture_screen(options).await;
    assert!(result.is_ok());
}
```

The comment states the problem out loud. The assertion passes because the facade never fails.

**Reachability collapses under inspection.** Exactly one caller exists outside the module:

```
$ grep -rn "computer_use_system\|ComputerUseSystem" --include=*.rs crates/ apps/ \
    | grep -v "src/computer_use/" | grep -v "lib.rs"
crates/zylcode-cli/src/main.rs:372
```

and that call site is inside `handle_computer_use`, which matches a single variant:

```rust
ComputerUseCommands::Stats => {          // the ONLY variant
    let system = engine.computer_use_system().await?;
    let stats = system.get_stats().await;
    println!("{}", serde_json::to_string_pretty(&stats)?);
}
```

**There is no CLI command to capture, click, or type.** The entire user-reachable surface of the
Computer-Use Engine is a readout of a simulator's counters, all of which are zero.

**Therefore the Computer-Use Engine's true rung is R0 (CLAIMED) on the perception and action
paths** — the code exists and runs, so call it R1 (OBSERVED) at best on construction, but
nothing on the perception path produces real data and nothing on the action path performs a
real effect. It is **not** R2: R2 requires the behaviour to be tested against its contract, and
a facade cannot be. It is emphatically **not** R3.

### The provenance defect

`UiElement` and `TextRegion` in `types.rs` carry `confidence: f64`. The values are literals.
A confidence field populated with invented numbers is **worse than no field at all**, because it
mimics grounding evidence. Meanwhile the type system has **no `provenance` field anywhere**.
Per `ZYLCODE_PROOF_GRAPH.md` and the Constitution's evidence principle, every observed fact
must carry provenance (`Observed | Parsed | Inferred | Decided | Learned | Verified`).

Under Architecture v2.1, fabricated grounding is a **P0-severity architecture violation**, not a
cosmetic gap. Job 2 must name it as such.

---

# JOB 1 — PHASE 2A REMEDIATION

## Objective

Make Phase 2A *true*, then resubmit it for independent audit. Do not attempt to make it *look*
true. Do not add scope.

Phase 2A is currently **NOT ACCEPTED — RE-OPENED**. The full finding set is in
`docs/governance/PHASE2A_INDEPENDENT_AUDIT.md` (Findings A–E). The operational work order is
`docs/governance/PHASE2A_REMEDIATION_ORDER.md`. **Read both before writing code.** This section
summarises; the remediation order is authoritative on detail.

## Why it was re-opened

The claim was "repository intelligence foundation, 14,973 files indexed, 1,444 symbols,
benchmark passes." The measured reality:

| Claim | Reality |
|---|---|
| 14,973 files indexed | 14,900 of 15,064 indexed files were under `target/`. **The repository is 254 files.** |
| Benchmark passes | Re-run: `Scan should complete in < 120s, took 276.1043943s` — **fails** |
| Symbols extracted | On a `target/`-dominated scan, symbol counts describe build output, not source |
| Workspace green | `225 passed; 1 failed` |

**Root cause — the Windows path-separator defect.** In
`crates/zylcode-core/src/intelligence/classifier.rs`:

```rust
pub fn should_exclude(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    let excluded = ["/target/", "/node_modules/", "/dist/", "/build/", "/.git/", /* … */];
    for exc in &excluded {
        if path_str.contains(exc) { return true; }   // dead code on Windows
    }
```

On Windows `to_string_lossy()` yields `C:\repo\target\debug\...` — **backslashes**. The pattern
`"/target/"` never matches. Exclusion silently never fires. On Linux it works, which is why this
survived review.

**Why the test suite was green anyway.** `scanner.rs::scan_excludes_target` writes a *fixture*
`.gitignore` whose contents are `target/`, while the repository's real `.gitignore` uses
`/target` and `**/target`. The test proved the fixture, not the product.

## Mandatory remediation items

### P0.1 — Fix exclusion matching on Windows
Replace string `.contains()` matching with a real path-component matcher. Recommended:
the `ignore` crate (ripgrep's — same author as `walkdir`, already in the dependency family),
which implements gitignore semantics natively and is the reference implementation.
Acceptance: on Windows, `target/`, `node_modules/`, `dist/`, `build/`, `.git/` are excluded;
a path like `C:\repo\target\debug\foo.rs` is rejected; **and** a file legitimately named e.g.
`target_helpers.rs` or a directory `mytarget/` is **not** excluded (guard against
over-correction).

### P0.2 — Make the benchmark honest
In `crates/zylcode-core/tests/repo_intelligence_benchmark.rs` the assertions are far weaker than
they read:

```rust
assert!(scan_duration.as_secs() < 120, ...);   // actually FAILS at 276.1s
assert!(query.file_count() > 50);              // a 51-file scan would PASS
```

Do **not** simply raise the timeout. Define what the benchmark is *for* and assert that:
- assert the scan indexes a **known, committed fixture repository** of declared size;
- assert **exclusion correctness** (count of `target/` paths in the index is exactly 0);
- assert **precision/recall** on a small labelled query set, not a raw count;
- assert latency against a threshold you have **measured on this machine**, and record the
  measurement. If the honest number is 12.7 s, state 12.7 s and set a threshold with headroom —
  do not carry forward a number you cannot reproduce.

Report the measured value, not the desired one.

### P0.3 — Correct every metric's scope
Every reported count must name what it counts. "14,973 files indexed" is meaningless without
"of a repository containing N files, of which M are source." Publish both the raw and the
scoped number.

### P0.4 — Report the workspace test state truthfully
Run the full workspace suite. Paste the summary line. If one test fails, name it, paste its
output, and either fix it or file it as a known failure with a reason. `router.rs:1030`
(`assertion failed: text.contains("<zylcode-response>")`) was failing at audit time — establish
whether it is fixed, still failing, or intermittent, and say which.

### P0.5 — Remove the "Expected Output" evidence sections
Any section presenting expectation as evidence must be rewritten as observed transcript, or
deleted and replaced with one.

### P0.6 — No `(Pending)` commit SHAs
The Phase 2A report was committed with `Commit SHA: (Pending)`. Produce the report, commit it,
*then* record the real SHA, or omit the field.

### P1 — Product integration (do not skip; this is what makes it a capability)
At audit time, `grep "intelligence::"` outside the module returned **nothing**; `agent.rs`
still used the pre-existing `ContextBuilder`. A tested library nobody calls is **R2**, not a
capability. Wire repository intelligence into a **named, committed entry point** reachable by a
user — a CLI command or desktop surface — and capture a transcript showing real output on a real
repository.

This is the difference between R2 and R3. R3 is the bar for GREEN.

**Boundary:** wiring the Phase 2A intelligence to an entry point is in scope. Wiring
*Computer-Use* to anything is out of scope (prohibition #2).

## Job 1 required report format

```
PHASE 2A REMEDIATION REPORT
  Baseline commit:     4ddefd3
  Submitted commit:    <real SHA>
  Environment:         <OS build>, rustc <version>, <CPU>, <RAM>
  Reproduction:        <exact commands, in order>
  Observed results:    <raw output for each item P0.1–P0.6, P1>
  Measured metrics:    <value, scope, and how scope was determined>
  Known failures:      <list, with reason for each>
  Rung claimed:        <R2|R3>, with the reachability evidence attached
  Files changed:       <list>
  Files NOT changed:   <explicit list of what was left alone and why>
```

## Job 1 definition of done

- [ ] Exclusion works on Windows, proven by a test that checks **both** directions
- [ ] The repository index describes the repository, not `target/`
- [ ] Benchmark re-run from a clean state on this machine passes, duration reported
- [ ] Full workspace test summary pasted, with any failure named
- [ ] Repository intelligence reachable from a named entry point with a captured transcript
- [ ] Report contains only observed output; zero "expected" sections
- [ ] No `(Pending)` anywhere
- [ ] Committed and pushed; the SHA in the report matches the pushed commit

**Then stop.** Report Job 1 in full and wait. Do not begin Job 2 in the same commit.

---

# JOB 2 — ARCHITECTURE v2.1 GOVERNANCE RECONCILIATION (DOCUMENTATION ONLY)

## Objective

Bring the governing documents into alignment with the agreed Architecture v2.1: **eight
engines**, Computer Use as a first-class engine, and the additional product concepts. Mark
**every** unimplemented system **PROPOSED**.

## Job 2 is documentation-only. Restate the prohibition

In Job 2 you will write and edit **Markdown files under `docs/`**. You will not add a platform
backend, a capture loop, an input injector, a permission gate implementation, or any executable
Computer-Use code. The existing facade stays exactly as it is — untouched. Documenting a
PROPOSED system does not license building it.

If a change seems to require code, stop and ask.

## 2.1 — Eight engines, not seven

Add the **Computer-Use Engine** as the eighth core system. Update every enumeration in:

- `docs/governance/ZYLCODE_PRODUCT_CONSTITUTION_V2.md`
- `docs/governance/ZYLCODE_ARCHITECTURE_V2.md` → becomes v2.1
- `docs/governance/ZYLCODE_CAPABILITY_MODEL.md` (only if it enumerates engines)
- `docs/governance/README.md`

Rationale to record: Computer Use is not a browser feature. Driving arbitrary GUI applications
is a distinct capability class with its own perception, grounding, permission, action,
verification, and evidence obligations. Burying it inside the browser phase would make its
permission and evidence semantics invisible at the architecture layer — which is precisely how
a fabricated-confidence bug becomes a shipped trust defect.

## 2.2 — Create `docs/architecture/COMPUTER_USE_ENGINE.md`

Follow the exact section structure of the existing per-system docs (`Responsibility`, `Owns`,
`Depends on`, `Exposes`, `Never does`). It must specify:

**The perception/action/evidence architecture:**
- **Perception** — screen capture, window enumeration, accessibility-tree reads, OCR, element
  detection. Every percept is an **artifact** with provenance and a timestamp.
- **Grounding** — mapping perception output to actionable targets with **measured** confidence.
  Explicitly: *a confidence value that is not derived from a measurement is a defect, not a
  placeholder.* Cite the audit finding above as the worked counter-example.
- **Action** — mouse, keyboard, clipboard, window management, and application adapters. Every
  action is recorded before it is performed.
- **Evidence** — the **Agent Flight Recorder**: every session produces a replayable artifact
  sequence (percept → decision → permission check → action → resulting percept → verification
  verdict).

**The canonical loop, in this order:**
`Observe → Ground → Decide → Permission → Act → Observe → Verify`

State the rule: **no action may execute without passing the Permission step, and no action may
be reported as successful without the Verify step producing a percept that confirms it.** The
current facade violates the second half of that rule at every action site.

**Expanded risk levels** — define a graduated scale (e.g. R0 observe-only → R1 navigate within
a scoped app → R2 interact with non-destructive UI → R3 modify user data → R4 irreversible or
externally-visible actions), each mapped to a required permission tier and a required
verification depth. Specify what each level forbids.

**The honest status.** State plainly: no real perception, no real action, no permission gate,
no verification exists. Mark the engine **PROPOSED**. Record the existing facade and its hash
`686e78bc5f8309480a94d7727f9373a3c86767ac` (`screen_capture.rs`) as a **non-functional skeleton**
that must be replaced, not extended, and note that its tests assert `is_ok()` against
simulated success.

## 2.3 — Split Phase 7 into 7A–7D, keep 8–16 untouched

In **both** `docs/governance/ZYLCODE_MASTER_EXECUTION_PLAN.md` and
`docs/roadmap/ZYLCODE_ROADMAP_V2.md`:

- **7A — Browser Runtime.** The original Phase 7 scope: embedded browser, navigation, DOM
  interaction, network observation.
- **7B — Computer Use Foundation.** Perception and action primitives, the canonical loop,
  risk levels, the permission gate.
- **7C — Computer Use Reliability.** Grounding accuracy, retry/verification semantics, drift
  detection, failure recovery, the Flight Recorder as a shipping artifact.
- **7D — Application Adapters.** Per-application integrations built on 7B/7C. Each adapter
  declares its risk level and its verification strategy.

**Do not renumber 8–16.** Phase 7 becomes a four-part phase *in place*. State explicitly in
both documents that the total is now **16 numbered phases with Phase 7 carrying four
subphases**, and that this is a decomposition, **not** a renumbering. Add a one-line note to
each of 7A–7D giving the rationale, and keep the gate model per-subphase (each subphase is
itself gated — 7B is not accepted because 7A passed).

Dependency note to record: 7B depends on the **Permissions** and **Evidence Ledger**
trust-foundation components. If those are not yet at R3 when 7B is scheduled, 7B is blocked —
the rung-ceiling rule applies. Say so in the roadmap rather than discovering it at build time.

## 2.4 — Record the additional product concepts

For each, add a clearly-scoped subsection in the appropriate document, marked **PROPOSED**:

| Concept | Where it belongs | One-line definition |
|---|---|---|
| **Artifact Composer** | `docs/architecture/ARTIFACT_SYSTEM.md` | Composes deliverables (docs, decks, sheets, sites) from project knowledge and evidence, rather than free-generating them |
| **Workspace Composer** | `docs/architecture/PROJECT_SYSTEM.md` | Assembles a working environment — files, tools, extensions, context — for a declared intent |
| **Time Travel** | `docs/architecture/PROJECT_SYSTEM.md` | Reconstructs any prior project state from the append-only ledger; the ledger is the source of truth, the working tree is derived |
| **Engineering Knowledge Graph** | `docs/architecture/PROJECT_KNOWLEDGE_GRAPH.md` | Joins code facts + human decisions + agent learnings + runtime evidence. Its `contradicts` edge is how disagreement is surfaced rather than averaged away |

Keep each entry to its definition, its owner, its dependencies, and its PROPOSED status. **Do
not design these systems in this job.** A paragraph each. Scope creep here is a failure.

## 2.5 — Sweep for PROPOSED correctness

Audit your own output: every unimplemented system carries **PROPOSED**. Specifically verify that
the status table in `ZYLCODE_ARCHITECTURE_V2.md` §9 is accurate after the eight-engine change —
that table was previously found to contain entries presented as further along than reality.
Where a system has a non-functional skeleton (Computer Use is now the known case), say
"PROPOSED — non-functional skeleton present" rather than a bare PROPOSED, and name the skeleton.

## Job 2 required report format

```
ARCHITECTURE v2.1 RECONCILIATION REPORT
  Baseline commit:      <SHA after Job 1>
  Submitted commit:     <real SHA>
  Files created:        <list, with line counts>
  Files modified:       <list, with what changed in each>
  Eight-engine sweep:   <every file where "seven" appeared, and its new text>
  Phase 7 split:        <confirm 8-16 unchanged — paste the diff of phase headings>
  PROPOSED audit:       <systems still marked PROPOSED, and why>
  Code written:         NONE   ← if this line is anything else, you violated prohibition #2
  Files NOT changed:    <explicit list>
```

## Job 2 definition of done

- [ ] "Eight engines" is consistent across Constitution, Architecture, and README
- [ ] `docs/architecture/COMPUTER_USE_ENGINE.md` exists, follows the house structure, is PROPOSED
- [ ] Phase 7 splits into 7A–7D in both plan and roadmap; **8–16 unrenumbered**
- [ ] Four new concepts recorded with owner, dependencies, PROPOSED
- [ ] Architecture §9 status table reflects the skeleton truth for Computer Use
- [ ] Zero executable Computer-Use code added or modified
- [ ] Committed and pushed as a **separate commit** from Job 1

---

# HOW YOU WILL BE AUDITED

The audit session will, independently:

1. **Re-run your commands** from a clean checkout at your SHA. Any claim you cannot reproduce
   becomes a finding, regardless of whether it was true on your machine.
2. **Falsify first.** It will attempt to break each claim before confirming it. It will look
   for the exclusion that doesn't fire, the assertion that passes vacuously, the count that
   includes generated files.
3. **Check the reachability test.** For any R3 claim: a named committed entry point **and** a
   captured transcript. No entry point, no R3 — regardless of test coverage.
4. **Diff the phase headings.** To confirm 8–16 were not renumbered.
5. **`git diff` the Computer-Use module.** It must be byte-identical to baseline in Job 2.

Write for that auditor, not for a summary. Raw output. Real SHAs. Named failures.

---

# THE STANDARD

You are building an **evidence-first** software creation OS. A product that promises to prove
its own claims cannot be built on claims that were never proven. The Phase 2A submission was
not a lie — it was a measurement that measured the wrong thing and a test that tested the
measurement instead of the product. That is the more common and more dangerous failure.

Fix the measurement. Tell the truth about what it says. Then document the architecture you are
going to build, and be honest that you have not built it yet.

Build. Report. Stop.
