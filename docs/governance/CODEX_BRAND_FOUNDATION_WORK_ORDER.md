# CODEX WORK ORDER — ZYLFORGE BRAND SYSTEM FOUNDATION

**Issued by:** owner (product owner / governing authority)
**Carried by:** architecture owner session
**Status:** AUTHORIZED — file plan APPROVED with five mandatory corrections
**Scope:** documentation / brand governance ONLY
**Do not commit. Do not push.** Return the report, then stop.

---

## 0. Verified repository state at issue time

Established by direct observation, not assumption. Codex must confirm these still hold before writing;
if any differ, stop and report rather than proceeding.

| Fact | Observed value |
|---|---|
| `HEAD` | `1faf90944a132fcb008c9a1dcad19f85286c3628` |
| `origin/main` | `1faf90944a132fcb008c9a1dcad19f85286c3628` (identical — in sync) |
| `docs/brand/` | **does not exist** |
| `docs/design/` | **does not exist** |
| `.agents/skills/` | **does not exist** |
| `docs/evidence/` | **does not exist** |
| `docs/` subdirs present | `architecture`, `community`, `contributing`, `devlog`, `governance`, `roadmap` |
| Existing `ZylForge` references | 9 tracked files (the PUBLIC-FOUNDATION set + `SECURITY.md`, `SUPPORT.md`) |
| `ZylMind` / `Aurora` references | **zero** in tracked files |

**Consequence Codex must respect:** every path named in the approved file plan is a **new** path. There is
no pre-existing `BRAND.md` to preserve and no pre-existing brand authority to inherit. `BRAND.md` is
being *created* as canonical, and Correction 4's hierarchy describes the state after this pass — it is
not a description of an existing hierarchy. Say so plainly in the report; do not imply a prior authority
that was superseded.

**Second consequence:** the tree carries pre-existing uncommitted edits from another session (many files
under `crates/`, `apps/`, plus root `.md` files). **Do not stage anything outside the brand/design
documentation paths.** Before any `git add`, run `git diff --cached --stat` and confirm the stat matches
your own edits exactly. A larger stat means foreign changes are staged — unstage and reassign them.

---

## 1. Owner's authorization and corrections (verbatim)

> **File plan APPROVED with the following mandatory corrections. Proceed with the documentation/governance implementation.**
>
> **Correction 1 — design tokens**
>
> Do not create final production `brand.tokens.json` values where exact colours have not yet been validated.
>
> Create the token architecture/schema, but distinguish clearly between:
>
> * `LOCKED` semantic architecture;
> * `PROPOSED` visual values;
> * `VALIDATED` production values.
>
> If necessary use:
>
> `docs/brand/tokens/brand.tokens.schema.json`
>
> and
>
> `docs/brand/tokens/brand.tokens.proposed.json`
>
> until Penpot, accessibility, dark/light, engineering-viewport and cross-platform validation have been completed.
>
> Do not infer Zylvex corporate colours from Concept Study 01.
>
> Do not promote arbitrary HEX values to production authority.
>
> **Correction 2 — Engineering Document Standard**
>
> The following requirements are approved and should be encoded as authoritative requirements rather than left unresolved.
>
> The default drawing/title-block architecture is:
>
> Customer/company logo | Project | Drawing title | Drawing number | Revision | Scale | Units | Material | Finish | Tolerances | Drawn by | Checked by | Approved by | Status | Date | Sheet | Provenance/QR
>
> with restrained attribution:
>
> `Generated with ZylForge`
>
> Customer/company identity is dominant. For internal Zylvex engineering work, the organization is `Zylvex Technologies Limited`.
>
> Approved document lifecycle states:
>
> `DRAFT → CALCULATED → VALIDATED → REVIEWED → APPROVED → RELEASED`
>
> These are governed engineering states, not decorative badges. Do not imply that `VALIDATED`, `APPROVED` or `RELEASED` automatically means certified, safe, or code-compliant.
>
> Released outputs may carry a small provenance QR verification record containing, where applicable:
>
> Document ID; Revision; Project; Status; Generated timestamp; Approved timestamp; Software/kernel version; Document hash; Model hash; Validation record.
>
> The QR establishes document identity/provenance. It does not imply certification.
>
> The approved Engineering Analysis Report structure is:
>
> `ZYLFORGE`
>
> `ENGINEERING ANALYSIS REPORT`
>
> Project | Document No. | Revision | Status | Prepared | Reviewed | Approved
>
> 01 Executive Summary
> 02 Model Definition
> 03 Geometry
> 04 Materials
> 05 Mesh
> 06 Loads & Boundary Conditions
> 07 Solver Configuration
> 08 Results
> 09 Analytical / Reference Validation
> 10 Mesh Convergence
> 11 Warnings & Limitations
> 12 Conclusions
> 13 Approval
> 14 Provenance
>
> Engineering result figures require appropriate legends, units, scales and captions.
>
> This uses the restrained **Evidence Voice**: professional A4/A3 white/light engineering documentation with minimal Aurora accents and no futuristic decoration around engineering results.
>
> The official `ZylForge Fabrication Job Pack` document family contains, as applicable:
>
> Cover / Job Summary; General Arrangement; Fabrication Drawings; Detail Drawings; BOM; Cut List; Material Schedule; Fastener Schedule; Weld Schedule; Machining Requirements; Tolerances; Surface Treatment/Paint; Assembly Sequence; Inspection/Test Plan; Workshop Notes; Cost Estimate; Revision Register; Approvals; Provenance.
>
> Do not invent standards, tolerances, weld procedures, materials, acceptance criteria or engineering calculations merely to populate templates.
>
> The approved Research Output Style supports:
>
> equations; references; plots; parameter tables; mesh studies; uncertainty; solver configuration; datasets; experiment IDs; reproducibility metadata; model hashes; software versions; citations; publication-friendly PDF/figures/data.
>
> Institutional co-branding follows:
>
> `UNIVERSITY / RESEARCH INSTITUTE`
>
> `Faculty / Laboratory / Department`
>
> `Powered by ZylForge Education & Research`
>
> The institution remains visually primary.
>
> Enterprise organizations may configure company identity, division, contact information, document numbering, revision conventions, approval roles, title blocks, report covers, approved material catalogs and standard notes, but cannot remove or falsify required underlying provenance.
>
> **Correction 3 — terminology**
>
> Encode these approved identities consistently:
>
> `ZylForge`
>
> `Engineering OS by Zylvex Technologies`
>
> Category: `Engineering OS`
>
> Expanded positioning: `AI-Native Engineering Operating System`
>
> Brand promise: `Engineering Intelligence. Proven by Evidence.`
>
> Origin statement: `Engineered in Nigeria. Built for the world.`
>
> Campaign/exhibition line: `From Intent to Engineered Reality.`
>
> ZylMind identity:
>
> `ZylMind — ZylForge Engineering Intelligence`
>
> Workspaces:
>
> `Design · Simulate · Manufacture · Plant & Infrastructure · Reality · Twin · Research · Learn · Cloud`
>
> Education architecture:
>
> `ZylForge Learn` = workspace inside the Engineering OS.
>
> `ZylForge Academy` = broader education platform/website.
>
> Commercial entitlement identities:
>
> `ZylForge Individual`
>
> `ZylForge Professional`
>
> `ZylForge Enterprise`
>
> `ZylForge Education & Research`
>
> **Correction 4 — authority hierarchy**
>
> Establish explicitly:
>
> `docs/brand/BRAND.md`
>
> = canonical brand governance authority.
>
> `docs/brand/README.md`
>
> = navigation/index only.
>
> `.agents/skills/zylforge-experience-brand/SKILL.md`
>
> = agent operating guidance that inherits BRAND.md; it is not an independent brand authority.
>
> `docs/design/DESIGN_SYSTEM_SPEC.md`
>
> = subordinate product/UI design specification.
>
> Penpot, when established,
>
> = approved visual and interaction design authority.
>
> Machine-readable tokens
>
> = implementation contract derived from approved brand/design decisions.
>
> Capability Matrix + implementation + tests/runtime validation
>
> = authority for what ZylForge actually does.
>
> Branding or visual mockups must never create capability claims.
>
> **Correction 5 — Concept Study 01**
>
> Register Concept Study 01 as:
>
> `APPROVED VISUAL DIRECTION — NOT PRODUCTION ARTWORK`
>
> Approved characteristics include Aurora teal/cyan ZylForge identity; black/graphite industrial technology environment; restrained Zylvex parent identity; real engineering/scientific imagery; professional workwear; restrained white engineering documentation; sophisticated engineering exhibition identity; professional technical merchandise.
>
> Do not derive authoritative lettering, logo geometry, HEX colours, dimensions, title-block geometry or document content from the generated image.
>
> Do not fabricate an asset path. Record the image itself as `SOURCE ASSET LOCATION: NOT YET REGISTERED` until an actual repository asset exists.
>
> **Proceed with the approved file plan after applying these corrections.**
>
> Remain within documentation/brand governance scope.
>
> Do not modify runtime UI, runtime themes, websites, engineering code, CXX/OCCT/IGES, Android or Apple/iOS.
>
> Do not download fonts.
>
> Do not replace logos.
>
> Do not commit or push.
>
> After writing, run the documentation-specific checks available in the repository and inspect every changed file.
>
> Return the previously specified **ZYLFORGE BRAND SYSTEM FOUNDATION REPORT**, including exact `git diff --stat` and `git status --short`, and then STOP.

---

## 2. Report requirements

The **ZYLFORGE BRAND SYSTEM FOUNDATION REPORT** is the deliverable. It must carry:

1. **Reproduction block** — `HEAD` at start, `git status --short`, `git diff --stat` (exact, unedited), and
   the specific commands used for the documentation checks.
2. **Changed-file inventory** — one row per file: path, status (new/modified), line count, and which
   correction it satisfies.
3. **Correction-by-correction compliance table** — for each of Corrections 1–5, name the file and section
   that implements it. A correction with no corresponding file is a **failure**, not a gap.
4. **Verification of the checks actually run** — raw output, not a summary. If a check could not be run,
   say which and why. Never convert a skipped check into a pass.
5. **Token-axis statement** — which token values are `LOCKED`, which are `PROPOSED`, and an explicit
   statement that **no** value is `VALIDATED`. If any value is marked `VALIDATED`, state the evidence that
   justifies it; absent that evidence it must be `PROPOSED`.
6. **Non-derivation statement** — explicit confirmation that no lettering, logo geometry, HEX colour,
   dimension, title-block geometry or document content was derived from the Concept Study 01 image, and
   that the asset location reads `SOURCE ASSET LOCATION: NOT YET REGISTERED`.
7. **Scope statement** — explicit confirmation that no runtime UI, runtime theme, website, engineering
   code, CXX/OCCT/IGES, Android or Apple/iOS file was touched, no font downloaded, no logo replaced, and
   nothing committed or pushed.
8. **Unresolved items** — anything the corrections do not settle, listed as open rather than filled in with
   invented content.

---

## 3. Standing constraints carried from the governance set

These are not new requirements; they are already governing and Codex is bound by them.

- **A phase number belongs to the governance roadmap and to nothing else** (Agent Operating Protocol §4.7).
  Brand work is a **named track** — `DESIGN-SYSTEM-NN` — never a phase number. No phase number may appear
  in brand or design documents except as roadmap metadata inside a capability's own context.
- **Branding and visual mockups never create capability claims** (Correction 4, and Public Communication
  Policy §2). A brand asset must not imply a capability state. **This is the single sharpest interaction
  between this work order and the rest of governance:** a beautiful engineering-report mockup is a
  `CONCEPT` visual and must be labelled as such, in the image, wherever it appears publicly.
- **Do not invent engineering content.** Correction 2 forbids inventing standards, tolerances, weld
  procedures, materials, acceptance criteria or calculations to populate templates. The same discipline
  applies to sample title blocks, report figures and BOM rows: use `PROPOSED`/`ILLUSTRATIVE` placeholders
  or omit, never plausible-looking fabricated values.
- **Living-template document family.** The Fabrication Job Pack list is a **superset** ("contains, as
  applicable"), not a fixed set of 19 pages. Encode it as an optional-section architecture, not as a
  mandatory page sequence — otherwise every job pack would have to carry an empty Weld Schedule.
- **`Generated with ZylForge` is attribution, not authorship.** The customer/company identity is dominant
  and the ZylForge mark is restrained. Do not design a title block where the ZylForge logo is the largest
  element.
- **Provenance QR is identity, not certification.** Correction 2 states this twice; encode the disclaimer
  in the standard itself so a downstream template cannot omit it.
- **Evidence-first discipline.** Where this report states a fact, it must be observed. Where something was
  not verified, say so. Do not describe intended output in the position where captured output belongs.

---

## 4. Next milestone (for context — not in scope now)

If this pass is accepted, the next brand milestone is **Brand Foundation → Penpot Design System**:
Aurora palette, light/dark systems, eight themes, typography scales, spacing, iconography, components,
engineering viewport semantics, ZylMind interaction language, and Adaptive Engineering Workspace screen
architecture.

Consequence for **this** pass: build the token architecture so that milestone can fill it. The schema —
which token *groups* exist, what a token *means*, and how the three authority axes (`LOCKED` /
`PROPOSED` / `VALIDATED`) are represented — is the durable part. The values are not, and must not be
guessed now.

---

## 5. Stop condition

Write the files. Run the documentation checks. Inspect every changed file. Return the report with exact
`git diff --stat` and `git status --short`. **Commit nothing. Push nothing. Stop.**
