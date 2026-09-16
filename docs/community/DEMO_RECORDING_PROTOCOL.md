# Demo Recording Protocol

**Track:** `PUBLIC-FOUNDATION-03` (screen-recording program)
**Purpose:** A repeatable way to produce recordings of ZylCode doing **real** work, so public demos are
evidence, not animation.

---

## Principle

> Record **real** software. Never generate promotional animation pretending to be product operation.

A recording that shows enough context to prove the operation is real is worth more than a polished one that
cannot be verified. When the product cannot yet do the thing, **say so** — do not fake it.

---

## Per-recording checklist

Each recording is planned as one unit:

| Field | Required |
|---|---|
| Scenario | What capability is being demonstrated |
| Starting commit | Exact SHA the recording was made from |
| Environment | OS, `rustc --version`, app version/commit, model in use |
| Command / action | The exact steps taken |
| Expected result | What should happen, stated before recording |
| Evidence | The artifact the demo produces (transcript, preview, proof node) |
| Recording steps | How it was captured |
| Redaction checklist | What was removed before publication |
| Publication checklist | Final gate before release |

---

## Scenario catalogue (start here)

- agent executes a real tool (e.g. a filesystem or git action with visible output)
- approval gate stops a dangerous action (show the deny, not just the allow)
- kill ZylCode and recover the session from the Evidence Ledger
- a test fails, then the agent repairs it, then it passes
- evidence inspection of a completed run
- repository intelligence query **once accepted** (currently R2, not yet integrated — do not stage this
  until Phase 2A re-audit passes)
- Mission execution **once implemented**
- Artifact preview **once implemented**

Do not stage a scenario for a capability that has not reached the rung the scenario claims.

---

## Recording steps

1. Note the starting commit: `git rev-parse HEAD`.
2. Capture the environment banner (OS + versions) at the top of the recording.
3. Perform the real action. Let the real output render — do not speed-paste a scripted result.
4. Capture the resulting artifact (transcript line, preview, proof node), not just the happy path.
5. If a step fails, record the failure too. A recording that hides failure is not evidence.

---

## Redaction checklist (mandatory before publication)

Remove or blur, always:

- API keys, MCP tokens, cloud credentials (Cloudflare/AWS), model provider keys
- SMTP / payment credentials
- personal information (names, emails, locations)
- private repository contents or paths that would leak internal structure
- secret environment variables
- anything shown in logs, screenshots, or the terminal that satisfies "secret" above

If a credential appears on screen, the recording is **not publishable** until redacted. Prefer using a
scrubbed demo account over redacting live credentials after the fact.

---

## Publication checklist (final gate)

- [ ] Starting commit SHA recorded and reachable.
- [ ] Environment matches what is shown.
- [ ] The action performed is real and reproducible from that commit.
- [ ] No fabricated or staged-only output presented as live behaviour.
- [ ] All secret/PII classes above redacted.
- [ ] The capability's status label (PROPOSED / IN DEVELOPMENT / DEVELOPER PREVIEW / AVAILABLE) matches
      the implemented rung.
- [ ] If the feature is a concept/mockup, it is labelled `CONCEPT` in the video itself, not only in text.

If any item fails, do **not** publish. Fix the recording or the underlying capability first.

---

## What this protects

The same rule as everything else in this project: a beautiful recording of a non-existent capability is a
false claim with extra steps. The protocol exists so that when we show ZylCode working, it provably was.
