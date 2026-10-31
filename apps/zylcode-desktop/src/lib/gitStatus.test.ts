import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fetchGitStatus, type GitStatusResult } from "./gitStatus";

// ---------------------------------------------------------------------------
// Git source-control service (lib/gitStatus.ts).
//
// Guarantee under test: the UI never invents a tree state. Any transport
// failure resolves to a controlled `unavailable` reason that tells the user
// how to get the data back (`start zylcode serve-intel`), rather than to a
// fabricated clean/dirty answer. The success path is tagged with which
// transport produced it so Diagnostics can tell them apart.
// ---------------------------------------------------------------------------

const SAMPLE: GitStatusResult = {
  branch: "feat/home-mission-pipeline",
  upstream: "origin/feat/home-mission-pipeline",
  ahead: 2,
  behind: 0,
  head: "43f11ed",
  clean: false,
  counts: { total: 3, staged: 0, unstaged: 3, untracked: 0 },
  entries: [{ status: " M", path: "README.md" }],
  diffstat: { files_changed: 3, insertions: 12, deletions: 4, rows: [] },
};

function jsonResponse(body: unknown, status = 200): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  } as Response;
}

// Both transports are exercised: the service path in the runner's TEST
// environment, the Tauri IPC path with the environment stubbed to desktop.
const env = { value: "TEST" };

vi.mock("./runtime", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./runtime")>();
  return { ...actual, detectEnvironment: () => env.value };
});

describe("fetchGitStatus", () => {
  const fetchMock = vi.fn();

  beforeEach(() => {
    env.value = "TEST";
    fetchMock.mockReset();
    vi.stubGlobal("fetch", fetchMock);
  });

  afterEach(() => vi.unstubAllGlobals());

  it("returns the service payload tagged with its transport", async () => {
    fetchMock.mockResolvedValue(jsonResponse(SAMPLE));

    const state = await fetchGitStatus();

    expect(state).toEqual({ kind: "ready", data: SAMPLE, via: "service" });
    expect(fetchMock).toHaveBeenCalledWith("/api/git/status", {
      headers: { Accept: "application/json" },
    });
  });

  it("treats the service's error envelope as unavailable", async () => {
    fetchMock.mockResolvedValue(jsonResponse({ error: "not a git repository" }));

    expect(await fetchGitStatus()).toEqual({
      kind: "unavailable",
      reason: "start `zylcode serve-intel` to see live source-control data",
    });
  });

  it("treats an HTTP failure as unavailable", async () => {
    fetchMock.mockResolvedValue(jsonResponse({}, 500));

    const state = await fetchGitStatus();
    expect(state.kind).toBe("unavailable");
    if (state.kind === "unavailable") {
      expect(state.reason).toBe("start `zylcode serve-intel` to see live source-control data");
    }
  });

  it("treats a rejected fetch as unavailable", async () => {
    fetchMock.mockRejectedValue(new Error("offline"));

    const state = await fetchGitStatus();
    expect(state.kind).toBe("unavailable");
  });

  it("records a diagnostic for every failure it swallows", async () => {
    fetchMock.mockResolvedValue(jsonResponse({}, 500));
    await fetchGitStatus();

    const { clearDiagnostics, getDiagnostics } = await import("./runtime");
    clearDiagnostics();
    fetchMock.mockRejectedValue(new Error("offline"));
    await fetchGitStatus();

    expect(getDiagnostics().map((d) => d.source)).toEqual(["gitStatus"]);
  });

  it("does not read an error envelope as a git status", async () => {
    // `error` present-but-null still identifies the payload as an error
    // envelope, so this must not become a ready state with an empty branch.
    fetchMock.mockResolvedValue(jsonResponse({ error: null }));

    expect((await fetchGitStatus()).kind).toBe("unavailable");
  });

  it("reads the tree through the git_status IPC command on desktop", async () => {
    env.value = "TAURI_DESKTOP";
    const invoke = vi.fn().mockResolvedValue(SAMPLE);
    vi.doMock("@tauri-apps/api/core", () => ({ invoke }));

    const state = await fetchGitStatus();

    expect(invoke).toHaveBeenCalledWith("git_status");
    expect(state).toEqual({ kind: "ready", data: SAMPLE, via: "desktop" });
    expect(fetchMock).not.toHaveBeenCalled();

    vi.doUnmock("@tauri-apps/api/core");
  });

  it("degrades to a desktop-specific reason when the engine cannot read git", async () => {
    env.value = "TAURI_DESKTOP";
    const invoke = vi.fn().mockRejectedValue(new Error("not a repo"));
    vi.doMock("@tauri-apps/api/core", () => ({ invoke }));

    const state = await fetchGitStatus();

    expect(state).toEqual({
      kind: "unavailable",
      reason: "desktop engine could not read the git state",
    });

    vi.doUnmock("@tauri-apps/api/core");
  });
});
