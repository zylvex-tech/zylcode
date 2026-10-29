import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  fetchArtifacts,
  fetchModels,
  fetchProjects,
  fetchProofs,
  type ModelsState,
} from "./foundation";

// ---------------------------------------------------------------------------
// Foundation-wave service views (lib/foundation.ts).
//
// Guarantee under test: these views mirror the Rust contract exactly,
// including its honest states. Two cases carry real weight:
//   * `store_state: "Unavailable"` must become `unavailable`, not a `ready`
//     view showing an empty project list — an empty list would read as "you
//     have no projects", which is a different claim from "the store is
//     broken".
//   * A model capability that is an object rather than a string (`{ Other: .. }`)
//     is normalised to `"other"` instead of `[object Object]` in the table.
// ---------------------------------------------------------------------------

function jsonResponse(body: unknown, status = 200): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  } as Response;
}

describe("foundation service views", () => {
  const fetchMock = vi.fn();

  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal("fetch", fetchMock);
  });

  afterEach(() => vi.unstubAllGlobals());

  describe("fetchProjects", () => {
    it("returns the store state and project list", async () => {
      fetchMock.mockResolvedValue(
        jsonResponse({
          store_state: "Healthy",
          projects: [{ id: "p1", name: "zylcode" }],
        }),
      );

      const state = await fetchProjects();

      expect(state).toEqual({
        kind: "ready",
        storeState: "Healthy",
        projects: [{ id: "p1", name: "zylcode" }],
      });
      expect(fetchMock).toHaveBeenCalledWith("/api/projects", {
        headers: { Accept: "application/json" },
      });
    });

    it("defaults an absent store state rather than crashing", async () => {
      fetchMock.mockResolvedValue(jsonResponse({}));

      expect(await fetchProjects()).toEqual({
        kind: "ready",
        storeState: "Unknown",
        projects: [],
      });
    });

    it("reports an Unavailable store as unavailable, never as an empty project list", async () => {
      fetchMock.mockResolvedValue(
        jsonResponse({ store_state: "Unavailable", error: "corrupt store: bad schema version" }),
      );

      expect(await fetchProjects()).toEqual({
        kind: "unavailable",
        reason: "corrupt store: bad schema version",
      });
    });

    it("falls back to a generic reason when the store gives no detail", async () => {
      fetchMock.mockResolvedValue(jsonResponse({ store_state: "Unavailable" }));

      expect(await fetchProjects()).toEqual({
        kind: "unavailable",
        reason: "project store unavailable",
      });
    });

    it("returns unavailable when the endpoint is unreachable", async () => {
      fetchMock.mockRejectedValue(new Error("offline"));

      expect(await fetchProjects()).toEqual({ kind: "unavailable", reason: "offline" });
    });
  });

  describe("fetchArtifacts", () => {
    it("defaults contract version and count when the payload omits them", async () => {
      fetchMock.mockResolvedValue(jsonResponse({}));

      expect(await fetchArtifacts()).toEqual({
        kind: "ready",
        contractVersion: 1,
        count: 0,
        artifacts: [],
      });
    });

    it("passes the contract version and count through", async () => {
      fetchMock.mockResolvedValue(
        jsonResponse({ contract_version: 3, count: 2, artifacts: [{ id: "a1" }, { id: "a2" }] }),
      );

      const state = await fetchArtifacts();
      expect(state.kind).toBe("ready");
      if (state.kind === "ready") {
        expect(state.contractVersion).toBe(3);
        expect(state.count).toBe(2);
      }
    });

    it("surfaces the service error envelope", async () => {
      fetchMock.mockResolvedValue(jsonResponse({ error: "artifact bus unavailable" }));

      expect(await fetchArtifacts()).toEqual({
        kind: "unavailable",
        reason: "artifact bus unavailable",
      });
    });

    it("surfaces an HTTP failure with its status", async () => {
      fetchMock.mockResolvedValue(jsonResponse({}, 502));

      const state = await fetchArtifacts();
      expect(state.kind).toBe("unavailable");
      if (state.kind === "unavailable") expect(state.reason).toContain("502");
    });
  });

  describe("fetchProofs", () => {
    it("reads chain integrity from the payload", async () => {
      fetchMock.mockResolvedValue(
        jsonResponse({
          schema_version: 2,
          chain_intact: true,
          current_commit: "43f11ed",
          count: 1,
          proofs: [],
        }),
      );

      expect(await fetchProofs()).toEqual({
        kind: "ready",
        schemaVersion: 2,
        chainIntact: true,
        currentCommit: "43f11ed",
        count: 1,
        proofs: [],
      });
    });

    it("does not assert chain integrity that the payload never claimed", async () => {
      fetchMock.mockResolvedValue(jsonResponse({}));

      const state = await fetchProofs();
      expect(state.kind).toBe("ready");
      if (state.kind === "ready") expect(state.chainIntact).toBe(false);
    });

    it("returns unavailable with a proof-engine-specific reason", async () => {
      fetchMock.mockRejectedValue(new Error("no ledger"));

      expect(await fetchProofs()).toEqual({ kind: "unavailable", reason: "no ledger" });
    });
  });

  describe("fetchModels", () => {
    it("normalises capability cells and availability flags", async () => {
      fetchMock.mockResolvedValue(
        jsonResponse({
          contract: "model-capability/1",
          note: "verified locally",
          models: [
            {
              model_id: "claude-3.5-sonnet",
              provider: "anthropic",
              text_input: "Supported",
              image_input: "Unsupported",
              file_input: { Other: "beta" },
              vision_interpretation: "Unsupported",
              structured_output: "Supported",
              tool_calling: "Supported",
              streaming: "Supported",
              local_available: false,
              provider_available: true,
              auth_state: "key_present",
              quota_state: "ok",
              runtime_verified: true,
            },
          ],
        }),
      );

      const state: ModelsState = await fetchModels();

      expect(state.kind).toBe("ready");
      if (state.kind === "ready") {
        expect(state.contract).toBe("model-capability/1");
        const row = state.models[0];
        expect(row.model_id).toBe("claude-3.5-sonnet");
        expect(row.text_input).toBe("Supported");
        // An `Other(..)` capability must not render as "[object Object]".
        expect(row.file_input).toBe("other");
        expect(row.local_available).toBe(false);
        expect(row.provider_available).toBe(true);
        expect(row.runtime_verified).toBe(true);
      }
    });

    it("fills in defaults for a sparse model row", async () => {
      fetchMock.mockResolvedValue(jsonResponse({ models: [{}] }));

      const state = await fetchModels();
      expect(state.kind).toBe("ready");
      if (state.kind === "ready") {
        expect(state.models[0]).toEqual({
          model_id: "?",
          provider: "?",
          text_input: "other",
          image_input: "other",
          file_input: "other",
          vision_interpretation: "other",
          structured_output: "other",
          tool_calling: "other",
          streaming: "other",
          local_available: false,
          provider_available: false,
          auth_state: "unknown",
          quota_state: "unknown",
          runtime_verified: false,
        });
      }
    });

    it("returns an empty registry when no models are published", async () => {
      fetchMock.mockResolvedValue(jsonResponse({}));

      const state = await fetchModels();
      expect(state.kind).toBe("ready");
      if (state.kind === "ready") expect(state.models).toEqual([]);
    });

    it("returns unavailable on failure", async () => {
      fetchMock.mockResolvedValue(jsonResponse({ error: "registry not built" }));

      expect(await fetchModels()).toEqual({
        kind: "unavailable",
        reason: "registry not built",
      });
    });
  });
});
