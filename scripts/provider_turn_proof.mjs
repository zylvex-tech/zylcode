#!/usr/bin/env node
// End-to-end proof for the headline claim: a saved provider change is used by
// the *next agent turn*, not just by the settings probe.
//
// Sets a Custom (OpenAI-compatible) entry to the top of the chain pointing at a
// loopback server, stores a key, then runs a real `process_intent` turn. The
// assertion is on what the server received: the endpoint, the model and the
// bearer credential that were saved through the panel commands.
//
// No egress: the only provider reachable in this configuration is this script.

import http from "node:http";

const cdpPort = process.argv[2] ?? "9222";
const KEY = "sk-turn-proof-5432";
const MODEL = "turn-proof-model";
const results = [];
const check = (name, pass, detail) => {
  results.push({ name, pass, detail });
  console.log(
    `${pass ? "PASS" : "FAIL"}  ${name}${detail ? ` — ${detail}` : ""}`,
  );
};

const seen = [];
const provider = http.createServer((req, res) => {
  let body = "";
  req.on("data", (c) => (body += c));
  req.on("end", () => {
    seen.push({ url: req.url, auth: req.headers.authorization ?? null, body });
    res.writeHead(200, { "content-type": "application/json" });
    res.end(
      JSON.stringify({
        choices: [
          {
            message: {
              role: "assistant",
              content: JSON.stringify({
                decision: "think",
                thought: "loopback answered",
              }),
            },
          },
        ],
        usage: { prompt_tokens: 10, completion_tokens: 5 },
      }),
    );
  });
});
await new Promise((resolve, reject) => {
  provider.once("error", reject);
  provider.listen(0, "127.0.0.1", resolve);
});
const endpoint = `http://127.0.0.1:${provider.address().port}`;

const targets = await (
  await fetch(`http://127.0.0.1:${cdpPort}/json/list`)
).json();
const page = targets.find((t) => t.type === "page");
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  ws.onopen = resolve;
  ws.onerror = reject;
});
let seq = 0;
const pending = new Map();
ws.onmessage = (ev) => {
  const msg = JSON.parse(ev.data);
  if (msg.id && pending.has(msg.id)) {
    pending.get(msg.id)(msg);
    pending.delete(msg.id);
  }
};
const rpc = (method, params) =>
  new Promise((resolve) => {
    const id = ++seq;
    pending.set(id, resolve);
    ws.send(JSON.stringify({ id, method, params }));
  });
await rpc("Runtime.enable", {});
const invoke = async (cmd, args) =>
  (
    await rpc("Runtime.evaluate", {
      expression: `(async () => {
        try { return { ok: true, value: await window.__TAURI_INTERNALS__.invoke(${JSON.stringify(cmd)}, ${JSON.stringify(args)}) }; }
        catch (e) { return { ok: false, error: String(e) }; }
      })()`,
      awaitPromise: true,
      returnByValue: true,
      userGesture: true,
    })
  ).result?.result?.value;

// Save the configuration exactly as the panel does.
const cfg = await invoke("set_provider_config", {
  kind: "custom",
  endpoint,
  model: MODEL,
  timeoutMs: 15000,
  enabled: true,
});
check("custom entry saved as the chain head", cfg.ok === true, cfg.error ?? "");
const reorder = await invoke("reorder_provider_chain", {
  order: [
    "custom",
    "ollama",
    "anthropic",
    "open_router",
    "open_ai",
    "deep_seek",
    "synthetic_offline",
  ],
});
check(
  "custom moved to first attempt",
  reorder.ok === true,
  reorder.error ?? "",
);
check(
  "the chain reports custom first",
  reorder.value?.[0]?.kind === "custom",
  reorder.value?.[0]?.kind,
);
const key = await invoke("set_provider_key", { kind: "custom", apiKey: KEY });
check("key stored", key.ok === true, key.error ?? "");

// A real agent turn through the engine. The prompt must be unique and the
// speculative/vector caches empty, or dispatch answers from cache and no
// request reaches any provider (which is correct behaviour, just not a proof).
await invoke("clear_vector_cache", {});
const turn = await invoke("process_intent", {
  prompt: `Say hello in one short sentence. run ${Date.now()}`,
  model: null,
});
check(
  "process_intent returned without a transport error",
  turn.ok === true,
  turn.error ?? "",
);

check(
  "the turn reached the saved endpoint",
  seen.length > 0 && seen[0].url === "/chat/completions",
  String(seen[0]?.url),
);
check(
  "the turn sent the saved model",
  seen.length > 0 && seen[0].body.includes(MODEL),
  String(seen[0]?.body?.slice(0, 120)),
);
check(
  "the turn sent the stored key",
  seen.length > 0 && seen[0].auth === `Bearer ${KEY}`,
  String(seen[0]?.auth),
);

// Leave nothing behind.
await invoke("delete_provider_key", { kind: "custom" });
await invoke("set_provider_config", {
  kind: "custom",
  endpoint: "",
  model: "",
  timeoutMs: 60000,
  enabled: false,
});
await invoke("reorder_provider_chain", {
  order: [
    "anthropic",
    "ollama",
    "open_router",
    "open_ai",
    "deep_seek",
    "custom",
    "synthetic_offline",
  ],
});

provider.close();
ws.close();
const failed = results.filter((r) => !r.pass);
console.log(`\n${results.length - failed.length}/${results.length} PASS`);
process.exit(failed.length === 0 ? 0 : 1);
