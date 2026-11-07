#!/usr/bin/env node
// Desktop IPC smoke — R-05.
//
// Drives the *running desktop application*, through the same
// `window.__TAURI_INTERNALS__.invoke` bridge the React UI uses, and checks that
// the input-validation boundary rejects malformed input before any work starts
// while still running valid input. It is not a unit test: the point is the real
// app process, the real webview, the real command handler and the real child
// process (or its absence).
//
// Preconditions (see the R-05 row in
// docs/governance/DATAFACTOR_IMPROVEMENT_REGISTER.md):
//
//   1. cargo build -p zylcode-desktop          # debug binary, embeds ../dist
//   2. pnpm --filter zylcode-desktop dev       # only needed by a dev-mode build,
//                                              # which loads devUrl :1420
//   3. WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222 \
//        ./target/debug/zylcode-desktop.exe
//
// Usage:  node scripts/desktop_ipc_smoke.mjs [port]
// Exits 0 only when every case behaves as asserted.
//
// Windows-only in practice: the assertions name PowerShell, the `zylcode $`
// prompt and `C:/Windows`. On another platform the valid-input case still works
// and the absolute-outside case must be repointed at its own system directory.

const port = process.argv[2] ?? "9222";
const base = `http://127.0.0.1:${port}`;

// The command is deliberately a file write: if validation leaked and a child
// process ran, the sentinel appears and the test fails on the filesystem rather
// than on a log line.
const SENTINEL = "zyl-ipc-smoke-sentinel.txt";
const SENTINEL_CMD = `Set-Content -Path "$env:TEMP\\${SENTINEL}" -Value ran`;

let targets;
try {
  targets = await (await fetch(`${base}/json/list`)).json();
} catch (err) {
  console.error(`Cannot reach the WebView2 debugger at ${base}.`);
  console.error(
    "Start the app with WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222",
  );
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

const evaluate = async (expression) => {
  const res = await rpc("Runtime.evaluate", {
    expression,
    awaitPromise: true,
    returnByValue: true,
    userGesture: true,
  });
  if (res.result?.exceptionDetails)
    return { threw: res.result.exceptionDetails.text };
  return res.result?.result?.value;
};

// The shape the frontend uses (apps/zylcode-desktop/src/lib/terminal.ts):
// invoke("terminal_exec", { request: toSnake(req) }).
const invoke = (command, args) => `(async () => {
  try {
    const value = await window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${args});
    return { ok: true, value };
  } catch (err) {
    return { ok: false, error: String(err && err.message ? err.message : err) };
  }
})()`;

const cases = [
  {
    label: "terminal_exec accepts a valid command in the workspace",
    command: "terminal_exec",
    args: { request: { command: "echo ZYL_SMOKE_OK", cwd: "." } },
    assert: (result) =>
      result.ok === true &&
      result.value?.exit_code === 0 &&
      String(result.value?.stdout_tail ?? "").includes("ZYL_SMOKE_OK"),
    describe: () => "expected exit_code 0 with ZYL_SMOKE_OK on stdout",
  },
  {
    label: 'terminal_exec rejects cwd ".." and spawns nothing',
    command: "terminal_exec",
    args: { request: { command: SENTINEL_CMD, cwd: ".." } },
    assert: (result) =>
      result.ok === false &&
      /must stay inside the permitted workspace/.test(result.error),
    describe: () => "expected the confinement error, no child process",
  },
  {
    label: "terminal_exec rejects an absolute path outside the workspace",
    command: "terminal_exec",
    args: { request: { command: "echo nope", cwd: "C:/Windows" } },
    assert: (result) =>
      result.ok === false &&
      /must stay inside the permitted workspace/.test(result.error),
    describe: () => "expected the confinement error",
  },
  {
    label: 'terminal_exec rejects "../.."',
    command: "terminal_exec",
    args: { request: { command: "echo nope", cwd: "../.." } },
    assert: (result) =>
      result.ok === false &&
      /must stay inside the permitted workspace/.test(result.error),
    describe: () => "expected the confinement error",
  },
  {
    label: "terminal_exec rejects a blank command",
    command: "terminal_exec",
    args: { request: { command: "   ", cwd: "." } },
    assert: (result) =>
      result.ok === false && /must not be empty/.test(result.error),
    describe: () => 'expected "must not be empty"',
  },
  {
    label: "terminal_exec rejects a 5000-character command",
    command: "terminal_exec",
    args: { request: { command: "x".repeat(5000), cwd: "." } },
    assert: (result) =>
      result.ok === false && /the limit is 4096/.test(result.error),
    describe: () => "expected the 4096-character limit",
  },
  {
    label: "terminal_exec rejects an unsupported shell mode",
    command: "terminal_exec",
    args: { request: { command: "echo hi", shell: "bash -c evil" } },
    assert: (result) =>
      result.ok === false && /unsupported shell mode/.test(result.error),
    describe: () => 'expected "unsupported shell mode"',
  },
  {
    label: "terminal_exec rejects a zero timeout",
    command: "terminal_exec",
    args: { request: { command: "echo hi", timeout_secs: 0 } },
    assert: (result) =>
      result.ok === false && /at least 1 second/.test(result.error),
    describe: () => 'expected "at least 1 second"',
  },
  {
    label: "package_start rejects a version with path separators",
    command: "package_start",
    args: { version: "1/../x" },
    assert: (result) =>
      result.ok === false &&
      /path separators are not allowed/.test(result.error),
    describe: () => 'expected "path separators are not allowed"',
  },
  {
    label: "marketplace_search rejects a blank query",
    command: "marketplace_search",
    args: { query: "  " },
    assert: (result) =>
      result.ok === false && /must not be empty/.test(result.error),
    describe: () => 'expected "must not be empty"',
  },
  {
    label: "marketplace_search still accepts a real query",
    command: "marketplace_search",
    args: { query: "git" },
    assert: (result) => result.ok === true && Array.isArray(result.value),
    describe: () => "expected an array (may be empty) and no validation error",
  },
  {
    label: "register_mcp_bridge rejects a non-http endpoint",
    command: "register_mcp_bridge",
    args: { id: "smoke", endpoint: "file:///C:/Windows/System32/cmd.exe" },
    assert: (result) =>
      result.ok === false && /only http:\/\/ and https:\/\//.test(result.error),
    describe: () => "expected the scheme error",
  },
  {
    label: "register_mcp_bridge rejects an id with a space",
    command: "register_mcp_bridge",
    args: { id: "a b", endpoint: "http://127.0.0.1:9999" },
    assert: (result) =>
      result.ok === false && /unexpected character/.test(result.error),
    describe: () => "expected the character error",
  },
];

console.log(`Desktop IPC smoke against ${page.url} (${page.title})`);
let failures = 0;
for (const testCase of cases) {
  const result = await evaluate(
    invoke(testCase.command, JSON.stringify(testCase.args)),
  );
  const passed = testCase.assert(result);
  if (!passed) failures++;
  const detail = result.ok
    ? `ok: ${JSON.stringify(result.value).slice(0, 120)}`
    : `rejected: ${result.error}`;
  console.log(
    `${passed ? "PASS" : "FAIL"}  ${testCase.label}\n        ${detail}`,
  );
  if (!passed) console.log(`        ${testCase.describe()}`);
}

// The rejected sentinel write must not have happened. The file lives in the
// app's TEMP, which is also this process's TEMP on the same machine.
const { existsSync } = await import("node:fs");
const { join } = await import("node:path");
const sentinelPath = join(process.env.TEMP ?? process.env.TMP ?? ".", SENTINEL);
const leaked = existsSync(sentinelPath);
if (leaked) {
  failures++;
  console.log(
    `FAIL  rejected cwd ".." ran its command anyway (${sentinelPath} exists)`,
  );
} else {
  console.log(
    'PASS  rejected cwd ".." left no sentinel — no child process ran',
  );
}
ws.close();

console.log(
  failures === 0
    ? `\nALL PASSED (${cases.length + 1} checks)`
    : `\n${failures} FAILED`,
);
process.exit(failures === 0 ? 0 : 1);
