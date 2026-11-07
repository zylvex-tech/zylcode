# Graphics scenes — recorded 2026-10-31

Built as self-contained HTML/CSS/JS at a true 1920×1080 stage, served locally on
`127.0.0.1:8099`, and captured with the browser recorder. Recorder output is 1280×720 VP8,
16:9 (verified with ffprobe on every clip) — upscale to 1080p at assembly with the app takes.

| Scene                                                                                              | Source                   | Recording                         | Poster still (1920×1080)               |
| -------------------------------------------------------------------------------------------------- | ------------------------ | --------------------------------- | -------------------------------------- |
| Mission lifecycle — intent → plan → tool calls → approval (amber) → verify → evidence              | `mission-lifecycle.html` | `scene-01-mission-lifecycle.webm` | `still-01-mission-lifecycle-clean.png` |
| Architecture — surface / service / evidence columns with flowing packets                           | `architecture.html`      | `scene-02-architecture.webm`      | `still-02-architecture-clean.png`      |
| Evidence ladder — recorded → derived → hypothesis → unknown → failure recovered, with label legend | `evidence-ladder.html`   | `scene-03-evidence-ladder.webm`   | `still-03-evidence-ladder-clean.png`   |

## Notes

- **Animation loops:** scene 1 ≈ 21 s per full cycle (2.6 s per step), scene 3 ≈ 20 s per
  cycle (2 s per rung); scene 2 runs continuous packet flow. Each clip may be trimmed to the
  narrated beat at assembly.
- **`?still=final`** renders each page in its finished state — used for the poster stills and
  available for a clean hold frame.
- **Host badge removal:** the Freebuff client composites a badge into _screenshots_ at the
  top-left (it is not in recorded page content — confirmed by inspecting a frame extracted
  from an app take). The three poster stills were cleaned with ffmpeg
  `delogo=x=4:y=4:w=112:h=46`; the badge area is empty background above the scene label, so
  nothing else was affected. The `-clean.png` files are the deliverables; the plain `.png`
  files are the raw captures.
- **Palette held to brand:** graphite/navy base, `#007BFF` primary, amber approval accent,
  technical grid, no vendor logos, no claims beyond the audited capability registry.
