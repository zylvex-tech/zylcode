# ZylCode product introduction film — preliminary outline

**Status:** PRELIMINARY. Per the brief, final narration is written _after_ footage is
captured and verified. This file maps each scene to the footage that can honestly be
recorded, using the audit in `../00-audit/REPOSITORY_AUDIT_2026-10-31.md` as the source of
truth.

**Target runtime:** 9:00 (acceptable 8:00–10:00) · 16:9 · 1920×1080
**Palette:** dark graphite/navy base, electric blue `#007BFF` primary, restrained amber/orange
accent, technical grid, clean sans typography.

---

## Footage inventory (what actually exists to cut)

1. **`FOOTAGE-01` — product opening.** Live app in browser preview: header, milestone label,
   workbench, version badge `v0.2.3`. Safe framing: status bar (version) or title bar (milestone
   label), never both at once.
2. **`FOOTAGE-02` — workbench surfaces.** Explorer `AVAILABLE / 672 files`, terminal
   `AVAILABLE`, source control `AVAILABLE`, activity rail, menu bar, bottom dock.
3. **`FOOTAGE-03` — repository understanding.** Live intel index (669 files, 2949 symbols,
   8 packages) and code-navigation queries in the Code Navigation surface.
4. **`FOOTAGE-04` — mission composition.** Mission composer + autonomy selector + `RUN MISSION`.
5. **`FOOTAGE-05` — agent execution, the hero.** Real `zylcode mission` run against the
   `first_mission` fixture in `target/film-mission`: plan, tool calls, a real failure
   (`run:after_mean_coverage`), repair, re-run.
6. **`FOOTAGE-06` — verification + evidence.** Final mission report: `status: COMPLETED`,
   `proven 2 / derived 5 / hypotheses 1 / unknowns 1`, `failures: 1 recovered with evidence`,
   `integrity: ledger=true graph=true claims=true`, engineering record + reproducibility package.
7. **`FOOTAGE-07` — real terminal.** `git log --oneline -3` in the app terminal with a real exit code.
8. **`FOOTAGE-08` — honest failure.** The `POST /api/intent` Windows error (`os error 123`) —
   used deliberately to show that ZylCode reports failures truthfully. Optional, scene-dependent.
9. **`FOOTAGE-09` — approvals & CI.** PR #44: reviewer approval, 5/5 required checks green,
   merged `d060b7c` (records human-approval discipline and verification, both real).
10. **Animated diagrams (generated HTML/CSS, recorded):** the mission lifecycle; the
    architecture workspace graph; the evidence ladder (observation → derived → hypothesis →
    unknown → failure). No fake product screenshots.

**Not in the film:** provider keys, any model call, N-ATLAS live endpoint, desktop-only IPC
paths, adoption/customer/partnership numbers, NVIDIA references.

---

## Structure

| #   | Time        | Scene                                              | Primary footage                                                    | Spoken intent (preliminary)                                              |
| --- | ----------- | -------------------------------------------------- | ------------------------------------------------------------------ | ------------------------------------------------------------------------ |
| 1   | 00:00–00:40 | Cinematic hook                                     | grid/motion + title card, brand line                               | An assistant can suggest code. A platform has to show its work.          |
| 2   | 00:40–01:30 | The problem with conventional AI coding assistance | diagram: suggestion → no record → no evidence                      | Unverifiable output is the real cost.                                    |
| 3   | 01:30–02:20 | Introducing ZylCode                                | **FOOTAGE-01**, logo, slogan                                       | Local-first, bring-your-own-key, evidence-first.                         |
| 4   | 02:20–03:30 | The workbench and engineering workflow             | **FOOTAGE-02, 07**                                                 | Surfaces a working engineer already needs.                               |
| 5   | 03:30–04:45 | Repository understanding and mission planning      | **FOOTAGE-03, 04**                                                 | Real index; a mission is a plan before it is a diff.                     |
| 6   | 04:45–06:00 | Agent execution and human approval                 | **FOOTAGE-05** (+ approval discipline via **09**)                  | Real tool calls, a real failure, a real repair; humans stay in the loop. |
| 7   | 06:00–07:00 | Verification and evidence                          | **FOOTAGE-06** (+ optional **08**)                                 | The ledger distinguishes recorded evidence from certification.           |
| 8   | 07:00–07:50 | Provider architecture and N-ATLAS                  | provider diagram + registry counts                                 | BYOK, controlled state; N-ATLAS is an integration workstream.            |
| 9   | 07:50–08:35 | Current capabilities, limitations, direction       | registry stats 22/2/12/1 + Phase 2A re-opened                      | Say plainly what works, what is disputed, what is next.                  |
| 10  | 08:35–09:00 | Close and call to action                           | brand, `Engineered in Nigeria. Built for the world.`, early access | Waitlist; repository is public.                                          |

Timings are targets; they move to match the captured takes rather than stretching footage.

---

## Open items before recording

- [ ] Rebuild from `d060b7c` so `/api/version` shows the merged commit (audit §8).
- [ ] Two recorded takes of the hero mission: a **fresh** workdir, and (if useful) the resumed state.
- [ ] Capture the intel index and one navigation query on camera.
- [ ] Capture PR #44 approval + checks (approval gate evidence).
- [ ] Decide whether scene 7 uses `FOOTAGE-08` (honest failure) — recommended: yes, short.
- [ ] Generate the three animated diagrams and record them at 1920×1080.
