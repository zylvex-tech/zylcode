# C3 — Developer Bridge: Architecture & Evidence (N-ATLAS × ZylCode)

- **Date:** 2026-10-08
- **Branch:** `competition/natlas-2026`
- **Component:** "Real-World Validation" + "N-ATLAS Integration" showables (rubric §mandatory)

> Runtime truth > test truth > documentation claims. The model outputs below are verbatim from a
> genuine invocation of the real `NCAIR1/N-ATLaS` weights through *our own* authenticated endpoint.
> No mock, no test-double.

---

## 1. What the Developer Bridge is

ZylCode is an evidence-first Software Creation OS. The N-ATLAS Developer Bridge lets a Nigerian
developer drive ZylCode's Agent Kernel in **Yoruba, Hausa, Igbo, or English** and receive real
engineering work — not chat. (Nigerian Pidgin comprehension is validated as a non-target bonus;
see EV-016.) The sovereign N-ATLAS model (NCAIR / Awarri / NITDA) is
the reasoning engine; ZylCode's execution + proof engine does the actual repo changes, tests, and
evidence capture.

```
Human instruction (YO/HA/IG/EN)
        │  natural-language engineering intent
        ▼
Strengthened SYSTEM Preamble  ──▶  JSON contract
        │
        │  NatlasTransport / LocalNatlasTransport (OpenAI-compatible seam)
        ▼
Controlled endpoint  ──▶  NCAIR1/N-ATLaS (real weights, ZeroGPU)
zylvex-natlas-zylcode-bridge.hf.space/v1
        │  genuine completion
        ▼
Bounded JSON Repair  ──▶  NatlasEngineeringIntent::parse_with_repair
        │
        ▼
TaskGraph (approval-gated DAG)  ──▶  Human Approval  ──▶  FactoryRunner
        │                                                            │
        ▼                                                            ▼
WriteFile / RunCommand / Verify  ──▶  real code change + test  ──▶  evidence
```

---

## 2. The N-ATLAS seam is deliberate (fabrication hazard avoided)

N-ATLAS is a **first-class competition provider** through `NatlasTransport` /
`LocalNatlasTransport` (OpenAI-compatible), living in
`crates/zylcode-core/src/competition/natlas/`. It is **deliberately NOT** in the product
`router.rs` `ModelProvider` enum, which contains three silent synthetic-degradation paths — the
exact mechanism that would let a claim "upgrade" from a non-answer to a fake success.

**Why this matters for the challenge:** The rubric's disqualification rule states *"Submissions
that wrap general-purpose models instead of N-ATLAS will be disqualified."* The dedicated boundary
prevents any accidental substitution. Even if the endpoint is unreachable, `NatlasResilienceState`
returns an honest error — never a synthetic completion from another provider.

Unit-tested for honest failure:
- `an_unreachable_runtime_is_a_transport_error_not_a_synthetic_answer`
- `endpoint_unavailable_is_blocked_with_no_synthetic_response`
- `auth_failure_is_stated_not_substituted`

---

## 3. Endpoint identity — not a wrapper

The controlled endpoint serves `NCAIR1/N-ATLaS` directly:

```
GET /healthz  ->  200
{
  "status": "healthy",
  "service": "natlas-engine",
  "model": "NCAIR1/N-ATLaS",
  "attribution": "N-ATLaS is an initiative of the Federal Ministry of Communications, Innovation and Digital Economy, and powered by Awarri Technologies."
}
```

This satisfies the rubric's disqualification rule. The fork is Apache-2.0, attribution preserved,
weights never redistributed.

---

## 4. Bounded JSON Repair Layer

N-ATLAS is a fine-tuned Llama-3 8B model. Like all LLMs, it occasionally emits JSON with
serialization defects — triple-quoted strings (`"""..."""`), raw newlines inside string values,
and trailing commas. Rather than silently accepting broken JSON or giving up, ZylCode uses a
**bounded, deterministic repair layer**:

| Defect | Repair | Scope |
|---|---|---|
| Triple-quoted strings | Escape inner content (`"""..."""` → `"..."`) | In scope |
| Raw newlines in strings | Escape to `\n` | In scope |
| Trailing commas | Remove `,]` / `,}` | In scope |
| Unescaped quotes mid-string | **Out of scope** — fail closed | Not repaired |
| Missing fields | **Out of scope** — fail closed | Not invented |

`parse_with_repair()` returns `(NatlasEngineeringIntent, was_repaired: bool)`. The caller always
knows whether normalization occurred. This is tested in `intent.rs` (18 tests) and the bridge
suite (`triple_quoted_model_output_is_repaired_and_parsed`).

**Honesty note:** The model still emits triple-quoted strings despite the strengthened SYSTEM
preamble (rev 2). The repair layer recovers deterministically — but the defect is documented, not
hidden.

---

## 5. The Full Pipeline — Verified Current Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  HUMAN SIDE                                                                  │
│  User instruction (Yoruba / Hausa / Igbo / English)                         │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  SYSTEM PREAMBLE (rev 2)                                                     │
│  6 JSON serialization rules + compact valid example                         │
│  - NEVER use triple-quoted strings                                          │
│  - Embedded newlines MUST be \n                                             │
│  - The 'content' field must contain valid Python code in English            │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  NATLAS TRANSPORT                                                            │
│  LocalNatlasTransport → POST /v1/chat/completions                          │
│  Dedicated boundary trait — NOT in ModelProvider enum                       │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  CONTROLLED ENDPOINT                                                         │
│  zylvex-natlas-zylcode-bridge.hf.space (ZeroGPU, Apache-2.0 fork)           │
│  Serves genuine NCAIR1/N-ATLaS gated weights                                │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  BOUNDARY LAYER                                                              │
│  repair_json() → deterministic, narrowly scoped                             │
│  parse_with_repair() → (intent, was_repaired)                               │
│  NatlasResilienceState → no fallback state                                  │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  ZYLCODE SIDE                                                                │
│  NatlasEngineeringIntent → to_task_graph()                                  │
│  TaskGraph → approval-gated DAG (structural dependency)                     │
│  approval_step_id() → computed upfront, proven by backward edge             │
│  FactoryRunner → WriteFile / RunCommand / Verify                            │
│  Evidence Ledger → hash-chained JSONL (INTENT → approval → mutation → test) │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 6. Genuine Live Acceptance Journey (EV-024, L3)

On 2026-10-07, the full uninterrupted chain was executed from a Yoruba instruction through real
N-ATLAS to a verified file mutation and passing test:

| Step | What Happened | Evidence |
|---|---|---|
| 1. User Instruction | Yoruba: *"Jọwọ, ṣẹda faili Python kékeré…"* | EV-024 |
| 2. Endpoint Health | `GET /healthz` → 200, model `NCAIR1/N-ATLaS` | EV-024 |
| 3. Real Inference | `POST /v1/chat/completions` → 200, real model output | EV-024 |
| 4. Bounded Repair | Triple-quoted string with raw newlines → escaped JSON | EV-023, EV-024 |
| 5. Strict Parse | `parse_with_repair` → valid intent, `was_repaired=true` | EV-024 |
| 6. TaskGraph | `to_task_graph()` → approval-gated DAG | Hermetic tests |
| 7. Pre-Approval | File does NOT exist; runner `AwaitingApproval` | EV-025 |
| 8. Human Approval | `runner.approve("step-01", "owner")` → gate opens | EV-024 |
| 9. File Mutation | `helloworld.py` written with `print('Hello ZylCode')` | EV-024 |
| 10. Test Execution | `python helloworld.py` → exit 0, stdout `Hello ZylCode` | EV-024 |
| 11. Verified Claim | Claim status `Verified` recorded | EV-024 |
| 12. Provenance | 8-node chain: INTENT → approval → mutation → test → claim | EV-026 |

**No simulated model response. No synthetic fallback. No fabricated data.**

---

## 7. Genuine Multilingual Round-Trip (EV-015, L3)

Four programming-concept prompts — one per competition target language — were sent to the controlled
endpoint. All returned HTTP 200 with coherent, **language-correct** model output.

| Lang | Prompt (translated intent) | Model output (verbatim, truncated) |
|---|---|---|
| EN | "Explain what a Rust trait is in one short sentence." | "A Rust trait is an abstract set of methods that can be implemented by different types, allowing for generic functionality across various data structures without requiring inheritance." |
| YO | "Explain what a 'function' is in computing." | "Nínú ìmò ṣiṣe kọ̀mpútà, iṣẹ́ (function) ni apá kan ti kóòdù tó ní àkójọpọ̀ àwọn òfin tí a pinnu láti ṣe iṣiro sí iye kan pato…" |
| HA | "Please explain 'variable' in computer science." | "A kimiyyar kwamfuta, ana amfani da kalmar 'variable' don wakiltar wani darajar da na iya canzawa… Misali, idan kana rubuta shirin Python: `x = 10; y = 20; sum = x + y`" |
| IG | "Explain what 'loop' means in computer programming." | "N'ezie! Loop bụ usoro nhazi dị mkpa na mmemme kọmputa nke na-enye ohere ka koodu rụọ ọrụ na-emegharị…" |

(Outputs truncated only by `max_tokens`; each is a genuine model completion, not curated.)

**Honesty note:** The Yoruba, Hausa, and Igbo prompts were team-authored, not native-speaker-authored.
The responses are correct, but native-speaker sign-off is recommended for final submission.

---

## 8. B1 Inference Gate (EV-014, L3)

The canonical challenge — prompt *"Return exactly: NATLAS_ZYLCODE_OK"* — returned
`choices[0].message.content == "NATLAS_ZYLCODE_OK"` over `HTTP 200`. This proves the controlled
endpoint serves the real model and honors our own auth (`NATLAS_API_KEY` secret; 503 if unset, 401
on mismatch).

---

## 9. What is proven vs pending

**Proven (L3, genuine):**
- Controlled, Zylvex-owned N-ATLAS endpoint is live and serves real `NCAIR1/N-ATLaS` weights.
- Endpoint auth + gated-model download work (secrets applied).
- English + Yoruba + Hausa + Igbo developer-assistance round-trip returns real, language-correct output.
- **Full live acceptance journey:** Yoruba instruction → real N-ATLAS → bounded repair → file mutation (`helloworld.py`) → test pass (`python helloworld.py` → exit 0, stdout "Hello ZylCode") → evidence (EV-024).
- The `LocalNatlasTransport` seam targets exactly this endpoint; its opt-in live test is the in-repo re-run.
- Bridge mechanics proven hermetically: 61 tests green, no live endpoint required.

**Pending (human / owner action):**
- ≥2 external beta testers (PS1 hard requirement) — owner recruitment.
- Final video screen-capture, team profile, CAC certificate (owner-supplied).
- Native-speaker-authored prompts for Hausa and Igbo (quality bar, not capability gap).

---

## 10. Reproduce

### Hermetic (no credentials, no network)

```bash
# Clone the competition branch
git clone --branch competition/natlas-2026 https://github.com/zylvex-tech/zylcode.git
cd zylcode

# Run all competition test suites
cargo test -p zylcode-core --lib competition::natlas::intent    # 18 tests
cargo test -p zylcode-core --test natlas_boundary                # 17 tests
cargo test -p zylcode-core --test natlas_runtime                 # 12 tests
cargo test -p zylcode-core --test natlas_bridge                  # 14 tests
```

### Live endpoint verification (no key needed)

```bash
curl -s https://zylvex-natlas-zylcode-bridge.hf.space/healthz
# Expected: {"status":"healthy","model":"NCAIR1/N-ATLaS",...}
```

### Live acceptance journey (requires `NATLAS_API_KEY`)

```bash
export NATLAS_URL=https://zylvex-natlas-zylcode-bridge.hf.space/v1
export NATLAS_API_KEY=<your-key>
cargo test -p zylcode-core --test natlas_live -- --ignored
```

> The endpoint runs on shared ZeroGPU — re-warm if quota is exhausted. The hermetic tests do not
> require the live endpoint and will always pass.

---

## 11. Why N-ATLAS Cannot Silently Degrade to Another Model

The product router (`router.rs`) contains a `ModelProvider` enum with multiple providers (Anthropic,
OpenAI, Google, DeepSeek, Ollama, etc.). When a provider fails, the router has silent
synthetic-degradation paths that could substitute another model's output without the user knowing.

N-ATLAS is **deliberately excluded** from this enum. It lives behind its own `NatlasTransport` trait
in `crates/zylcode-core/src/competition/natlas/`. The `NatlasResilienceState` type enumerates every
failure mode:

```rust
pub enum NatlasResilienceState {
    Ok,           // genuine N-ATLAS response
    Warming,      // endpoint starting up
    Loading,      // model loading
    Timeout,      // request timed out
    Quota,        // rate limit hit
    AuthFailure,  // credential mismatch
    Unavailable,  // endpoint unreachable
    Blocked,      // transport blocked
    Failed,       // unclassified failure
}
```

**There is no `Synthetic` or `Fallback` variant.** If N-ATLAS is unreachable, the state is
`Unavailable` or `Blocked` — and the caller knows exactly why. This architectural isolation is the
correct answer to the challenge's disqualification rule.

---

*Last updated: 2026-10-08. Verified against branch `competition/natlas-2026`.*
