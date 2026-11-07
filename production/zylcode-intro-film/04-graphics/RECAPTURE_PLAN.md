# Animated diagram scenes — 1920×1080 capture plan

**Status:** DRAFT. The three scenes are currently **1280×720** (VP8, 16:9) because the
browser recorder captured them at that resolution. The brief asks for **1920×1080**.
This file records the option to re-capture at true 1920×1080 so the master is not
upscaled from 720p content.

---

## What already exists (1280×720)

| Scene             | File                                          | Subject                                                       |
| ----------------- | --------------------------------------------- | ------------------------------------------------------------- |
| Mission lifecycle | `04-graphics/scene-01-mission-lifecycle.webm` | Intent → Plan → Tool calls → Approval → Verify → Evidence     |
| Architecture      | `04-graphics/scene-02-architecture.webm`      | Surface / Service / Evidence columns + flowing packets        |
| Evidence ladder   | `04-graphics/scene-03-evidence-ladder.webm`   | Recorded → Derived → Hypothesis → Unknown → Failure recovered |

## Why re-capture

- **1920×1080 native** renders: sharper text, cleaner 4K master, no upscale in the
  critical picture path, matches the brief literally.
- **1280×720 native** content can still be used in the room, upscaled at the editor's
  discretion — but the brief says record at 1920×1080, so a 1920×1080 take will be the
  primary capture.

## What needs to be done to re-capture

1. Re-open each of the three scene pages at a **1920×1080 browser viewport**.
2. Start the recorder (`preview_recording_start`) on the same tab.
3. Replay the animation through a full cycle (scene 1 ≈ 21 s, scene 3 ≈ 20 s, scene 2
   continuous packet flow).
4. Stop the recorder, extract a single 1920×1080 WebM per scene.
5. Verify dimensions with `ffprobe` (width=1920, height=1080) before treating the
   previous 1280×720 clips as backup.

## Post-capture checks

1. `ffprobe -v error -select_streams v:0 -show_entries stream=width,height` →
   `1920,1080`.
2. No brand badge visible in the recorded page content (Freebuff composites it in
   screenshots, not in recorded page content — confirmed in earlier work).
3. Re-run the cleaned poster stills from the new captures for `still-01/02/03-*.png`.

## What stays unchanged

- The HTML/CSS/JS of the three scenes: they are already 1080p stage sized
  (`#stage{width:1920px;height:1080px}`) and render correctly; only the recorder
  output resolution needs to be set to 1920×1080.
- The master assembly pipeline does not change — it already rescales every feed to
  1920×1080@23.976fps and pads to 1920×1080.
