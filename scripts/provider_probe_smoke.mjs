#!/usr/bin/env node
// Provider probe smoke — AI provider settings verification.
//
// Drives the *running desktop application* through
// `window.__TAURI_INTERNALS__.invoke` — the same bridge the React settings
// panel uses — and checks the one claim the panel now makes: that
// "Test connection" sends a real request and reports what the provider
// actually said.
//
// Nothing here reaches the internet. The "connected" case is answered by a
// loopback HTTP server this script starts itself, so the check proves the whole
// path (IPC argument mapping -> probe -> HTTP client -> classification ->
// serialised result) without a provider account and without egress.
//
// Preconditions (same as scripts/desktop_ipc_smoke.mjs):
//
//   1. cargo build -p zylcode-desktop
//   2. pnpm --filter zylcode-desktop dev      # dev-mode build loads devUrl :1420
//   3. launch with no ANTHROPIC_API_KEY in the environment and
//      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222
//
// Usage:  node scripts/provider_probe_smoke.mjs [port]
// Exits 0 only when every case behaves as asserted.

import http from "node:http";

const port = process.argv[2] ?? "9222";
const base = `http://127.0.0.1:${port}`;

const results = [];
const check = (name, pass, detail) => {
  results.push({ name, pass, detail });
  console.log(`${pass ? "PASS" : "FAIL"}  ${name}${detail ? ` — ${detail}` : ""}`);
};

// ---------------------------------------------------------------------------
// A loopback provider: answers one chat request with an Ollama-shaped body.
// ---------------------------------------------------------------------------
const answerProvider = () =>
  new Promise((resolve) => {
    const server = http.createServer((req, res) => {
      let body = "";
      req.on("data", (c) => (body += c));
      req.on("end", () => {
        res.writeHead(200, { "content-type": "application/json" });
        res.end(
          JSON.stringify({
            message: { role: "assistant", content: "OK" },
            prompt_eval_count: 3,
            eval_count: 1,
          }),
        );
        server.close();
        resolve({ server, received: body });
      });
    });
    server.listen(0, "127.0.0.1", () =>
      resolve({ server, url: `http://127.0.0.1:${server.address().port}` }),
    );
  });

// A port that is bound then released: reliably closed, never reused here.
const closedPortUrl = () =>
  new Promise((resolve) => {
    const s = http.createServer();
    s.listen(0, "127.0.0.1", () => {
      const url = `http://127.0.0.1:${s.address().port}`;
      s.close(() => resolve(url));
    });
  });

// ---------------------------------------------------------------------------
// CDP plumbing (same approach as desktop_ipc_smoke.mjs)
// ---------------------------------------------------------------------------
let targets;
try {
  targets = await (await fetch(`${base}/json/list`)).json();
} catch (err) {
  console.error(`Cannot reach the WebView2 debugger at ${base}.`);
  console.error(String(err));
  process.exit(2);
}

const page = targets.find((t) => t.type === "page");
if (!page) {
  console.error("No page target: the app window is not loaded yet.");
  process.exit(2);
}

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

/** Invoke a Tauri command in the app and return `{ok,value}` or `{ok,error}`. */
const invoke = async (cmd, args) =>
  (
    await rpc("Runtime.evaluate", {
      expression: `(async () => {
        try {
          const v = await window.__TAURI_INTERNALS__.invoke(${JSON.stringify(cmd)}, ${JSON.stringify(args)});
          return { ok: true, value: v };
        } catch (e) {
          return { ok: false, error: String(e) };
        }
      })()`,
      awaitPromise: true,
      returnByValue: true,
      userGesture: true,
    })
  ).result?.result?.value;

// ---------------------------------------------------------------------------
// Cases
// ---------------------------------------------------------------------------

// 1. A real endpoint that answers: connected, with the model and key source
//    the probe used.
{
  const { url } = await answerProvider();
  const r = await invoke("test_provider_connection", {
    kind: "ollama",
    endpoint: url,
    model: "llama3.1",
    timeoutMs: 15000,
  });
  const p = r?.value;
  check(
    "a reachable endpoint reports connected",
    r?.ok === true && p?.status === "connected" && p?.response_chars === 2,
    p ? `${p.status} ${p.latency_ms}ms ${p.endpoint}` : r?.error,
  );
  check(
    "the probe reports the credential source, never a key",
    p?.key_source === "not_required" && !JSON.stringify(r).includes("sk-"),
    String(p?.key_source),
  );
}

// 2. A provider that needs a key and has none: refused before any request.
{
  const r = await invoke("test_provider_connection", { kind: "anthropic" });
  const p = r?.value;
  check(
    "a missing credential is configuration_incomplete, not a failed request",
    p?.status === "configuration_incomplete" && p?.http_status == null,
    p ? `${p.status} key_source=${p.key_source}` : r?.error,
  );
  check(
    "the incompleteness message names the variable to set",
    String(p?.detail ?? "").includes("ANTHROPIC_API_KEY"),
    String(p?.detail ?? "").slice(0, 80),
  );
}

// 3. Nothing listening: a transport failure is its own answer.
{
  const dead = await closedPortUrl();
  const r = await invoke("test_provider_connection", {
    kind: "ollama",
    endpoint: dead,
    timeoutMs: 4000,
  });
  const p = r?.value;
  check(
    "a closed port is network_error",
    p?.status === "network_error" && p?.http_status == null,
    p?.status ?? r?.error,
  );
}

// 4. The synthetic provider never sends anything.
{
  const r = await invoke("test_provider_connection", {
    kind: "synthetic_offline",
  });
  const p = r?.value;
  check(
    "synthetic offline is reported as offline, not as connected",
    p?.status === "offline_provider" && p?.endpoint === "(no network)",
    p?.status ?? r?.error,
  );
}

// 5. The IPC boundary rejects a non-http endpoint before any request is built.
{
  const r = await invoke("test_provider_connection", {
    kind: "ollama",
    endpoint: "file:///etc/passwd",
  });
  check(
    "a non-http endpoint is rejected at the boundary",
    r?.ok === false && /http:\/\/ and https:\/\//.test(String(r?.error)),
    String(r?.error ?? "").slice(0, 100),
  );
}

// 6. An unusable model id is rejected at the boundary.
{
  const r = await invoke("test_provider_connection", {
    kind: "ollama",
    model: "bad model;drop",
  });
  check(
    "an invalid model id is rejected at the boundary",
    r?.ok === false && /model/.test(String(r?.error)),
    String(r?.error ?? "").slice(0, 100),
  );
}

ws.close();
const failed = results.filter((r) => !r.pass);
console.log(`\n${results.length - failed.length}/${results.length} checks passed`);
process.exit(failed.length === 0 ? 0 : 1);
