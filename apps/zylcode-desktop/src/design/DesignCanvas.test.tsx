import { beforeAll, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { getComponent } from "./catalog";
import { loadDesignComponent } from "./registry";
import { DesignCanvas, type CanvasSelection } from "./DesignCanvas";
import indexData from "./generated/design-index.json";
import type { DesignIndex } from "./types";

const index = indexData as unknown as DesignIndex;
const button = getComponent(index, "components/ui/index#Button")!;

// The first mount pays for the real module import (framer-motion and the rest
// of the barrel), so warm it once instead of inside a 1s query timeout.
beforeAll(async () => {
  await loadDesignComponent(button);
}, 30_000);

function renderCanvas(overrides: Partial<Parameters<typeof DesignCanvas>[0]> = {}) {
  const onSelect = vi.fn<(selection: CanvasSelection | null) => void>();
  const onWidth = vi.fn();
  const onZoom = vi.fn();
  render(
    <DesignCanvas
      index={index}
      entry={button}
      theme={index.themes[0].id}
      width={1024}
      zoom={1}
      props={{ children: "Design Studio", variant: "primary" }}
      selection={null}
      onWidth={onWidth}
      onZoom={onZoom}
      onSelect={onSelect}
      blockedReason={null}
      {...overrides}
    />,
  );
  return { onSelect, onWidth, onZoom };
}

// ---------------------------------------------------------------------------
// The canvas is the part that must not be a picture: it mounts the component,
// reads a real computed style on selection, and refuses to fake a component it
// cannot construct.
// ---------------------------------------------------------------------------

describe("DesignCanvas", () => {
  it("mounts the real component inside a themed frame", async () => {
    renderCanvas();
    const frame = screen.getByTestId("canvas-frame");
    expect(frame.getAttribute("data-theme")).toBe(index.themes[0].id);
    const rendered = await screen.findByRole(
      "button",
      { name: "Design Studio" },
      { timeout: 5000 },
    );
    expect(rendered.className).toContain("btn-primary");
    expect(frame.contains(rendered)).toBe(true);
  });

  it("selects the element that was clicked", async () => {
    const { onSelect } = renderCanvas();
    const rendered = await screen.findByRole(
      "button",
      { name: "Design Studio" },
      { timeout: 5000 },
    );
    fireEvent.click(rendered);

    await waitFor(() => expect(onSelect).toHaveBeenCalled());
    const selection = onSelect.mock.calls[0][0]!;
    expect(selection.tag).toBe("button");
    expect(selection.classes).toContain("btn-primary");
    expect(selection.depth).toBeGreaterThan(0);
    expect(selection.snapshot.color).toBeDefined();
  });

  it("keeps the component's own handlers out of the way while inspecting", async () => {
    const onClick = vi.fn();
    const { onSelect } = renderCanvas({ props: { children: "Design Studio", onClick } });
    fireEvent.click(await screen.findByRole("button", { name: "Design Studio" }));

    expect(onSelect).toHaveBeenCalled();
    expect(onClick).not.toHaveBeenCalled();
  });

  it("reports a component it cannot construct instead of rendering it", () => {
    renderCanvas({
      blockedReason: "requires `onClose: () => void`, which the canvas cannot construct",
    });
    expect(screen.getByText("NOT MOUNTABLE")).toBeInTheDocument();
    expect(screen.getByText(/requires `onClose/)).toBeInTheDocument();
    expect(screen.getByText(`${button.file}:${button.line}`)).toBeInTheDocument();
  });

  it("switches viewport from the presets and a custom width", () => {
    const { onWidth } = renderCanvas();
    fireEvent.click(screen.getByRole("button", { name: "1440" }));
    expect(onWidth).toHaveBeenCalledWith(1440);

    const custom = screen.getByLabelText("Custom viewport width in pixels");
    fireEvent.change(custom, { target: { value: "900" } });
    expect(onWidth).toHaveBeenCalledWith(900);

    fireEvent.change(custom, { target: { value: "100" } });
    expect(onWidth).not.toHaveBeenCalledWith(100);
  });

  it("reports the breakpoint the width lands in", () => {
    renderCanvas({ width: 800 });
    expect(screen.getByTestId("active-breakpoint").textContent).toContain("md");
  });

  it("zooms, and compares the presets side by side", async () => {
    const { onZoom } = renderCanvas();
    fireEvent.click(screen.getByRole("button", { name: "+" }));
    expect(onZoom).toHaveBeenCalledWith(1.1);

    fireEvent.click(screen.getByRole("button", { name: "compare presets" }));
    expect(screen.getByText(/1440px ·/)).toBeInTheDocument();
    expect(screen.getByText(/768px ·/)).toBeInTheDocument();
  });

  it("reports an import failure instead of drawing a placeholder", async () => {
    const broken = {
      ...button,
      id: "nope#Missing",
      file: "src/components/nope.tsx",
      name: "Missing",
    };
    renderCanvas({ entry: broken });
    expect(await screen.findByText("IMPORT FAILED")).toBeInTheDocument();
  });
});
