# ZylCode film — master assembly sequence

**Status:** DRAFT for the editor. Picture feeds and narration are ready; the final
`zylcode-intro-master.mp4` has not been rendered yet. Resolution target: 1920×1080
(16:9, 23.976 fps picture, stereo 48 kHz / 16-bit mix).

---

## Feeds

### Picture

| Feed                        | Source                                          | Native           | Use                                             |
| --------------------------- | ----------------------------------------------- | ---------------- | ----------------------------------------------- |
| Scene 1 — mission lifecycle | `04-graphics/scene-01-mission-lifecycle.webm`   | 1280×720         | Opening: lifecycle animation                    |
| Scene 2 — architecture      | `04-graphics/scene-02-architecture.webm`        | 1280×720         | Architecture workspace graph                    |
| Scene 3 — evidence ladder   | `04-graphics/scene-03-evidence-ladder.webm`     | 1280×720         | Evidence ladder animation                       |
| Footage 01                  | `01-footage/take-01-workbench.webm`             | check before use | Product opening                                 |
| Footage 02                  | `01-footage/take-02-agent-execution.webm`       | check before use | Workbench surfaces                              |
| Footage 03                  | `01-footage/take-03-verification-evidence.webm` | check before use | Repository understanding                        |
| Footage 04                  | to be captured                                  | —                | Mission composer + autonomy selector            |
| Footage 05                  | to be captured                                  | —                | Hero build: plan → failure → repair             |
| Footage 06                  | to be captured                                  | —                | Final report: proven/derived/hypotheses/unknown |
| Footage 07                  | to be captured                                  | —                | `git log --oneline -3` with real exit code      |
| Footage 08                  | to be captured                                  | —                | `POST /api/intent` Windows error                |
| Footage 09                  | to be captured                                  | —                | PR #44 approval + 5/5 checks + merge            |

### Sound

| Feed               | Path                                        | Role                             |
| ------------------ | ------------------------------------------- | -------------------------------- |
| Narration 15 clips | `05-audio/narration/narration-01..15.mp3`   | Speaker: en-AU-NatashaNeural     |
| Music cue 1        | `05-audio/music/cue-1-open-swell.mp3`       | Title / open swell               |
| Music cue 2        | `05-audio/music/cue-2-documentary-tone.mp3` | Documentary bed (main body)      |
| Music cue 3        | `05-audio/music/cue-3-resolve.mp3`          | Resolve under closing brand line |

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

## Master render

The picture feeds are currently captured at 1280×720 / 636×931 native resolution
(the brief asks for 1920×1080). The assembly upscales every feed to 1920×1080@23.976,
so the master will be delivered at the right resolution; a true 1920×1080 re-capture of
the three scenes and the footage takes is planned as a follow-up.

**Picture filter chain** (all feeds → 1920×1080@23.976):

```
# [0:v] scene-01
[fps=23.976][scale=1920:1080:flags=lanczos][pad=1920:1080:(ow-iw)/2:(oh-ih)/2:color=black] → s1
# [1:v] scene-02
... same → s2
# [2:v] scene-03
... same → s3
# [3:v..5:v] footage takes 01..03
... same → s4..s6
```

**Audio filter chain:**

```
# narration bus (ducking disabled; each narration clip is a distinct stream)
[10] ... [na]  loudnorm=I=-16:TP=-1.5:LRA=11 → na
[11] ... [nb]  loudnorm=I=-16:TP=-1.5:LRA=11 → nb
...
# music bus with ducking under speech
[16] loudnorm=I=-16:TP=-1.5:LRA=11[m1]
[21] loudnorm=I=-16:TP=-1.5:LRA=11[m2]
[25] loudnorm=I=-16:TP=-1.5:LRA=11[m3]
# ducking: sidechain from narration to music
[m1][na][nb]...[np] [...] amix=inputs=...:weights=...:normalize=0 → aout
```

**Final container:** MP4, `libx264` 1920×1080 yuv420p, `aac` 192 kbps stereo.

---

## Master file

```
production/zylcode-intro-film/03-deliverables/zylcode-intro-master.mp4
```

Verify after render:

1. `ffprobe -v error -select_streams v:0 -show_entries stream=width,height` → 1920×1080.
2. `ffprobe -v error -show_entries format=duration` → 8:00–10:00.
3. Narration intelligibility: no clipping (true-peak over -1 dBTP), no dead stretches.
4. Music ducking verified by ear.
5. Brand palette holds.

---

## Cleanup of test files (not deliverables)

- `05-audio/narration/_probe.mp3`, `_probe2.mp3` — probe renders.
- `05-audio/narration/natasha-test-01.mp3` — narrator audition.
- `04-graphics/still-01-mission-lifecycle.png` — raw capture; use the `-clean` stills.
