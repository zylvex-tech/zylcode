import { describe, expect, it } from "vitest";
import {
  activeBreakpoint,
  a11yFindings,
  componentsUsingToken,
  coercePropValue,
  componentChildren,
  componentsOfKind,
  contrastMatrix,
  getComponent,
  mountPlan,
  parentComponents,
  searchComponents,
  searchTokens,
  tokenValue,
  tokensOfTheme,
  usagesOfToken,
} from "./catalog";
import type { ComponentEntry, DesignIndex, DesignToken } from "./types";

// ---------------------------------------------------------------------------
// The catalogue is the only thing standing between a generated index and the
// UI, so its rules are tested directly: what is mountable, what a prop edit may
// become, and which verdicts static analysis is allowed to produce.
// ---------------------------------------------------------------------------

function component(overrides: Partial<ComponentEntry>): ComponentEntry {
  return {
    id: "ui/Button#Button",
    name: "Button",
    file: "src/components/ui/Button.tsx",
    line: 10,
    kind: "design-system",
    props: [],
    propsUnknown: false,
    extends: [],
    children: [],
    classes: [],
    classSites: [],
    a11y: {
      ariaLabels: 0,
      roles: 0,
      labelledControls: 0,
      imagesWithoutAlt: 0,
      clickHandlers: 0,
      nonInteractiveClickHandlers: 0,
      interactiveElements: 0,
      tabIndexes: 0,
    },
    tokens: [],
    ...overrides,
  };
}

const token = (
  name: string,
  value: string,
  theme: string,
  category: DesignToken["category"] = "color",
): DesignToken => ({
  name,
  value,
  category,
  theme,
  file: "src/styles/themes.css",
  line: 1,
});

const index: DesignIndex = {
  contract: "design-studio-index/1",
  sourceRoot: "apps/zylcode-desktop/src",
  note: "fixture",
  stats: {
    files: 1,
    components: 3,
    designSystemComponents: 2,
    screens: 1,
    pages: 0,
    themes: 1,
    baseTokens: 1,
    themeTokens: 2,
    tokens: 3,
    tokenUsages: 1,
  },
  breakpoints: [
    { name: "sm", minWidth: 640, source: "tailwind-default" },
    { name: "lg", minWidth: 1024, source: "tailwind-default" },
  ],
  themes: [{ id: "midnight-pro", isDark: true, line: 71, tokenCount: 2 }],
  tokens: [
    token("--space-4", "1rem", "base", "spacing"),
    token("--color-primary", "#3b82f6", "midnight-pro"),
    token("--color-text-primary", "#f8fafc", "midnight-pro"),
    token("--color-background", "#0f172a", "midnight-pro"),
  ],
  usages: {
    "--color-primary": [{ componentId: "ui/Button#Button", class: "bg-primary", line: 20 }],
  },
  components: [
    component({
      props: [
        {
          name: "variant",
          type: '"primary" | "ghost"',
          required: false,
          values: ["primary", "ghost"],
          kind: "enum",
        },
        { name: "children", type: "React.ReactNode", required: true, values: null, kind: "node" },
      ],
      children: [{ name: "Badge", from: "./Badge" }],
    }),
    component({ id: "ui/Badge#Badge", name: "Badge", kind: "design-system" }),
    component({
      id: "Modal#Modal",
      name: "Modal",
      kind: "component",
      file: "src/components/Modal.tsx",
      props: [
        { name: "onClose", type: "() => void", required: true, values: null, kind: "function" },
      ],
    }),
    component({
      id: "shell/SurfaceHost#SurfaceHost",
      name: "SurfaceHost",
      kind: "screen",
      propsUnknown: true,
    }),
  ],
  screens: [],
  tailwind: { colors: [], spacing: [], radius: [], fontSize: [], shadow: [] },
};

describe("lookups", () => {
  it("finds a component by id and returns undefined for unknown ids", () => {
    expect(getComponent(index, "ui/Badge#Badge")?.name).toBe("Badge");
    expect(getComponent(index, "nope")).toBeUndefined();
  });

  it("filters by kind", () => {
    expect(componentsOfKind(index, "design-system").map((c) => c.name)).toEqual([
      "Button",
      "Badge",
    ]);
    expect(componentsOfKind(index, "all")).toHaveLength(4);
  });

  it("searches name, file, classes and tokens", () => {
    expect(searchComponents(index, "badge").map((c) => c.name)).toEqual(["Badge"]);
    expect(searchComponents(index, "Modal.tsx").map((c) => c.name)).toEqual(["Modal"]);
    expect(searchComponents(index, "")).toHaveLength(4);
  });

  it("returns only the tokens of the requested theme", () => {
    expect(tokensOfTheme(index, "base").map((t) => t.name)).toEqual(["--space-4"]);
    expect(tokensOfTheme(index, "midnight-pro")).toHaveLength(3);
  });

  it("filters tokens by query and category", () => {
    const theme = tokensOfTheme(index, "midnight-pro");
    expect(searchTokens(theme, "primary", "all").map((t) => t.name)).toEqual([
      "--color-primary",
      "--color-text-primary",
    ]);
    expect(searchTokens(theme, "", "spacing")).toHaveLength(0);
  });

  it("traces a token back to the components that use it", () => {
    expect(usagesOfToken(index, "--color-primary", "midnight-pro")).toEqual([
      { componentId: "ui/Button#Button", class: "bg-primary", line: 20 },
    ]);
    expect(
      componentsUsingToken(index, "--color-primary", "midnight-pro").map((c) => c.name),
    ).toEqual(["Button"]);
  });

  it("falls back to the base scale for scale tokens", () => {
    expect(tokenValue(index, "--space-4", "midnight-pro")?.theme).toBe("base");
  });

  it("resolves component children and parents through the index", () => {
    const button = getComponent(index, "ui/Button#Button")!;
    const badge = getComponent(index, "ui/Badge#Badge")!;
    expect(componentChildren(index, button).map((c) => c.name)).toEqual(["Badge"]);
    expect(parentComponents(index, badge).map((c) => c.name)).toEqual(["Button"]);
  });
});

describe("mountPlan", () => {
  it("mounts a component whose only required prop is children", () => {
    const plan = mountPlan(getComponent(index, "ui/Button#Button")!);
    expect(plan.mountable).toBe(true);
    expect(plan.reason).toBeNull();
    expect(plan.initialProps).toMatchObject({ children: "Design Studio", variant: "primary" });
  });

  it("refuses a component that needs a callback and names the prop", () => {
    const plan = mountPlan(getComponent(index, "Modal#Modal")!);
    expect(plan.mountable).toBe(false);
    expect(plan.reason).toContain("onClose");
  });

  it("refuses screens and unresolved props", () => {
    expect(mountPlan(getComponent(index, "shell/SurfaceHost#SurfaceHost")!).reason).toContain(
      "state",
    );
    const unresolved = mountPlan(component({ propsUnknown: true }));
    expect(unresolved.mountable).toBe(false);
    expect(unresolved.reason).toContain("props type");
  });
});

describe("coercePropValue", () => {
  const variant = {
    name: "variant",
    type: '"primary" | "ghost"',
    required: false,
    values: ["primary", "ghost"],
    kind: "enum" as const,
  };

  it("accepts an enum member and rejects anything else", () => {
    expect(coercePropValue(variant, "ghost")).toEqual({ ok: true, value: "ghost" });
    expect(coercePropValue(variant, "danger").ok).toBe(false);
  });

  it("parses booleans strictly", () => {
    const prop = {
      name: "loading",
      type: "boolean",
      required: false,
      values: null,
      kind: "boolean" as const,
    };
    expect(coercePropValue(prop, "true")).toEqual({ ok: true, value: true });
    expect(coercePropValue(prop, "yes").ok).toBe(false);
  });

  it("rejects non-finite numbers", () => {
    const prop = {
      name: "count",
      type: "number",
      required: false,
      values: null,
      kind: "number" as const,
    };
    expect(coercePropValue(prop, "12")).toEqual({ ok: true, value: 12 });
    expect(coercePropValue(prop, "NaN").ok).toBe(false);
  });

  it("caps text props and refuses props the canvas cannot construct", () => {
    const text = {
      name: "children",
      type: "ReactNode",
      required: true,
      values: null,
      kind: "node" as const,
    };
    expect(coercePropValue(text, "x".repeat(201)).ok).toBe(false);
    const fn = {
      name: "onClose",
      type: "() => void",
      required: true,
      values: null,
      kind: "function" as const,
    };
    expect(coercePropValue(fn, "alert(1)").ok).toBe(false);
  });
});

describe("a11yFindings", () => {
  it("uses UNKNOWN where static analysis cannot decide", () => {
    const findings = a11yFindings(component({}));
    expect(findings.every((f) => f.verdict !== "WARNING")).toBe(true);
    expect(findings.find((f) => f.rule === "accessible-name")?.verdict).toBe("UNKNOWN");
    expect(findings.find((f) => f.rule === "focus-order")?.detail).toContain("rendered tree");
  });

  it("warns about images without alt text and click handlers on inert elements", () => {
    const findings = a11yFindings(
      component({
        a11y: {
          ariaLabels: 0,
          roles: 0,
          labelledControls: 0,
          imagesWithoutAlt: 2,
          clickHandlers: 1,
          nonInteractiveClickHandlers: 1,
          interactiveElements: 0,
          tabIndexes: 0,
        },
      }),
    );
    expect(findings.find((f) => f.rule === "image-alt")?.verdict).toBe("WARNING");
    expect(findings.find((f) => f.rule === "click-target")?.verdict).toBe("WARNING");
  });

  it("passes a labelled interactive component", () => {
    const findings = a11yFindings(
      component({
        a11y: {
          ariaLabels: 1,
          roles: 0,
          labelledControls: 1,
          imagesWithoutAlt: 0,
          clickHandlers: 1,
          nonInteractiveClickHandlers: 0,
          interactiveElements: 1,
          tabIndexes: 0,
        },
      }),
    );
    expect(findings.find((f) => f.rule === "accessible-name")?.verdict).toBe("PASS");
    expect(findings.find((f) => f.rule === "click-target")?.verdict).toBe("PASS");
  });
});

describe("contrastMatrix", () => {
  it("measures the pairs the theme defines and marks unparsable values UNKNOWN", () => {
    const rows = contrastMatrix(index, "midnight-pro");
    const body = rows.find(
      (r) => r.foreground === "--color-text-primary" && r.background === "--color-background",
    );
    expect(body?.ratio).toBeCloseTo(17.06, 1);
    expect(body?.verdict).toBe("PASS");

    const withGradient: DesignIndex = {
      ...index,
      tokens: [
        ...index.tokens,
        token("--color-surface", "linear-gradient(red, blue)", "midnight-pro"),
      ],
    };
    const unknown = contrastMatrix(withGradient, "midnight-pro").find(
      (r) => r.background === "--color-surface",
    );
    expect(unknown?.verdict).toBe("UNKNOWN");
  });
});

describe("activeBreakpoint", () => {
  it("reports the highest breakpoint the width reaches", () => {
    expect(activeBreakpoint(index, 1440)?.name).toBe("lg");
    expect(activeBreakpoint(index, 800)?.name).toBe("sm");
    expect(activeBreakpoint(index, 500)).toBeNull();
  });
});
