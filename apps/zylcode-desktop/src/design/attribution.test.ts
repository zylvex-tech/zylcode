import { describe, expect, it } from "vitest";
import { lengthToPx, matchTokens } from "./attribution";
import type { DesignIndex } from "./types";

// ---------------------------------------------------------------------------
// Attribution is the claim "this pixel comes from this token". It may only be
// made when the numbers agree; a near miss must stay unattributed, because a
// wrong attribution is worse than none.
// ---------------------------------------------------------------------------

const index: DesignIndex = {
  contract: "design-studio-index/1",
  sourceRoot: "apps/zylcode-desktop/src",
  note: "fixture",
  stats: {
    files: 1,
    components: 0,
    designSystemComponents: 0,
    screens: 0,
    pages: 0,
    themes: 1,
    baseTokens: 4,
    themeTokens: 3,
    tokens: 7,
    tokenUsages: 0,
  },
  breakpoints: [],
  themes: [{ id: "midnight-pro", isDark: true, line: 71, tokenCount: 3 }],
  tokens: [
    {
      name: "--space-4",
      value: "1rem",
      category: "spacing",
      theme: "base",
      file: "themes.css",
      line: 1,
    },
    {
      name: "--radius-lg",
      value: "0.5rem",
      category: "radius",
      theme: "base",
      file: "themes.css",
      line: 2,
    },
    {
      name: "--text-sm",
      value: "0.875rem",
      category: "typography",
      theme: "base",
      file: "themes.css",
      line: 3,
    },
    {
      name: "--text-lg",
      value: "1.125rem",
      category: "typography",
      theme: "base",
      file: "themes.css",
      line: 4,
    },
    {
      name: "--leading-normal",
      value: "1.5",
      category: "typography",
      theme: "base",
      file: "themes.css",
      line: 5,
    },
    {
      name: "--color-primary",
      value: "#3b82f6",
      category: "color",
      theme: "midnight-pro",
      file: "themes.css",
      line: 6,
    },
    {
      name: "--color-surface",
      value: "#1e293b",
      category: "color",
      theme: "midnight-pro",
      file: "themes.css",
      line: 7,
    },
    {
      name: "--color-border",
      value: "#334155",
      category: "color",
      theme: "midnight-pro",
      file: "themes.css",
      line: 8,
    },
  ],
  components: [],
  screens: [],
  usages: {},
  tailwind: { colors: [], spacing: [], radius: [], fontSize: [], shadow: [] },
};

describe("lengthToPx", () => {
  it("converts px and rem", () => {
    expect(lengthToPx("16px")).toBe(16);
    expect(lengthToPx("1rem")).toBe(16);
    expect(lengthToPx("0.5rem")).toBe(8);
  });

  it("returns null for values it cannot resolve", () => {
    expect(lengthToPx("auto")).toBeNull();
    expect(lengthToPx("50%")).toBeNull();
  });
});

describe("matchTokens", () => {
  it("attributes a computed colour to the token that defines it", () => {
    const tokens = matchTokens(index, "midnight-pro", { backgroundColor: "rgb(59, 130, 246)" });
    expect(tokens).toContain("--color-primary");
  });

  it("does not attribute a colour that only nearly matches", () => {
    const tokens = matchTokens(index, "midnight-pro", { backgroundColor: "rgb(60, 130, 246)" });
    expect(tokens).not.toContain("--color-primary");
  });

  it("attributes radius, spacing, font size and line height from computed values", () => {
    const tokens = matchTokens(index, "midnight-pro", {
      borderRadius: "8px",
      paddingTop: "16px",
      fontSize: "14px",
      lineHeight: "21px",
      borderColor: "rgb(51, 65, 85)",
    });
    expect(tokens).toEqual(
      expect.arrayContaining([
        "--radius-lg",
        "--space-4",
        "--text-sm",
        "--leading-normal",
        "--color-border",
      ]),
    );
  });

  it("stays empty when nothing is measurable", () => {
    expect(matchTokens(index, "midnight-pro", {})).toEqual([]);
  });

  it("does not attribute transparent backgrounds", () => {
    expect(matchTokens(index, "midnight-pro", { backgroundColor: "rgba(0, 0, 0, 0)" })).toEqual([]);
  });
});
