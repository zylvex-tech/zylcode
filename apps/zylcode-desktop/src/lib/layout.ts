// ---------------------------------------------------------------------------
// Layout persistence + resize handles (lib/layout.ts)
// ---------------------------------------------------------------------------

import { useCallback, useEffect, useRef, useState } from "react";

/** State persisted to localStorage under a namespaced key, with a fallback. */
export function usePersistentState<T>(
  key: string,
  initial: T,
): [T, (v: T | ((p: T) => T)) => void] {
  const storageKey = `zylcode.layout.${key}`;
  const [value, setValue] = useState<T>(() => {
    try {
      const raw = window.localStorage.getItem(storageKey);
      if (raw !== null) return JSON.parse(raw) as T;
    } catch {
      /* fall through to initial */
    }
    return initial;
  });

  const set = useCallback(
    (v: T | ((p: T) => T)) => {
      setValue((prev) => {
        const next = typeof v === "function" ? (v as (p: T) => T)(prev) : v;
        try {
          window.localStorage.setItem(storageKey, JSON.stringify(next));
        } catch {
          /* persistence is best-effort */
        }
        return next;
      });
    },
    [storageKey],
  );

  return [value, set];
}

/**
 * Pointer-based drag resize for a vertical boundary. Returns props to spread
 * on the handle element. The `onChange` callback receives a pixel delta.
 */
export function useDragResize(onChange: (deltaPx: number) => void) {
  const dragging = useRef(false);
  const lastX = useRef(0);

  const onPointerDown = useCallback((e: React.PointerEvent) => {
    dragging.current = true;
    lastX.current = e.clientX;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  }, []);

  const onPointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (!dragging.current) return;
      const delta = e.clientX - lastX.current;
      lastX.current = e.clientX;
      onChange(delta);
    },
    [onChange],
  );

  const onPointerUp = useCallback((e: React.PointerEvent) => {
    dragging.current = false;
    (e.target as HTMLElement).releasePointerCapture(e.pointerId);
  }, []);

  return {
    onPointerDown,
    onPointerMove,
    onPointerUp,
    style: { cursor: "col-resize" as const },
  };
}
