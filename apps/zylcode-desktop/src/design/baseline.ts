// ---------------------------------------------------------------------------
// Baseline comparison foundation (design/baseline.ts)
//
// A visual-regression service compares pixels. This is the smaller, honest
// step in that direction: capture the computed style of a selected element
// (and the tokens attributed to it), then diff a later capture against it.
// Every difference is a property the browser actually reported.
//
// It is not a pixel diff and it does not run in CI. `docs/design-studio/`
// records that limitation.
// ---------------------------------------------------------------------------

import type { ComputedSnapshot } from "./attribution";

export interface BaselineSnapshot {
  capturedAt: string;
  theme: string;
  width: number;
  values: Record<string, string>;
  tokens: string[];
}

export interface SnapshotDiff {
  property: string;
  baseline: string;
  current: string;
  changed: boolean;
}

export function toBaseline(
  snapshot: ComputedSnapshot,
  theme: string,
  width: number,
  tokens: string[],
): BaselineSnapshot {
  const values: Record<string, string> = {};
  for (const [key, value] of Object.entries(snapshot)) {
    if (typeof value === "string" && value) values[key] = value;
  }
  return {
    capturedAt: new Date().toISOString(),
    theme,
    width,
    values,
    tokens: [...tokens].sort(),
  };
}

export function diffSnapshots(
  baseline: BaselineSnapshot,
  current: ComputedSnapshot,
): SnapshotDiff[] {
  const keys = new Set([...Object.keys(baseline.values), ...Object.keys(current)]);
  const diffs: SnapshotDiff[] = [];
  for (const key of [...keys].sort()) {
    const before = baseline.values[key] ?? "";
    const after = (current as Record<string, string | undefined>)[key] ?? "";
    if (!before && !after) continue;
    diffs.push({ property: key, baseline: before, current: after, changed: before !== after });
  }
  return diffs;
}

export function baselineKey(componentId: string, tag: string, classes: string): string {
  return `zylcode-design-baseline:${componentId}:${tag}:${classes.slice(0, 80)}`;
}

export function saveBaseline(key: string, baseline: BaselineSnapshot): boolean {
  try {
    if (typeof localStorage === "undefined") return false;
    localStorage.setItem(key, JSON.stringify(baseline));
    return true;
  } catch {
    return false;
  }
}

export function readBaseline(key: string): BaselineSnapshot | null {
  try {
    if (typeof localStorage === "undefined") return null;
    const raw = localStorage.getItem(key);
    if (!raw) return null;
    return JSON.parse(raw) as BaselineSnapshot;
  } catch {
    return null;
  }
}

export function clearBaseline(key: string): void {
  try {
    localStorage.removeItem(key);
  } catch {
    // A blocked storage write is not an error the inspector needs to surface.
  }
}
