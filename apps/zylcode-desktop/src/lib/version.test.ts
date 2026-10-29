import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fetchVersion, type VersionInfo } from "./version";
import { clearDiagnostics, getDiagnostics } from "./runtime";

// ---------------------------------------------------------------------------
// Version service (lib/version.ts).
//
// The guarantee under test: `fetchVersion` never throws. Every failure mode —
// HTTP error status, an `{ "error": ... }` payload, a thrown fetch — resolves
// to a controlled `unavailable` state carrying a human-readable reason, and
// records a diagnostic so the raw cause stays reachable from Diagnostics.
//
// The test runner is classified as TEST, so the browser/service path is the
// one exercised here; the Tauri path is gated on an environment this suite
// deliberately does not emulate.
// ---------------------------------------------------------------------------

const SAMPLE: VersionInfo = {
  app_version: "0.2.2",
  engine: "zylcode-core",
  head_commit: "43f11ed",
  branch: "main",
  dirty: false,
};

function jsonResponse(body: unknown, status = 200): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  } as Response;
}

describe("fetchVersion", () => {
  const fetchMock = vi.fn();

  beforeEach(() => {
    clearDiagnostics();
    fetchMock.mockReset();
    vi.stubGlobal("fetch", fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("returns ready with the payload the service returned", async () => {
    fetchMock.mockResolvedValue(jsonResponse(SAMPLE));

    const state = await fetchVersion();

    expect(state).toEqual({ kind: "ready", data: SAMPLE });
  });

  it("requests JSON explicitly so the service does not answer with HTML", async () => {
    fetchMock.mockResolvedValue(jsonResponse(SAMPLE));

    await fetchVersion();

    expect(fetchMock).toHaveBeenCalledWith("/api/version", {
      headers: { Accept: "application/json" },
    });
  });

  it("turns an HTTP error status into an unavailable state naming the status", async () => {
    fetchMock.mockResolvedValue(jsonResponse({}, 503));

    const state = await fetchVersion();

    expect(state.kind).toBe("unavailable");
    if (state.kind === "unavailable") {
      expect(state.reason).toContain("503");
    }
  });

  it("unwraps the service's own error envelope into the reason", async () => {
    fetchMock.mockResolvedValue(jsonResponse({ error: "git version failed" }));

    const state = await fetchVersion();

    expect(state).toEqual({ kind: "unavailable", reason: "git version failed" });
  });

  it("reports a rejected fetch as unavailable rather than throwing", async () => {
    fetchMock.mockRejectedValue(new Error("network down"));

    const state = await fetchVersion();

    expect(state).toEqual({ kind: "unavailable", reason: "network down" });
  });

  it("records a diagnostic for every failure path it swallows", async () => {
    fetchMock.mockResolvedValue(jsonResponse({}, 500));
    await fetchVersion();

    fetchMock.mockRejectedValue(new Error("boom"));
    await fetchVersion();

    const sources = getDiagnostics().map((d) => d.source);
    expect(sources).toEqual(["version", "version"]);
  });

  it("normalises a non-Error rejection into a readable reason", async () => {
    fetchMock.mockRejectedValue("plain string failure");

    const state = await fetchVersion();

    expect(state.kind).toBe("unavailable");
    if (state.kind === "unavailable") {
      expect(state.reason).toBe("plain string failure");
    }
  });
});
