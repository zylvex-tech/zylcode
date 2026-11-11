// ---------------------------------------------------------------------------
// Design Studio — the editing surface
//
// Elements are positioned in canvas coordinates and dragged with the mouse.
// Drag movements are reported live (`onMove`) and committed once on release
// (`onMoveEnd`), so one gesture is one undo step. Window-level listeners keep
// the drag alive when the cursor leaves the element.
// ---------------------------------------------------------------------------

import { useEffect, useRef, useState } from "react";
import type { CSSProperties, MouseEvent as ReactMouseEvent } from "react";
import type { DesignNode, DesignProject } from "./model";

interface DesignCanvasProps {
  project: DesignProject;
  selectedId: string | null;
  onSelect: (id: string | null) => void;
  onMoveStart: (id: string) => void;
  onMove: (id: string, x: number, y: number) => void;
  onMoveEnd: () => void;
}

interface DragState {
  id: string;
  startX: number;
  startY: number;
  originX: number;
  originY: number;
  moved: boolean;
}

function clamp(value: number, max: number): number {
  return Math.min(Math.max(0, value), Math.max(0, max));
}

export function DesignCanvas({
  project,
  selectedId,
  onSelect,
  onMoveStart,
  onMove,
  onMoveEnd,
}: DesignCanvasProps) {
  const drag = useRef<DragState | null>(null);
  const [dragging, setDragging] = useState(false);

  useEffect(() => {
    if (!dragging) return;
    function handleMove(event: MouseEvent) {
      const state = drag.current;
      if (!state) return;
      const node = project.nodes.find((n) => n.id === state.id);
      if (!node) return;
      const x = clamp(
        state.originX + (event.clientX - state.startX),
        project.canvas.width - node.width,
      );
      const y = clamp(
        state.originY + (event.clientY - state.startY),
        project.canvas.height - node.height,
      );
      state.moved = true;
      onMove(state.id, Math.round(x), Math.round(y));
    }
    function handleUp() {
      const state = drag.current;
      drag.current = null;
      setDragging(false);
      if (state?.moved) onMoveEnd();
    }
    window.addEventListener("mousemove", handleMove);
    window.addEventListener("mouseup", handleUp);
    return () => {
      window.removeEventListener("mousemove", handleMove);
      window.removeEventListener("mouseup", handleUp);
    };
  }, [dragging, project, onMove, onMoveEnd]);

  function startDrag(event: ReactMouseEvent<HTMLDivElement>, node: DesignNode) {
    if (event.button !== 0) return;
    event.stopPropagation();
    onSelect(node.id);
    onMoveStart(node.id);
    drag.current = {
      id: node.id,
      startX: event.clientX,
      startY: event.clientY,
      originX: node.x,
      originY: node.y,
      moved: false,
    };
    setDragging(true);
  }

  return (
    <div
      className="relative overflow-auto rounded-lg border border-border bg-background"
      style={{ maxHeight: 460 }}
      onMouseDown={() => onSelect(null)}
    >
      <div
        className="relative"
        style={{
          width: project.canvas.width,
          height: project.canvas.height,
          background: project.canvas.background,
        }}
      >
        {project.nodes.length === 0 && (
          <p className="absolute inset-0 flex items-center justify-center text-xs text-text-muted">
            Empty canvas — add a text or shape element to begin.
          </p>
        )}
        {project.nodes.map((node) => {
          const selected = node.id === selectedId;
          const style: CSSProperties = {
            position: "absolute",
            left: node.x,
            top: node.y,
            width: node.width,
            height: node.height,
            color: node.color,
            fontSize: node.fontSize,
            cursor: "grab",
            userSelect: "none",
            outline: selected ? "2px solid var(--color-primary, #3b82f6)" : "none",
            outlineOffset: 2,
          };
          if (node.kind === "rect") style.background = node.color;
          return (
            <div
              key={node.id}
              role="button"
              tabIndex={0}
              aria-label={`Element ${node.text || node.kind}`}
              className="flex items-start overflow-hidden rounded"
              style={style}
              onMouseDown={(e) => startDrag(e, node)}
              onFocus={() => onSelect(node.id)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  onSelect(node.id);
                }
              }}
            >
              {node.kind === "text" ? node.text : ""}
            </div>
          );
        })}
      </div>
    </div>
  );
}
