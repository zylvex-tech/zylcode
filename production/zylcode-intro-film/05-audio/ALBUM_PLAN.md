# ZylCode film — audio album plan

**Status:** DRAFT. Sound: narrator `en-AU-NatashaNeural` (female, Australian English, Natural Neural).
Mixed down at 48 kHz / 16-bit stereo. Music beds are placeholder textures generated locally
(from `anoisesrc` + parametric EQ/phaser/flanger/afade) for the editor; they are to be swapped
by a sound designer for the final master.

---

## Cue sheet

| Cue | File                                        | Place                       | Palette                                                        | Length  |
| --- | ------------------------------------------- | --------------------------- | -------------------------------------------------------------- | ------- |
| c1  | `05-audio/music/cue-1-open-swell.mp3`       | Title / 00:00–00:12         | Low graphite & navy pulse under a single rising bell           | 18 s    |
| n1  | `05-audio/narration/narration-01.mp3`       | 00:12–00:15                 | "An assistant can suggest code."                               | 2.86 s  |
| n2  | `05-audio/narration/narration-02.mp3`       | 00:15–00:18                 | "A platform has to show its work."                             | 3.00 s  |
| n3  | `05-audio/narration/narration-03.mp3`       | 00:18–00:40                 | "Most AI coding tools answer a question…"                      | 16.87 s |
| n4  | `05-audio/narration/narration-04.mp3`       | 00:40–01:10                 | "ZylCode. Local-first. Bring your own key…"                    | 14.90 s |
| n5  | `05-audio/narration/narration-05.mp3`       | 01:10–01:40                 | "The workbench is the surface of a real engineering workflow…" | 21.86 s |
| n6  | `05-audio/narration/narration-06.mp3`       | 02:10–02:50                 | "A mission is a plan before it is a diff…"                     | 18.31 s |
| n7  | `05-audio/narration/narration-07.mp3`       | 02:50–03:35                 | "The agent executes. It gathers context…"                      | 20.98 s |
| n8  | `05-audio/narration/narration-08.mp3`       | 03:35–04:10                 | "Here is how claims are graded…"                               | 29.06 s |
| n9  | `05-audio/narration/narration-09.mp3`       | 04:10–04:50                 | "Verification is a gate, not a suggestion…"                    | 21.89 s |
| n10 | `05-audio/narration/narration-10.mp3`       | 04:50–05:30                 | "On the repository side, the index is the evidence…"           | 22.73 s |
| n11 | `05-audio/narration/narration-11.mp3`       | 05:30–06:10                 | "The provider layer is a contracted integration…"              | 24.31 s |
| n12 | `05-audio/narration/narration-12.mp3`       | 06:10–06:50                 | "Nothing here hides a failure…"                                | 19.37 s |
| n13 | `05-audio/narration/narration-13.mp3`       | 06:50–07:20                 | "Human approval is part of the record. PR 44…"                 | 22.78 s |
| n14 | `05-audio/narration/narration-14.mp3`       | 07:20–08:00                 | "What works is measured…"                                      | 20.02 s |
| n15 | `05-audio/narration/narration-15.mp3`       | 08:00–08:30                 | "Local-first. Bring your own key. Show your work…"             | 15.10 s |
| c2  | `05-audio/music/cue-2-documentary-tone.mp3` | Documentary bed 00:18–07:55 | Warm, steady, unobtrusive                                      | 54 s    |
| c3  | `05-audio/music/cue-3-resolve.mp3`          | Resolve 08:00–08:30         | Warmer, resolving under brand line                             | 18 s    |

---

## Mix rules (for the sound designer)

1. **Narration always wins.** Set the narration bus to -3 dBFS RMS target, true peak -1.5 dBTP.
2. **Music ducks on speech.** Sidechain the music bus to the narration; 6–9 dB duck when
   narration plays, release fully on the <2 s silences.
3. **Cue 1 is a header.** Fades in at 0.0 and out under the first narration line; never
   carries past 00:15.
4. **Cue 2 carries the body.** Swells under 00:18 and holds to ~07:55; ducks hard for the
   PR #44 line and the closing line.
5. **Cue 3 resolves the close.** Over the brand card at 08:00; drop to silence by 08:30.

---

## Music provenance

The three cue beds are synthesized locally with ffmpeg filters (anoisesrc + highpass /
lowpass / parametric EQ / phaser / flanger / afade). No copyrighted music was used and
none is required. They are placeholders: replace with a composed score in the same
graphite/navy / amber palette before the master is locked.

---

## Format notes

- Narration files: edge-tts `en-AU-NatashaNeural`, 128 kbps mono MP3.
- Music beds: 48 kHz / 16-bit stereo placeholder.
- Final mix: 48 kHz / 16-bit stereo AAC 192 kbps (or PCM for the master).
