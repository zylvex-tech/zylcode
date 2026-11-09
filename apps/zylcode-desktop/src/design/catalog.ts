// ---------------------------------------------------------------------------
// Design catalogue (design/catalog.ts)
//
// Every read the Design Studio performs on the generated index. Pure functions,
// no React, so the same code answers the UI and the tests.
//
// The rule throughout: report what the index and the maths prove. A prop whose
// type did not resolve is `unknown` and blocks mounting; a colour that is not a
// literal is UNKNOWN contrast rather than a plausible number.
// ---------------------------------------------------------------------------

import { contrastRatio } from "./contrast";
import type {
  ComponentEntry,
  ComponentKind,
  ContrastRow,
  DesignIndex,
  DesignToken,
  Finding,
  PropSpec,
  TokenCategory,
  TokenUsage,
} from "./types";

export interface MountPlan {
  mountable: boolean;
  /** Why this component cannot be mounted, in the inspector's own words. */
  reason: string | null;
  initialProps: Record<string, unknown>;
}

/** Canvas widths the brief names, plus whatever the user types. */
export const VIEWPORT_PRESETS = [1440, 1024, 768] as const;

export const TOKEN_CATEGORIES: TokenCategory[] = [
  "color",
  "spacing",
  "radius",
  "typography",
  "shadow",
  "motion",
  "depth",
  "other",
];

export function getComponent(index: DesignIndex, id: string): ComponentEntry | undefined {
  return index.components.find((c) => c.id === id);
}

export function componentsOfKind(
  index: DesignIndex,
  kind: ComponentKind | "all",
): ComponentEntry[] {
  return kind === "all" ? index.components : index.components.filter((c) => c.kind === kind);
}

export function searchComponents(index: DesignIndex, query: string): ComponentEntry[] {
  const q = query.trim().toLowerCase();
  if (!q) return index.components;
  return index.components.filter(
    (c) =>
      c.name.toLowerCase().includes(q) ||
      c.file.toLowerCase().includes(q) ||
      c.tokens.some((t) => t.includes(q)) ||
      c.classes.some((c2) => c2.toLowerCase().includes(q)),
  );
}

/** Tokens of one theme, or the shared `base` scale. */
export function tokensOfTheme(index: DesignIndex, theme: string): DesignToken[] {
  return index.tokens.filter((t) => t.theme === theme);
}

export function tokenValue(
  index: DesignIndex,
  name: string,
  theme: string,
): DesignToken | undefined {
  return (
    index.tokens.find((t) => t.name === name && t.theme === theme) ??
    index.tokens.find((t) => t.name === name && t.theme === "base")
  );
}

export function searchTokens(
  tokens: DesignToken[],
  query: string,
  category: TokenCategory | "all",
): DesignToken[] {
  const q = query.trim().toLowerCase();
  return tokens.filter((t) => {
    if (category !== "all" && t.category !== category) return false;
    if (!q) return true;
    return t.name.toLowerCase().includes(q) || t.value.toLowerCase().includes(q);
  });
}

export function usagesOfToken(index: DesignIndex, name: string, theme: string): TokenUsage[] {
  if (!tokenValue(index, name, theme)) return [];
  return index.usages[name] ?? [];
}

/** Distinct components that use a token, deduplicated and ordered by name. */
export function componentsUsingToken(
  index: DesignIndex,
  name: string,
  theme: string,
): ComponentEntry[] {
  const ids = new Set(usagesOfToken(index, name, theme).map((u) => u.componentId));
  return index.components.filter((c) => ids.has(c.id)).sort((a, b) => a.name.localeCompare(b.name));
}

/**
 * Which props the canvas can supply, and which make mounting impossible.
 *
 * `children` is the one required `node` the canvas can fill with real text.
 * Anything else it cannot construct — a callback, a data object, an array of
 * domain records — is a genuine reason not to render the component, and the
 * inspector says so instead of showing a broken frame.
 */
export function mountPlan(component: ComponentEntry): MountPlan {
  if (component.kind === "screen" || component.kind === "page") {
    return {
      mountable: false,
      reason: `${component.kind === "page" ? "page" : "screen"} composes the application shell and needs its state`,
      initialProps: {},
    };
  }
  if (component.propsUnknown) {
    return {
      mountable: false,
      reason: "the index could not resolve this component's props type",
      initialProps: {},
    };
  }

  const initialProps: Record<string, unknown> = {};

  for (const prop of component.props) {
    if (prop.name === "children") {
      if (prop.required) initialProps.children = "Design Studio";
      continue;
    }
    if (!prop.required) {
      if (prop.kind === "enum" && prop.values?.length) initialProps[prop.name] = prop.values[0];
      else if (prop.kind === "boolean") initialProps[prop.name] = false;
      continue;
    }
    if (prop.kind === "enum" && prop.values?.length) {
      initialProps[prop.name] = prop.values[0];
      continue;
    }
    if (prop.kind === "boolean") {
      initialProps[prop.name] = false;
      continue;
    }
    if (prop.kind === "string" || prop.kind === "number") {
      initialProps[prop.name] = prop.kind === "string" ? "" : 0;
      continue;
    }
    return {
      mountable: false,
      reason: `requires \`${prop.name}: ${prop.type}\`, which the canvas cannot construct`,
      initialProps: {},
    };
  }

  return { mountable: true, reason: null, initialProps };
}

export type CoercedProp = { ok: true; value: unknown } | { ok: false; reason: string };

/**
 * Convert inspector input into a prop value without evaluating anything.
 *
 * There is no `new Function` here on purpose: the canvas renders application
 * components, so prop input is a data-format problem, not a scripting one.
 */
export function coercePropValue(prop: PropSpec, raw: string | boolean | number): CoercedProp {
  if (prop.kind === "enum") {
    const text = String(raw);
    if (!prop.values?.includes(text)) {
      return {
        ok: false,
        reason: `\`${text}\` is not one of ${prop.values?.join(" | ") ?? "the allowed values"}`,
      };
    }
    return { ok: true, value: text };
  }

  if (prop.kind === "boolean") {
    if (typeof raw === "boolean") return { ok: true, value: raw };
    if (raw === "true" || raw === "false") return { ok: true, value: raw === "true" };
    return { ok: false, reason: "a boolean prop takes true or false" };
  }

  if (prop.kind === "number") {
    const value = typeof raw === "number" ? raw : Number(raw);
    if (!Number.isFinite(value))
      return { ok: false, reason: "a number prop takes a finite number" };
    return { ok: true, value };
  }

  if (prop.kind === "string") {
    const value = String(raw);
    if (value.length > 200)
      return { ok: false, reason: "text props are limited to 200 characters" };
    return { ok: true, value };
  }

  if (prop.kind === "node" && prop.name === "children") {
    const value = String(raw);
    if (value.length > 200)
      return { ok: false, reason: "preview text is limited to 200 characters" };
    return { ok: true, value };
  }

  return { ok: false, reason: `\`${prop.name}: ${prop.type}\` cannot be edited here` };
}

/**
 * Accessibility findings that static analysis can actually support.
 *
 * An absence of aria attributes is not a failure — a `<button>Label</button>`
 * has an accessible name from its text — so those cases are UNKNOWN, and the
 * panel says what would decide them.
 */
export function a11yFindings(component: ComponentEntry): Finding[] {
  const a = component.a11y;
  const findings: Finding[] = [];

  if (a.imagesWithoutAlt > 0) {
    findings.push({
      rule: "image-alt",
      verdict: "WARNING",
      detail: `${a.imagesWithoutAlt} <img> element(s) without alt text`,
    });
  } else if (a.interactiveElements > 0 || a.roles > 0) {
    findings.push({ rule: "image-alt", verdict: "PASS", detail: "no <img> without alt text" });
  }

  if (a.nonInteractiveClickHandlers > 0) {
    findings.push({
      rule: "click-target",
      verdict: "WARNING",
      detail: `${a.nonInteractiveClickHandlers} click handler(s) on non-interactive element(s); keyboard users cannot reach them`,
    });
  } else if (a.clickHandlers > 0) {
    findings.push({
      rule: "click-target",
      verdict: "PASS",
      detail: "every click handler sits on an interactive element",
    });
  }

  if (a.interactiveElements === 0) {
    findings.push({
      rule: "accessible-name",
      verdict: "UNKNOWN",
      detail: "no interactive elements in this component; nothing to name",
    });
  } else if (a.ariaLabels > 0 || a.labelledControls > 0) {
    findings.push({
      rule: "accessible-name",
      verdict: "PASS",
      detail: `${a.ariaLabels} aria label(s), ${a.labelledControls} labelled control(s)`,
    });
  } else {
    findings.push({
      rule: "accessible-name",
      verdict: "UNKNOWN",
      detail: "no aria-label or htmlFor in source; a text child may still name the control",
    });
  }

  findings.push({
    rule: "focus-order",
    verdict: "UNKNOWN",
    detail:
      a.tabIndexes > 0
        ? `${a.tabIndexes} explicit tabIndex value(s); ordering needs a rendered keyboard pass`
        : "tab order is a property of the rendered tree, not of this source",
  });

  return findings;
}

const CONTRAST_PAIRS: Array<[string, string]> = [
  ["--color-text-primary", "--color-background"],
  ["--color-text-secondary", "--color-background"],
  ["--color-text-muted", "--color-background"],
  ["--color-text-primary", "--color-surface"],
  ["--color-text-muted", "--color-surface"],
  ["--color-primary-foreground", "--color-primary"],
  ["--color-secondary-foreground", "--color-secondary"],
  ["--color-border", "--color-background"],
];

/** WCAG AA contrast for the pairs the palette actually defines in one theme. */
export function contrastMatrix(index: DesignIndex, theme: string): ContrastRow[] {
  const rows: ContrastRow[] = [];
  for (const [fg, bg] of CONTRAST_PAIRS) {
    const fgToken = tokenValue(index, fg, theme);
    const bgToken = tokenValue(index, bg, theme);
    if (!fgToken || !bgToken) continue;
    const ratio = contrastRatio(fgToken.value, bgToken.value);
    // Borders are measured as UI boundaries (WCAG 1.4.11, 3:1); text is body
    // text (1.4.3, 4.5:1). A ratio that cannot be parsed has no verdict.
    const threshold = fg === "--color-border" ? 3 : 4.5;
    const verdict = ratio === null ? "UNKNOWN" : ratio >= threshold ? "PASS" : "WARNING";
    rows.push({ foreground: fg, background: bg, ratio, threshold, verdict });
  }
  return rows;
}

/** Breakpoint active at a canvas width; null below the smallest. */
export function activeBreakpoint(index: DesignIndex, width: number) {
  const sorted = [...index.breakpoints].sort((a, b) => a.minWidth - b.minWidth);
  let active: (typeof sorted)[number] | null = null;
  for (const breakpoint of sorted) {
    if (width >= breakpoint.minWidth) active = breakpoint;
  }
  return active;
}

/** Components that render another component, resolved through the index. */
export function componentChildren(index: DesignIndex, component: ComponentEntry): ComponentEntry[] {
  const names = new Map(index.components.map((c) => [c.name, c]));
  return component.children
    .map((child) => names.get(child.name))
    .filter((c): c is ComponentEntry => c !== undefined);
}

export function parentComponents(index: DesignIndex, component: ComponentEntry): ComponentEntry[] {
  return index.components.filter((c) => c.children.some((child) => child.name === component.name));
}
