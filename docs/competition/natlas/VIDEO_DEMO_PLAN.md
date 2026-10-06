# VIDEO DEMO PLAN

**Purpose:** plan the 3–5 minute competition demonstration video.

**Status:** **PLAN ONLY. No video has been recorded.** It cannot be recorded until a genuine
N-ATLAS invocation exists — a video that narrates a workflow without demonstrating it would
violate the directive.

---

## 1. Principle

The video must **demonstrate**, not narrate. Every claim on screen must be backed by something
visible happening: a real runtime identity, a real response, a real approval, a real exit code, a
real evidence record.

**If the runtime is not working on the day of recording, do not record a fake.** Record what works
and state what does not.

---

## 2. Timing

| Time | Section | On screen |
|---|---|---|
| 0:00–0:25 | **The problem** | A Nigerian developer, a local-language request, the friction of existing tooling. Keep it concrete. |
| 0:25–0:50 | **The proposition** | ZylCode × N-ATLAS Developer Bridge, one diagram. |
| 0:50–1:15 | **Setup and runtime identity** | `ollama list`, `curl /api/tags` showing the model, the playground showing **server-reported** model identity. |
| 1:15–2:45 | **The real workflow** | The bounded slice end to end: instruction → genuine response → structured intent → proposed operation → **approval gate** → execution → verification with the actual exit code. |
| 2:45–3:20 | **Multilingual** | The same workflow in Yoruba / Hausa / Igbo, showing actual model behaviour — **including any weakness**. |
| 3:20–3:50 | **Verification and evidence** | The evidence record: timestamp, model identity, latency, request id, redaction count. |
| 3:50–4:20 | **External beta** | Real tester feedback (only if real testers exist). |
| 4:20–4:45 | **Impact and scaling** | What this unlocks for Nigerian developer infrastructure. |

Adjust to actual results. **Cut any section for which there is no genuine evidence** rather than
filling it with narration.

---

## 3. Non-negotiables

1. The model identity shown must be what the **server** reported.
2. The `TEST DOUBLE` badge must never appear in the final video (and if demo mode is used at all,
   it must be labelled on screen).
3. Any failure shown must be a **real** failure, presented as such.
4. No fabricated benchmarks, no invented tester quotes.
5. If a multilingual demo performs poorly, show it. That is a finding, not an embarrassment.

---

## 4. Pre-recording checklist

- [ ] A genuine N-ATLAS invocation has succeeded and its evidence is captured.
- [ ] The runtime is reachable and stable.
- [ ] The repository and task are small and the run is repeatable.
- [ ] The evidence file is ready to display.
- [ ] Real tester feedback exists, or the beta section is cut.
- [ ] A fallback plan exists if the runtime fails mid-recording: **state it, do not fake it**.

---

## 5. What is not ready

| Item | Status |
|---|---|
| Genuine invocation to demonstrate | ⛔ `BLOCKED_NATLAS_ACCESS` |
| Multilingual results to show | ⛔ none measured |
| Real tester feedback | ⛔ zero testers |
| Video recording | ⛔ not started |
