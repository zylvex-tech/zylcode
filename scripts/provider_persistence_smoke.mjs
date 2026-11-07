#!/usr/bin/env node
// Provider persistence + credential smoke — the two claims the settings panel
// cannot make on its own:
//
//   1. A saved configuration is what the *next* process uses. Saving writes a
//      file, and the restart in the middle of this script is the only honest
//      way to test that.
//   2. A key stored through the panel reaches a real request, lives in the OS
//      credential store, and appears in no IPC payload and no settings file.
//
// Runs in two phases around a real restart of the desktop application:
//
//   node scripts/provider_persistence_smoke.mjs before   # save + store + probe
//   <kill and relaunch the app>
//   node scripts/provider_persistence_smoke.mjs after    # survives + cleans up
//
// The provider is a loopback HTTP server this script owns, on a fixed port so
// phase 2 can serve the same saved endpoint. No egress, no provider account.
//
// Env: ZYLCODE_CONFIG_DIR must match the directory the app was launched with,
//      so this script inspects the same providers.json the app writes.

import http from "node:http";
import fs from "node:fs";
import path from "node:path";

const phase = process.argv[2];
const cdpPort = process.argv[3] ?? "9222";
const providerPort = Number(process.env.SMOKE_PROVIDER_PORT ?? 32123);
const KEY = "sk-smoke-provider-key-9876";
const MODEL = "smoke-model-1";
const ENDPOINT = `http://127.0.0.1:${providerPort}`;

if (!["before", "after"].includes(phase)) {
  console.error("usage: node scripts/provider_persistence_smoke.mjs <before|after> [cdpPort]");
  process.exit(2);
}

const configDir =
  process.env.ZYLCODE_CONFIG_DIR ??
  path.join(process.env.APPDATA ?? path.join(process.env.HOME ?? ".", ".config"), "ZylCode");
const settingsFile = path.join(configDir, "providers.json");

const results = [];
const check = (name, pass, detail) => {
  results.push({ name, pass, detail });
  console.log(`${pass ? "PASS" : "FAIL"}  ${name}${detail ? ` — ${detail}` : ""}`);
};

// ---------------------------------------------------------------------------
// The provider under test: records what it was sent, answers OpenAI-shaped.
// ---------------------------------------------------------------------------
let lastRequest = null;
const provider = http.createServer((req, res) => {
  let body = "";
  req.on("data", (c) => (body += c));
  req.on("end", () => {
    lastRequest = {
      url: req.url,
      authorization: req.headers.authorization ?? null,
      body,
    };
    res.writeHead(200, { "content-type": "application/json" });
    res.end(
      JSON.stringify({
        choices: [{ message: { role: "assistant", content: "pong" } }],
        usage: { prompt_tokens: 4, completion_tokens: 1 },
      }),
    );
  });
});
await new Promise((resolve, reject) => {
  provider.once("error", reject);
  provider.listen(providerPort, "127.0.0.1", resolve);
});

// ---------------------------------------------------------------------------
// CDP plumbing (same approach as the other desktop smoke scripts)
// ---------------------------------------------------------------------------
const targets = await (await fetch(`http://127.0.0.1:${cdpPort}/json/list`)).json();
const page = targets.find((t) => t.type === "page");
if (!page) {
  console.error("No page target: the app window is not loaded yet.");
  provider.close();
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
// Phase "before": save, store a key, prove the probe uses both.
// ---------------------------------------------------------------------------
if (phase === "before") {
  const chain = await invoke("get_provider_configs");
  check(
    "the chain lists every supported provider",
    chain.ok && chain.value.length === 7,
    `${chain.value?.length} entries`,
  );
  check(
    "OpenAI, DeepSeek and Custom are present but not enabled by default",
    chain.ok &&
      ["open_ai", "deep_seek", "custom"].every((k) => {
        const entry = chain.value.find((c) => c.kind === k);
        return entry && entry.enabled === false;
      }),
  );

  const saved = await invoke("set_provider_config", {
    kind: "custom",
    endpoint: ENDPOINT,
    model: MODEL,
    timeoutMs: 20000,
    enabled: true,
  });
  const custom = saved.value?.find((c) => c.kind === "custom");
  check(
    "saving a Custom endpoint returns the live chain with those values",
    saved.ok && custom?.endpoint === ENDPOINT && custom?.model === MODEL && custom.enabled === true,
    JSON.stringify(custom ?? saved.error),
  );

  const keyResult = await invoke("set_provider_key", {
    kind: "custom",
    apiKey: KEY,
  });
  const customKey = keyResult.value?.find((k) => k.kind === "custom");
  check(
    "the stored key is reported by source, not by value",
    keyResult.ok && customKey?.source === "credential_store" && customKey?.configured === true,
    JSON.stringify(customKey ?? keyResult.error),
  );
  check(
    "the masked hint reveals at most four characters",
    customKey?.hint === "…9876",
    String(customKey?.hint),
  );
  check(
    "no key appears in the credential-status payload",
    !JSON.stringify(keyResult.value ?? "").includes(KEY),
  );

  const status = await invoke("provider_key_status");
  check("no key appears in any status payload", !JSON.stringify(status.value ?? "").includes(KEY));

  const probe = await invoke("test_provider_connection", {
    kind: "custom",
    endpoint: null,
    model: null,
    timeoutMs: null,
    saved: true,
  });
  check(
    "the saved configuration connects (probe reports connected)",
    probe.ok && probe.value?.status === "connected",
    `${probe.value?.status ?? probe.error}`,
  );
  check("the probe used the saved model", probe.value?.model === MODEL, String(probe.value?.model));
  check(
    "the stored key reached the request as a bearer credential",
    lastRequest?.authorization === `Bearer ${KEY}`,
    String(lastRequest?.authorization),
  );

  const onDisk = fs.existsSync(settingsFile) ? fs.readFileSync(settingsFile, "utf8") : "";
  check("the settings file exists after saving", onDisk.length > 0, settingsFile);
  check(
    "the settings file holds the endpoint and model",
    onDisk.includes(ENDPOINT) && onDisk.includes(MODEL),
  );
  check(
    "the settings file holds no API key",
    !onDisk.includes(KEY) && !onDisk.includes("sk-smoke"),
  );

  const chainPayload = JSON.stringify(chain.value ?? "");
  check("no key appears in the provider-chain payload", !chainPayload.includes(KEY));
}

// ---------------------------------------------------------------------------
// Phase "after": the restart must not have lost anything.
// ---------------------------------------------------------------------------
if (phase === "after") {
  const chain = await invoke("get_provider_configs");
  const custom = chain.value?.find((c) => c.kind === "custom");
  check(
    "the saved Custom endpoint and model survived the restart",
    chain.ok && custom?.endpoint === ENDPOINT && custom?.model === MODEL && custom.enabled === true,
    JSON.stringify(custom ?? chain.error),
  );
  check(
    "the saved order survived the restart",
    chain.ok &&
      chain.value.every((c, i, all) => i === 0 || all[i - 1].fallback_order <= c.fallback_order),
  );

  const status = await invoke("provider_key_status");
  const customKey = status.value?.find((k) => k.kind === "custom");
  check(
    "the key is still in the OS credential store after the restart",
    status.ok && customKey?.source === "credential_store" && customKey?.configured === true,
    JSON.stringify(customKey ?? status.error),
  );
  check(
    "the restarted app returns no key over IPC",
    !JSON.stringify(status.value ?? "").includes(KEY),
  );

  const probe = await invoke("test_provider_connection", {
    kind: "custom",
    endpoint: null,
    model: null,
    timeoutMs: null,
    saved: true,
  });
  check(
    "a saved probe after the restart connects with the restored key",
    probe.ok && probe.value?.status === "connected",
    `${probe.value?.status ?? probe.error}`,
  );
  check(
    "the restored key reached the provider",
    lastRequest?.authorization === `Bearer ${KEY}`,
    String(lastRequest?.authorization),
  );

  // Cleanup: remove the credential and reset the provider this script enabled.
  const removed = await invoke("delete_provider_key", { kind: "custom" });
  const afterDelete = removed.value?.find((k) => k.kind === "custom");
  check(
    "deleting the key removes it from the store",
    removed.ok && afterDelete?.source === "none" && afterDelete?.configured === false,
    JSON.stringify(afterDelete ?? removed.error),
  );
  const reset = await invoke("set_provider_config", {
    kind: "custom",
    endpoint: "",
    model: "",
    timeoutMs: 60000,
    enabled: false,
  });
  const cleaned = reset.value?.find((c) => c.kind === "custom");
  check(
    "the Custom entry is reset to its default state",
    reset.ok && cleaned?.endpoint === "" && cleaned?.model === "" && cleaned.enabled === false,
    JSON.stringify(cleaned ?? reset.error),
  );
  // A second delete is a no-op, not an error.
  const again = await invoke("delete_provider_key", { kind: "custom" });
  check("deleting an absent key is idempotent", again.ok === true, String(again.error ?? ""));
}

provider.close();
ws.close();

const failed = results.filter((r) => !r.pass);
console.log(`\n${results.length - failed.length}/${results.length} PASS  phase=${phase}`);
process.exit(failed.length === 0 ? 0 : 1);
