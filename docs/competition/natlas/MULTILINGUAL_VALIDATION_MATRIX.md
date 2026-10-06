# MULTILINGUAL VALIDATION MATRIX

**Status:** **EMPTY. No language has been tested with a genuine N-ATLAS invocation.**

> The competition targets four languages: **Yoruba, Hausa, Igbo, and Nigerian-accented English**.
> These are **targets, not validated claims**. A language may only be reported as supported if the
> finished artefact genuinely exercises it and the evidence is captured.
>
> **Do not manipulate the evidence to preserve all four checkboxes.** If N-ATLAS performs poorly in
> a language, record that. An honest three-of-four is worth more than a fabricated four-of-four.

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

## Matrix (unpopulated)

| Language | Date/time | Model/runtime identity | Repository / task | Raw response location | Parsed intent valid? | Operation proposed | Approved? | Verification result | Latency | Observed language quality | Failure / limitations | Evidence id |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Yoruba | — | — | — | — | — | — | — | — | — | — | — | — |
| Hausa | — | — | — | — | — | — | — | — | — | — | — | — |
| Igbo | — | — | — | — | — | — | — | — | — | — | — | — |
| English (Nigerian) | — | — | — | — | — | — | — | — | — | — | — | — |

**Rows recorded: 0**

---

## Gate status

| Gate | Description | State |
|---|---|---|
| ML-1 | Yoruba tested with genuine N-ATLAS | ⛔ blocked |
| ML-2 | Hausa tested with genuine N-ATLAS | ⛔ blocked |
| ML-3 | Igbo tested with genuine N-ATLAS | ⛔ blocked |
| ML-4 | English / Nigerian-English tested and evidenced | ⛔ blocked |

All four are blocked on the same single dependency: a reachable genuine N-ATLAS runtime.

---

## Harness readiness

The toolkit is ready to run these tests the moment the runtime is up:

- the SDK propagates a `language` field into the evidence record;
- the playground has a language selector (`en-NG`, `ha`, `yo`, `ig`);
- the evidence schema records language and observed outcome.

What is missing is the model, not the harness.
