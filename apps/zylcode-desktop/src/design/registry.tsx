// ---------------------------------------------------------------------------
// Design Studio component registry (design/registry.tsx)
//
// The canvas must mount the *application's* components, not pictures of them.
// The index records each component's file; this module is the only place that
// turns that path into a module, and it does so through Vite's own module graph
// (`import.meta.glob`), so the canvas renders exactly the code the app ships.
//
// Loading is per component, cached, and failure is a state the canvas reports —
// an import that throws is a fact about the code, not a reason to draw a
// placeholder.
// ---------------------------------------------------------------------------

import { useEffect, useState } from "react";
import type { ComponentType } from "react";
import type { ComponentEntry } from "./types";

const modules = import.meta.glob([
  "../components/**/*.tsx",
  "!../components/**/*.test.tsx",
  "!../components/**/__snapshots__/**",
]);

export type MountedComponent = ComponentType<Record<string, unknown>>;

export type LoadState =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "ready"; Component: MountedComponent }
  | { status: "error"; reason: string };

/** `src/components/ui/index.tsx` → `../components/ui/index.tsx` */
export function moduleKeyOf(file: string): string {
  return `../${file.replace(/^src\//, "")}`;
}

export function isLoadable(file: string): boolean {
  return moduleKeyOf(file) in modules;
}

const cache = new Map<string, LoadState>();

export async function loadDesignComponent(entry: ComponentEntry): Promise<LoadState> {
  const cached = cache.get(entry.id);
  if (cached) return cached;

  const key = moduleKeyOf(entry.file);
  const loader = modules[key];
  if (!loader) {
    const state: LoadState = {
      status: "error",
      reason: `no module in the build graph for ${entry.file}`,
    };
    cache.set(entry.id, state);
    return state;
  }

  try {
    const mod = (await loader()) as Record<string, unknown>;
    const exported = mod[entry.name];
    if (typeof exported !== "function" && typeof exported !== "object") {
      const state: LoadState = {
        status: "error",
        reason: `${entry.file} does not export a component named ${entry.name}`,
      };
      cache.set(entry.id, state);
      return state;
    }
    const state: LoadState = { status: "ready", Component: exported as MountedComponent };
    cache.set(entry.id, state);
    return state;
  } catch (error) {
    const state: LoadState = {
      status: "error",
      reason: error instanceof Error ? error.message : String(error),
    };
    cache.set(entry.id, state);
    return state;
  }
}

/** Load a component for the canvas, reporting progress and failure. */
export function useDesignComponent(entry: ComponentEntry | null): LoadState {
  const [state, setState] = useState<LoadState>({ status: "idle" });

  useEffect(() => {
    if (!entry) {
      setState({ status: "idle" });
      return;
    }
    let cancelled = false;
    setState({ status: "loading" });
    void loadDesignComponent(entry).then((next) => {
      if (!cancelled) setState(next);
    });
    return () => {
      cancelled = true;
    };
  }, [entry]);

  return state;
}

/** Test hook: forget cached modules so a reload exercises the real import path. */
export function resetDesignComponentCache(): void {
  cache.clear();
}
