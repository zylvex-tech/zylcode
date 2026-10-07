# Submission Checklist — N-ATLAS × ZylCode Developer Bridge (PS1, Track B)

Mapped to the 7 official components (see NATLAS_CHALLENGE_RUBRIC_2026-10-07.md).
Status: ✅ done/real · 🔧 in progress · ⛔ blocked (human action required).

## 1. Working Artefact
- ✅ **Controlled N-ATLAS endpoint** — `zylvex/natlas-zylcode-bridge` (ZeroGPU, public).
  Endpoint: `https://zylvex-natlas-zylcode-bridge.hf.space/v1`
- ✅ **ZylCode integration** — `NatlasTransport` / `LocalNatlasTransport` (OpenAI-compatible),
  first-class competition provider; `tests/natlas_live.rs` exercises the real chain.
- 🔧 **Repo artefact** — `competition/natlas-2026` branch, frozen + documented.

## 2. N-ATLAS Integration Evidence
- ✅ `NATLAS_CONTRACT_VERIFICATION_2026-10-07.md` (3 verified contracts).
- ✅ Apache-2.0 fork of `samuelolubukun/NATLaS-Sovereign-Engine`; model `NCAIR1/N-ATLaS`.
- ✅ B1 inference gate (EV-014, L3) + multilingual round-trip (EV-015, L3).
- 🔧 `ARCHITECTURE_BRIDGE.md` write-up (C3).

## 3. Real-World Validation
- ✅ Live benchmark: real inference gate `NATLAS_ZYLCODE_OK` (B1 gate, **EV-014, L3**).
- ✅ Multilingual round-trip proven: EN/YO/HA/IG programming prompts → genuine, language-correct
  model output through our own endpoint (**EV-015, L3**).
- ⛔ **≥2 external beta testers (PS1 hard requirement)** — owner must recruit. We provide the
  invite + feedback template + a `/gradio` or local harness. **Cannot be self-issued.**
- 🔧 Repo-changing C3 task demo (multilingual prompt → actual code change) pending harness.

## 4. Technical Documentation
- ✅ `README` (Space) — attribution + Apache-2.0 + API surface.
- 🔧 `ARCHITECTURE_BRIDGE.md` — how ZylCode → N-ATLAS works (C3 write-up).
- 🔧 Setup/usage docs for judges.

## 5. Video Demonstration (3–5 min)
- 🔧 Explainer video (VideoGen from script) covering: sovereign model → developer bridge →
  multilingual engineering task → real code change → test → evidence.
- ⛔ Final screen-capture of the live demo is a human recording step (owner or me, with the
  running endpoint). Script + storyboard provided.

## 6. Team Profile
- ⛔ Owner-supplied: names, affiliations, roles (Zylvex Technologies Limited). Human input.

## 7. Endorsement / Registration
- ⛔ **CAC certificate** (Track B) — owner-supplied. Human input.

## Evaluation-criteria readiness
| Criterion | Status | Note |
|---|---|---|
| Working Artefact & Rigour | ✅ | endpoint + integration real |
| N-ATLAS Integration | ✅ | genuine, not a wrapper (disqualification avoided) |
| Real-World Validation | ✅ | gate (EV-014) + multilingual (EV-015) genuine; beta testers pending (human) |
| Impact Potential | ✅ | sovereign dev-infra narrative |
| Scalability & Sustainability | 🔧 | ZeroGPU quota risk documented; dedicate-GPU path noted |
| Team Capability | ⛔ | profile pending (human) |

## Flagship differentiators (lead with these)
1. **Sovereign + local:** Nigerian model (NCAIR/Awarri) inside a Nigerian-built Engineering OS.
2. **Evidence integrity by design:** every claim reproducible (promotion gate) — survives scrutiny.
3. **Real engineering, not chat:** C3 performs actual repo changes in Nigerian languages.
4. **Licence-clean:** Apache-2.0 fork, attribution preserved, weights never redistributed.
