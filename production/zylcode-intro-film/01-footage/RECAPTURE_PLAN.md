# Footage takes — re-capture plan for the brief

**Status:** DRAFT. The film brief asks for the footage recorded at **1920×1080**.
Current takes in `01-footage/` are the app's own browser-recorder captures, whose
native resolution must be confirmed before the assembly treats them as primary.

---

## Current state

| File                                 | Directory     | Subject                                                                                               | Native resolution |
| ------------------------------------ | ------------- | ----------------------------------------------------------------------------------------------------- | ----------------- |
| `take-01-workbench.webm`             | `01-footage/` | Product opening (header, milestone label, workbench, version badge `v0.2.3`)                          | to verify         |
| `take-02-agent-execution.webm`       | `01-footage/` | Hero build: plan → tool calls → real failure → repair → re-run                                        | to verify         |
| `take-03-verification-evidence.webm` | `01-footage/` | Final mission report: proven 2 / derived 5 / hypotheses 1 / unknowns 1, ledger+graph+claims integrity | to verify         |

## To record at true 1920×1080

1. Open the ZylCode app (browser preview) at a **1920×1080 viewport**.
2. Start the recorder (`preview_recording_start`).
3. Capture the relevant scene (see the row above for what each take shows).
4. Stop the recorder, keep the file if it is 1920×1080, verify with `ffprobe`.
5. If the recorder still returns 1280×720, use the 1280×720 file in the room and note
   the limitation in the delivery manifest — do not claim a 1080p take it never made.

## Things to verify before the master cut

- All picture feeds are 1920×1080 native, or the assembly upscales each feed at its
  known cost in sharpness.
- The narration clips (15 × `narration-NN.mp3`) line up to the cut points in
  `06-narration/NARRATION_SCRIPT.md`.
- Music beds duck under narration at every line, release on silence.
- Brand palette holds: `#007BFF` primary, graphite/navy base, amber approval accent,
  no vendor logos, no claims beyond the audited capability registry.

## Cleanup of intermediate test files

- `05-audio/narration/_probe.mp3`, `_probe2.mp3` — test renders, not deliverables.
- `05-audio/narration/natasha-test-01.mp3` — narrator audition, not a final take.
- `04-graphics/still-01-mission-lifecycle.png` (raw capture, no badge) — superseded by
  the cleaned `-clean.png` stills.
