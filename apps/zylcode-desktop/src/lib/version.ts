// ---------------------------------------------------------------------------
// Version & build info service (lib/version.ts)
// ---------------------------------------------------------------------------

import { detectEnvironment, recordDiagnostic } from "./runtime";

export type VersionInfo = {
  app_version: string;
  engine: string;
  head_commit: string;
  branch: string;
  dirty: boolean;
};

export type VersionState =
  | { kind: "loading" }
  | { kind: "ready"; data: VersionInfo }
  | { kind: "unavailable"; reason: string };

async function fetchFromService(): Promise<VersionInfo> {
  const response = await fetch("/api/version", { headers: { Accept: "application/json" } });
  if (!response.ok) throw new Error(`version service returned HTTP ${response.status}`);
  const payload = await response.json();
  if (payload && typeof payload === "object" && "error" in payload) {
    throw new Error(String(payload.error));
  }
  return payload as VersionInfo;
}

/** Fetch real version/build info. Desktop first; browser via service. */
export async function fetchVersion(): Promise<VersionState> {
  if (detectEnvironment() === "TAURI_DESKTOP") {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      return { kind: "ready", data: await invoke<VersionInfo>("app_version") };
    } catch (e) {
      recordDiagnostic("version", String(e));
      return { kind: "unavailable", reason: String(e) };
    }
  }
  try {
    return { kind: "ready", data: await fetchFromService() };
  } catch (e) {
    recordDiagnostic("version", String(e));
    return { kind: "unavailable", reason: e instanceof Error ? e.message : String(e) };
  }
}
