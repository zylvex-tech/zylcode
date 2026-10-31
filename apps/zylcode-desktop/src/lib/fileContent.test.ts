import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fetchFileContent, type FileContentResult } from "./fileContent";
import { clearDiagnostics, getDiagnostics } from "./runtime";

// ---------------------------------------------------------------------------
// File content service (lib/fileContent.ts).
//
// The guarantee under test: the editor never fabricates content. A missing
// file, an HTTP failure or a transport error all resolve to `unavailable`
// with a reason — never to an empty `ready` document, and never to a throw.
// The requested path is also URI-encoded, so a path containing `&`, `?` or a
// space cannot smuggle a second query parameter into the request.
// ---------------------------------------------------------------------------

const SAMPLE: FileContentResult = {
  path: "src/lib/runtime.ts",
  size: 2048,
  lines: 120,
  lossy: false,
  content: "export function detectEnvironment() {}\n",
};

// Both transports are exercised: the service path in the runner's TEST
// environment, the Tauri IPC path with the environment stubbed to desktop.
const env = { value: "TEST" };

vi.mock("./runtime", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./runtime")>();
  return { ...actual, detectEnvironment: () => env.value };
});

function jsonResponse(body: unknown, status = 200): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  } as Response;
}

describe("fetchFileContent", () => {
  const fetchMock = vi.fn();

  beforeEach(() => {
    env.value = "TEST";
    clearDiagnostics();
    fetchMock.mockReset();
    vi.stubGlobal("fetch", fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("returns ready with the content the service returned", async () => {
    fetchMock.mockResolvedValue(jsonResponse(SAMPLE));

    const state = await fetchFileContent("src/lib/runtime.ts");

    expect(state).toEqual({ kind: "ready", data: SAMPLE });
  });

  it("URI-encodes the path so query metacharacters cannot break the request", async () => {
    fetchMock.mockResolvedValue(jsonResponse(SAMPLE));

    await fetchFileContent("docs/a b&c=d.ts");

    const url = fetchMock.mock.calls[0][0] as string;
    expect(url).toBe("/api/file-content?path=docs%2Fa%20b%26c%3Dd.ts");
    expect(url).not.toContain("&c=d");
  });

  it("reports an HTTP error status as unavailable, not as an empty file", async () => {
    fetchMock.mockResolvedValue(jsonResponse({}, 404));

    const state = await fetchFileContent("missing.ts");

    expect(state.kind).toBe("unavailable");
    if (state.kind === "unavailable") {
      expect(state.reason).toContain("404");
    }
    // The critical invariant: no fabricated ready state.
    expect(state).not.toHaveProperty("data");
  });

  it("unwraps the service's error envelope into the reason", async () => {
    fetchMock.mockResolvedValue(jsonResponse({ error: "file is a directory" }));

    const state = await fetchFileContent("src");

    expect(state).toEqual({ kind: "unavailable", reason: "file is a directory" });
  });

  it("resolves a rejected fetch instead of propagating the rejection", async () => {
    fetchMock.mockRejectedValue(new Error("ECONNREFUSED"));

    const state = await fetchFileContent("src/lib/runtime.ts");

    expect(state).toEqual({ kind: "unavailable", reason: "ECONNREFUSED" });
  });

  it("records a diagnostic for each swallowed failure", async () => {
    fetchMock.mockRejectedValue(new Error("nope"));
    await fetchFileContent("a.ts");
    await fetchFileContent("b.ts");

    expect(getDiagnostics().map((d) => d.source)).toEqual(["fileContent", "fileContent"]);
  });

  it("normalises a non-Error rejection into a readable reason", async () => {
    fetchMock.mockRejectedValue({ weird: true });

    const state = await fetchFileContent("a.ts");

    expect(state.kind).toBe("unavailable");
    if (state.kind === "unavailable") {
      expect(state.reason).toBe("[object Object]");
    }
  });

  it("reads the file through the file_content IPC command on desktop", async () => {
    env.value = "TAURI_DESKTOP";
    const invoke = vi.fn().mockResolvedValue(SAMPLE);
    vi.doMock("@tauri-apps/api/core", () => ({ invoke }));

    const state = await fetchFileContent("src/lib/runtime.ts");

    expect(invoke).toHaveBeenCalledWith("file_content", { path: "src/lib/runtime.ts" });
    expect(state).toEqual({ kind: "ready", data: SAMPLE });
    expect(fetchMock).not.toHaveBeenCalled();

    vi.doUnmock("@tauri-apps/api/core");
  });

  it("degrades to unavailable when the desktop engine rejects", async () => {
    env.value = "TAURI_DESKTOP";
    const invoke = vi.fn().mockRejectedValue(new Error("engine offline"));
    vi.doMock("@tauri-apps/api/core", () => ({ invoke }));

    const state = await fetchFileContent("src/lib/runtime.ts");

    expect(state).toEqual({ kind: "unavailable", reason: "Error: engine offline" });
    expect(getDiagnostics().map((d) => d.source)).toEqual(["fileContent"]);

    vi.doUnmock("@tauri-apps/api/core");
  });
});
