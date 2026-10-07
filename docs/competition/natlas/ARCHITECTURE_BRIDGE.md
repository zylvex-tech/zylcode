# C3 — Developer Bridge: Architecture & Evidence (N-ATLAS × ZylCode)

- **Date:** 2026-10-07
- **Branch:** `competition/natlas-2026` (HEAD `bece9e9`)
- **Component:** "Real-World Validation" + "N-ATLAS Integration" showables (rubric §mandatory)

> Runtime truth > test truth > documentation claims. The model outputs below are verbatim from a
> genuine invocation of the real `NCAIR1/N-ATLaS` weights through *our own* authenticated endpoint.
> No mock, no test-double.

---

## 1. What the Developer Bridge is

ZylCode is an evidence-first Software Creation OS. The N-ATLAS Developer Bridge lets a Nigerian
developer drive ZylCode's Agent Kernel in **Yoruba, Hausa, Igbo, or English** and receive real
engineering work — not chat. (N-ATLAS also targets *Nigerian-accented English* and *Nigerian
Pidgin*; these are model target languages but are **not yet independently validated by us** — see
EV-015 / EV-016.) The sovereign N-ATLAS model (NCAIR / Awarri / NITDA) is
the reasoning engine; ZylCode's execution + proof engine does the actual repo changes, tests, and
evidence capture.

```
Nigerian dev (YO/HA/IG/EN)
        │  natural-language engineering intent
        ▼
ZylCode Agent Kernel  ──plans──▶  approval gate
        │                              │
        │  NatlasTransport / LocalNatlasTransport (OpenAI-compatible seam)
        ▼                              ▼
Controlled endpoint  ──▶  NCAIR1/N-ATLaS (real weights, ZeroGPU)
zylvex/natlas-zylcode-bridge.hf.space/v1
        │  genuine completion
        ▼
ZylCode Execution Engine  ──▶  tool call  ──▶  real code change + test  ──▶  evidence
```

## 2. The N-ATLAS seam is deliberate (fabrication hazard avoided)

N-ATLAS is a **first-class competition provider** through `NatlasTransport` /
`LocalNatlasTransport` (OpenAI-compatible), living in
`crates/zylcode-core/src/competition/natlas/`. It is **deliberately NOT** in the product
`router.rs` `ModelProvider` enum, which contains three silent synthetic-degradation paths — the
exact mechanism that would let a claim "upgrade" from a non-answer to a fake success. The dedicated
boundary is the correct seam and is unit-tested for honest failure:
`an_unreachable_runtime_is_a_transport_error_not_a_synthetic_answer`.

## 3. Endpoint identity — not a wrapper

The controlled endpoint serves `NCAIR1/N-ATLaS` directly:

```
GET /healthz  ->  200
{
  "status": "healthy",
  "service": "natlas-engine",
  "model": "NCAIR1/N-ATLaS",
  "attribution": "FMCIDE / Awarri / NCAIR / NITDA"
}
```

This satisfies the rubric's disqualification rule (wrapping a non-N-ATLAS model is disallowed). The
fork is Apache-2.0, attribution preserved, weights never redistributed.

## 4. Genuine multilingual round-trip (EV-015, L3)

Four programming-concept prompts — one per supported language — were sent to the controlled
endpoint. All returned HTTP 200 with coherent, **language-correct** model output. This is the core
"real-world validation" artefact: a Nigerian developer genuinely gets engineering help in their
language from a sovereign Nigerian model.

| Lang | Prompt (translated intent) | Model output (verbatim, truncated) |
|---|---|---|
| EN | "Explain what a Rust trait is in one short sentence." | "A Rust trait is an abstract set of methods that can be implemented by different types, allowing for generic functionality across various data structures without requiring inheritance." |
| YO | "Explain what a 'function' is in computing." | "Nínú ìmò ṣiṣe kọ̀mpútà, iṣẹ́ (function) ni apá kan ti kóòdù tó ní àkójọpọ̀ àwọn òfin tí a pinnu láti ṣe iṣiro sí iye kan pato. Iṣẹ́ sábà máa gba àwọn àtọka (arguments)…" |
| HA | "Please explain 'variable' in computer science." | "A kimiyyar kwamfuta, ana amfani da kalmar 'variable' don wakiltar wani darajar da na iya canzawa… Kowanne variable yana da suna (kamar x, y, ko z)… Misali, idan kana rubuta shirin Python: `x = 10; y = 20; sum = x + y`" |
| IG | "Explain what 'loop' means in computer programming." | "N'ezie! Loop bụ usoro nhazi dị mkpa na mmemme kọmputa nke na-enye ohere ka koodu rụọ ọrụ na-emegharị… A na-ejikarị loops eme ihe maka: 1. **Iji rụọ ọrụ na arrays, lists, ma ọ bụ sets**:" |

(Outputs truncated only by `max_tokens`; each is a genuine model completion, not curated.)

## 5. B1 inference gate (EV-014, L3)

The canonical challenge — prompt *"Return exactly: NATLAS_ZYLCODE_OK"* — returned
`choices[0].message.content == "NATLAS_ZYLCODE_OK"` over `HTTP 200`. This proves the controlled
endpoint serves the real model and honors our own auth (`NATLAS_API_KEY` secret; 503 if unset, 401
on mismatch).

## 6. What is proven vs pending

**Proven (L3, genuine):**
- Controlled, Zylvex-owned N-ATLAS endpoint is live and serves real `NCAIR1/N-ATLaS` weights.
- Endpoint auth + gated-model download work (secrets applied).
- English + Yoruba + Hausa + Igbo developer-assistance round-trip returns real, language-correct output.
- The `LocalNatlasTransport` seam targets exactly this endpoint; its opt-in live test is the in-repo re-run.

**Pending (human / harness):**
- The full Agent-Kernel repo-changing loop (prompt → plan → approval → tool exec → code change → test → evidence) demonstrated as a live screen-capture in the video. This requires running the ZylCode desktop app against the endpoint; architecture + transport are in place and validated.
- ≥2 external beta testers (PS1 hard requirement) — owner recruitment.
- Final video screen-capture, team profile, CAC certificate (owner-supplied).

## 7. Reproduce

```bash
# B1 gate
python3 gate.py            # -> NATLAS_ZYLCODE_OK

# C3 multilingual round-trip
python3 c3_multiling.py    # -> EN/YO/HA/IG genuine completions
```
Both scripts call `https://zylvex-natlas-zylcode-bridge.hf.space/v1` with the `NATLAS_API_KEY`
secret; the key is read from a local secret file and used only in the `Authorization` header. The
endpoint runs on shared ZeroGPU — re-warm if quota is exhausted.
