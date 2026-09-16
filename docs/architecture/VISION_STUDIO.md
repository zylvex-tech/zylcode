# VISION_STUDIO.md — ZylCode Vision Studio

**Status: PROPOSED**
**Rung: R0 (CLAIMED)**
**Phases: 8A (foundation) · 8B (canvas) · 8C (design↔code) · 9 (visual intelligence)**
**Governing docs:** `ZYLCODE_PRODUCT_CONSTITUTION_V2.md` §8 · `ZYLCODE_ARCHITECTURE_V2.md` §3.4

> ⚠️ **This system does not exist.** No design model, no canvas, no UI-IR, no adapters.

---

## 1. The Governing Decisions

### Decision 1 — Design is structured data

> **Design must be machine-readable and agent-addressable.**

The AI must manipulate **design primitives**, not screenshots. Screenshots are *observation*.
Primitives are the *medium*.

If the agent edits a screenshot, every operation is a lossy guess. If it edits primitives, every
operation is exact, undoable, and explainable.

### Decision 2 — Model before canvas

Build the canonical representation first. A canvas built before the model will encode the model
implicitly and badly.

### Decision 3 — Not web-centric

> **React DOM must never be the design model.**

```
DESIGN MODEL
     ↕
UI INTERMEDIATE REPRESENTATION  (UI-IR)
     ↕
FRAMEWORK ADAPTER
```

Adapters: React · Next.js · HTML/CSS · Flutter · Jetpack Compose · SwiftUI · React Native.

If React DOM becomes the design model, Vision Studio is permanently web-centric and every
non-web target becomes a translation of a translation — with compounding fidelity loss.

---

## 2. Phase 8A — Design Model

```
DesignDocument
├── pages[]        Page
├── tokens         Token          (colour, spacing, radius, typography scale)
├── variables      Variable       (modes: light/dark/compact)
├── styles[]       Style          (named, reusable)
└── libraries[]    reference to shared component libraries

Page
└── frames[]       Frame          (artboard / screen / region)

Frame
├── children[]     Node
├── layout         Layout         (auto-layout: direction, gap, padding, alignment)
├── constraints    Constraint[]   (resize behaviour)
├── grid           Grid?
└── interactions[] Interaction    (prototype links, triggers, transitions)

Node (one of)
├── Frame
├── Component      definition
├── Instance       of a Component, with overrides
├── Variant        a Component variant axis
├── Text
├── Vector
└── Image
```

### 2.1 Requirements

| # | Requirement |
|---|---|
| A1 | Every node has a **stable id**, independent of position in the tree |
| A2 | Every node is **addressable** — an agent can reference it by id and path |
| A3 | Every mutation is **undoable** |
| A4 | The model is **serialisable** and diffable |
| A5 | Tokens are **first-class**, not hard-coded values |
| A6 | Components have **variants** as explicit axes |
| A7 | Instances record **overrides** explicitly |
| A8 | Constraints and auto-layout are **explicit**, not inferred at render time |

**A2 is the one that makes the AI useful.** If a node cannot be addressed, an agent cannot say
"set the padding on the primary button in the signup frame" — it can only say "change this
picture".

---

## 3. Phase 8B — Canvas

**Then build:** selection · move · resize · zoom · pan · alignment · distribution · grids ·
constraints · auto layout · components · variants · typography · design tokens · responsive
frames · prototyping · interactions.

**Benchmark:** a designer completes a realistic multi-screen layout **without fighting the tool**.
The measure is workflow friction, not feature count.

---

## 4. Phase 8C — Design ↔ Code

```
   DESIGN MODEL
        ↕                        round-trip
   UI-IR (canonical)             must be lossless
        ↕                        in both directions
   FRAMEWORK ADAPTER
        ↕
   React | Next.js | HTML/CSS | Flutter | Jetpack Compose | SwiftUI | React Native
```

### 4.1 The UI-IR must be genuinely intermediate

| Property | Requirement |
|---|---|
| Target-neutral | no React concepts, no Compose concepts |
| Expressive enough | can represent every design primitive without loss |
| Diffable | a design change produces a readable code diff |
| Reversible | code → UI-IR → design must not lose intent |

### 4.2 Fidelity is measured, not asserted

**Round-trip fidelity must be a number**, produced by a comparison procedure, for at least two
**structurally different** targets (e.g. React and Jetpack Compose).

"It looks right" is R1. A fidelity metric is R3.

### 4.3 Adapter contract

```
trait FrameworkAdapter {
    fn emit(&self, ir: &UiIr) -> Vec<SourceFile>;
    fn parse(&self, sources: &[SourceFile]) -> Result<UiIr>;   // reverse direction
    fn capabilities(&self) -> AdapterCapabilities;             // what it cannot express
}
```

`capabilities()` is mandatory: when a design uses a primitive a target cannot express, the
adapter must **report it**, not silently approximate it. Fail closed.

---

## 5. Phase 9 — Visual Intelligence & Self-Repair

```
EXPECTED DESIGN
       ↓
RENDERED SOFTWARE
       ↓
SCREENSHOT
       ↓
VISUAL ANALYSIS
       ↓
DIFFERENCE MODEL        ← structured, not a pixel-diff blob
       ↓
REPAIR PLAN             ← what to change, where, why
       ↓
CODE / DESIGN CHANGE
       ↓
RENDER AGAIN
```

### 5.1 The difference model must be structured

A pixel diff says "3% of pixels differ". A difference model says:

```
- Frame "Signup" / Instance "PrimaryButton": padding.left 12 → 16
- Frame "Signup" / Text "Terms": colour token text.secondary missing, fell back to #000
- Frame "Signup" / auto-layout gap: 8 → 12
```

**Only the second can produce a repair plan.** This is why the design model must be
machine-readable: the difference model is expressed in the *design model's* terms.

### 5.2 Benchmark

A seeded visual defect (spacing, colour, or layout) is:
1. **detected** automatically,
2. **localised** to a design primitive,
3. **repaired** without human intervention,
4. **re-verified** after re-render.

Evidence: before/after screenshots, the difference model, the code/design diff, and the
re-verification result — all as artifacts, all cited.

---

## 6. Interfaces

```
zylcode design open <document>
zylcode design export --target react|compose|html
zylcode design diff <expected> <rendered>
zylcode design repair --plan
```

Plus agent design tools: `design.create_node`, `design.set_property`, `design.add_variant`,
`design.bind_token`, …

---

## 7. Benchmark / Acceptance

| Phase | Criterion | Rung |
|---|---|---|
| 8A | An agent constructs and mutates a design through primitives only, with no image manipulation | R3 |
| 8B | A designer completes a realistic multi-screen layout without fighting the tool | R3 |
| 8C | Round-trip fidelity **measured** for two structurally different targets | R3 |
| 8C | An inexpressible primitive is **reported**, not approximated | R3 (negative test) |
| 9 | A seeded visual defect is detected, localised, repaired and re-verified automatically | R3 |

---

## 8. Anti-Requirements

- Do not make React DOM the design model.
- Do not manipulate screenshots where primitives exist.
- Do not build the canvas before the model.
- Do not silently approximate a primitive a target cannot express.
- Do not claim round-trip fidelity without a measured number.
- Do not store design state outside the Project System.
