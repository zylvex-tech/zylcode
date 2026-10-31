import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  fetchBuildState,
  fetchDeploy,
  startBuild,
  startPackage,
  type BuildState,
  type DeployState,
} from "./delivery";

// ---------------------------------------------------------------------------
// Delivery service client (lib/delivery.ts).
//
// Guarantees under test:
//   * Every failure mode — non-2xx, an `error` payload, a rejected fetch —
//     resolves to `unavailable` with a reason. Nothing throws to the caller.
//   * Raw payloads are normalised into the typed views: an absent target
//     becomes an explicit UNCOMMISSIONED view rather than `undefined`, and a
//     non-numeric `exit_code` becomes `null` rather than a string the UI would
//     render as if it were a code.
//   * Mutations (startBuild/startPackage) report `started: false` on failure
//     instead of rejecting, so the panel can stay on screen.
// ---------------------------------------------------------------------------

function jsonResponse(body: unknown, status = 200): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  } as Response;
}

describe("fetchDeploy", () => {
  const fetchMock = vi.fn();

  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal("fetch", fetchMock);
  });

  afterEach(() => vi.unstubAllGlobals());

  it("maps a full payload into the typed ready view", async () => {
    fetchMock.mockResolvedValue(
      jsonResponse({
        head: "43f11ed",
        github_release: { commissioned: true, state: "LIVE", note: "v0.2.2" },
        crates_io: { commissioned: false, state: "UNCOMMISSIONED", note: "" },
        remote_server: { commissioned: false, state: "UNCOMMISSIONED", note: "" },
      }),
    );

    const state = await fetchDeploy();

    expect(state.kind).toBe("ready");
    if (state.kind === "ready") {
      expect(state.head).toBe("43f11ed");
      expect(state.github_release).toEqual({
        commissioned: true,
        state: "LIVE",
        note: "v0.2.2",
      });
    }
  });

  it("coerces a missing target into an explicit UNCOMMISSIONED view", async () => {
    fetchMock.mockResolvedValue(jsonResponse({ head: "abc" }));

    const state = await fetchDeploy();

    expect(state.kind).toBe("ready");
    if (state.kind === "ready") {
      expect(state.crates_io).toEqual({ commissioned: false, state: "UNCOMMISSIONED", note: "" });
      expect(state.remote_server).toEqual({
        commissioned: false,
        state: "UNCOMMISSIONED",
        note: "",
      });
    }
  });

  it("treats the service's error envelope as unavailable", async () => {
    fetchMock.mockResolvedValue(jsonResponse({ error: "no repository at workspace" }));

    const state: DeployState = await fetchDeploy();

    expect(state).toEqual({ kind: "unavailable", reason: "no repository at workspace" });
  });

  it("names the HTTP status when the endpoint fails", async () => {
    fetchMock.mockResolvedValue(jsonResponse({}, 500));

    const state = await fetchDeploy();

    expect(state.kind).toBe("unavailable");
    if (state.kind === "unavailable") expect(state.reason).toContain("500");
  });

  it("resolves a rejected fetch rather than propagating it", async () => {
    fetchMock.mockRejectedValue(new Error("ECONNREFUSED"));

    const state = await fetchDeploy();

    expect(state).toEqual({ kind: "unavailable", reason: "ECONNREFUSED" });
  });
});

describe("fetchBuildState", () => {
  const fetchMock = vi.fn();

  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal("fetch", fetchMock);
  });

  afterEach(() => vi.unstubAllGlobals());

  it("normalises steps, keeping only numeric exit codes as numbers", async () => {
    fetchMock.mockResolvedValue(
      jsonResponse({
        state: "running",
        steps: [
          { name: "test", command: "cargo test", passed: true, exit_code: 0, duration_ms: 1200 },
          {
            name: "build",
            command: "cargo build",
            passed: false,
            exit_code: "1",
            duration_ms: null,
          },
          { name: "package", command: "zip", passed: false },
        ],
      }),
    );

    const state: BuildState = await fetchBuildState();

    expect(state.kind).toBe("ready");
    if (state.kind === "ready") {
      expect(state.state).toBe("running");
      expect(state.steps).toHaveLength(3);
      expect(state.steps[0]).toMatchObject({ exit_code: 0, duration_ms: 1200 });
      // A stringified exit code must not survive as if it were a code.
      expect(state.steps[1].exit_code).toBeNull();
      expect(state.steps[2]).toEqual({
        name: "package",
        command: "zip",
        passed: false,
        exit_code: null,
        duration_ms: null,
      });
    }
  });

  it("defaults to idle and an empty step list when the payload is sparse", async () => {
    fetchMock.mockResolvedValue(jsonResponse({}));

    const state = await fetchBuildState();

    expect(state).toEqual({ kind: "ready", state: "idle", steps: [] });
  });

  it("passes through packaged metadata when present and omits it otherwise", async () => {
    fetchMock.mockResolvedValue(
      jsonResponse({
        state: "done",
        passed: true,
        packaged: { version: "0.2.2", file_count: 41 },
      }),
    );
    const withPackaged = await fetchBuildState();

    fetchMock.mockResolvedValue(jsonResponse({ state: "idle" }));
    const withoutPackaged = await fetchBuildState();

    if (withPackaged.kind === "ready") {
      expect(withPackaged.packaged).toEqual({ version: "0.2.2", file_count: 41 });
      expect(withPackaged.passed).toBe(true);
    }
    if (withoutPackaged.kind === "ready") {
      expect(withoutPackaged.packaged).toBeUndefined();
      expect(withoutPackaged.passed).toBeUndefined();
    }
  });

  it("returns unavailable on HTTP failure", async () => {
    fetchMock.mockResolvedValue(jsonResponse({}, 503));

    const state = await fetchBuildState();

    expect(state.kind).toBe("unavailable");
    if (state.kind === "unavailable") expect(state.reason).toContain("503");
  });
});

describe("startBuild / startPackage", () => {
  const fetchMock = vi.fn();

  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal("fetch", fetchMock);
  });

  afterEach(() => vi.unstubAllGlobals());

  it("reports a successful launch acknowledgement", async () => {
    fetchMock.mockResolvedValue(jsonResponse({ started: true, note: "pipeline queued" }));

    expect(await startBuild()).toEqual({
      started: true,
      note: "pipeline queued",
      error: undefined,
    });
  });

  it("reports started:false with the note when a pipeline is already running", async () => {
    fetchMock.mockResolvedValue(jsonResponse({ started: false, note: "already running" }));

    const result = await startBuild();

    expect(result.started).toBe(false);
    expect(result.note).toBe("already running");
  });

  it("captures an HTTP failure as an error instead of rejecting", async () => {
    fetchMock.mockResolvedValue(jsonResponse({}, 409));

    const result = await startBuild();

    expect(result.started).toBe(false);
    expect(result.error).toContain("409");
  });

  it("sends the trimmed version and omits an empty one", async () => {
    fetchMock.mockResolvedValue(jsonResponse({ started: true }));

    await startPackage("  0.2.3  ", false);

    const init = fetchMock.mock.calls[0][1] as RequestInit;
    expect(init.method).toBe("POST");
    expect(JSON.parse(String(init.body))).toEqual({ version: "0.2.3" });
  });

  it("sends package_only only when packaging without a build", async () => {
    fetchMock.mockResolvedValue(jsonResponse({ started: true }));

    await startPackage("0.2.3", true);
    expect(JSON.parse(String((fetchMock.mock.calls[0][1] as RequestInit).body))).toEqual({
      version: "0.2.3",
      package_only: true,
    });

    fetchMock.mockClear();
    fetchMock.mockResolvedValue(jsonResponse({ started: true }));
    await startPackage("", false);
    expect(JSON.parse(String((fetchMock.mock.calls[0][1] as RequestInit).body))).toEqual({});
  });

  it("captures a rejected fetch as an error instead of rejecting", async () => {
    fetchMock.mockRejectedValue(new Error("offline"));

    const result = await startPackage("0.2.3", true);

    expect(result).toEqual({ started: false, error: "offline" });
  });
});

// ---------------------------------------------------------------------------
// Both sides of every optional field. Each `x ? String(x) : undefined` and
// each `a ?? b` in the normalisers has to be exercised: a backend that starts
// sending a field (or stops sending one) must not change the shape of what
// the panels render. Non-`Error` rejections are pinned too, because the
// fallback reason string is what the user sees when transport wraps a failure.
// ---------------------------------------------------------------------------
describe("delivery — optional fields on both sides", () => {
  const fetchMock = vi.fn();

  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal("fetch", fetchMock);
  });

  afterEach(() => vi.unstubAllGlobals());

  it("falls back to an empty head when the service omits one", async () => {
    fetchMock.mockResolvedValue(jsonResponse({}));

    const state = await fetchDeploy();

    expect(state).toMatchObject({ kind: "ready", head: "" });
  });

  it("renders a non-Error deploy failure with the generic reason", async () => {
    fetchMock.mockRejectedValue("transport wrapper string");

    expect(await fetchDeploy()).toEqual({
      kind: "unavailable",
      reason: "deploy status unavailable",
    });
  });

  it("renders a non-Error build failure with the generic reason", async () => {
    fetchMock.mockRejectedValue("transport wrapper string");

    expect(await fetchBuildState()).toEqual({
      kind: "unavailable",
      reason: "build status unavailable",
    });
  });

  it("marks a step with no name or command rather than dropping it", async () => {
    fetchMock.mockResolvedValue(jsonResponse({ state: "running", steps: [{ passed: false }] }));

    const state = await fetchBuildState();

    expect(state.kind).toBe("ready");
    if (state.kind === "ready") {
      expect(state.steps[0]).toEqual({
        name: "?",
        command: "",
        passed: false,
        exit_code: null,
        duration_ms: null,
      });
    }
  });

  it("passes through a fully populated build payload", async () => {
    fetchMock.mockResolvedValue(
      jsonResponse({
        state: "done",
        started_at: "2026-10-30T10:00:00Z",
        finished_at: "2026-10-30T10:04:00Z",
        passed: false,
        error: "cargo test failed",
        note: "attempt 2",
        steps: [{ name: "test", command: "cargo test", passed: false }],
        packaged: {
          release_dir: "dist/",
          artifact_id: "art-1",
          version: "0.2.3",
          content_hash: "abc123",
          file_count: 12,
        },
      }),
    );

    const state = await fetchBuildState();

    expect(state.kind).toBe("ready");
    if (state.kind === "ready") {
      expect(state.started_at).toBe("2026-10-30T10:00:00Z");
      expect(state.finished_at).toBe("2026-10-30T10:04:00Z");
      expect(state.error).toBe("cargo test failed");
      expect(state.note).toBe("attempt 2");
      expect(state.passed).toBe(false);
      expect(state.packaged).toEqual({
        release_dir: "dist/",
        artifact_id: "art-1",
        version: "0.2.3",
        content_hash: "abc123",
        file_count: 12,
      });
    }
  });

  it("leaves build timings and note undefined when the payload omits them", async () => {
    fetchMock.mockResolvedValue(jsonResponse({ state: "idle", packaged: {} }));

    const state = await fetchBuildState();

    expect(state.kind).toBe("ready");
    if (state.kind === "ready") {
      expect(state.started_at).toBeUndefined();
      expect(state.finished_at).toBeUndefined();
      expect(state.error).toBeUndefined();
      expect(state.note).toBeUndefined();
      // `packaged` present but empty: every field degrades to undefined.
      expect(state.packaged).toEqual({});
    }
  });

  it("surfaces the engine's error on a build launch acknowledgement", async () => {
    fetchMock.mockResolvedValue(jsonResponse({ started: false, error: "toolchain missing" }));

    const result = await startBuild();

    expect(result.started).toBe(false);
    expect(result.error).toBe("toolchain missing");
    expect(result.note).toBeUndefined();
  });

  it("stringifies a non-Error launch failure instead of losing it", async () => {
    fetchMock.mockRejectedValue("plain string failure");

    expect(await startBuild()).toEqual({ started: false, error: "plain string failure" });
  });

  it("reports an HTTP failure from the package endpoint as an error", async () => {
    fetchMock.mockResolvedValue(jsonResponse({}, 502));

    const result = await startPackage("0.2.3", false);

    expect(result.started).toBe(false);
    expect(result.error).toContain("HTTP 502");
  });

  it("passes both note and error through on a package acknowledgement", async () => {
    fetchMock.mockResolvedValue(
      jsonResponse({ started: true, note: "packaging", error: "partial upload" }),
    );

    expect(await startPackage("0.2.3", false)).toEqual({
      started: true,
      note: "packaging",
      error: "partial upload",
    });
  });

  it("stringifies a non-Error package failure instead of losing it", async () => {
    fetchMock.mockRejectedValue("plain string failure");

    expect(await startPackage("0.2.3", false)).toEqual({
      started: false,
      error: "plain string failure",
    });
  });
});
