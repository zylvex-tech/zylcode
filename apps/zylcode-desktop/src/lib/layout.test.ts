import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useDragResize, usePersistentState } from "./layout";

// ---------------------------------------------------------------------------
// Layout persistence + drag resize (lib/layout.ts).
//
// Two guarantees under test:
//   1. `usePersistentState` round-trips through localStorage under a
//      namespaced key, and degrades to the initial value when storage is
//      unavailable or holds unparseable data — persistence is best-effort and
//      must never throw out of a render.
//   2. `useDragResize` reports the *delta* between pointer moves (not the
//      absolute position), ignores moves outside a drag, and stops after the
//      pointer is released.
// ---------------------------------------------------------------------------

const KEY_PREFIX = "zylcode.layout.";

function stored(key: string): string | null {
  return window.localStorage.getItem(KEY_PREFIX + key);
}

describe("usePersistentState", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  afterEach(() => {
    window.localStorage.clear();
    vi.restoreAllMocks();
  });

  it("starts from the initial value when nothing is stored", () => {
    const { result } = renderHook(() => usePersistentState("sidebar", 240));
    expect(result.current[0]).toBe(240);
  });

  it("restores a previously persisted value from the namespaced key", () => {
    window.localStorage.setItem(KEY_PREFIX + "sidebar", JSON.stringify(320));

    const { result } = renderHook(() => usePersistentState("sidebar", 240));

    expect(result.current[0]).toBe(320);
  });

  it("writes the new value to localStorage on set", () => {
    const { result } = renderHook(() => usePersistentState("sidebar", 240));

    act(() => result.current[1](300));

    expect(result.current[0]).toBe(300);
    expect(stored("sidebar")).toBe("300");
  });

  it("supports a functional update based on the previous value", () => {
    const { result } = renderHook(() => usePersistentState("sidebar", 240));

    act(() => result.current[1]((prev) => prev + 60));

    expect(result.current[0]).toBe(300);
    expect(stored("sidebar")).toBe("300");
  });

  it("keeps a distinct value per key", () => {
    const a = renderHook(() => usePersistentState("left", 1));
    const b = renderHook(() => usePersistentState("right", 2));

    act(() => a.result.current[1](10));

    expect(a.result.current[0]).toBe(10);
    expect(b.result.current[0]).toBe(2);
  });

  it("falls back to the initial value when stored JSON is corrupt", () => {
    window.localStorage.setItem(KEY_PREFIX + "sidebar", "{not json");

    const { result } = renderHook(() => usePersistentState("sidebar", 240));

    expect(result.current[0]).toBe(240);
  });

  it("still updates state when localStorage.setItem throws", () => {
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("QuotaExceededError");
    });

    const { result } = renderHook(() => usePersistentState("sidebar", 240));

    act(() => result.current[1](310));

    // The write failed, but the in-memory state must not be lost.
    expect(result.current[0]).toBe(310);
  });

  it("falls back to the initial value when localStorage.getItem throws", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("SecurityError");
    });

    const { result } = renderHook(() => usePersistentState("sidebar", 240));

    expect(result.current[0]).toBe(240);
  });
});

describe("useDragResize", () => {
  type Handler = {
    onPointerDown: (e: React.PointerEvent) => void;
    onPointerMove: (e: React.PointerEvent) => void;
    onPointerUp: (e: React.PointerEvent) => void;
    style: { cursor: string };
  };

  /** The fake event target: a pointer-capturing element in the browser, and
   * the two spies this test asserts on. Typed explicitly so the assertions
   * below typecheck — `React.PointerEvent["target"]` alone is an `EventTarget`,
   * which carries neither method. */
  type CaptureTarget = EventTarget & {
    setPointerCapture: ReturnType<typeof vi.fn>;
    releasePointerCapture: ReturnType<typeof vi.fn>;
  };
  type CapturingPointerEvent = React.PointerEvent & { target: CaptureTarget };

  function pointerEvent(clientX: number, pointerId = 1): CapturingPointerEvent {
    return {
      clientX,
      pointerId,
      target: {
        setPointerCapture: vi.fn(),
        releasePointerCapture: vi.fn(),
      },
    } as unknown as CapturingPointerEvent;
  }

  function drag(onChange: (delta: number) => void): {
    handler: Handler;
    onChange: ReturnType<typeof vi.fn>;
  } {
    const spy = vi.fn(onChange);
    const { result } = renderHook(() => useDragResize(spy));
    return { handler: result.current as Handler, onChange: spy };
  }

  it("exposes a resize cursor for the handle", () => {
    const { handler } = drag(() => {});
    expect(handler.style.cursor).toBe("col-resize");
  });

  it("captures the pointer on press so the drag survives leaving the handle", () => {
    const { handler } = drag(() => {});
    const event = pointerEvent(100);

    handler.onPointerDown(event);

    expect(event.target.setPointerCapture).toHaveBeenCalledWith(1);
  });

  it("reports the pixel delta between moves, not the absolute position", () => {
    const { handler, onChange } = drag(() => {});

    handler.onPointerDown(pointerEvent(100));
    handler.onPointerMove(pointerEvent(130));
    handler.onPointerMove(pointerEvent(120));

    expect(onChange.mock.calls.map((c) => c[0])).toEqual([30, -10]);
  });

  it("ignores pointer moves that happen outside a drag", () => {
    const { handler, onChange } = drag(() => {});

    // No onPointerDown: the pointer was never captured by this handle.
    handler.onPointerMove(pointerEvent(500));

    expect(onChange).not.toHaveBeenCalled();
  });

  it("stops reporting deltas after the pointer is released", () => {
    const { handler, onChange } = drag(() => {});

    handler.onPointerDown(pointerEvent(100));
    handler.onPointerMove(pointerEvent(140));
    handler.onPointerUp(pointerEvent(140));
    handler.onPointerMove(pointerEvent(900));

    expect(onChange.mock.calls.map((c) => c[0])).toEqual([40]);
  });

  it("releases pointer capture on release", () => {
    const { handler } = drag(() => {});
    const event = pointerEvent(100);

    handler.onPointerDown(event);
    handler.onPointerUp(event);

    expect(event.target.releasePointerCapture).toHaveBeenCalledWith(1);
  });
});
