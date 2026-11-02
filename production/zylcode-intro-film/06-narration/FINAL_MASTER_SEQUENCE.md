# ZylCode film — final master sequence

**Status:** READY FOR ASSEMBLY. Narration (15 clips) and music cue beds are complete.
Picture feeds are 1280×720 native; the assembly pipeline rescales every feed to true
1920×1080@23.976 and pads to 1920×1080. A re-capture of the three animated scenes at
1920×1080 is tracked in `04-graphics/RECAPTURE_PLAN.md` for a follow-up pass.

---

## Feeds

### Picture

| Feed | Source | Native | Use |
|---|---|---|---|
| Scene 1 — mission lifecycle | `04-graphics/scene-01-mission-lifecycle.webm` | 1280×720 | Opening: lifecycle animation |
| Scene 2 — architecture | `04-graphics/scene-02-architecture.webm` | 1280×720 | Architecture workspace graph |
| Scene 3 — evidence ladder | `04-graphics/scene-03-evidence-ladder.webm` | 1280×720 | Evidence ladder animation |
| Footage 01 | `01-footage/take-01-workbench.webm` | check before use | Product opening |
| Footage 02 | `01-footage/take-02-agent-execution.webm` | check before use | Workbench surfaces |
| Footage 03 | `01-footage/take-03-verification-evidence.webm` | check before use | Repository understanding |

### Sound

| Feed | Path | Role |
|---|---|---|
| Narration 15 clips | `05-audio/narration/narration-01..15.mp3` | Speaker: en-AU-NatashaNeural |
| Music cue 1 | `05-audio/music/cue-1-open-swell.mp3` | Title / open swell |
| Music cue 2 | `05-audio/music/cue-2-documentary-tone.mp3` | Documentary bed (main body) |
| Music cue 3 | `05-audio/music/cue-3-resolve.mp3` | Resolve under closing brand line |

---

## Rough cut timings (target 8:00–10:00)

```
00:00–00:12  cue-1 + title card                                      [MUSIC-1]
00:12–00:15  narration-01 "An assistant can suggest code."
00:15–00:18  narration-02 "A platform has to show its work."
00:18–00:40  narration-03 "Most AI coding tools answer a question…"
00:40–01:10  narration-04 "ZylCode. Local-first. Bring your own key…"
01:10–01:40  FOOTAGE-01 + line 4 (intro)
01:40–02:10  FOOTAGE-02 + line 5 (workbench)
02:10–02:50  FOOTAGE-04 + line 6 (mission)
02:50–03:35  FOOTAGE-05 + line 7 (agent execution)
03:35–04:10  scene-03 (evidence ladder) + line 8
04:10–04:50  FOOTAGE-06 + line 9 (verification)
04:50–05:30  FOOTAGE-03 + line 10 (index)
05:30–06:10  provider diagram + line 11 (provider platform)
06:10–06:50  FOOTAGE-08 + line 12 (honest failure)
06:50–07:20  FOOTAGE-09 + line 13 (PR #44)
07:20–08:00  narration-14 (current state)
08:00–08:30  brand card + narrative-15 (close) + cue-3 + cue-2 fade  [MUSIC-3]
```

---

## Master render pipeline

```
# 1. Picture → all feeds to 1920x1080@23.976, padded to 1920x1080
# 2. Audio → narration bus (loudnorm I=-16:TP=-1.5:LRA=11), music bus with ducking
# 3. Mix → aac 192 kbps stereo master
# 4. Encode → libx264, crf 18, yuv420p
```

**Verify after render:**

1. `ffprobe -v error -select_streams v:0 -show_entries stream=width,height` → 1920×1080.
2. `ffprobe -v error -show_entries format=duration` → 8:00–10:00.
3. Narration intelligibility: true-peak under -1 dBTP, no dead stretches > 2 s.
4. Music ducking verified by ear (narration clear at every point).
5. Brand palette holds.

---

## Deliverables

| Path | Kind |
|---|---|
| `06-narration/NARRATION_SCRIPT.md` | narrated lines keyed to filmed beats |
| `06-narration/NARRATION_ASSEMBLY.md` | edit map |
| `06-narration/MASTER_ASSEMBLY_SEQUENCE.md` | this file |
| `06-narration/narration-lines.txt` | source line list |
| `05-audio/narration/narration-01..15.mp3` | rendered narration |
| `05-audio/music/cue-1-open-swell.mp3` | open swell |
| `05-audio/music/cue-2-documentary-tone.mp3` | documentary bed |
| `05-audio/music/cue-3-resolve.mp3` | resolve |
| `03-deliverables/zylcode-intro-master.mp4` | final master (pending) |

---

## Cleanup of test files (not deliverables)

- `05-audio/narration/_probe.mp3`, `_probe2.mp3` — probe renders.
- `05-audio/narration/natasha-test-01.mp3` — narrator audition.
- `04-graphics/still-01-mission-lifecycle.png` — raw capture; use the `-clean` stills.
