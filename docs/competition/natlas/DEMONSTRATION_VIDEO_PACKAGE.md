# DEMONSTRATION VIDEO — PRODUCTION PACKAGE

**Product:** N-ATLAS × ZylCode Developer Bridge
**Challenge:** NAIC 2026 — PS1 Developer Infrastructure (Track B)
**Target length:** 4–5 minutes
**Date prepared:** 2026-10-10

> ## STATUS: NOT RECORDED · NOT UPLOADED
>
> No video has been recorded and no video has been uploaded. This document is a
> **production package** — a script, a shot list, and a checklist. It does not claim that
> a video exists. When the recording is done, the status line above is the single place
> that changes.

---

## 1. Deliverable specification

| Field | Requirement |
|---|---|
| Length | 3–5 minutes (target 4:30) |
| Resolution | 1920×1080 minimum |
| Frame rate | 30 or 60 fps |
| Audio | Clear narration; no music over speech peaks |
| Format | MP4 (H.264 + AAC) |
| Content rule | Every demo segment is a **real** capture. No fabricated screens. |

---

## 2. Narration script (full, timed)

Total target: **4:30**. Timings are cumulative.

| Time | On screen | Narration (read verbatim or paraphrase) |
|---|---|---|
| 0:00–0:15 | Title card: *"N-ATLAS × ZylCode — A Sovereign Developer Bridge"* | "Nigeria built the model. Nigeria built the operating system. This is where they meet." |
| 0:15–0:40 | Nigeria map → NCAIR / Awarri mark → Zylvex mark | "N-ATLAS is a sovereign multilingual model from NCAIR and Awarri, under the Federal Ministry of Communications, Innovation and Digital Economy. ZylCode is an evidence-first engineering operating system from Zylvex Technologies." |
| 0:40–1:05 | Split screen: greyed English-only tooling vs ZylCode lit | "Most AI developer tooling assumes English and a foreign model. We assume neither." |
| 1:05–1:35 | Animated architecture diagram | "Here is the bridge. An agent kernel, a deliberate transport seam, and a controlled endpoint serving the genuine N-ATLAS weights. The seam is deliberate: N-ATLAS is not routed through the product's general model provider, which has silent degradation paths. A competition that disqualifies wrappers deserves a boundary that cannot silently fall back." |
| 1:35–2:00 | Terminal: `curl /healthz` → `model: NCAIR1/N-ATLaS` | "The endpoint reports the real model, the real engine, and the real hardware — a shared ZeroGPU pool." |
| 2:00–2:30 | Terminal: genuine chat call returning an answer | "This is a live inference. Not a mock, not a test double. The model answers." |
| 2:30–3:10 | Terminal: multilingual prompts | "Yoruba, Hausa, Igbo, Nigerian English. Engineering help in the language the developer actually thinks in." |
| 3:10–4:00 | Desktop app: prompt → plan → approval gate → file written → test run → evidence panel | "And it does the work. The instruction becomes a plan. The plan stops at a human approval gate. On approval, a real file is written, a real test runs, and the whole chain is recorded as tamper-evident evidence." |
| 4:00–4:20 | Licence + attribution card | "Apache-2.0. Attribution preserved. Model weights never redistributed. Every claim in this video is reproducible from the repository." |
| 4:20–4:30 | Closing card: *"Engineered in Nigeria. Built for the world."* | "Engineered in Nigeria. Built for the world." |

---

## 3. Recording sequence

Record in this order. Each step names what to capture and what must be true first.

| Step | Segment | Action | Precondition |
|---|---|---|---|
| R1 | Title + map + logos | Insert animated cards | — |
| R2 | Architecture diagram | Screen-record the diagram (or animate it) | `ARCHITECTURE_BRIDGE.md` §1 open |
| R3 | Endpoint identity | Terminal: `curl -s .../healthz` | Endpoint warm — see §4 |
| R4 | Genuine inference | Terminal: one real chat call | Quota available |
| R5 | Multilingual | Terminal: prompts in YO / HA / IG / EN-NG | Quota available; may need several attempts |
| R6 | Agent kernel loop | Desktop app: prompt → approve → file + test → evidence | App running against the endpoint |
| R7 | Licence card + close | Insert cards | — |
| R8 | Narration | Record voiceover against the picture lock | — |
| R9 | Assembly | Edit, colour, level audio, export MP4 | — |

---

## 4. The quota constraint — read before recording

**ZeroGPU is a shared, quota-metered pool.** On 2026-10-10 a single successful inference
was followed by failures on every subsequent call (EV-028). This directly threatens the
recording session.

**Mitigations, in order of preference:**

1. **Warm the endpoint first.** Poll `/healthz` until it returns 200 before recording.
2. **Record the demo segments in one short window**, immediately after a successful probe,
   rather than across a long session.
3. **Record R3 (identity) separately** — it needs no quota.
4. **If quota is spent mid-session:** stop. Do **not** substitute a mocked response. Either
   resume later or use the documented fallback in `LOCAL_RUNTIME_FALLBACK.md` — and label
   any non-N-ATLAS segment clearly on screen.
5. **Never** present a synthetic or test-double response as a genuine one.

> A video that shows a real error and explains it honestly is worth more than a video that
> shows a fabricated success. The competition explicitly disqualifies wrappers.

---

## 5. Screen-capture checklist

Before hitting record:

- [ ] Endpoint `/healthz` returns 200 (`NCAIR1/N-ATLaS`).
- [ ] `NATLAS_API_KEY` is **not visible** in any terminal, file, or window.
- [ ] No other API keys, tokens, or `.env` contents are on screen.
- [ ] Terminal font is large enough to read at 1080p.
- [ ] Notifications silenced (Do Not Disturb on).
- [ ] Desktop wallpaper is neutral / branded, not personal.
- [ ] Browser bookmarks bar hidden.
- [ ] Any tester PII is off screen.
- [ ] Clock/region in the corner is acceptable to show.

After recording:

- [ ] Verify no secrets appear in any frame (scrub frame-by-frame at 2× speed).
- [ ] Verify the model output shown is the real captured output.
- [ ] Verify the architecture diagram is labelled "illustrative" if animated.
- [ ] Verify total length is within 3–5 minutes.

---

## 6. Assets required

| Asset | Source | Status |
|---|---|---|
| Title card | to be produced | ⬜ not produced |
| Nigeria map + partner marks | to be produced (respect brand guidelines) | ⬜ not produced |
| Architecture diagram | `ARCHITECTURE_BRIDGE.md` §1 | ✅ source exists |
| Endpoint capture | live terminal | ⬜ not recorded |
| Inference capture | live terminal | ⬜ not recorded |
| Multilingual capture | live terminal | ⬜ not recorded |
| Agent-kernel capture | desktop app | ⬜ not recorded |
| Licence/attribution card | to be produced | ⬜ not produced |
| Narration audio | to be recorded | ⬜ not recorded |
| Background music | licensed or royalty-free | ⬜ not selected |

---

## 7. Evidence references used by the script

| Claim in video | Evidence id | Where |
|---|---|---|
| Endpoint serves `NCAIR1/N-ATLaS` | EV-028, EV-000 | `evidence/EV-028-...json`, `/healthz` |
| Genuine inference, no mock | EV-014, EV-024 | `NATLAS_EVIDENCE_INDEX.md` |
| Multilingual replies | EV-015 | `MULTILINGUAL_VALIDATION_MATRIX.md` |
| Repo-changing journey | EV-024 | `C3_LIVE_RUNTIME_VERIFICATION_REPORT.md` |
| Bounded repair layer | EV-023, EV-017 | `ARCHITECTURE_BRIDGE.md` |
| Competition tests green | EV-027 | `evidence/EV-027-...log` |

---

## 8. What must NOT be claimed in the video

| Do not say | Why |
|---|---|
| "Four languages fully validated" | Multilingual validation is **PARTIAL**; prompts were team-authored. |
| "Pidgin supported" | Pidgin generation is untested; the model replied in English. |
| "99.9% uptime" / throughput figures | No such measurement exists. |
| "Production-ready" | Nothing is production-approved. |
| "Always available" | The endpoint is quota-metered and has failed live. |
| Any fabricated screen | Disqualification risk. |

*End of production package.*
