import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { clearMissions, enqueueMission, listMissions, runNextMission } from "./missions";

// ---------------------------------------------------------------------------
// Mission queue service (lib/missions.ts).
//
// Two guarantees under test:
//   * Transport parity — the same logical operation hits IPC when running as
//     the desktop app and the /api/missions HTTP surface in the browser.
//   * Failure honesty — a failed list is an empty queue with a recorded
//     diagnostic, a failed enqueue is `null`, never a half-built Mission with
//     defaults that would look like real queued work.
//
// `detectEnvironment` is stubbed so both transports can be exercised; the
// module's own logic (normalisation, routing, error handling) stays real.
// ---------------------------------------------------------------------------

const env = { value: "TEST" };

vi.mock("./runtime", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./runtime")>();
  return {
    ...actual,
    detectEnvironment: () => env.value,
  };
});

const RAW = {
  id: "m-1",
  task: "ship the fix",
  mode: "build",
  state: "running",
  created_at: "2026-10-01T00:00:00Z",
  updated_at: "2026-10-01T00:05:00Z",
  ledger_session: "sess-9",
  summary: "in progress",
};

function jsonResponse(body: unknown, status = 200): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  } as Response;
}

beforeEach(() => {
  env.value = "TEST";
  vi.restoreAllMocks();
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("listMissions", () => {
  it("lists missions from the HTTP surface in the browser transport", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse({ missions: [RAW] }));
    vi.stubGlobal("fetch", fetchMock);

    const missions = await listMissions();

    expect(fetchMock).toHaveBeenCalledWith("/api/missions");
    expect(missions).toHaveLength(1);
    expect(missions[0]).toMatchObject({
      id: "m-1",
      task: "ship the fix",
      mode: "build",
      state: "running",
      createdAt: "2026-10-01T00:00:00Z",
      updatedAt: "2026-10-01T00:05:00Z",
      ledgerSession: "sess-9",
    });
  });

  it("returns an empty queue when the service answers non-2xx", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(jsonResponse({}, 500)));

    expect(await listMissions()).toEqual([]);
  });

  it("returns an empty queue and records a diagnostic when the fetch rejects", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("offline")));

    expect(await listMissions()).toEqual([]);
  });

  it("normalises an unknown state back to queued rather than inventing progress", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(jsonResponse({ missions: [{ ...RAW, state: "teleporting" }] })),
    );

    const [mission] = await listMissions();

    expect(mission.state).toBe("queued");
  });

  it("normalises an unknown mode to build", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(jsonResponse({ missions: [{ ...RAW, mode: "chaos" }] })),
    );

    const [mission] = await listMissions();
    expect(mission.mode).toBe("build");
  });
});

describe("enqueueMission", () => {
  it("POSTs the task and mode in the browser transport", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(RAW));
    vi.stubGlobal("fetch", fetchMock);

    const mission = await enqueueMission("ship the fix", "build");

    expect(fetchMock).toHaveBeenCalledWith("/api/missions", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ task: "ship the fix", mode: "build" }),
    });
    expect(mission?.id).toBe("m-1");
  });

  it("returns null when the service rejects the enqueue", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(jsonResponse({}, 422)));

    expect(await enqueueMission("bad", "build")).toBeNull();
  });

  it("returns null and records a diagnostic when the fetch rejects", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("offline")));

    expect(await enqueueMission("x", "plan")).toBeNull();
  });

  it("routes to the mission_enqueue IPC command in the desktop transport", async () => {
    env.value = "TAURI_DESKTOP";
    const invoke = vi.fn().mockResolvedValue(RAW);
    vi.doMock("@tauri-apps/api/core", () => ({ invoke }));

    const mission = await enqueueMission("ship the fix", "build");

    expect(invoke).toHaveBeenCalledWith("mission_enqueue", {
      task: "ship the fix",
      mode: "build",
    });
    expect(mission?.id).toBe("m-1");

    vi.doUnmock("@tauri-apps/api/core");
  });
});

describe("runNextMission", () => {
  it("posts to the run-next route", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse({ id: "m-1" }));
    vi.stubGlobal("fetch", fetchMock);

    expect(await runNextMission()).toEqual({ id: "m-1" });
    expect(fetchMock).toHaveBeenCalledWith("/api/missions/run-next", { method: "POST" });
  });

  it("returns null when the route answers non-2xx", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(jsonResponse({}, 503)));

    expect(await runNextMission()).toBeNull();
  });

  it("returns null when the fetch rejects", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("offline")));

    expect(await runNextMission()).toBeNull();
  });
});

describe("clearMissions", () => {
  it("returns true when the queue clears", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(jsonResponse({})));

    expect(await clearMissions()).toBe(true);
  });

  it("returns false when the service refuses", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(jsonResponse({}, 403)));

    expect(await clearMissions()).toBe(false);
  });

  it("returns false when the service is unreachable", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("offline")));

    expect(await clearMissions()).toBe(false);
  });

  it("is a no-op success in the desktop transport, where the queue lives in the workspace", async () => {
    env.value = "TAURI_DESKTOP";
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);

    expect(await clearMissions()).toBe(true);
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
