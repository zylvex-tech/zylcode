# FIRST_WORKFLOW — one bounded developer task, end to end

This is the smallest genuine workflow the bridge supports. It is deliberately small, observable
and reproducible.

---

## The task

> *"Add a health-check endpoint to the router and a test for it."*

---

## The path

```
developer selects a repository
   -> ZylCode builds bounded repository context
   -> developer states an engineering instruction (any supported language)
   -> the request + context go to the REAL N-ATLAS runtime
   -> N-ATLAS returns an answer
   -> the answer is parsed into a structured engineering intent
   -> the intent is validated against a strict schema
   -> a bounded TaskGraph is generated
   -> the developer sees the proposed operation
   -> a HUMAN APPROVAL GATE is required
   -> the approved operation executes through an allowlisted tool path
   -> a REAL verification command runs
   -> the actual exit code is captured
   -> an evidence chain links request -> invocation -> response -> intent
      -> approval -> action -> result -> verification
```

At no point does the model execute arbitrary shell commands, and at no point does anything modify
the repository without an explicit human approval.

---

## 1. Prepare a small repository

Any small repository works. For a first run, a toy project is ideal:

```bash
mkdir hello-natlas && cd hello-natlas
git init
printf 'pub fn router() {}\n' > routes.rs
```

---

## 2. Build bounded context

Context is chosen by ZylCode's repository intelligence, not by the model. Keep it small — a
handful of files — and record which paths were sent:

| Field | Example |
|---|---|
| path | `routes.rs` |
| excerpt | `pub fn router() {}` |

The context is attached to the **user turn** with explicit path labels, so the model can see
provenance and so nothing is presented as if it were the developer's own words.

---

## 3. Invoke N-ATLAS

Via the SDK:

```ts
const { invocation, intent } = await bridge.submitIntent({
  intent: "Add a health-check endpoint to the router and a test for it.",
  system: INTENT_SYSTEM_PREAMBLE,
  context: [{ path: "routes.rs", excerpt: "pub fn router() {}" }],
});
```

Via the playground: open `apps/natlas-sdk/playground/index.html`, fill in the runtime and the
instruction, press **Invoke N-ATLAS**.

---

## 4. Read the structured intent

The model must answer in this contract (ZylCode's own — not N-ATLAS's native format):

```json
{
  "summary": "Add a health endpoint",
  "steps": [
    { "kind": "analyse",   "description": "find the router" },
    { "kind": "implement", "description": "add GET /health", "target_path": "routes.rs" },
    { "kind": "test",      "description": "run the suite" },
    { "kind": "approve",   "description": "supervisor sign-off" }
  ]
}
```

Allowed kinds: `analyse`, `implement`, `test`, `document`, `approve`.

A reply that does not satisfy this is **rejected** — not partially accepted, and not silently
repaired.

---

## 5. Inspect the proposed operation

The intent becomes a **dependency-chained TaskGraph**: each step depends on the previous one, and
the first step is the only one initially ready.

The `approve` step maps to a **real** human-approval gate. The `implement` and `test` steps map to
*notes* in the current phase, because ZylCode will not invent file contents or a test command that
the model did not supply. Executing a real write requires the approved content to be present.

> **This is deliberate.** Fabricating a patch or a test command would be worse than not executing
> one. The bounded slice stops at the approval gate and reports what it has.

---

## 6. Approve or reject

The developer sees the proposal and chooses. Rejection ends the workflow with **no change to the
repository** and an evidence record saying so.

---

## 7. Execute and verify

When an approved, concrete operation exists, it runs through the **allowlisted tool path**
(`zylcode_mcp::dispatch` behind the permission gate), and a real verification command runs. The
**actual exit code** is captured — not a claim about it.

---

## 8. Inspect the evidence

Every invocation writes a JSONL evidence record:

```json
{
  "timestamp": "2026-10-06T12:00:00Z",
  "provider": "natlas",
  "model": "n-atlas:latest",
  "requestId": "chatcmpl-...",
  "requestClassification": "implementation",
  "status": "succeeded",
  "latencyMs": 4210,
  "success": true,
  "promptChars": 84,
  "responseChars": 512,
  "secretRedactions": 0
}
```

The API key never appears. See [EVIDENCE_AND_VALIDATION.md](EVIDENCE_AND_VALIDATION.md).

---

## What is genuinely working today, and what is not

| Stage | State |
|---|---|
| Configuration, boundary, parsing, evidence, redaction | **TESTED** |
| Local transport (real HTTP, OpenAI-compatible) | **TESTED** against a stub server |
| Runtime probe | **TESTED** |
| Intent -> TaskGraph handoff | **TESTED** |
| **A genuine N-ATLAS invocation** | **BLOCKED_NATLAS_ACCESS** until the model is served |
| Approved-write execution | **PROPOSED** — requires the concrete patch/test command |

Read that table before claiming the workflow "works". It does not, yet, until a real model answers.
