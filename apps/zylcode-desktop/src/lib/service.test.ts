import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fetchMetrics, fetchProviders, fetchTools } from "./service";

// ---------------------------------------------------------------------------
// Service-backed capability views (lib/service.ts).
//
// Guarantee under test: read-only capability views are either `ready` with a
// fully normalised shape (absent collections become empty arrays, absent
// numbers become 0) or `unavailable` with a reason. The panels that render
// these never receive `undefined` where they expect a collection.
// ---------------------------------------------------------------------------

function jsonResponse(body: unknown, status = 200): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  } as Response;
}

describe("capability views", () => {
  const fetchMock = vi.fn();

  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal("fetch", fetchMock);
  });

  afterEach(() => vi.unstubAllGlobals());

  describe("fetchTools", () => {
    it("returns the catalogue with its source", async () => {
      const tools = [
        {
          id: "read_file",
          transport: "native",
          description: "Read a file",
          enabled: true,
          executor_bound: true,
        },
      ];
      fetchMock.mockResolvedValue(jsonResponse({ source: "mcp.tools.yaml", tools }));

      const state = await fetchTools();

      expect(state).toEqual({ kind: "ready", source: "mcp.tools.yaml", tools });
      expect(fetchMock).toHaveBeenCalledWith("/api/tools", {
        headers: { Accept: "application/json" },
      });
    });

    it("defaults the source label and to an empty catalogue", async () => {
      fetchMock.mockResolvedValue(jsonResponse({}));

      const state = await fetchTools();

      expect(state).toEqual({ kind: "ready", source: "mcp.tools.yaml", tools: [] });
    });

    it("surfaces the service's error envelope as the reason", async () => {
      fetchMock.mockResolvedValue(jsonResponse({ error: "mcp.tools.yaml missing" }));

      expect(await fetchTools()).toEqual({
        kind: "unavailable",
        reason: "mcp.tools.yaml missing",
      });
    });

    it("names the HTTP status on a server failure", async () => {
      fetchMock.mockResolvedValue(jsonResponse({}, 502));

      const state = await fetchTools();
      expect(state.kind).toBe("unavailable");
      if (state.kind === "unavailable") expect(state.reason).toContain("502");
    });

    it("resolves a rejected fetch as unavailable", async () => {
      fetchMock.mockRejectedValue(new Error("proxy down"));

      expect(await fetchTools()).toEqual({ kind: "unavailable", reason: "proxy down" });
    });
  });

  describe("fetchProviders", () => {
    it("returns the chain and measured ranking", async () => {
      fetchMock.mockResolvedValue(
        jsonResponse({
          primary_model: "anthropic/claude-3.5-sonnet",
          fallback_model: "llama3.1",
          chain: [{ kind: "openrouter", model: "m", enabled: true }],
          measured_ranking: [{ provider: "openrouter", successes: 3, failures: 1 }],
        }),
      );

      const state = await fetchProviders();

      expect(state.kind).toBe("ready");
      if (state.kind === "ready") {
        expect(state.primary_model).toBe("anthropic/claude-3.5-sonnet");
        expect(state.chain).toHaveLength(1);
        expect(state.measured_ranking[0].provider).toBe("openrouter");
      }
    });

    it("defaults a sparse payload to empty collections", async () => {
      fetchMock.mockResolvedValue(jsonResponse({}));

      const state = await fetchProviders();

      expect(state).toEqual({
        kind: "ready",
        primary_model: "",
        fallback_model: "",
        chain: [],
        measured_ranking: [],
      });
    });

    it("returns unavailable with a reason on failure", async () => {
      fetchMock.mockRejectedValue(new Error("no router"));

      expect(await fetchProviders()).toEqual({ kind: "unavailable", reason: "no router" });
    });
  });

  describe("fetchMetrics", () => {
    it("returns numeric counters", async () => {
      fetchMock.mockResolvedValue(
        jsonResponse({
          input_tokens: 1200,
          output_tokens: 400,
          verification_saved_tokens: 90,
          fallback_count: 2,
        }),
      );

      expect(await fetchMetrics()).toEqual({
        kind: "ready",
        input_tokens: 1200,
        output_tokens: 400,
        verification_saved_tokens: 90,
        fallback_count: 2,
      });
    });

    it("coerces absent counters to 0 so the panel can render them", async () => {
      fetchMock.mockResolvedValue(jsonResponse({}));

      expect(await fetchMetrics()).toEqual({
        kind: "ready",
        input_tokens: 0,
        output_tokens: 0,
        verification_saved_tokens: 0,
        fallback_count: 0,
      });
    });

    it("coerces a non-numeric counter to NaN rather than silently treating it as data", async () => {
      fetchMock.mockResolvedValue(jsonResponse({ input_tokens: "lots" }));

      const state = await fetchMetrics();
      expect(state.kind).toBe("ready");
      if (state.kind === "ready") expect(Number.isNaN(state.input_tokens)).toBe(true);
    });

    it("returns unavailable when the endpoint errors", async () => {
      fetchMock.mockResolvedValue(jsonResponse({ error: "no scorecard" }));

      expect(await fetchMetrics()).toEqual({ kind: "unavailable", reason: "no scorecard" });
    });
  });
});
