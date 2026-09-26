// ---------------------------------------------------------------------------
// Delivery service client (lib/delivery.ts)
//
// Transport parity with lib/foundation.ts: fetch through the Vite proxy
// (/api/*). Failed fetches resolve to a controlled unavailable state —
// never fabricated data.
//
// Routes (zylcode-cli serve-intel):
//   GET  /api/deploy  — commissioned deploy targets (GitHub release, crates.io, remote)
//   GET  /api/build   — current/last build pipeline state
//   POST /api/build   — start the real pipeline (tests → release build → frontend)
//   POST /api/package — package a versioned release into .zylcode/releases/
// ---------------------------------------------------------------------------

export type DeployTargetView = {
  commissioned: boolean;
  state: string;
  note: string;
};

export type DeployState =
  | { kind: "loading" }
  | { kind: "unavailable"; reason: string }
  | {
      kind: "ready";
      head: string;
      github_release: DeployTargetView;
      crates_io: DeployTargetView;
      remote_server: DeployTargetView;
    };

export type BuildStepView = {
  name: string;
  command: string;
  passed: boolean;
  exit_code?: number | null;
  duration_ms?: number | null;
};

export type BuildState =
  | { kind: "loading" }
  | { kind: "unavailable"; reason: string }
  | {
      kind: "ready";
      state: string;
      started_at?: string;
      finished_at?: string;
      passed?: boolean;
      error?: string;
      note?: string;
      steps: BuildStepView[];
      packaged?: {
        release_dir?: string;
        artifact_id?: string;
        version?: string;
        content_hash?: string;
        file_count?: number;
      };
    };

async function getJson(url: string): Promise<Record<string, unknown>> {
  const response = await fetch(url, { headers: { Accept: "application/json" } });
  if (!response.ok) throw new Error(`HTTP ${response.status} from ${url}`);
  return (await response.json()) as Record<string, unknown>;
}

function target(value: unknown): DeployTargetView {
  const t = (value ?? {}) as Record<string, unknown>;
  return {
    commissioned: Boolean(t.commissioned),
    state: String(t.state ?? "UNCOMMISSIONED"),
    note: String(t.note ?? ""),
  };
}

export async function fetchDeploy(): Promise<DeployState> {
  try {
    const data = await getJson("/api/deploy");
    if (data.error) throw new Error(String(data.error));
    return {
      kind: "ready",
      head: String(data.head ?? ""),
      github_release: target(data.github_release),
      crates_io: target(data.crates_io),
      remote_server: target(data.remote_server),
    };
  } catch (e) {
    return {
      kind: "unavailable",
      reason: e instanceof Error ? e.message : "deploy status unavailable",
    };
  }
}

export async function fetchBuildState(): Promise<BuildState> {
  try {
    const data = await getJson("/api/build");
    const steps = ((data.steps as Record<string, unknown>[]) ?? []).map(
      (s): BuildStepView => ({
        name: String(s.name ?? "?"),
        command: String(s.command ?? ""),
        passed: Boolean(s.passed),
        exit_code: typeof s.exit_code === "number" ? s.exit_code : null,
        duration_ms: typeof s.duration_ms === "number" ? s.duration_ms : null,
      }),
    );
    const rawPackaged = data.packaged as Record<string, unknown> | undefined;
    return {
      kind: "ready",
      state: String(data.state ?? "idle"),
      started_at: data.started_at ? String(data.started_at) : undefined,
      finished_at: data.finished_at ? String(data.finished_at) : undefined,
      passed: data.passed === undefined ? undefined : Boolean(data.passed),
      error: data.error ? String(data.error) : undefined,
      note: data.note ? String(data.note) : undefined,
      steps,
      packaged: rawPackaged
        ? {
            release_dir: rawPackaged.release_dir
              ? String(rawPackaged.release_dir)
              : undefined,
            artifact_id: rawPackaged.artifact_id
              ? String(rawPackaged.artifact_id)
              : undefined,
            version: rawPackaged.version ? String(rawPackaged.version) : undefined,
            content_hash: rawPackaged.content_hash
              ? String(rawPackaged.content_hash)
              : undefined,
            file_count:
              typeof rawPackaged.file_count === "number"
                ? rawPackaged.file_count
                : undefined,
          }
        : undefined,
    };
  } catch (e) {
    return {
      kind: "unavailable",
      reason: e instanceof Error ? e.message : "build status unavailable",
    };
  }
}

/** Kick off the build pipeline. Returns the launch acknowledgement; poll
 * fetchBuildState() for the outcome. started=false means one is running. */
export async function startBuild(): Promise<{ started: boolean; note?: string; error?: string }> {
  try {
    const data = await getJsonPost("/api/build");
    return {
      started: Boolean(data.started),
      note: data.note ? String(data.note) : undefined,
      error: data.error ? String(data.error) : undefined,
    };
  } catch (e) {
    return { started: false, error: e instanceof Error ? e.message : String(e) };
  }
}

/** Kick off packaging (includes the build pipeline unless packageOnly). */
export async function startPackage(
  version: string,
  packageOnly: boolean,
): Promise<{ started: boolean; note?: string; error?: string }> {
  try {
    const response = await fetch("/api/package", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        version: version.trim() || undefined,
        package_only: packageOnly || undefined,
      }),
    });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const data = (await response.json()) as Record<string, unknown>;
    return {
      started: Boolean(data.started),
      note: data.note ? String(data.note) : undefined,
      error: data.error ? String(data.error) : undefined,
    };
  } catch (e) {
    return { started: false, error: e instanceof Error ? e.message : String(e) };
  }
}

async function getJsonPost(url: string): Promise<Record<string, unknown>> {
  const response = await fetch(url, { method: "POST" });
  if (!response.ok) throw new Error(`HTTP ${response.status} from ${url}`);
  return (await response.json()) as Record<string, unknown>;
}
