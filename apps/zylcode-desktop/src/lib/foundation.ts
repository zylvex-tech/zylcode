// ---------------------------------------------------------------------------
// Foundation-wave service views (lib/foundation.ts)
//
// Read-only clients for the wave's new persistence systems: Project System,
// Artifact Bus, Proof Engine, and the model capability registry. Every type
// mirrors the Rust contract exactly — including the honest states
// (Corrupt store, Tampered artifact, NotRun proof, VisionUnverified model).
// Failed fetches resolve to controlled unavailable states; nothing here
// fabricates data to fill a gap.
// ---------------------------------------------------------------------------

export type ProjectRecord = {
  id: string;
  name: string;
  root_path: string;
  repository_remote?: string;
  notes?: string;
  created_at: string;
  last_accessed: string;
  schema_version: number;
};

export type ProjectsState =
  | { kind: "loading" }
  | { kind: "unavailable"; reason: string }
  | {
      kind: "ready";
      storeState: string;
      projects: ProjectRecord[];
    };

export type ArtifactTransitionView = {
  from: string | null;
  to: string;
  actor: string;
  reason: string;
  at: string;
};

export type ArtifactRecordView = {
  id: string;
  kind: string | { Other: string };
  schema_version: number;
  project_id: string;
  mission_id?: string;
  producer: string;
  source_commit?: string;
  content_hash: string;
  content_len: number;
  permissions: string;
  provenance: string;
  verification_status: string;
  retention_policy: string;
  lifecycle: string;
  created_at: string;
  updated_at: string;
  history: ArtifactTransitionView[];
};

export type ArtifactsState =
  | { kind: "loading" }
  | { kind: "unavailable"; reason: string }
  | {
      kind: "ready";
      contractVersion: number;
      count: number;
      artifacts: ArtifactRecordView[];
    };

export type ProofView = {
  record: {
    id: string;
    project_id: string;
    mission_id?: string;
    command: string;
    source_commit: string;
    tree_clean: boolean;
    expected: string;
    actual?: string;
    exit_status?: number;
    duration_ms?: number;
    artifact_refs: string[];
    log_path?: string;
    reviewer?: string;
    acceptance: string;
    limitations?: string;
    verification: string;
    source: string;
    prev_hash: string;
    entry_hash: string;
    created_at: string;
  };
  stale: boolean;
  chain_intact: boolean;
};

export type ProofsState =
  | { kind: "loading" }
  | { kind: "unavailable"; reason: string }
  | {
      kind: "ready";
      schemaVersion: number;
      chainIntact: boolean;
      currentCommit: string;
      count: number;
      proofs: ProofView[];
    };

export type ModelCapabilityRow = {
  model_id: string;
  provider: string;
  text_input: string;
  image_input: string;
  file_input: string;
  vision_interpretation: string;
  structured_output: string;
  tool_calling: string;
  streaming: string;
  local_available: boolean;
  provider_available: boolean;
  auth_state: string;
  quota_state: string;
  runtime_verified: boolean;
};

export type ModelsState =
  | { kind: "loading" }
  | { kind: "unavailable"; reason: string }
  | {
      kind: "ready";
      contract: string;
      note: string;
      models: ModelCapabilityRow[];
    };

async function getJson(url: string): Promise<Record<string, unknown>> {
  const response = await fetch(url, { headers: { Accept: "application/json" } });
  if (!response.ok) throw new Error(`service returned HTTP ${response.status}`);
  const payload = await response.json();
  if (payload && typeof payload === "object" && "error" in payload) {
    throw new Error(String((payload as Record<string, unknown>).error));
  }
  return payload as Record<string, unknown>;
}

function kindOf(v: unknown): string {
  if (typeof v === "string") return v;
  return "other";
}

export async function fetchProjects(): Promise<ProjectsState> {
  try {
    const data = await getJson("/api/projects");
    const storeState = String(data.store_state ?? "Unknown");
    if (storeState === "Unavailable") {
      throw new Error(String(data.error ?? "project store unavailable"));
    }
    return {
      kind: "ready",
      storeState,
      projects: (data.projects as ProjectRecord[]) ?? [],
    };
  } catch (e) {
    return {
      kind: "unavailable",
      reason: e instanceof Error ? e.message : "project store unavailable",
    };
  }
}

export async function fetchArtifacts(): Promise<ArtifactsState> {
  try {
    const data = await getJson("/api/artifacts");
    return {
      kind: "ready",
      contractVersion: Number(data.contract_version ?? 1),
      count: Number(data.count ?? 0),
      artifacts: (data.artifacts as ArtifactRecordView[]) ?? [],
    };
  } catch (e) {
    return {
      kind: "unavailable",
      reason: e instanceof Error ? e.message : "artifact bus unavailable",
    };
  }
}

export async function fetchProofs(): Promise<ProofsState> {
  try {
    const data = await getJson("/api/proofs");
    return {
      kind: "ready",
      schemaVersion: Number(data.schema_version ?? 1),
      chainIntact: Boolean(data.chain_intact),
      currentCommit: String(data.current_commit ?? ""),
      count: Number(data.count ?? 0),
      proofs: (data.proofs as ProofView[]) ?? [],
    };
  } catch (e) {
    return {
      kind: "unavailable",
      reason: e instanceof Error ? e.message : "proof engine unavailable",
    };
  }
}

export async function fetchModels(): Promise<ModelsState> {
  try {
    const data = await getJson("/api/models");
    const models = ((data.models as Record<string, unknown>[]) ?? []).map(
      (m): ModelCapabilityRow => ({
        model_id: String(m.model_id ?? "?"),
        provider: String(m.provider ?? "?"),
        text_input: kindOf(m.text_input),
        image_input: kindOf(m.image_input),
        file_input: kindOf(m.file_input),
        vision_interpretation: kindOf(m.vision_interpretation),
        structured_output: kindOf(m.structured_output),
        tool_calling: kindOf(m.tool_calling),
        streaming: kindOf(m.streaming),
        local_available: Boolean(m.local_available),
        provider_available: Boolean(m.provider_available),
        auth_state: String(m.auth_state ?? "unknown"),
        quota_state: String(m.quota_state ?? "unknown"),
        runtime_verified: Boolean(m.runtime_verified),
      }),
    );
    return {
      kind: "ready",
      contract: String(data.contract ?? "?"),
      note: String(data.note ?? ""),
      models,
    };
  } catch (e) {
    return {
      kind: "unavailable",
      reason: e instanceof Error ? e.message : "model registry unavailable",
    };
  }
}
