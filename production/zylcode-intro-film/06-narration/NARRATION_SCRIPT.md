# ZylCode product introduction film — narration script

**Generated:** 2026-10-31
**Status:** DRAFT — narration audio not yet rendered; edit points and music cue markers are marked and ready for the cutting room.
**Voice:** en-AU-NatashaNeural (female, Australian English) — Natural Neural, non-robotic.
**Pitch / pace:** general documentary; measured for the "showing your work" beats, warmer and slightly slower for the closing line.

---

## Notes on this script

- All spoken lines are copy-editing of the source material in this project, never invented. Where the source is a live product claim (audit, CI output, PR #44), the line is anchored to what the repository actually reports.
- `[MUSIC: ...]` blocks are music cue markers for a sound designer; they are not narration.
- **[CUT ON QUOTE]** marks the place where the narrator hands to a recorded clip.
- Timing targets are 9:00 (acceptable 8:00–10:00), 16:9, 1920×1080.

---

## SCRIPT

### 00:00–00:15 — Title / cinematic hook

[MUSIC: open swell — low graphite pulse, then a single bell tone rising under a title card]

> "An assistant can suggest code. A platform has to show its work."

### 00:15–00:45 — The problem with conventional AI coding assistance

[MUSIC: held, warm, documentary tone]

> "Most AI coding tools answer a question with a suggestion. They do not leave a trace of why they answered that way. The code is there, the diff is there — but the reasoning is not. And without the reasoning, a suggestion is not a result."

### 00:45–01:20 — Introducing ZylCode

[CUT ON QUOTE] — FOOTAGE-01: live app in browser preview (header, milestone label `v0.3.0-convergence`, workbench, version badge `v0.2.3`)

> "ZylCode. Local-first. Bring your own key. Every word it generates is recorded, every tool it calls is logged, and every result is held in an evidence ledger you can audit yourself."

### 01:20–02:00 — The workbench

[CUT ON QUOTE] — FOOTAGE-02: explorer, terminal, source control, activity rail

> "The workbench is the surface of a real engineering workflow. Explorer, editor, source control, terminal that runs real commands with real exit codes, and an agent that plans before it executes. Nothing here is simulated — the verification pipeline runs the actual test suite in an isolated worktree."

### 02:00–02:40 — Mission planning

[CUT ON QUOTE] — FOOTAGE-04: mission composer + autonomy selector + RUN MISSION

> "A mission is a plan before it is a diff. You describe an outcome, and ZylCode turns it into files, components, and a verification path. Consequential changes wait for a human to approve them. Nobody auto-approves the hard parts."

### 02:40–03:30 — Agent execution

[CUT ON QUOTE] — FOOTAGE-05: real `zylcode mission` run against the first_mission fixture

> "The agent executes. It gathers context, writes a plan, and calls real tools against the repository. In the hero build we watch it run through a real failure — `run:after_mean_coverage` — and repair it on the next pass. That is not a demo of a model being right. That is the platform showing its own work."

### 03:30–04:10 — The evidence ladder

[CUT ON QUOTE] — evidence ladder diagram, animated

> "Here is how claims are graded. Recorded observation is logged verbatim from a real command. Derived results are computed from those recorded facts. A hypothesis is marked as a hypothesis — plausible, not proven. An unknown is stated as unknown, never invented. A failure is kept, with its cause and its recovery, in the record. Everything has a label, and nothing is disguised."

### 04:10–04:50 — Verification

[CUT ON QUOTE] — FOOTAGE-06: final mission report

> "Verification is a gate, not a suggestion. The real test suite decides pass or fail, and it runs in an isolated worktree so it cannot reach outside the candidate it is judging. The record keeps the result, the ledger, and the integrity check — `ledger = true, graph = true, claims = true`. That is the difference between a report and an engineering record."

### 04:50–05:30 — Repository understanding

[CUT ON QUOTE] — FOOTAGE-03: live intel index and code-navigation query

> "On the repository side, the index is the evidence. Six hundred and sixty-nine files, two thousand nine hundred and forty-nine symbols, eight packages. Every symbol is pulled from a real scan, and the navigation graph is built from parsed imports and references — each edge labelled Deterministic, Heuristic, or Unsupported. If the parser cannot resolve it, the tool says Unsupported, not a guess."

### 05:30–06:10 — Provider platform

[CUT ON QUOTE] — provider diagram + registry counts, animated

> "The provider layer is a contracted integration, not a platform. You keep your own credentials. ZylCode does not call a model without a provider you have configured and funded. Every reply passes through a documented allowlist, a per-user daily limit, and loopback-only development access. The model comes and goes; the evidence stays."

### 06:10–06:50 — Honest failures

[CUT ON QUOTE] — FOOTAGE-08: POST /api/intent Windows error (os error 123), short

> "Nothing here hides a failure. The terminal shows a real command with a real exit code. The mission record shows a failed rung with its cause and its recovery. That is the design: a platform that tells you when it is wrong, because that is the only way to trust what it gets right."

### 06:50–07:20 — PR #44, human approval

[CUT ON QUOTE] — FOOTAGE-09: PR #44 approval + 5/5 checks green, merged

> "Human approval is part of the record. PR 44 — coverage floor of 80% per file, live-backend captures, Respan gateway — passed 5 of 5 required checks, was approved by one reviewer, and was merged into main through the standard branch-protection path. The closing branch was deleted; the merge is in the ledger."

### 07:20–08:00 — Current state

> "What works is measured. What is disputed is labelled. What is next is a line in the roadmap. ZylCode is not a chat window that answers questions. It is a workspace where a question becomes a plan, a plan becomes a change, and a change becomes evidence."

### 08:00–08:30 — Close and call to action

[MUSIC: swell, warmer, resolving]

> "Local-first. Bring your own key. Show your work."
> "Built for the world. Engineered in Nigeria."
> "To join the early access waitlist, visit the public repository. The voice you heard tonight is yours to verify."

---

## EDIT NOTES

| Beat              | Source material             | Cue                  |
| ----------------- | --------------------------- | -------------------- |
| Hook              | brand line + title card     | title card animation |
| Problem           | evidence ladder diagram     | animation            |
| Introducing       | FOOTAGE-01                  | app preview          |
| Workbench         | FOOTAGE-02, 07              | explorer + terminal  |
| Planning          | FOOTAGE-04                  | composer             |
| Agent execution   | FOOTAGE-05                  | hero mission         |
| Evidence ladder   | three animated diagrams     | 01/02/03             |
| Verification      | FOOTAGE-06                  | final mission report |
| Provider platform | registry + provider diagram | diagram              |
| Honest failures   | FOOTAGE-08                  | failure clip         |
| Human approval    | FOOTAGE-09                  | PR 44                |
| Close             | brand + closing line        | brand card           |

## Audio/Human-voice note (to the user)

You asked for your podcast voice and then asked to use the Australian female voice and not your own. I have built the script for the Australian female voice (`en-AU-NatashaNeural`) as now instructed. The podcast file remains unused as a voice source. If you later want your own recorded voice over the same script, the only way to do it without fabricating a "clone" is to record clean read-throughs of these lines and drop the clips in at the `[CUT ON QUOTE]` points.

## Music cues

The three cue types above (`open swell`, `held documentary tone`, `swell warmer resolving`) are placeholders for a sound designer. The film needs:

1. Low graphite/navy pulse under the hook.
2. Warm documentary tone for the explanatory middle.
3. A resolve on the closing brand line.

No music was generated in this pass. If you want, I can generate a short 5-second music bed per cue with a local synthesis option, or hand the cue markers to an audio contractor.
