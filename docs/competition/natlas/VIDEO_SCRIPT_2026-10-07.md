# Submission Video — Script, Storyboard & 3 Demo Scripts
### N-ATLAS × ZylCode Developer Bridge (NAIC PS1 · Developer Infrastructure · Track B)

- **Target length:** 4–5 minutes
- **Tone:** confident, engineering-led, evidence-first. No hype claims we cannot reproduce.
- **Narration voice:** calm, technical. On-screen text minimal and factual.
- **Screen-capture note:** the live demo segments must be recorded by a human (owner or agent) with
  the endpoint warm. Scripts below are exactly what to run on screen. Animated segments (sovereign
  model map, architecture diagram) can be generated; the *demo* segments are real captures.

---

## Narrative arc (why we win)

1. **Sovereign + local (0:00–0:40).** Nigeria built N-ATLAS (NCAIR / Awarri / NITDA). Nigeria built
   ZylCode (Zylvex). We connect them: a Nigerian Engineering OS driven by a Nigerian sovereign model
   in Nigerian languages. Not a chat toy — real software creation.
2. **The problem we solve (0:40–1:10).** Most AI dev tooling is English-only and foreign-model.
   Nigerian developers deserve an OS that reasons in Yoruba, Hausa, Igbo, and Nigerian English, and
   *does the work* — code, tests, evidence — not just talk.
3. **The bridge (1:10–1:50).** Architecture: Agent Kernel → `LocalNatlasTransport` (deliberate seam,
   not the fabricated-degradation router) → controlled N-ATLAS endpoint → real weights. Show the
   diagram (ARCHITECTURE_BRIDGE.md §1).
4. **Genuine integration, proven (1:50–2:40).** Live: `GET /healthz` → `model: NCAIR1/N-ATLaS`.
   B1 gate: prompt "Return exactly: NATLAS_ZYLCODE_OK" → model returns it. Cut to the terminal.
5. **Multilingual real-world validation (2:40–3:40).** Live round-trip: EN / YO / HA / IG prompts →
   real, language-correct completions (EV-015). A Nigerian dev gets engineering help in their language.
6. **Real engineering, not chat (3:40–4:20).** Show the Agent Kernel taking a prompt, planning, asking
   approval, making a real repo change + test, capturing evidence. (Screen-capture of the desktop app.)
7. **Evidence integrity + licence-clean (4:20–4:45).** Every claim reproducible; Apache-2.0 fork,
   attribution preserved, weights never redistributed. Close on "Engineered in Nigeria. Built for the world."

---

## Storyboard

| # | Time | On screen | Voiceover |
|---|---|---|---|
| 1 | 0:00 | Title card: "N-ATLAS × ZylCode — A Sovereign Developer Bridge" | "Nigeria built the model. Nigeria built the OS. Today they build together." |
| 2 | 0:15 | Map of Nigeria → NCAIR/Awarri logo → Zylvex logo | "N-ATLAS is our sovereign LLM. ZylCode is our evidence-first Engineering OS." |
| 3 | 0:40 | English-only dev-tools montage (greyed) vs ZylCode (lit) | "Most AI tooling assumes English and a foreign model. We don't." |
| 4 | 1:10 | Architecture diagram (animated) | "The bridge: Agent Kernel, a deliberate N-ATLAS seam, and a controlled endpoint running the real weights." |
| 5 | 1:50 | Terminal: `curl …/healthz` → model NCAIR1/N-ATLaS | "No wrapper. The endpoint serves the genuine N-ATLAS weights." |
| 6 | 2:10 | Terminal: B1 gate → `NATLAS_ZYLCODE_OK` | "Our own auth, our own endpoint, a real model answer." |
| 7 | 2:40 | Terminal: `c3_multiling.py` → EN/YO/HA/IG outputs | "Yoruba, Hausa, Igbo, English — real engineering help, in your language." |
| 8 | 3:40 | Desktop app: prompt → plan → approve → code change + test → evidence | "And it does the work: real changes, real tests, real evidence." |
| 9 | 4:20 | Licence + attribution card | "Apache-2.0, attribution kept, weights never redistributed." |
| 10 | 4:45 | "Engineered in Nigeria. Built for the world." | (closing line) |

---

## Demo Script 1 — Endpoint identity + B1 gate (live, ~40s)

```bash
# 1. Prove the endpoint serves the REAL gated model
curl -s https://zylvex-natlas-zylcode-bridge.hf.space/healthz \
  | python3 -m json.tool | grep -E '"model"|"service"|"status"'

# 2. B1 inference gate (genuine N-ATLAS returns the challenge token)
python3 gate.py
# expect: healthz HTTP 200 ; chat HTTP 200 ; RAW_MODEL_OUTPUT: 'NATLAS_ZYLCODE_OK'
```
*Screen-capture the terminal. Narration: "Our controlled endpoint, our auth, the real model."*

## Demo Script 2 — Multilingual round-trip (live, ~50s)

```bash
python3 c3_multiling.py
# expect: EN/YO/HA/IG HTTP 200, each a coherent, language-correct completion
# (Yoruba in Yoruba, Hausa in Hausa, Igbo in Igbo)
```
*Screen-capture. Narration: "Four languages. One sovereign model. Real answers."*

## Demo Script 3 — Agent Kernel loop (live desktop app, ~60s)

> Requires the ZylCode desktop app (`apps/tauri`) running with `LocalNatlasTransport` pointed at the
> controlled endpoint. This is the human screen-capture segment.

1. In the ZylCode workspace, type (in Yoruba or English): *"Ṣe e lè ṣèjàde fáìlù Python tó ń kí àwọn
   nọ́mbà 1 sí 10?"* ("Can you create a Python file that prints numbers 1 to 10?")
2. Agent Kernel plans → shows plan → **approval gate** appears.
3. Approve → Execution Engine writes `count.py` → runs `python count.py` → captures output + SHA.
4. Evidence panel shows: command, output, file hash, timestamp, reviewer = automated.
5. Narration: "Plan, approve, real change, real test, captured evidence — not a chat."

*Screen-capture the full loop. This is the "Real engineering, not chat" proof.*

---

## Production notes

- **Do NOT fabricate** any screen segment. If the endpoint is cold (ZeroGPU quota), re-warm it first
  (`curl …/healthz` until 200) before recording Demos 1–2.
- Keep on-screen tokens/keys out of frame (the `NATLAS_API_KEY` is a Space secret; never show it).
- Animated segments (map, diagram) are illustrative and labelled as such; demo segments are real.
- The three demo scripts are reproducible from this repo's `AppData/Local/Temp/natlas_bridge/`
  helpers and `ARCHITECTURE_BRIDGE.md`.
