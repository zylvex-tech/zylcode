# SECURITY AND APPROVALS

The bridge is designed so that a model **cannot** silently change your repository, and so that a
failure can never masquerade as a success.

---

## 1. The approval gate is real

A model's output is *prose until proven otherwise*. Before anything touches the filesystem, a
human must approve a concrete, bounded operation.

- The intent schema includes an explicit `approve` step.
- That step maps to a **real** `TaskAction::Approval` gate — the task graph parks there.
- Rejection ends the workflow with **no change** and an evidence record saying so.

There is **no** path in which model output executes directly.

---

## 2. No arbitrary shell execution from the model

The model proposes; it does not execute.

- A `test` step becomes a *note* unless a concrete command is supplied — ZylCode will not invent a
  command and run it.
- Executable actions go through the single chokepoint `zylcode_mcp::dispatch()`, behind a
  `PermissionGate` with a risk ceiling (`RECOMMENDED_RISK_CEILING = RiskLevel::Write`).
- Refusals are recorded as evidence, not swallowed.

---

## 3. No silent fallback

This is the core safety property, and it is enforced in code:

| Situation | What happens |
|---|---|
| Runtime unreachable | `NatlasError::Transport` — **no** synthesised answer |
| Runtime times out | `NatlasError::Timeout` |
| Non-2xx status | `NatlasError::HttpStatus` with a body excerpt |
| Reply malformed | `NatlasError::MalformedResponse` |
| Reply omits model identity | **rejected** — identity cannot be borrowed |
| Config incomplete | `NatlasError::NotConfigured`, listing every missing variable |

The transport layer contains **no branch that returns a fabricated response**. A test asserts that
an absent runtime never produces a `Succeeded` status.

---

## 4. Secrets

- Configuration is read from the environment; **nothing is hardcoded**.
- `NatlasConfig`'s `Debug` implementation redacts the key by construction, so an accidental
  `{:?}` cannot leak it.
- `RedactedNatlasConfig` carries the key's **presence and length only**.
- Every free-text field that could echo a secret passes through `redact_secrets` before storage,
  and the **number of redactions is recorded** — so a reader can see that redaction happened
  rather than trusting it silently.
- The API key never appears in an evidence record. A test asserts this by having a stub server
  echo the key back and then checking the stored evidence.
- **Never** paste a token into source code, a `Modelfile`, or a chat message.

---

## 5. Network posture

- `LocalNatlasTransport` uses `.no_proxy()`. A local runtime lives on the loopback interface and
  **must not** be routed through an ambient `HTTP_PROXY` — doing so would send local traffic to a
  third party, or fail confusingly. This is asserted by a test.
- `HttpNatlasTransport` **requires** an explicitly configured auth header. It refuses to guess an
  authentication scheme, because choosing one would be inventing an N-ATLAS interface detail.

---

## 6. What the model is trusted to do

| Trusted | Not trusted |
|---|---|
| Propose a plan | Execute anything |
| Name a file it would change | Write to it |
| Suggest a test | Run an arbitrary command |
| Return JSON | Have that JSON trusted without validation |

Every model output crosses a **strict schema** before it has any effect. A reply that does not
satisfy the contract is rejected outright.

---

## 7. Reporting a security issue

See the repository's `SECURITY.md`. Do not open a public issue for a vulnerability.
