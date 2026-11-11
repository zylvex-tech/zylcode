// ---------------------------------------------------------------------------
// Design Studio — undo/redo over whole-document snapshots
//
// Snapshots, not inverse operations: a design document is small, and an undo
// that reconstructs the past from deltas is a place for "undo did something
// unexpected" bugs to live.
// ---------------------------------------------------------------------------

export interface History<T> {
  past: T[];
  present: T;
  future: T[];
}

/** Ceiling on retained snapshots: memory is bounded, undo depth is stated. */
export const MAX_HISTORY = 50;

export function initHistory<T>(present: T): History<T> {
  return { past: [], present, future: [] };
}

/** Record an edit. Any redo branch is discarded, as in every editor. */
export function commit<T>(history: History<T>, next: T): History<T> {
  const past = [...history.past, history.present].slice(-MAX_HISTORY);
  return { past, present: next, future: [] };
}

/**
 * Replace the present without recording a step.
 *
 * Used for live drag previews and for reloading a document from disk, neither
 * of which should be an undo target.
 */
export function replace<T>(history: History<T>, next: T): History<T> {
  return { ...history, present: next };
}

/**
 * Record a completed gesture whose intermediate states were `replace`d.
 *
 * A drag previews itself through `replace`, so the `past` entry has to be the
 * document as it stood when the gesture began: one drag, one undo step.
 */
export function commitGesture<T>(history: History<T>, before: T): History<T> {
  return {
    past: [...history.past, before].slice(-MAX_HISTORY),
    present: history.present,
    future: [],
  };
}

export function undo<T>(history: History<T>): History<T> {
  if (history.past.length === 0) return history;
  const previous = history.past[history.past.length - 1];
  return {
    past: history.past.slice(0, -1),
    present: previous,
    future: [history.present, ...history.future].slice(0, MAX_HISTORY),
  };
}

export function redo<T>(history: History<T>): History<T> {
  if (history.future.length === 0) return history;
  const next = history.future[0];
  return {
    past: [...history.past, history.present].slice(-MAX_HISTORY),
    present: next,
    future: history.future.slice(1),
  };
}

export function canUndo<T>(history: History<T>): boolean {
  return history.past.length > 0;
}

export function canRedo<T>(history: History<T>): boolean {
  return history.future.length > 0;
}
