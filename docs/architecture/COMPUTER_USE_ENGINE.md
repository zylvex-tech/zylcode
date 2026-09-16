# COMPUTER_USE_ENGINE.md — ZylCode Computer-Use Engine

**Status: PROPOSED — non-functional skeleton present**
**Rung: R0 (CLAIMED)**
**Phases: 7B (Foundation) · 7C (Reliability) · 7D (Adapters)**
**Governing docs:** `ZYLCODE_ARCHITECTURE_V2.md` §3.8 · `ZYLCODE_PROOF_GRAPH.md` ·
`ZYLCODE_PRODUCT_CONSTITUTION_V2.md` §2.0.1–2.0.2

> ⚠️ **This system does not exist.** `crates/zylcode-core/src/computer_use/` (7 files, 1,763
> lines, commit `29cc936`) is a **simulation facade**: 31 simulation sites, all-zero capture
> buffers, `tokio::time::sleep` in place of input synthesis, hardcoded OCR text, and fabricated
> confidence values. It must be **replaced, not extended**.
>
> The distinction that matters: a facade that returns plausible data is **not** an early version
> of a working system. It is a system that will pass its own tests forever while doing nothing.

---

## 1. Responsibility

Perceive and drive the user's **real desktop** and arbitrary third-party applications, under
permission, producing replayable evidence for every action.

**In scope:** screen and window perception · accessibility-tree reads · element grounding ·
input synthesis (mouse, keyboard, clipboard) · window management · per-application adapters ·
session recording and replay.

**Out of scope:** browser DOM manipulation (that is the browser runtime, 7A) · deciding whether an
action is permitted (that is Permissions) · interpreting whether a claim is proven (that is the
Proof Engine).

---

## 2. Why This Is An Engine And Not A Browser Feature

This is a v2.1 architectural decision and it is load-bearing.

| | Browser Runtime (7A) | Computer-Use Engine (7B–7D) |
|---|---|---|
| Target | a browser it launched | applications the user already runs |
| Side effects land on | a sandboxed page | the user's real files, accounts, devices |
| Reversibility | reload the page | often none |
| Permission granularity | per-origin, per-navigation | per-action, per-application |
| Failure mode | wrong test result | deleted file, sent message, purchase |
| Verification | DOM is queryable — the truth is inspectable | only the screen is available — the truth must be *re-observed* |

Folding this into the browser phase would bury the per-action permission requirement and the
re-observation requirement at the architecture layer. Those two obligations are precisely what
separate controlled automation from a macro recorder, and they are the first things lost when a
system is specified as a feature rather than an engine.

---

## 3. The Canonical Loop

Every interaction, without exception, follows this order:

```
Observe ──► Ground ──► Decide ──► Permission ──► Act ──► Observe ──► Verify
   │           │          │            │           │         │          │
 percept    target +   chosen      gate +      effect     fresh     match?
 artifact   measured   action      recorded    issued      percept   → verdict
            conf.                  decision                artifact
```

**Two rules are absolute.**

1. **No action executes without passing `Permission`.** Not a cached grant from ten steps ago,
   not a category-level allowance. Every action carries a risk level (CU-0…CU-4) and is checked
   against it. A denial is recorded as evidence and produces **no** action.
2. **No action is reported successful without `Verify`.** The second `Observe` is not optional
   bookkeeping. An action whose effect was not re-perceived is **UNVERIFIED**, which is a
   distinct outcome from both success and failure (Architecture A11).

The current skeleton violates rule 2 at every action site: `move_mouse`, `click`, and `type_text`
return `Ok(())` after a timer, with no observation of any effect.

---

## 4. Perception

**Inputs:** screen capture (full, region, window) · window enumeration · accessibility tree ·
(in 7C) OCR and element detection.

**Every percept is an artifact**, carrying:

| Field | Meaning |
|---|---|
| `captured_at` | monotonic + wall-clock timestamp |
| `source` | screen / window(id) / accessibility-tree |
| `provenance` | `Observed` — never `Inferred`, never `Decided` |
| `method` | the concrete mechanism that produced it (e.g. `Win32::BitBlt`) |
| `display_context` | resolution, DPI scale, virtual-desktop origin |

**A percept without a `method` is inadmissible.** The proof ladder cannot cite it.

### 4.1 The grounding rule

> **A confidence value that is not derived from a measurement is a defect, not a placeholder.**
> (Constitution §2.0.2)

`UiElement.confidence` must be produced by a stated measurement — detector score, template match
score, accessibility-attribute confidence — and the measurement method must travel with the
value. If no measurement exists, the field is **absent**, not defaulted.

A fabricated `confidence: 0.85` is worse than no confidence field, because downstream reasoning
cannot distinguish it from a measured one. This is not hypothetical: it is the defect found in
`vision_ai.rs:99-130`, where a "Submit" button at `(100.0, 200.0)` with `confidence: 0.85` is
emitted whenever `image.width > 800`, for any image whatsoever.

**Consequence for the type system:** every type carrying a confidence score must also carry its
provenance and method, or must not carry a score. There is currently **no `provenance` field
anywhere** in `computer_use/types.rs`. That is a P0 defect.

---

## 5. Grounding (`Ground`)

Maps percepts to **actionable targets** — a screen coordinate, a window-relative rect, an
accessible element handle, or a text match.

**Method precedence** (most trustworthy first, and the order is normative):

1. **Accessibility tree** — a named, addressable element. Preferred, because it survives layout
   changes and yields an identity that can be re-resolved.
2. **Element detection** — a measured detector score against a labelled set.
3. **Template match** — a measured correlation score.
4. **Pixel/coordinate heuristic** — last resort; **always** requires re-observation after acting,
   and is never cacheable across sessions.

Grounding output must be sufficient to **re-locate** the target, not merely to point at it once.
A coordinate is not an identity. Storing `(412, 883)` is not grounding; storing "the Save button
in window `Untitled — Notepad`" plus a resolution recipe is.

---

## 6. Action

**Synthesis:** mouse move / click / drag / scroll · keyboard key / hotkey / text · clipboard
read/write · window focus / move / resize / close.

**Two invariants:**

- **Record before acting.** The intended action is written to the ledger *before* it is issued,
  so that a crash mid-action leaves a record of what was attempted.
- **Every action completes with `Observe`+`Verify`, or the session halts.** A failed verification
  is not a warning to log and continue past.

---

## 7. Risk Levels

Every action declares exactly one level. Each maps to a permission tier and a verification depth.

| Level | Scope | Example | Permission | Verification depth |
|---|---|---|---|---|
| **CU-0** | Observe only | capture a region, read the accessibility tree | standing grant | none (no side effect) |
| **CU-1** | Navigate within a scoped app | focus a window, switch a tab, scroll | standing grant | target-state re-observed |
| **CU-2** | Interact, non-destructive | type into a scratch field, hover | standing grant | effect re-observed |
| **CU-3** | Modify user data | save a file, send a message, edit a record | **explicit approval, per action class** | effect re-observed **and** reconciled against external truth where available |
| **CU-4** | Irreversible or externally visible | delete, purchase, publish, transmit | **explicit approval, per instance** | effect re-observed; external system queried |

**Rules:**

- Level is declared by the adapter, not inferred at call time. An action must not be able to
  *lower* its own declared level.
- CU-3/CU-4 approvals are not session-wide. A grant for "save a document" is not a grant for
  "send an email".
- Ambiguity resolves **upward** (fail closed, Architecture A6). If a call site cannot prove an
  action is CU-2, it is treated as CU-3.

---

## 8. Evidence — The Agent Flight Recorder

Every session produces a replayable artifact sequence:

```
session/
  ├── percept_0001.png        (+ provenance, method, display context)
  ├── decision_0001.json      (chosen action, rationale, risk level)
  ├── permission_0001.json    (grant or deny, the rule that decided it)
  ├── action_0001.json        (what was issued, when)
  ├── percept_0002.png        (the re-observation)
  └── verdict_0001.json       (VERIFIED | UNVERIFIED | FAILED, and why)
```

The recorder is not debugging output. It is the **proof substrate**: it is what lets the Proof
Engine cite a Computer-Use claim, what lets a user audit what was done on their machine, and what
makes a failed run diagnosable rather than merely repeatable.

**A Computer-Use capability cannot exceed R2 without a Flight Recorder**, because R3 requires
captured evidence and there is nothing else to capture.

---

## 9. Interfaces

```rust
// Perception
fn observe(&self, target: ObserveTarget) -> Result<Percept>;
// Grounding
fn ground(&self, percept: &Percept, intent: &Intent) -> Result<Vec<Candidate>>;
// Permission (delegated — this engine never decides)
fn check(&self, action: &Action) -> Result<PermissionDecision>;
// Action
fn act(&self, action: Action) -> Result<ActionReceipt>;
// Verification
fn verify(&self, receipt: &ActionReceipt) -> Result<ActionVerdict>;
// Replay
fn replay(&self, session_id: SessionId) -> Result<RecordedSession>;
```

`PermissionDecision` and the verification `ActionVerdict` are owned by Permissions and the Proof
Engine respectively. This engine **calls** them; it does not implement alternatives.

---

## 10. Benchmark / Acceptance

For 7B (Foundation):

1. **Capture is real.** Capture the screen on a known display configuration and assert the buffer
   is *not* constant, that its dimensions match the actual display, and that a known on-screen
   marker appears in the capture. *(The current skeleton returns `vec![0; 1920*1080*4]` and would
   fail all three.)*
2. **Grounding is measured.** Against a labelled fixture set, report precision and recall **and
   the measurement method**. A confidence value with no stated method fails this benchmark.
3. **Action asserts effect, not success.** Click a control in a fixture application and assert the
   application's *state changed*. `assert!(result.is_ok())` is not a test of action.
4. **Permission blocks.** A CU-3 action without an explicit grant produces a recorded denial and
   **no** observable effect on the target application.
5. **Verification is load-bearing.** Inject a failure between `Act` and the effect; assert the
   loop reports `UNVERIFIED`, not success.
6. **Replay works.** A recorded session replays to the same percept sequence.

For 7C: add grounding precision/recall across three window sizes and two DPI scales; drift
detection under a moved UI element; idempotency under a mid-sequence interruption.

---

## 11. Anti-Requirements

The following are **forbidden**, not merely discouraged:

- **A confidence value without a measurement method.** (§4.1)
- **An action path that returns success without re-observation.** (A11)
- **A test that asserts `is_ok()` on an action.** That asserts the absence of a panic, not the
  presence of an effect.
- **A local allow-list that duplicates the Permission gate.** If Permissions is below R3, 7B is
  blocked — it is not unblocked by reimplementing permissions locally.
- **Extending the existing facade.** Its structure — `sleep`-based actions, fabricated percepts —
  is the defect. Replace the module; do not add a real backend underneath a synthetic interface,
  because the synthetic interface will keep its fabricated defaults and they will be reached.
- **Caching a coordinate as a grounded target.** Coordinates are not identities (§5).
- **Driving the user's desktop without a recorded session.** No Flight Recorder, no CU capability.
