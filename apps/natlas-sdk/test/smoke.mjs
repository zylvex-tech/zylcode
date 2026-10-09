/**
 * SDK smoke test — proves the TypeScript SDK performs a real HTTP round trip
 * and never fabricates a result.
 *
 * The `stub` server below is a TEST DOUBLE. It is not N-ATLAS.
 *
 * Run:  node --experimental-strip-types test/smoke.mjs
 */
import http from "node:http";
import assert from "node:assert/strict";

import {
  NatlasBridge,
  NatlasError,
  OPENAI_CHAT_PATH,
  INTENT_SYSTEM_PREAMBLE,
  classifyHttp,
  guidance,
  humanMessage,
  isRetryable,
  parseEngineeringIntent,
  redactSecrets,
  resilienceStateOf,
  translateOpenAiReply,
} from "../src/index.ts";

const INTENT_JSON = JSON.stringify({
  summary: "Add a health endpoint",
  steps: [
    { kind: "analyse", description: "find the router" },
    { kind: "implement", description: "add GET /health", target_path: "src/routes.rs" },
    { kind: "approve", description: "supervisor sign-off" },
  ],
});

const OPENAI_REPLY = JSON.stringify({
  id: "stub-req-1",
  model: "n-atlas-stub:latest",
  choices: [
    {
      index: 0,
      message: { role: "assistant", content: INTENT_JSON },
      finish_reason: "stop",
    },
  ],
  usage: { prompt_tokens: 31, completion_tokens: 57 },
});

function startStub(handler) {
  return new Promise((resolve) => {
    const server = http.createServer(handler);
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address();
      resolve({ url: `http://127.0.0.1:${port}`, close: () => server.close() });
    });
  });
}

let passed = 0;
let failed = 0;
async function test(name, fn) {
  try {
    await fn();
    passed++;
    console.log(`ok   ${name}`);
  } catch (e) {
    failed++;
    console.log(`FAIL ${name}\n     ${e.message}`);
  }
}

// ---------------------------------------------------------------------------

await test("pure: translateOpenAiReply maps a documented reply", () => {
  const r = translateOpenAiReply(OPENAI_REPLY);
  assert.equal(r.model, "n-atlas-stub:latest");
  assert.equal(r.requestId, "stub-req-1");
  assert.equal(r.usage.inputTokens, 31);
  assert.equal(r.finishReason, "stop");
});

await test("pure: a reply without model identity is rejected", () => {
  const bad = JSON.stringify({ choices: [{ message: { content: "hi" } }] });
  assert.throws(() => translateOpenAiReply(bad), (e) => e instanceof NatlasError && e.code === "MALFORMED_RESPONSE");
});

await test("pure: parseEngineeringIntent is strict", () => {
  const intent = parseEngineeringIntent(INTENT_JSON);
  assert.equal(intent.steps.length, 3);
  assert.throws(() => parseEngineeringIntent('{"summary":"s","steps":[]}'));
  assert.throws(() => parseEngineeringIntent('{"summary":"s","steps":[{"kind":"deploy","description":"x"}]}'));
});

await test("pure: redactSecrets removes and counts", () => {
  const { text, count } = redactSecrets("key stub-key-1234 here", ["stub-key-1234"]);
  assert.ok(!text.includes("stub-key-1234"));
  assert.equal(count, 1);
});

await test("http: real round trip reaches the runtime and parses the intent", async () => {
  const stub = await startStub((req, res) => {
    res.writeHead(200, { "content-type": "application/json" });
    res.end(OPENAI_REPLY);
  });
  try {
    const bridge = NatlasBridge.configure({
      baseUrl: stub.url,
      requestPath: OPENAI_CHAT_PATH,
      model: "n-atlas-stub",
      apiKey: "stub-key-1234",
    });
    const { invocation, intent } = await bridge.submitIntent({
      intent: "add a health endpoint",
      system: INTENT_SYSTEM_PREAMBLE,
    });
    assert.equal(invocation.status, "succeeded");
    assert.equal(invocation.response.model, "n-atlas-stub:latest");
    assert.equal(invocation.evidence.success, true);
    assert.ok(intent, "the intent must parse");
    assert.equal(intent.summary, "Add a health endpoint");
  } finally {
    stub.close();
  }
});

await test("http: the probe reports the runtime and the model it found", async () => {
  const stub = await startStub((req, res) => {
    if (req.url === "/api/tags") {
      res.writeHead(200, { "content-type": "application/json" });
      res.end(JSON.stringify({ models: [{ name: "n-atlas-stub:latest" }] }));
      return;
    }
    res.writeHead(404).end();
  });
  try {
    const bridge = NatlasBridge.configure({
      baseUrl: stub.url,
      requestPath: OPENAI_CHAT_PATH,
      model: "n-atlas-stub:latest",
    });
    const health = await bridge.checkStatus();
    assert.equal(health.reachable, true);
    assert.equal(health.modelPresent, true);
  } finally {
    stub.close();
  }
});

await test("http: an absent runtime fails and never fabricates", async () => {
  const bridge = NatlasBridge.configure({
    baseUrl: "http://127.0.0.1:1",
    requestPath: OPENAI_CHAT_PATH,
    model: "n-atlas-stub",
    timeoutMs: 2000,
  });
  const inv = await bridge.invoke({ intent: "hello", system: "sys" });
  assert.equal(inv.status, "failed");
  assert.equal(inv.response, undefined);
  assert.equal(inv.error.code, "TRANSPORT");
});

await test("http: a 500 is an HTTP_STATUS error, not a success", async () => {
  const stub = await startStub((req, res) => {
    res.writeHead(500, { "content-type": "application/json" });
    res.end('{"error":"boom"}');
  });
  try {
    const bridge = NatlasBridge.configure({
      baseUrl: stub.url,
      requestPath: OPENAI_CHAT_PATH,
      model: "n-atlas-stub",
    });
    const inv = await bridge.invoke({ intent: "x", system: "y" });
    assert.equal(inv.status, "failed");
    assert.equal(inv.error.code, "HTTP_STATUS");
    assert.equal(inv.evidence.httpStatus, 500);
  } finally {
    stub.close();
  }
});

await test("http: a secret echoed in an error is redacted from evidence", async () => {
  const stub = await startStub((req, res) => {
    res.writeHead(401, { "content-type": "application/json" });
    res.end('{"error":"bad key stub-key-1234 rejected"}');
  });
  try {
    const bridge = NatlasBridge.configure({
      baseUrl: stub.url,
      requestPath: OPENAI_CHAT_PATH,
      model: "n-atlas-stub",
      apiKey: "stub-key-1234",
    });
    const inv = await bridge.invoke({ intent: "x", system: "y" });
    const serialised = JSON.stringify(inv.evidence);
    assert.ok(!serialised.includes("stub-key-1234"), "the key must never reach evidence");
    assert.ok(inv.evidence.secretRedactions > 0);
  } finally {
    stub.close();
  }
});

await test("config: fromEnv lists every missing variable rather than defaulting", () => {
  assert.throws(
    () => NatlasBridge.fromEnv({}),
    (e) => e instanceof NatlasError && e.code === "NOT_CONFIGURED" && e.message.includes("NATLAS_BASE_URL"),
  );
});

// --- NAT-A-001 / NAT-A-003: quota classification + human-readable guidance ----

await test("resilience: a ZeroGPU quota 503 is QUOTA, not warming", () => {
  const body = "You have exceeded your GPU quota (0s left). Please try again later.";
  assert.equal(classifyHttp(503, body), "quota");
  assert.equal(classifyHttp(503, "GPU quota exceeded"), "quota");
  assert.equal(classifyHttp(402, ""), "quota");
  assert.equal(classifyHttp(429, "slow down"), "quota");
  // A genuine cold start is still warming / loading.
  assert.equal(classifyHttp(503, "model is loading"), "loading");
  assert.equal(classifyHttp(502, "bad gateway"), "warming");
});

await test("resilience: a spent quota is not retryable; a cold start is", () => {
  assert.equal(isRetryable("quota"), false);
  assert.equal(isRetryable("auth_failure"), false);
  assert.equal(isRetryable("blocked"), false);
  assert.equal(isRetryable("warming"), true);
  assert.equal(isRetryable("loading"), true);
});

await test("resilience: every state has non-empty, actionable guidance", () => {
  const states = [
    "ok",
    "warming",
    "loading",
    "timeout",
    "quota",
    "auth_failure",
    "unavailable",
    "blocked",
    "failed",
  ];
  for (const s of states) {
    const g = guidance(s);
    assert.ok(g.length > 20, `${s} guidance is too terse: ${JSON.stringify(g)}`);
  }
  assert.ok(guidance("quota").includes("LOCAL_RUNTIME_FALLBACK"), "quota guidance must point somewhere real");
});

await test("resilience: an HTTP_STATUS error carries a human message, not a bare code", () => {
  const err = new NatlasError("HTTP_STATUS", "N-ATLAS returned HTTP 503", "You have exceeded your GPU quota");
  assert.equal(resilienceStateOf(err), "quota");
  const msg = humanMessage(err);
  assert.ok(msg.includes("quota"), "human message must name the cause");
  assert.ok(msg.split("\n").length >= 2, "human message must have cause + guidance lines");
});

// --- NAT-A-002 / NAT-A-004: state on the invocation + language propagation ----

await test("invoke: a 503 quota body yields state=quota, not a fabricated success", async () => {
  const stub = await startStub((req, res) => {
    res.writeHead(503, { "content-type": "text/plain" });
    res.end("You have exceeded your GPU quota (0s left).");
  });
  try {
    const bridge = NatlasBridge.configure({
      baseUrl: stub.url,
      requestPath: OPENAI_CHAT_PATH,
      model: "n-atlas-stub",
    });
    const inv = await bridge.invoke({ intent: "x", system: "y" });
    assert.equal(inv.status, "failed");
    assert.equal(inv.state, "quota");
    assert.equal(inv.response, undefined, "a quota failure must never carry a response");
  } finally {
    stub.close();
  }
});

await test("invoke: a stated language is recorded in evidence; an unstated one is omitted", async () => {
  const stub = await startStub((req, res) => {
    res.writeHead(200, { "content-type": "application/json" });
    res.end(OPENAI_REPLY);
  });
  try {
    const bridge = NatlasBridge.configure({
      baseUrl: stub.url,
      requestPath: OPENAI_CHAT_PATH,
      model: "n-atlas-stub",
    });

    const withLang = await bridge.invoke({ intent: "x", system: "y", language: "ig" });
    assert.equal(withLang.evidence.language, "ig", "Igbo must be recorded");

    const blank = await bridge.invoke({ intent: "x", system: "y", language: "   " });
    assert.equal(blank.evidence.language, undefined, "a blank tag is not a language");
    assert.ok(!JSON.stringify(blank.evidence).includes('"language"'), "an unstated language must be omitted");

    const none = await bridge.invoke({ intent: "x", system: "y" });
    assert.equal(none.evidence.language, undefined, "language must never be inferred");
  } finally {
    stub.close();
  }
});

// ---------------------------------------------------------------------------
console.log(`\n${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);
