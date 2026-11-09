import { describe, expect, it } from "vitest";
import { contrastRatio, contrastVerdict, parseColor, relativeLuminance } from "./contrast";

// ---------------------------------------------------------------------------
// The inspector reports contrast as PASS / WARNING / UNKNOWN. These tests pin
// the maths to WCAG 2.1 and pin the honesty rule: an unreadable value is
// UNKNOWN, never a number.
// ---------------------------------------------------------------------------

describe("parseColor", () => {
  it("parses six-digit hex", () => {
    expect(parseColor("#3b82f6")).toEqual({ r: 59, g: 130, b: 246, a: 1 });
  });

  it("expands three-digit hex", () => {
    expect(parseColor("#fff")).toEqual({ r: 255, g: 255, b: 255, a: 1 });
  });

  it("parses rgba with alpha", () => {
    expect(parseColor("rgba(15, 23, 42, 0.5)")).toEqual({ r: 15, g: 23, b: 42, a: 0.5 });
  });

  it("returns null for values that are not colours", () => {
    expect(parseColor("var(--color-primary)")).toBeNull();
    expect(parseColor("linear-gradient(...)")).toBeNull();
    expect(parseColor("transparent")).toBeNull();
  });
});

describe("relativeLuminance", () => {
  it("is 0 for black and 1 for white", () => {
    expect(relativeLuminance({ r: 0, g: 0, b: 0, a: 1 })).toBeCloseTo(0, 5);
    expect(relativeLuminance({ r: 255, g: 255, b: 255, a: 1 })).toBeCloseTo(1, 5);
  });
});

describe("contrastRatio", () => {
  it("is 21:1 for black on white", () => {
    expect(contrastRatio("#000000", "#ffffff")).toBeCloseTo(21, 1);
  });

  it("is symmetric", () => {
    expect(contrastRatio("#3b82f6", "#0f172a")).toBeCloseTo(
      contrastRatio("#0f172a", "#3b82f6")!,
      5,
    );
  });

  it("is null when either side cannot be parsed", () => {
    expect(contrastRatio("var(--x)", "#ffffff")).toBeNull();
  });
});

describe("contrastVerdict", () => {
  it("passes body text at 4.5:1 and fails below it", () => {
    expect(contrastVerdict(4.5)).toBe("PASS");
    expect(contrastVerdict(4.49)).toBe("WARNING");
  });

  it("passes large text at 3:1", () => {
    expect(contrastVerdict(3.1, true)).toBe("PASS");
    expect(contrastVerdict(2.9, true)).toBe("WARNING");
  });

  it("is UNKNOWN without a ratio", () => {
    expect(contrastVerdict(null)).toBe("UNKNOWN");
  });
});
