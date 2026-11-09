// ---------------------------------------------------------------------------
// Design canvas (design/DesignCanvas.tsx)
//
// Renders the selected application component inside a frame whose width is the
// chosen viewport and whose `data-theme` is the chosen theme — the same
// attribute the shell sets on <html>, so the canvas is a real second view of
// the real tokens rather than a restatement of them.
//
// Clicking an element inside the frame selects it and reads its computed style.
// The canvas takes that click in the capture phase: while you are inspecting,
// the component's own handlers stay out of the way, which is stated in the UI.
// ---------------------------------------------------------------------------

import { useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent, WheelEvent as ReactWheelEvent } from "react";
import { activeBreakpoint, VIEWPORT_PRESETS } from "./catalog";
import { matchTokens, snapshotElement, type ComputedSnapshot } from "./attribution";
import { useDesignComponent } from "./registry";
import type { ComponentEntry, DesignIndex } from "./types";

export interface CanvasSelection {
  tag: string;
  classes: string;
  text: string;
  tokens: string[];
  snapshot: ComputedSnapshot;
  depth: number;
}

interface DesignCanvasProps {
  index: DesignIndex;
  entry: ComponentEntry | null;
  theme: string;
  width: number;
  zoom: number;
  props: Record<string, unknown>;
  selection: CanvasSelection | null;
  onWidth: (width: number) => void;
  onZoom: (zoom: number) => void;
  onSelect: (selection: CanvasSelection | null) => void;
  /** Why this component cannot be mounted, when it cannot. */
  blockedReason: string | null;
}

function depthFrom(frame: HTMLElement, node: HTMLElement): number {
  let depth = 0;
  let current: HTMLElement | null = node;
  while (current && current !== frame) {
    depth += 1;
    current = current.parentElement;
  }
  return depth;
}

export function DesignCanvas({
  index,
  entry,
  theme,
  width,
  zoom,
  props,
  selection,
  onWidth,
  onZoom,
  onSelect,
  blockedReason,
}: DesignCanvasProps) {
  const load = useDesignComponent(entry);
  const frameRef = useRef<HTMLDivElement>(null);
  const panning = useRef<{ x: number; y: number; moved: boolean } | null>(null);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [mode, setMode] = useState<"single" | "presets">("single");
  const breakpoint = activeBreakpoint(index, width);

  const inspect = (target: HTMLElement) => {
    const frame = frameRef.current;
    if (!frame) return;
    const snapshot = snapshotElement(target);
    onSelect({
      tag: target.tagName.toLowerCase(),
      classes: typeof target.className === "string" ? target.className : "",
      text: (target.textContent ?? "").trim().slice(0, 120),
      tokens: matchTokens(index, theme, snapshot),
      snapshot,
      depth: depthFrom(frame, target),
    });
  };

  const handlePointerDown = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    panning.current = { x: event.clientX - pan.x, y: event.clientY - pan.y, moved: false };
  };

  const handlePointerMove = (event: ReactPointerEvent<HTMLDivElement>) => {
    const start = panning.current;
    if (!start) return;
    const next = { x: event.clientX - start.x, y: event.clientY - start.y };
    if (Math.abs(next.x - pan.x) > 3 || Math.abs(next.y - pan.y) > 3) start.moved = true;
    setPan(next);
  };

  const handlePointerUp = () => {
    const moved = panning.current?.moved ?? false;
    panning.current = null;
    if (moved) {
      // A drag is a pan, not a selection.
      onSelect(selection);
    }
  };

  const handleWheel = (event: ReactWheelEvent<HTMLDivElement>) => {
    if (!event.ctrlKey) return;
    event.preventDefault();
    onZoom(Math.min(2, Math.max(0.25, Number((zoom - event.deltaY * 0.0015).toFixed(2)))));
  };

  const frameBody = blockedReason ? (
    <div className="flex h-full flex-col items-center justify-center gap-2 p-6 text-center">
      <span className="text-[10px] font-mono uppercase tracking-[0.16em] text-warning">
        NOT MOUNTABLE
      </span>
      <p className="max-w-sm text-xs text-text-muted">{blockedReason}</p>
      {entry && (
        <p className="font-mono text-[10px] text-text-muted">
          {entry.file}:{entry.line}
        </p>
      )}
    </div>
  ) : load.status === "error" ? (
    <div className="flex h-full flex-col items-center justify-center gap-2 p-6 text-center">
      <span className="text-[10px] font-mono uppercase tracking-[0.16em] text-error">
        IMPORT FAILED
      </span>
      <p className="max-w-sm text-xs text-text-muted">{load.reason}</p>
    </div>
  ) : load.status === "ready" && entry ? (
    <load.Component {...props} />
  ) : (
    <p className="p-6 text-xs text-text-muted">loading component…</p>
  );

  const presets = VIEWPORT_PRESETS.map((preset) => ({
    preset,
    scale: Math.min(1, 360 / preset),
  }));

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex flex-wrap items-center gap-2 border-b border-border px-2 py-1.5">
        <span className="text-[10px] font-mono uppercase tracking-[0.16em] text-text-muted">
          viewport
        </span>
        {VIEWPORT_PRESETS.map((preset) => (
          <button
            key={preset}
            type="button"
            onClick={() => onWidth(preset)}
            className={`rounded border px-2 py-0.5 text-[11px] font-mono ${
              width === preset ? "border-primary text-primary" : "border-border text-text-muted"
            }`}
          >
            {preset}
          </button>
        ))}
        <label className="flex items-center gap-1 text-[10px] font-mono text-text-muted">
          custom
          <input
            type="number"
            min={240}
            max={3840}
            value={width}
            onChange={(e) => {
              const next = Number(e.target.value);
              if (Number.isFinite(next) && next >= 240 && next <= 3840) onWidth(Math.round(next));
            }}
            className="w-20 rounded border border-border bg-background px-1 py-0.5 text-[11px] text-text-primary"
            aria-label="Custom viewport width in pixels"
          />
          px
        </label>

        <span
          className="ml-auto text-[10px] font-mono text-text-muted"
          data-testid="active-breakpoint"
        >
          {breakpoint ? `${breakpoint.name} ≥ ${breakpoint.minWidth}px` : "below sm"}
        </span>

        <div className="flex items-center gap-1">
          <button
            type="button"
            onClick={() => onZoom(Math.max(0.25, Number((zoom - 0.1).toFixed(2))))}
            className="rounded border border-border px-1.5 py-0.5 text-[11px]"
          >
            −
          </button>
          <span className="w-10 text-center font-mono text-[10px] text-text-muted">
            {Math.round(zoom * 100)}%
          </span>
          <button
            type="button"
            onClick={() => onZoom(Math.min(2, Number((zoom + 0.1).toFixed(2))))}
            className="rounded border border-border px-1.5 py-0.5 text-[11px]"
          >
            +
          </button>
          <button
            type="button"
            onClick={() => {
              onZoom(1);
              setPan({ x: 0, y: 0 });
            }}
            className="rounded border border-border px-1.5 py-0.5 text-[10px] font-mono text-text-muted"
          >
            reset
          </button>
        </div>

        <button
          type="button"
          onClick={() => setMode(mode === "single" ? "presets" : "single")}
          className="rounded border border-border px-2 py-0.5 text-[10px] font-mono text-text-muted"
        >
          {mode === "single" ? "compare presets" : "single view"}
        </button>
      </div>

      <div
        className="relative min-h-0 flex-1 overflow-hidden bg-background/40"
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerLeave={handlePointerUp}
        onWheel={handleWheel}
        data-testid="canvas-viewport"
      >
        {mode === "presets" ? (
          <div className="flex h-full items-start gap-4 overflow-x-auto p-4">
            {presets.map(({ preset, scale }) => (
              <div key={preset} className="shrink-0">
                <p className="mb-1 font-mono text-[10px] text-text-muted">
                  {preset}px · {activeBreakpoint(index, preset)?.name ?? "below sm"}
                </p>
                <div
                  style={{ width: preset * scale, height: 320 * scale }}
                  className="overflow-hidden rounded border border-border"
                >
                  <div
                    data-theme={theme}
                    style={{
                      width: preset,
                      transform: `scale(${scale})`,
                      transformOrigin: "top left",
                    }}
                    className="bg-background text-text-primary"
                  >
                    {frameBody}
                  </div>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div
            style={{
              transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
              transformOrigin: "top left",
            }}
            className="absolute left-4 top-4"
          >
            <div
              ref={frameRef}
              data-theme={theme}
              data-testid="canvas-frame"
              onClickCapture={(event) => {
                event.preventDefault();
                event.stopPropagation();
                inspect(event.target as HTMLElement);
              }}
              style={{ width }}
              className="min-h-[320px] rounded border border-border bg-background p-4 text-text-primary"
            >
              {frameBody}
            </div>
          </div>
        )}
      </div>

      <p className="border-t border-border px-3 py-1.5 text-[10px] text-text-muted">
        Canvas clicks select an element and do not run the component&apos;s handlers. Drag to pan,
        ctrl+wheel to zoom.
      </p>
    </div>
  );
}
