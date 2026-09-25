// ---------------------------------------------------------------------------
// File content service (lib/fileContent.ts)
//
// Transport parity with lib/surfaces.ts: Tauri desktop invokes the
// `file_content` command; the browser preview fetches /api/file-content
// through the Vite proxy. Failures resolve to a controlled unavailable
// state — the editor never fabricates content.
// ---------------------------------------------------------------------------

import { detectEnvironment, recordDiagnostic } from "./runtime";

export type FileContentResult = {
  path: string;
  size: number;
  lines: number;
  lossy: boolean;
  content: string;
};

export type FileContentState =
  | { kind: "loading" }
  | { kind: "ready"; data: FileContentResult }
  | { kind: "unavailable"; reason: string };

async function fetchFromService(relPath: string): Promise<FileContentResult> {
  const response = await fetch(
    `/api/file-content?path=${encodeURIComponent(relPath)}`,
    { headers: { Accept: "application/json" } },
  );
  if (!response.ok) {
    throw new Error(`file content service returned HTTP ${response.status}`);
  }
  const payload = await response.json();
  if (payload && typeof payload === "object" && "error" in payload) {
    throw new Error(String(payload.error));
  }
  return payload as FileContentResult;
}

/** Fetch a workspace file's content for the editor. Desktop first; browser via service. */
export async function fetchFileContent(relPath: string): Promise<FileContentState> {
  const env = detectEnvironment();

  if (env === "TAURI_DESKTOP") {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const data = await invoke<FileContentResult>("file_content", { path: relPath });
      return { kind: "ready", data };
    } catch (e) {
      recordDiagnostic("fileContent", String(e));
      return { kind: "unavailable", reason: String(e) };
    }
  }

  try {
    const data = await fetchFromService(relPath);
    return { kind: "ready", data };
  } catch (e) {
    recordDiagnostic("fileContent", String(e));
    return { kind: "unavailable", reason: e instanceof Error ? e.message : String(e) };
  }
}
