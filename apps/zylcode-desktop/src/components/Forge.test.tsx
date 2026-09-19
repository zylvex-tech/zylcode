import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import Forge from "./Forge";
import { FORGE_CATEGORIES, FORGE_FUTURE_PACKS } from "../lib/forge";

// ---------------------------------------------------------------------------
// Forge marketplace shell (v3 §13–§19): taxonomy browsing, honest NOT
// INSTALLED states, live manifest validation. No fake packs, no checkout.
// ---------------------------------------------------------------------------

describe("Forge", () => {
  it("renders the marketplace shell with an honest LIMITED status", () => {
    render(<Forge />);
    expect(screen.getByText("ZYLCODE FORGE")).toBeInTheDocument();
    expect(screen.getByText("LIMITED")).toBeInTheDocument();
    expect(screen.getByText(/taxonomy concepts, explicitly marked NOT INSTALLED/i)).toBeInTheDocument();
  });

  it("lists taxonomy concept packs as NOT INSTALLED", () => {
    render(<Forge />);
    expect(screen.getAllByText("NOT INSTALLED").length).toBeGreaterThan(0);
    expect(screen.getByText("Flutter Engineer")).toBeInTheDocument();
    expect(screen.getByText("FreeCAD Automation")).toBeInTheDocument();
  });

  it("filters concept packs by search query", () => {
    render(<Forge />);
    const input = screen.getByPlaceholderText(/search capability packs/i);
    (input as HTMLInputElement).value = "";
    // Type via native setter so React state updates.
    const setter = Object.getOwnPropertyDescriptor(
      window.HTMLInputElement.prototype,
      "value",
    )?.set;
    setter?.call(input, "Blender");
    input.dispatchEvent(new Event("input", { bubbles: true }));
    expect(screen.getByText("Blender Automation")).toBeInTheDocument();
    expect(screen.queryByText("Flutter Engineer")).not.toBeInTheDocument();
  });

  it("shows the empty state when nothing matches", () => {
    render(<Forge />);
    const input = screen.getByPlaceholderText(/search capability packs/i);
    const setter = Object.getOwnPropertyDescriptor(
      window.HTMLInputElement.prototype,
      "value",
    )?.set;
    setter?.call(input, "zzz-no-such-pack");
    input.dispatchEvent(new Event("input", { bubbles: true }));
    expect(screen.getByText(/no concepts match the filter/i)).toBeInTheDocument();
  });

  it("seeds the full marketplace taxonomy", () => {
    // §17: 21 categories incl. Zylvex-specific engineering entries.
    expect(FORGE_CATEGORIES.length).toBe(21);
    expect(FORGE_FUTURE_PACKS).toContain("CAD Automation");
    expect(FORGE_FUTURE_PACKS).toContain("MCP Builder");
  });
});
