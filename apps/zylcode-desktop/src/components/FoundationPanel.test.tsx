import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FoundationPanel } from "./FoundationPanel";
import type {
  ArtifactsState,
  ModelsState,
  ProjectsState,
  ProofsState,
} from "../lib/foundation";

// ---------------------------------------------------------------------------
// Foundation panel (components/FoundationPanel.tsx).
//
// The rules under test — all four persistence systems render their real
// service payload, including the awkward states:
//   * corrupt project store, empty registry (honest ".zylcode/projects.json");
//   * tampered artifacts flagged red, lifecycle labels verbatim;
//   * proofs: chain broken verbatim, "not run"/"blocked" never converted to
//     passes, stale proofs labelled with the commit mismatch;
//   * vision-unverified models flagged, runtime-verified ones badged.
// ---------------------------------------------------------------------------

vi.mock("../lib/foundation", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/foundation")>();
  return {
    ...actual,
    fetchProjects: vi.fn(),
    fetchArtifacts: vi.fn(),
    fetchProofs: vi.fn(),
    fetchModels: vi.fn(),
  };
});

const { fetchProjects, fetchArtifacts, fetchProofs, fetchModels } = await import(
  "../lib/foundation"
);
const proj = vi.mocked(fetchProjects);
const art = vi.mocked(fetchArtifacts);
const prf = vi.mocked(fetchProofs);
const mdl = vi.mocked(fetchModels);

const projectsReady: ProjectsState = {
  kind: "ready",
  storeState: "ok",
  projects: [
    {
      id: "p1",
      name: "zylcode",
      root_path: "C:/Projects/zylcode",
      created_at: "2026-01-01T00:00:00Z",
      last_accessed: "2026-10-28T00:00:00Z",
      schema_version: 1,
    },
  ],
};

const artifactsReady: ArtifactsState = {
  kind: "ready",
  contractVersion: 3,
  count: 2,
  artifacts: [
    {
      id: "art-ok",
      kind: "Design",
      schema_version: 1,
      project_id: "p1",
      producer: "agent",
      content_hash: "h1",
      content_len: 10,
      permissions: "rw",
      provenance: "mission",
      verification_status: "verified",
      retention_policy: "keep",
      lifecycle: "accepted",
      created_at: "2026-10-28T00:00:00Z",
      updated_at: "2026-10-28T00:00:00Z",
      history: [],
    },
    {
      id: "art-bad",
      kind: { Other: "Spec" },
      schema_version: 1,
      project_id: "p1",
      producer: "agent",
      content_hash: "h2",
      content_len: 20,
      permissions: "rw",
      provenance: "mission",
      verification_status: "tampered",
      retention_policy: "keep",
      lifecycle: "validated",
      created_at: "2026-10-28T00:00:00Z",
      updated_at: "2026-10-28T00:00:00Z",
      history: [],
    },
  ],
};

function proof(over: Partial<ProofsState> = {}): ProofsState {
  return {
    kind: "ready",
    schemaVersion: 1,
    chainIntact: true,
    currentCommit: "abc1234",
    count: 3,
    proofs: [
      {
        record: {
          id: "pr1",
          project_id: "p1",
          command: "cargo test",
          source_commit: "abc1234",
          tree_clean: true,
          expected: "0",
          actual: "0",
          exit_status: 0,
          duration_ms: 1200,
          artifact_refs: [],
          acceptance: "accepted",
          verification: "passed",
          source: "runner",
          prev_hash: "0",
          entry_hash: "1",
          created_at: "2026-10-28T00:00:00Z",
        },
        stale: false,
        chain_intact: true,
      },
      {
        record: {
          id: "pr2",
          project_id: "p1",
          command: "pnpm lint",
          source_commit: "old0000",
          tree_clean: true,
          expected: "0",
          artifact_refs: [],
          acceptance: "accepted",
          verification: "not_run",
          source: "runner",
          prev_hash: "1",
          entry_hash: "2",
          created_at: "2026-10-28T00:00:00Z",
        },
        stale: true,
        chain_intact: true,
      },
      {
        record: {
          id: "pr3",
          project_id: "p1",
          command: "cargo clippy",
          source_commit: "abc1234",
          tree_clean: true,
          expected: "0",
          artifact_refs: [],
          acceptance: "accepted",
          verification: "failed",
          source: "runner",
          prev_hash: "2",
          entry_hash: "3",
          created_at: "2026-10-28T00:00:00Z",
        },
        stale: false,
        chain_intact: true,
      },
    ],
    ...over,
  } as ProofsState;
}

const modelsReady: ModelsState = {
  kind: "ready",
  contract: "v1",
  note: "Capability rows are recorded observations.",
  models: [
    {
      model_id: "claude-sonnet",
      provider: "anthropic",
      text_input: "yes",
      image_input: "yes",
      file_input: "yes",
      vision_interpretation: "vision_unverified",
      structured_output: "yes",
      tool_calling: "yes",
      streaming: "yes",
      local_available: false,
      provider_available: true,
      auth_state: "ok",
      quota_state: "ok",
      runtime_verified: true,
    },
    {
      model_id: "gpt-4o",
      provider: "openai",
      text_input: "yes",
      image_input: "yes",
      file_input: "no",
      vision_interpretation: "interpretation_recorded",
      structured_output: "yes",
      tool_calling: "yes",
      streaming: "yes",
      local_available: false,
      provider_available: true,
      auth_state: "ok",
      quota_state: "ok",
      runtime_verified: false,
    },
  ],
};

beforeEach(() => {
  proj.mockReset().mockResolvedValue({ kind: "loading" });
  art.mockReset().mockResolvedValue({ kind: "loading" });
  prf.mockReset().mockResolvedValue({ kind: "loading" });
  mdl.mockReset().mockResolvedValue({ kind: "loading" });
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("FoundationPanel", () => {
  it("reads all four services and shows honest loading states first", () => {
    render(<FoundationPanel />);
    expect(screen.getByText("Reading the project store…")).toBeInTheDocument();
    expect(screen.getByText("Reading artifact records…")).toBeInTheDocument();
    expect(screen.getByText("Reading proof records…")).toBeInTheDocument();
    expect(screen.getByText("Reading the model registry…")).toBeInTheDocument();
    expect(proj).toHaveBeenCalledTimes(1);
    expect(art).toHaveBeenCalledTimes(1);
    expect(prf).toHaveBeenCalledTimes(1);
    expect(mdl).toHaveBeenCalledTimes(1);
  });

  it("renders real project, artifact, proof and model payloads", async () => {
    proj.mockResolvedValue(projectsReady);
    art.mockResolvedValue(artifactsReady);
    prf.mockResolvedValue(proof());
    mdl.mockResolvedValue(modelsReady);
    render(<FoundationPanel />);

    // Projects
    expect(await screen.findByText("Store state:")).toBeInTheDocument();
    expect(screen.getByText("ok")).toBeInTheDocument();
    expect(screen.getByText("zylcode")).toBeInTheDocument();
    expect(screen.getByText("C:/Projects/zylcode")).toBeInTheDocument();

    // Artifacts — contract line + verified/tampered verdicts + kind fallback
    expect(
      screen.getByText(/Contract v3 — 2 artifacts/),
    ).toBeInTheDocument();
    expect(screen.getByText("art-ok")).toBeInTheDocument();
    expect(screen.getByText("art-bad")).toBeInTheDocument();
    expect(screen.getByText("Spec")).toBeInTheDocument();
    expect(screen.getByText("accepted")).toBeInTheDocument();

    // Proofs — chain, per-proof verdicts, stale flag, honesty note
    expect(await screen.findByText("cargo test")).toBeInTheDocument();
    expect(screen.getByText("pnpm lint")).toBeInTheDocument();
    expect(screen.getByText("cargo clippy")).toBeInTheDocument();
    expect(screen.getByText("intact")).toBeInTheDocument();
    expect(screen.getByText("not run")).toBeInTheDocument();
    expect(screen.getByText("failed")).toBeInTheDocument();
    expect(screen.getByText("stale")).toBeInTheDocument();
    expect(
      screen.getByText(/never converted to passes/),
    ).toBeInTheDocument();

    // Models — note, vision flag, runtime badge
    expect(
      await screen.findByText("Capability rows are recorded observations."),
    ).toBeInTheDocument();
    expect(screen.getByText("VISION_UNVERIFIED")).toBeInTheDocument();
    expect(screen.getByText("runtime ✓")).toBeInTheDocument();
    expect(screen.getByText("interpretation_recorded")).toBeInTheDocument();

    // All four sections AVAILABLE
    await waitFor(() => {
      const badges = screen.getAllByText("AVAILABLE");
      expect(badges).toHaveLength(4);
    });
  });

  it("says so when the project registry is empty", async () => {
    proj.mockResolvedValue({ kind: "ready", storeState: "missing", projects: [] });
    render(<FoundationPanel />);
    expect(
      await screen.findByText(/No projects registered yet/),
    ).toBeInTheDocument();
    expect(screen.getByText(".zylcode/projects.json")).toBeInTheDocument();
    expect(screen.getByText("missing")).toBeInTheDocument();
  });

  it("flags a broken proof chain verbatim and falls back for an unknown HEAD", async () => {
    prf.mockResolvedValue(
      proof({ chainIntact: false, currentCommit: "", count: 1 }),
    );
    render(<FoundationPanel />);
    expect(
      await screen.findByText("BROKEN — history was modified"),
    ).toBeInTheDocument();
    expect(screen.getByText("?")).toBeInTheDocument();
    expect(screen.getByText(/· 1 proof · HEAD/)).toBeInTheDocument();
  });

  it("renders singular artifact count and unavailable reasons as BLOCKED", async () => {
    art.mockResolvedValue({ kind: "ready", contractVersion: 1, count: 1, artifacts: [] });
    proj.mockResolvedValue({ kind: "unavailable", reason: "store corrupt" });
    prf.mockResolvedValue({ kind: "unavailable", reason: "proof store missing" });
    mdl.mockResolvedValue({ kind: "unavailable", reason: "registry unreachable" });
    render(<FoundationPanel />);

    expect(await screen.findByText("store corrupt")).toBeInTheDocument();
    expect(screen.getByText("proof store missing")).toBeInTheDocument();
    expect(screen.getByText("registry unreachable")).toBeInTheDocument();
    expect(screen.getByText(/Contract v1 — 1 artifact;/)).toBeInTheDocument();
    await waitFor(() => {
      const blocked = screen.getAllByText("BLOCKED");
      expect(blocked).toHaveLength(3);
    });
  });
});