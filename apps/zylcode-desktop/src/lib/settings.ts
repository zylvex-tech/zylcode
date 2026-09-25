// ---------------------------------------------------------------------------
// ZylCode settings store (lib/settings.ts)
//
// Typed, persisted, validated app settings. Every field is consumed by a
// real feature — there are no decorative toggles. Unknown persisted values
// fall back to defaults (schema evolution is safe).
// ---------------------------------------------------------------------------

import type { AutonomyMode } from "./mission";

/** The settings schema, versioned so future migrations stay honest. */
export const SETTINGS_VERSION = 1 as const;

export interface ZylSettings {
  /** Version of the persisted shape. */
  version: typeof SETTINGS_VERSION;

  // --- Agent & Sessions -----------------------------------------------------
  /** Autonomy preselected for new missions. */
  defaultAutonomy: AutonomyMode;
  /** Default mission mode in the agent composer. */
  defaultMissionMode: "build" | "plan";
  /** Summarize consecutive tool calls into one row while streaming. */
  toolCallSummary: boolean;
  /** Auto-collapse finished tool sections when a mission turn ends. */
  autoFoldMessages: boolean;

  // --- Editor ---------------------------------------------------------------
  /** Tab width for the editor gutter. */
  editorTabSize: 2 | 4;
  /** Word-wrap long lines in the editor. */
  editorWordWrap: boolean;
  /** Show line numbers in the editor. */
  editorLineNumbers: boolean;

  // --- Appearance ------------------------------------------------------------
  /** Reduce non-essential animation. */
  reducedMotion: boolean;
  /** Compact density for panels and lists. */
  compactDensity: boolean;

  // --- Diagnostics ------------------------------------------------------------
  /** Capture frontend diagnostics (already recorded internally). */
  captureDiagnostics: boolean;
}

export const DEFAULT_SETTINGS: ZylSettings = {
  version: SETTINGS_VERSION,
  defaultAutonomy: "GUIDED",
  defaultMissionMode: "build",
  toolCallSummary: true,
  autoFoldMessages: true,
  editorTabSize: 4,
  editorWordWrap: false,
  editorLineNumbers: true,
  reducedMotion: false,
  compactDensity: false,
  captureDiagnostics: true,
};

const STORAGE_KEY = "zylcode.settings.v1";

/** Fields a persisted record may contain, with their expected types. */
const FIELD_TYPES: Record<string, "string" | "boolean" | "number"> = {
  defaultAutonomy: "string",
  defaultMissionMode: "string",
  toolCallSummary: "boolean",
  autoFoldMessages: "boolean",
  editorTabSize: "number",
  editorWordWrap: "boolean",
  editorLineNumbers: "boolean",
  reducedMotion: "boolean",
  compactDensity: "boolean",
  captureDiagnostics: "boolean",
};

/** Restrict enums to real option sets. */
const AUTONOMY_VALUES: ReadonlySet<string> = new Set([
  "OBSERVE",
  "GUIDED",
  "BUILD",
  "MISSION",
  "SOVEREIGN",
]);

/**
 * Merge a persisted record over defaults, dropping fields with wrong types
 * or out-of-set enum values. Returns defaults for null/undefined input.
 */
export function mergeSettings(raw: unknown): ZylSettings {
  if (raw === null || typeof raw !== "object") return { ...DEFAULT_SETTINGS };
  const source = raw as Record<string, unknown>;
  const merged: ZylSettings = { ...DEFAULT_SETTINGS };
  for (const [field, type] of Object.entries(FIELD_TYPES)) {
    const value = source[field];
    if (value === undefined || value === null) continue;
    if (typeof value !== type) continue;
    if (field === "defaultAutonomy" && !AUTONOMY_VALUES.has(String(value))) continue;
    if (
      field === "defaultMissionMode" &&
      value !== "build" &&
      value !== "plan"
    )
      continue;
    if (field === "editorTabSize" && value !== 2 && value !== 4) continue;
    (merged as unknown as Record<string, unknown>)[field] = value;
  }
  return merged;
}

/** Read settings from localStorage, falling back to defaults. */
export function loadSettings(): ZylSettings {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (raw === null) return { ...DEFAULT_SETTINGS };
    return mergeSettings(JSON.parse(raw));
  } catch {
    return { ...DEFAULT_SETTINGS };
  }
}

/** Persist settings; best-effort (private mode etc. silently keeps runtime state). */
export function saveSettings(settings: ZylSettings): void {
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch {
    /* persistence unavailable — runtime state still applies */
  }
}

/** React hook: typed settings with update + reset. */
import { useCallback, useEffect, useState } from "react";

export function useSettings(): {
  settings: ZylSettings;
  update: <K extends keyof ZylSettings>(key: K, value: ZylSettings[K]) => void;
  reset: () => void;
} {
  const [settings, setSettings] = useState<ZylSettings>(() => loadSettings());

  useEffect(() => {
    const sync = (e: StorageEvent) => {
      if (e.key === STORAGE_KEY) setSettings(loadSettings());
    };
    window.addEventListener("storage", sync);
    return () => window.removeEventListener("storage", sync);
  }, []);

  const update = useCallback(
    <K extends keyof ZylSettings>(key: K, value: ZylSettings[K]) => {
      setSettings((prev) => {
        const next = { ...prev, [key]: value };
        saveSettings(next);
        return next;
      });
    },
    [],
  );

  const reset = useCallback(() => {
    const next = { ...DEFAULT_SETTINGS };
    setSettings(next);
    saveSettings(next);
  }, []);

  return { settings, update, reset };
}
