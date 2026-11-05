# ZylCode film — narration and music assembly script

**Status:** DRAFT for the audio editor. Narration cues are free-text lines read by the Australian female voice `en-AU-NatashaNeural`; music cues are placeholder markers for a sound designer. No audio was synthesized in this pass.

---

## Assembly pipeline

```
zylcode-intro-film/
  04-graphics/
    scene-01-mission-lifecycle.webm   # 1280x720, recorded animation
    scene-02-architecture.webm
    scene-03-evidence-ladder.webm
  01-footage/
    take-01-workbench.webm
    take-02-agent-execution.webm
    take-03-verification-evidence.webm
  05-audio/
    narration/          # free-text narration lines (speaker: en-AU-NatashaNeural)
    music/
      cue-1-open-swell.mp3        # graphite pulse under title
      cue-2-documentary-tone.mp3  # warm documentary bed
      cue-3-resolve.mp3           # warmer resolve for closing brand line
  06-narration/
    NARRATION_SCRIPT.md
  03-deliverables/
    zylcode-intro-master.mp4       # final 1920x1080 cut
```

**Step 1 — Render narration** with edge-tts:

```bash
python -c "
import asyncio, edge_tts
lines = open('production/zylcode-intro-film/06-narration/narration-lines.txt').read().strip().splitlines()
for i, line in enumerate(lines, 1):
    communicate = edge_tts.Communicate(line.strip(), voice='en-AU-NatashaNeural',
                                       rate='+0%', volume='+0%')
    await communicate.save(f'production/zylcode-intro-film/05-audio/narration/narration-{i:02d}.mp3')
asyncio.run(asyncio.get_event_loop())
"
```

**Step 2 — Render the three music beds** as the cues above.

**Step 3 — Build the mix** with the narration ducked under the music (narration level ~ -3 dB, music bed ~ -12 dB below the voice, sidechain duck on speech).

**Step 4 — Cut the picture**: 1280×720 clips upconverted to 1920×1080, with the narration track and music beds around it. Transition matches for the shot changes; no other brand overlay until the final brand card.

**Step 5 — Master**: `production/zylcode-intro-film/03-deliverables/zylcode-intro-master.mp4`.

---

## Narration list (speaker: en-AU-NatashaNeural)

Each line is a standalone clip; the editor places them to the nearest edit point in the script.

1. "An assistant can suggest code."
2. "A platform has to show its work."
3. "Most AI coding tools answer a question with a suggestion. They do not leave a trace of why they answered that way. The code is there, the diff is there, but the reasoning is not. And without the reasoning, a suggestion is not a result."
4. "ZylCode. Local-first. Bring your own key. Every word it generates is recorded, every tool it calls is logged, and every result is held in an evidence ledger you can audit yourself."
5. "The workbench is the surface of a real engineering workflow. Explorer, editor, source control, a terminal that runs real commands with real exit codes, and an agent that plans before it executes. Nothing here is simulated; the verification pipeline runs the actual test suite in an isolated worktree."
6. "A mission is a plan before it is a diff. You describe an outcome, and ZylCode turns it into files and components and a verification path. Consequential changes wait for a human to approve them. Nobody auto-approves the hard parts."
7. "The agent executes. It gathers context, writes a plan, and calls real tools against the repository. In the hero build we watch it run through a real failure, then repair it on the next pass. That is not a demo of a model being right. That is the platform showing its own work."
8. "Here is how claims are graded. Recorded observation is logged verbatim from a real command. Derived results are computed from those recorded facts. A hypothesis is marked as a hypothesis, plausible but not proven. An unknown is stated as unknown, never invented. A failure is kept, with its cause and its recovery, in the record. Everything has a label, and nothing is disguised."
9. "Verification is a gate, not a suggestion. The real test suite decides pass or fail, and it runs in an isolated worktree so it cannot reach outside the candidate it is judging. The record keeps the result, the ledger, and the integrity check. That is the difference between a report and an engineering record."
10. "On the repository side, the index is the evidence. Six hundred and sixty-nine files, two thousand nine hundred and forty-nine symbols, eight packages. Every symbol is pulled from a real scan, and the navigation graph is built from parsed imports and references, each edge labelled Deterministic, Heuristic, or Unsupported."
11. "The provider layer is a contracted integration, not a platform. You keep your own credentials. ZylCode does not call a model without a provider you have configured and funded. Every reply passes through a documented allowlist, a per-user daily limit, and loopback-only development access. The model comes and goes; the evidence stays."
12. "Nothing here hides a failure. The terminal shows a real command with a real exit code. The mission record shows a failed rung with its cause and its recovery. That is the design: a platform that tells you when it is wrong, because that is the only way to trust what it gets right."
13. "Human approval is part of the record. PR 44 — coverage floor of 80% per file, live-backend captures, Respan gateway — passed 5 of 5 required checks, was approved by one reviewer, and was merged into main through the standard branch-protection path. The closing branch was deleted; the merge is in the ledger."
14. "What works is measured. What is disputed is labelled. What is next is a line in the roadmap. ZylCode is not a chat window that answers questions. It is a workspace where a question becomes a plan, a plan becomes a change, and a change becomes evidence."
15. "Local-first. Bring your own key. Show your work. Built for the world. Engineered in Nigeria. To join the early access waitlist, visit the public repository."

---

## Music cue list (for the sound designer)

| Cue                    | Place               | Palette                                                    |
| ---------------------- | ------------------- | ---------------------------------------------------------- |
| cue-1-open-swell       | Title / 00:00–00:15 | Low graphite & navy pulse under a single rising bell       |
| cue-2-documentary-tone | 00:15–07:20         | Warm documentary bed, steady, unobtrusive, no hook or drop |
| cue-3-resolve          | 08:00–08:30         | Warmer, resolving bed under the closing brand line         |

## Short-form / social cuts (optional, if the film is trimmed)

- Hook + title: lines 1–2 + cue-1 (15 s).
- "Show your work" thesis: lines 3–4 + cue-2 (30 s).
- Evidence ladder / verification: lines 8–9 + cue-2 (30 s).
- Close + brand: lines 14–15 + cue-3 (30 s).

---

## Audio/Human-voice note

The script above uses the Australian female voice `en-AU-NatashaNeural` as instructed. The podcast file you shared is not used as a voice source; it remains a reference recording only. If you later want your own recorded voice over the same lines, the correct workflow is to record clean read-throughs of these lines and place the clips at the `[CUT ON QUOTE]` edit points in the script. The script is compatible with that, so the two options do not lock you out of either choice.
