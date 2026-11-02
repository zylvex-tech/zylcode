# ZylCode film — master assembly manifest

**Status:** DRAFT for the editor/sound designer. Picture and narration audio are
ready; the master cut (`zylcode-intro-master.mp4`) has not been rendered yet.
**Resolution:** 1920×1080 · 16:9 · 23.976 fps target · stereo 48 kHz / 16-bit mix.

---

## Feeds

### Picture

| Source | Path | Resolution | Length | Use |
|---|---|---|---|---|
| Animated diagram scene 1 | `04-graphics/scene-01-mission-lifecycle.webm` | 1280×720 | recorded anim | Opening: mission lifecycle |
| Animated diagram scene 2 | `04-graphics/scene-02-architecture.webm` | 1280×720 | recorded anim | Architecture workspace graph |
| Animated diagram scene 3 | `04-graphics/scene-03-evidence-ladder.webm` | 1280×720 | recorded anim | Evidence ladder |
| FOOTAGE-01 product opening | `01-footage/take-01-workbench.webm` | 1280×720 | recorded app | Title + intro |
| FOOTAGE-02 workbench surfaces | `01-footage/take-02-agent-execution.webm` | 1280×720 | recorded app | Explorer/terminal/control surfaces |
| FOOTAGE-03 intel index | `01-footage/take-03-verification-evidence.webm` | 1280×720 | recorded app | Repository understanding |
| FOOTAGE-04 mission composition | (to be captured) | 1280×720 | — | Mission composer + autonomy selector |
| FOOTAGE-05 agent execution | (to be captured) | 1280×720 | — | Hero build: plan → failure → repair |
| FOOTAGE-06 verification + evidence | (to be captured) | 1280×720 | — | Final report: proven/derived/hypotheses/unknown |
| FOOTAGE-07 real terminal | (to be captured) | 1280×720 | — | `git log --oneline -3` with real exit code |
| FOOTAGE-08 honest failure | (to be captured) | 1280×720 | — | `POST /api/intent` Windows error |
| FOOTAGE-09 approvals + CI | (to be captured) | 1280×720 | — | PR #44 approval + 5/5 checks + merge |

**Note on scene sizes:** `04-graphics/*.webm` were recorded at **1280×720** (VP8/16:9)
with the browser recorder. The brief asks for **1920×1080**. The assembly upscales every
feed to 1920×1080; the animated scenes carry the same 16:9 content, just at 720p native.
If true 1920×1080 captures are planned (the brief explicitly says "at 1920x1080 and record
them"), re-capture the three scenes and the footage takes at 1920×1080 so the master is
neater and the upscale stays out of the critical path.

### Sound

| Feed | Path | Role |
|---|---|---|
| Narration 15 clips | `05-audio/narration/narration-01..15.mp3` | Speaker: en-AU-NatashaNeural |
| Music cue 1 | `05-audio/music/cue-1-open-swell.mp3` | Title / open swell |
| Music cue 2 | `05-audio/music/cue-2-documentary-tone.mp3` | Documentary bed (main body) |
| Music cue 3 | `05-audio/music/cue-3-resolve.mp3` | Resolve under closing brand line |

---

## Rough cut timings (target)

```
00:00–00:12  cue-1 + title card                                       [MUSIC-1]
00:12–00:40  clip 1-2 (hook) + script lines 1–2                       narration-01/02
00:40–01:10  clip 3 + lines 3–4                                       narration-03
01:10–01:40  FOOTAGE-01 + line 4 (intro)                              narration-04
01:40–02:10  FOOTAGE-02 + line 5 (workbench)                          narration-05
02:10–02:50  FOOTAGE-04 + line 6 (mission)                            narration-06
02:50–03:35  FOOTAGE-05 + line 7 (agent execution)                    narration-07
03:35–04:10  scene-03 (evidence ladder) + line 8                      narration-08
04:10–04:50  FOOTAGE-06 + line 9 (verification)                       narration-09
04:50–05:30  FOOTAGE-03 + line 10 (index)                             narration-10
05:30–06:10  provider diagram + line 11 (provider platform)            narration-11
06:10–06:50  FOOTAGE-08 + line 12 (honest failure)                    narration-12
06:50–07:20  FOOTAGE-09 + line 13 (PR #44)                            narration-13
07:20–08:00  clip 14 + line 14 (current state)                        narration-14
08:00–08:30  brand card + line 15 (close) + cue-3 + cue-2 fade       narration-15 [MUSIC-3]
```

Discipline to preserve in the room: the music bed sits **under** narration, not over it.
Duck the music 6–9 dB whenever narration-01..15 plays, release on the silences.

---

## Master render

```bash
ffmpeg -y \
  -framerate 23.976 \
  -i production/zylcode-intro-film/04-graphics/scene-01-mission-lifecycle.webm \
  -i production/zylcode-intro-film/04-graphics/scene-02-architecture.webm \
  -i production/zylcode-intro-film/04-graphics/scene-03-evidence-ladder.webm \
  -i production/zylcode-intro-film/01-footage/take-01-workbench.webm \
  -i production/zylcode-intro-film/01-footage/take-02-agent-execution.webm \
  -i production/zylcode-intro-film/01-footage/take-03-verification-evidence.webm \
  -i production/zylcode-intro-film/05-audio/narration/narration-01.mp3 \
  -i production/zylcode-intro-film/05-audio/narration/narration-02.mp3 \
  -i production/zylcode-intro-film/05-audio/narration/narration-03.mp3 \
  -i production/zylcode-intro-film/05-audio/narration/narration-04.mp3 \
  -i production/zylcode-intro-film/05-audio/narration/narration-05.mp3 \
  -i production/zylcode-intro-film/05-audio/narration/narration-06.mp3 \
  -i production/zylcode-intro-film/05-audio/narration/narration-07.mp3 \
  -i production/zylcode-intro-film/05-audio/narration/narration-08.mp3 \
  -i production/zylcode-intro-film/05-audio/narration/narration-09.mp3 \
  -i production/zylcode-intro-film/05-audio/narration/narration-10.mp3 \
  -i production/zylcode-intro-film/05-audio/narration/narration-11.mp3 \
  -i production/zylcode-intro-film/05-audio/narration/narration-12.mp3 \
  -i production/zylcode-intro-film/05-audio/narration/narration-13.mp3 \
  -i production/zylcode-intro-film/05-audio/narration/narration-14.mp3 \
  -i production/zylcode-intro-film/05-audio/narration/narration-15.mp3 \
  -i production/zylcode-intro-film/05-audio/music/cue-1-open-swell.mp3 \
  -i production/zylcode-intro-film/05-audio/music/cue-2-documentary-tone.mp3 \
  -i production/zylcode-intro-film/05-audio/music/cue-3-resolve.mp3 \
  -filter_complex "
    # ---- converts and rescales all picture feeds to 1920x1080@23.976 ----
    [0:v]fps=23.976,scale=1920:1080:flags=lanczos,pad=1920:1080:(ow-iw)/2:(oh-ih)/2:color=black[s1];
    [1:v]fps=23.976,scale=1920:1080:flags=lanczos,pad=1920:1080:(ow-iw)/2:(oh-ih)/2:color=black[s2];
    [2:v]fps=23.976,scale=1920:1080:flags=lanczos,pad=1920:1080:(ow-iw)/2:(oh-ih)/2:color=black[s3];
    [3:v]fps=23.976,scale=1920:1080:flags=lanczos,pad=1920:1080:(ow-iw)/2:(oh-ih)/2:color=black[s4];
    [4:v]fps=23.976,scale=1920:1080:flags=lanczos,pad=1920:1080:(ow-iw)/2:(oh-ih)/2:color=black[s5];
    [5:v]fps=23.976,scale=1920:1080:flags=lanczos,pad=1920:1080:(ow-iw)/2:(oh-ih)/2:color=black[s6];
    # ---- audio ----
    [10]loudnorm=I=-16:TP=-1.5:LRA=11[na];
    [11]loudnorm=I=-16:TP=-1.5:LRA=11[nb];
    [12]loudnorm=I=-16:TP=-1.5:LRA=11[nc];
    [13]loudnorm=I=-16:TP=-1.5:LRA=11[nd];
    [14]loudnorm=I=-16:TP=-1.5:LRA=11[ne];
    [15]loudnorm=I=-16:TP=-1.5:LRA=11[nf];
    [16]loudnorm=I=-16:TP=-1.5:LRA=11[ng];
    [17]loudnorm=I=-16:TP=-1.5:LRA=11[nh];
    [18]loudnorm=I=-16:TP=-1.5:LRA=11[ni];
    [19]loudnorm=I=-16:TP=-1.5:LRA=11[nj];
    [20]loudnorm=I=-16:TP=-1.5:LRA=11[nk];
    [21]loudnorm=I=-16:TP=-1.5:LRA=11[nl];
    [22]loudnorm=I=-16:TP=-1.5:LRA=11[nm];
    [23]loudnorm=I=-16:TP=-1.5:LRA=11[nn];
    [24]loudnorm=I=-16:TP=-1.5:LRA=11[no];
    [25]loudnorm=I=-16:TP=-1.5:LRA=11[np];
    # ---- assemble a 9-minute track matching the edit above ----
    # (this must be driven by the actual in/out points of the picture cuts;
    #  the fixed concat filter below is a placeholder for the editor)
    ...
  " \
  -map "[vout]" -map "[an]" \
  -c:v libx264 -pix_fmt yuv420p -preset fast -crf 18 \
  -c:a aac -b:a 192k \
  production/zylcode-intro-film/03-deliverables/zylcode-intro-master.mp4
```

> The filter chain above is the editor's frame. A production cut uses a strict
> sequence of `-ss`/`-to` segments per source instead of the `concat` filter, so that a
> single clip is never cut mid-sentence and every narration clip starts and ends on its
> own boundary. Replace the `...` block with the targeted concat once the picture takes
> are captured.

---

## Audit after render

When the master exists, verify:

1. `ffprobe -v error -show_entries format=duration` → expect 8:00–10:00.
2. `ffprobe -v error -select_streams v:0 -show_entries stream=width,height` → 1920×1080.
3. Narration intelligibility: no clipping (`ebur128` true-peak over -1 dBTP), no dead
   sections longer than 2 s.
4. Music ducking verified by ear: the narrator is clear at every point, music bed
   never fights the voice.
5. Brand compliance: `#007BFF` primary, graphite/navy base, amber approval accent,
   no vendor logos, no claims beyond the audited capability registry.

---

## Deliverables in the film tree

| Path | Kind |
|---|---|
| `06-narration/NARRATION_SCRIPT.md` | narrated lines keyed to the filmed beats |
| `06-narration/NARRATION_ASSEMBLY.md` | this file |
| `06-narration/narration-lines.txt` | source line list |
| `05-audio/narration/narration-01..15.mp3` | rendered narration |
| `05-audio/narration/_probe.mp3`, `_probe2.mp3` | test clips (ignored at delivery) |
| `05-audio/music/cue-1-open-swell.mp3` | open swell |
| `05-audio/music/cue-2-documentary-tone.mp3` | documentary bed |
| `05-audio/music/cue-3-resolve.mp3` | resolve |
| `03-deliverables/zylcode-intro-master.mp4` | final master (pending) |
