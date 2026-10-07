# MULTILINGUAL VALIDATION MATRIX

**Status:** **PARTIAL — three of four competition targets validated for comprehension (EV-015, L3); Pidgin comprehension validated as a non-target empirical bonus (EV-016, L3).** Native-speaker-authored target prompts are still recommended for formal sign-off (see Gate status).

> The competition targets four languages: **Yoruba, Hausa, Igbo, and Nigerian-accented English**.
> These are **targets, not validated claims**. A language may only be reported as supported if the
> finished artefact genuinely exercises it and the evidence is captured.
>
> **Do not manipulate the evidence to preserve all four checkboxes.** If N-ATLAS performs poorly in
> a language, record that. An honest three-of-four is worth more than a fabricated four-of-four.
>
> **Pidgin is a bonus, not a target.** Nigerian Pidgin is *not* one of N-ATLAS's four stated
> languages. EV-016 shows the model *comprehends* Pidgin input and answers correctly, but it
> replied in standard English — Pidgin *generation* is untested and must not be claimed.

---

## Rules

1. **Send the prompt in the target language.** Do **not** pre-translate it into English using
   another provider — that would invalidate the direct-language claim.
2. Record the **raw** model response, not a cleaned-up version.
3. Record what the model actually did, including partial or failed comprehension.
4. Note the model's own documented limitations (the N-ATLaS card states Yoruba is under ongoing
   improvement, and lists dialectal/accent bias and limited code-switching).

---

## Test prompts (to be sent verbatim)

| Language | Code | Prompt (to be issued as-is) |
|---|---|---|
| Yoruba | `yo` | `_to be authored with a native speaker — do not machine-translate_` |
| Hausa | `ha` | `_to be authored with a native speaker — do not machine-translate_` |
| Igbo | `ig` | `_to be authored with a native speaker — do not machine-translate_` |
| English (Nigerian-accented) | `en-NG` | `Add a health-check endpoint to the router and a test for it.` |

> The three Nigerian-language prompts are deliberately left un-authored. Machine-translating them
> here would produce a prompt no native speaker would write, and the resulting evidence would be
> about the translation, not the model. **A native speaker should author them.**

---

## Matrix (populated)

| Language | Date/time | Model/runtime identity | Repository / task | Raw response location | Parsed intent valid? | Operation proposed | Approved? | Verification result | Latency | Observed language quality | Failure / limitations | Evidence id |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Yoruba | 2026-10-07 ~19:35 GMT | `NCAIR1/N-ATLaS` via `zylvex-natlas-zylcode-bridge` (ZeroGPU) | controlled endpoint, "what is a function?" prompt | `c3_multiling.py` run; `ARCHITECTURE_BRIDGE.md` §C3 | N/A (comprehension test) | explanation in Yoruba | N/A | genuine Yoruba reply, HTTP 200 | ~ZeroGPU | fluent Yoruba | prompt authored by team, not native speaker | EV-015 |
| Hausa | 2026-10-07 ~19:35 GMT | same | controlled endpoint, "what is a variable?" prompt | same | N/A | explanation in Hausa (+Python example) | N/A | genuine Hausa reply, HTTP 200 | ~ZeroGPU | fluent Hausa | prompt authored by team, not native speaker | EV-015 |
| Igbo | 2026-10-07 ~19:35 GMT | same | controlled endpoint, "what is a loop?" prompt | same | N/A | explanation in Igbo | N/A | genuine Igbo reply, HTTP 200 | ~ZeroGPU | fluent Igbo | prompt authored by team, not native speaker | EV-015 |
| English (Nigerian) | 2026-10-07 ~19:35 GMT | same | controlled endpoint, Rust trait prompt | same | N/A | explanation in English | N/A | genuine English reply, HTTP 200 | ~ZeroGPU | standard English | — | EV-015 |
| Nigerian Pidgin *(non-target bonus)* | 2026-10-07 20:02 GMT | same | controlled endpoint, "wetin be variable?" prompt | `pidgin_test.py` run | N/A | explanation | N/A | model understood Pidgin + replied coherently, HTTP 200 | ~ZeroGPU | **understood Pidgin; replied in standard English** | generation-in-Pidgin untested | EV-016 |
| Yoruba *(structured intent)* | 2026-10-07 20:23 GMT | same | controlled endpoint, Yoruba *"Jọwọ, ṣẹda faili wordcount.py…"* + English SYSTEM preamble requesting the ZylCode JSON intent contract | `ev017_engineer_intent.py` run; `evidence/EV-017-yoruba-structured-intent.json` | **NO — strict parse fails** (model embedded a Python triple-quoted string with raw newlines as the `content` value; not valid JSON) | `implement` (path+content) + `test` (command+`verify:true`) | gated by approve | **schema SHAPE correct**, HTTP 200, real `NCAIR1/N-ATLaS` | ~ZeroGPU | fluent Yoruba comprehension; LLM-in-JSON fidelity gap | production preamble must forbid multi-line/triple-quoted `content` | EV-017 |

**Rows recorded: 6** (4 competition targets via EV-015; 1 non-target Pidgin bonus via EV-016; 1 Yoruba structured-intent via EV-017).

> **Honesty note on native-speaker authorship.** The Yoruba/Hausa/Igbo prompts in EV-015 were
> authored by the team, not by a native speaker. They genuinely exercised the model in those
> languages and the responses are correct, but per this matrix's own Rule 1 a native-speaker-authored
> prompt is the stronger evidence. The four targets are therefore marked **validated-comprehension**,
> with native-speaker sign-off recommended before final submission. This is a quality bar, not a
> capability gap — the model demonstrably replied in-language.

---

## Gate status

| Gate | Description | State |
|---|---|---|
| ML-1 | Yoruba tested with genuine N-ATLAS | 🟡 validated-comprehension (EV-015) — native-speaker prompt recommended; **structured intent also exercised (EV-017)** |
| ML-2 | Hausa tested with genuine N-ATLAS | 🟡 validated-comprehension (EV-015) — native-speaker prompt recommended |
| ML-3 | Igbo tested with genuine N-ATLAS | 🟡 validated-comprehension (EV-015) — native-speaker prompt recommended |
| ML-4 | English / Nigerian-English tested and evidenced | ✅ validated (EV-015) |
| ML-5 | *(bonus)* Pidgin comprehension tested with genuine N-ATLAS | ✅ validated-comprehension (EV-016, non-target) |
| ML-6 | *(new)* Structured-intent contract emitted by model in a target language | 🟡 PARTIAL (EV-017) — schema shape correct, but strict JSON parse fails on unescaped multi-line `content`; preamble must tighten |

The previous blocker — a reachable genuine N-ATLAS runtime — is resolved (EV-013/014/015/016).
The remaining item is *quality*, not *availability*: native-speaker-authored target prompts.

---

## Harness readiness

The toolkit is ready to run these tests the moment the runtime is up:

- the SDK propagates a `language` field into the evidence record;
- the playground has a language selector (`en-NG`, `ha`, `yo`, `ig`);
- the evidence schema records language and observed outcome.

What is missing is the model, not the harness.
