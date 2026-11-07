# @zylvex/natlas-bridge

JavaScript/TypeScript SDK for the **ZylCode × N-ATLAS Developer Bridge**.

Talk to a locally-served N-ATLAS model, get a structured engineering intent back, and capture a
secret-free evidence record — with **no silent fallback**.

---

## Install

```bash
pnpm add @zylvex/natlas-bridge      # or npm install / yarn add
```

Node.js ≥ 18 is required (the SDK uses the built-in `fetch`).

---

## Quick start

```ts
import { NatlasBridge, OPENAI_CHAT_PATH, INTENT_SYSTEM_PREAMBLE } from "@zylvex/natlas-bridge";

const bridge = NatlasBridge.configure({
  baseUrl: "http://127.0.0.1:11434",
  requestPath: OPENAI_CHAT_PATH,
  model: "n-atlas:latest",
});

// 1. Is the runtime genuinely up?
const health = await bridge.checkStatus();
console.log(health.reachable, health.modelPresent);

// 2. Invoke it and parse a structured intent
const { invocation, intent } = await bridge.submitIntent({
  intent: "Add a health-check endpoint to the router and a test for it.",
  system: INTENT_SYSTEM_PREAMBLE,
  context: [{ path: "src/routes.rs", excerpt: "pub fn router() {}" }],
});

console.log(invocation.status); // "succeeded" | "failed"
console.log(invocation.response?.model); // the SERVER's identity
console.log(invocation.evidence); // secret-free record
console.log(intent?.summary);
```

Or read configuration from the environment:

```bash
export NATLAS_BASE_URL=http://127.0.0.1:11434
export NATLAS_REQUEST_PATH=/v1/chat/completions
export NATLAS_MODEL=n-atlas:latest
export NATLAS_API_KEY=            # may be empty for a local runtime, but must be present
```

```ts
const bridge = NatlasBridge.fromEnv();
```

---

## Respan hosted gateway

Set `RESPAN_API_KEY` (in the process environment — it is a **server secret**, never a
browser-visible one) and the same `invoke()` call goes to Respan's hosted gateway instead of a
local runtime. Respan logs every request it serves, so the call is traced without a tracing SDK.

```bash
export RESPAN_API_KEY=            # gateway key; leave unset to keep the N-ATLAS runtime
export RESPAN_MODEL=gpt-5.5       # optional; the model the gateway is asked for
export RESPAN_BASE_URL=           # optional; Respan Enterprise endpoint
```

```ts
const bridge = NatlasBridge.fromEnv(); // gateway when RESPAN_API_KEY is set
const inv = await bridge.invoke({
  intent: "add a health endpoint",
  system: INTENT_SYSTEM_PREAMBLE,
});
console.log(inv.evidence.provider); // "respan" — not "natlas"
```

This mirrors `RouterConfig::connect_respan_gateway` in `zylcode-core`: the endpoint and the
credential change, the request body and the guarantees below do not. `evidence.provider` records
which surface served the call, and the model identity still comes from the server's reply. Unset
the key and the `NATLAS_*` variables above are read exactly as before.

---

## What the SDK guarantees

| Guarantee                        | How                                                                                        |
| -------------------------------- | ------------------------------------------------------------------------------------------ |
| **No fallback**                  | an unreachable runtime throws / returns a `TRANSPORT` failure — never a synthesised answer |
| **Identity is not borrowed**     | a reply that does not report a `model` is rejected                                         |
| **Strict contract**              | a reply that does not satisfy the intent schema is rejected, not partially accepted        |
| **Secrets stay out of evidence** | redaction is applied and **counted**                                                       |
| **No invented defaults**         | required config has no defaults; missing values are listed                                 |

---

## Error codes

`NOT_CONFIGURED` · `BLOCKED_NATLAS_ACCESS` · `TRANSPORT` · `TIMEOUT` · `HTTP_STATUS` ·
`MALFORMED_RESPONSE`

There is deliberately **no** catch-all "something went wrong".

---

## Playground

A self-contained developer playground lives at `playground/index.html`. Open it in a browser — no
build step.

It shows runtime status, model identity, language, instruction, bounded context, the invocation,
the structured intent, the proposed operation, the approval gate and the evidence record.

**It never shows "connected", "success", "N-ATLAS" or "verified" unless a real call succeeded.**
The built-in offline demo is labelled `TEST DOUBLE` everywhere it appears.

---

## Tests

```bash
node --experimental-strip-types test/smoke.mjs
```

10 checks plus 4 gateway-selection checks, using an in-process stub server. The stub is a **test
double**, not N-ATLAS; the gateway checks read configuration and prove the provider label, they do
not call Respan.

---

## Licence

Apache-2.0 for this SDK. **N-ATLAS model weights are a separate work** under the N-ATLaS
Open-Source Research and Innovation License — see
`docs/competition/natlas/developer/NATLAS_SETUP.md`.
