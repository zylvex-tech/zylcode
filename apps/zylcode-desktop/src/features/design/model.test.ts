import { describe, expect, it } from "vitest";
import {
  MAX_NODES,
  draftProject,
  formatUpdated,
  isSafeId,
  newNode,
  validateProject,
  type DesignNode,
} from "./model";

function project() {
  return draftProject("p1", "Test project", 1_700_000_000_000);
}

function node(overrides: Partial<DesignNode> = {}): DesignNode {
  return { ...newNode("text", 0), ...overrides };
}

describe("design document model", () => {
  it("accepts a well-formed project", () => {
    const p = project();
    p.nodes.push(node());
    expect(validateProject(p)).toEqual([]);
  });

  it("reports every reason a document cannot be written", () => {
    const p = project();
    p.id = "../escape";
    p.name = "   ";
    p.canvas.background = "navy";
    p.nodes.push(
      node({ id: "bad id", x: Number.NaN, width: 0, color: "red" }),
      node({ id: "ok", height: -5, fontSize: 5000 }),
    );

    const errors = validateProject(p);
    expect(errors.some((e) => e.includes("filename-safe"))).toBe(true);
    expect(errors.some((e) => e.includes("name is required"))).toBe(true);
    expect(errors.some((e) => e.includes("must be #rrggbb"))).toBe(true);
    expect(errors.some((e) => e.includes("must be a finite number"))).toBe(true);
    expect(errors.some((e) => e.includes("width must be between"))).toBe(true);
    expect(errors.some((e) => e.includes("height must be between"))).toBe(true);
    expect(errors.some((e) => e.includes("font size must be between"))).toBe(true);
  });

  it("refuses more elements than the store will persist", () => {
    const p = project();
    p.nodes = Array.from({ length: MAX_NODES + 1 }, (_, i) => node({ id: `n${i}` }));
    expect(validateProject(p).some((e) => e.includes(`more than ${MAX_NODES}`))).toBe(true);
  });

  it("only accepts filename-safe ids", () => {
    for (const id of ["p1", "p-1", "p_1", "A".repeat(64)]) expect(isSafeId(id)).toBe(true);
    for (const id of ["", "../x", "a/b", "a b", "A".repeat(65), "é"])
      expect(isSafeId(id)).toBe(false);
  });

  it("formats timestamps without inventing a date for a missing clock", () => {
    expect(formatUpdated(0)).toBe("unknown");
    expect(formatUpdated(Number.NaN)).toBe("unknown");
    expect(formatUpdated(Date.UTC(2026, 10, 11, 9, 30))).toBe("2026-11-11 09:30 UTC");
  });

  it("derives nodes with distinct ids and honest defaults", () => {
    const text = newNode("text", 2);
    const rect = newNode("rect", 0);
    expect(text.kind).toBe("text");
    expect(text.text).toBe("New label");
    expect(rect.kind).toBe("rect");
    expect(rect.text).toBe("");
    expect(text.id).not.toBe(rect.id);
  });
});
