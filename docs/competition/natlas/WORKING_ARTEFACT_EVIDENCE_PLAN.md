# WORKING ARTEFACT EVIDENCE PLAN

**Purpose:** identify exactly what URL(s) will eventually be pasted into the NAIC application, and
what each one proves.

**Status:** **PREPARED, NOT YET POPULATED.** No public deployment has been performed and no URL
below is live.

---

## 1. The requirement

The application requires a **working artefact evidence URL** — something an evaluator can open and
judge. It must demonstrate a genuine N-ATLAS-powered workflow, not a screenshot of an unconnected
UI.

---

## 2. What we will paste, and what it proves

| # | Artefact | Intended URL | What it proves | Status |
|---|---|---|---|---|
| 1 | **Public GitHub repository** | `https://github.com/zylvex-tech/zylcode` (branch `competition/natlas-2026`) | the source is real, open-source (Apache-2.0), and reviewable | ⛔ not published |
| 2 | **Tagged release** | `…/releases/tag/natlas-c1` | a fixed, reproducible snapshot | ⛔ not created |
| 3 | **Developer documentation** | `…/blob/…/docs/competition/natlas/developer/QUICKSTART.md` | a first-time developer can onboard | ✅ written, not published |
| 4 | **Evidence bundle** | `…/blob/…/docs/competition/natlas/evidence/` | genuine invocation records | ⛔ blocked — no genuine invocation yet |
| 5 | **Hosted playground** (optional) | TBD | interactive evaluation | 🚫 not deployed; needs owner authorisation |
| 6 | **Demonstration video** | TBD | the workflow, demonstrated | ⛔ not recorded — see `VIDEO_DEMO_PLAN.md` |

---

## 3. The single most important artefact

Artefact **#4 — the evidence bundle** is the one that cannot be substituted. It is the only thing
that proves the N-ATLAS call was genuine rather than narrated.

It cannot exist until a real invocation has occurred. **This is the blocking dependency for the
whole submission.**

---

## 4. What each artefact must NOT be

| Anti-pattern | Why it is forbidden |
|---|---|
| A screenshot of the playground with the TEST DOUBLE badge | proves nothing about N-ATLAS |
| A "demo" driven by `MockNatlasTransport` | a test double is not integration |
| A video that narrates without showing a real response | the directive requires demonstration, not narration |
| A hosted UI pointed at a different provider, labelled N-ATLAS | relabelling is fabrication |
| Fabricated benchmark numbers | forbidden outright |

---

## 5. Deployment decisions requiring the owner

None of the following has been done. Each needs explicit authorisation:

1. Making the GitHub repository **public**.
2. Creating a **tagged release**.
3. Hosting the **playground** anywhere.
4. Publishing the **evidence bundle** (which must first be reviewed for repository content).
5. Uploading the **video**.

---

## 6. Readiness summary

| Component | Ready to publish? |
|---|---|
| Source code | ✅ yes (after the owner reviews the pre-existing untracked files — see `PUBLIC_REPO_READINESS_2026-10-06.md` §5) |
| Developer docs | ✅ yes |
| SDK + playground | ✅ yes |
| Tests | ✅ yes |
| **Genuine invocation evidence** | ⛔ **no — does not exist yet** |
| Video | ⛔ no |

**Bottom line:** everything except the evidence of a genuine N-ATLAS call is ready. That call is
blocked on the model access gate.
