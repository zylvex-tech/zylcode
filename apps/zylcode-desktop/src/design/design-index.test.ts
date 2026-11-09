import { describe, expect, it } from "vitest";
import appSource from "../App.tsx?raw";
import { mountPlan } from "./catalog";
import indexData from "./generated/design-index.json";
import type { DesignIndex } from "./types";

// ---------------------------------------------------------------------------
// The generated index is the Design Studio's only source of truth, so it gets
// two kinds of test: the artefact must still agree with the source it was built
// from, and what it claims must be internally consistent.
//
// Freshness is checked against the source text itself (raw imports, no Node
// APIs in a browser test): every recorded line still contains the name it was
// recorded for, every component file still produces a record, and every theme
// and token still exists where the index says it does. The byte-exact check is
// `design:index:check`, which `pnpm test` runs before Vitest.
// ---------------------------------------------------------------------------

const index = indexData as unknown as DesignIndex;

const componentSources = import.meta.glob(
  ["../components/**/*.tsx", "!../components/**/*.test.tsx", "!../components/**/__snapshots__/**"],
  { query: "?raw", import: "default", eager: true },
) as Record<string, string>;

/** `../components/ui/index.tsx` → `src/components/ui/index.tsx` */
function indexedPathOf(globKey: string): string {
  return `src/${globKey.replace(/^\.\.\//, "")}`;
}

describe("design index freshness", () => {
  it("records lines that still contain the component they describe", () => {
    for (const component of index.components) {
      if (component.file === "src/App.tsx") {
        expect(appSource.split(/\r?\n/)[component.line - 1]).toContain(component.name);
        continue;
      }
      const key = `../${component.file.replace(/^src\//, "")}`;
      const source = componentSources[key];
      expect(source, `${component.file} is not in the build graph`).toBeDefined();
      expect(source.split(/\r?\n/)[component.line - 1]).toContain(component.name);
    }
  });

  it("indexes every component module in the build graph", () => {
    for (const key of Object.keys(componentSources)) {
      const file = indexedPathOf(key);
      expect(
        index.components.some((c) => c.file === file),
        `${file} produced no component record`,
      ).toBe(true);
    }
  });

  it("keeps the theme ids distinct and sourced from the stylesheet", () => {
    // The stylesheet itself is compared byte for byte by `design:index:check`,
    // which `pnpm test` runs before Vitest; here the shape is checked.
    expect(new Set(index.themes.map((t) => t.id)).size).toBe(index.themes.length);
    for (const theme of index.themes) {
      expect(theme.tokenCount).toBeGreaterThan(0);
      expect(theme.line).toBeGreaterThan(0);
    }
  });
});

describe("design index contents", () => {
  it("indexes the application's components, themes and tokens", () => {
    expect(index.contract).toBe("design-studio-index/1");
    expect(index.stats.files).toBeGreaterThan(20);
    expect(index.stats.components + index.stats.designSystemComponents).toBeGreaterThan(30);
    expect(index.stats.designSystemComponents).toBeGreaterThanOrEqual(10);
    expect(index.themes.map((t) => t.id)).toContain("midnight-pro");
    expect(index.themes).toHaveLength(8);
    expect(index.tokens.length).toBeGreaterThan(100);
    expect(index.stats.tokenUsages).toBeGreaterThan(100);
  });

  it("records where every token and component lives", () => {
    for (const token of index.tokens) {
      expect(token.name.startsWith("--")).toBe(true);
      expect(token.value.length).toBeGreaterThan(0);
      expect(token.line).toBeGreaterThan(0);
      expect(token.file).toContain("themes.css");
    }
    for (const component of index.components) {
      expect(component.file.startsWith("src/")).toBe(true);
      expect(component.line).toBeGreaterThan(0);
      expect(component.id).toBe(
        `${component.file.replace(/^src\//, "").replace(/\.tsx$/, "")}#${component.name}`,
      );
    }
  });

  it("only attributes tokens to components it indexed", () => {
    const ids = new Set(index.components.map((c) => c.id));
    const usages = Object.entries(index.usages).flatMap(([name, sites]) =>
      sites.map((site) => ({ name, ...site })),
    );
    expect(usages.length).toBe(index.stats.tokenUsages);
    for (const usage of usages) {
      expect(ids.has(usage.componentId)).toBe(true);
      expect(usage.line).toBeGreaterThan(0);
    }
  });

  it("resolves the design system's union props", () => {
    const button = index.components.find((c) => c.id === "components/ui/index#Button");
    expect(button?.kind).toBe("design-system");
    const variant = button?.props.find((p) => p.name === "variant");
    expect(variant?.values).toEqual(["primary", "secondary", "outline", "ghost", "danger"]);

    const badge = index.components.find((c) => c.id === "components/ui/index#Badge");
    expect(badge?.props.find((p) => p.name === "variant")?.kind).toBe("enum");
    expect(mountPlan(button!).mountable).toBe(true);
  });

  it("lists the shell's screens as not mountable with a reason", () => {
    expect(index.screens.length).toBeGreaterThanOrEqual(10);
    expect(index.screens.some((s) => s.id === "design")).toBe(true);
    for (const screen of index.screens) {
      expect(screen.mountable).toBe(false);
      expect(screen.reason.length).toBeGreaterThan(0);
    }
  });

  it("takes breakpoints from the config or says they are defaults", () => {
    expect(index.breakpoints.map((b) => b.minWidth)).toContain(1024);
    for (const breakpoint of index.breakpoints) {
      expect(
        breakpoint.source === "tailwind.config.js" || breakpoint.source === "tailwind-default",
      ).toBe(true);
    }
  });
});
