// ---------------------------------------------------------------------------
// Design Studio model (design/types.ts)
//
// The shape of `design-index.json`. The indexer writes this file from the
// application's own source; nothing here is authored by hand, and every field
// that could be wrong is either derived mechanically or marked `unknown`.
// ---------------------------------------------------------------------------

export type TokenCategory =
  "color" | "spacing" | "radius" | "typography" | "shadow" | "motion" | "depth" | "other";

export type ComponentKind = "component" | "design-system" | "screen" | "page";

export type PropKind =
  "string" | "number" | "boolean" | "node" | "function" | "array" | "enum" | "unknown";

export interface PropSpec {
  name: string;
  /** The TypeScript type exactly as written, e.g. `"sm" | "md"`. */
  type: string;
  required: boolean;
  /** Union members, when the type resolves to literals. */
  values: string[] | null;
  kind: PropKind;
}

export interface ClassSite {
  utility: string;
  line: number;
}

export interface TokenUsage {
  componentId: string;
  /** The utility that carries the token, e.g. `bg-surface`. */
  class: string;
  line: number;
}

export interface A11ySignals {
  ariaLabels: number;
  roles: number;
  labelledControls: number;
  imagesWithoutAlt: number;
  clickHandlers: number;
  nonInteractiveClickHandlers: number;
  interactiveElements: number;
  tabIndexes: number;
}

export interface RenderedChild {
  name: string;
  from: string;
}

export interface DesignToken {
  name: string;
  value: string;
  category: TokenCategory;
  /** `base` for the shared scale, otherwise a `[data-theme]` id. */
  theme: string;
  file: string;
  line: number;
}

export interface ComponentEntry {
  /** Stable identity: `<module path without extension>#<export name>`. */
  id: string;
  name: string;
  /** Workspace-relative source path. */
  file: string;
  line: number;
  kind: ComponentKind;
  props: PropSpec[];
  /** True when the props type could not be resolved, so requirements are unknown. */
  propsUnknown: boolean;
  extends: string[];
  children: RenderedChild[];
  classes: string[];
  classSites: ClassSite[];
  a11y: A11ySignals;
  /** Token names this component's utilities resolve to. */
  tokens: string[];
}

export interface DesignTheme {
  id: string;
  isDark: boolean;
  line: number;
  tokenCount: number;
}

export interface Breakpoint {
  name: string;
  minWidth: number;
  /** Where the value came from — an application choice or the framework default. */
  source: string;
}

export interface ScreenEntry {
  id: string;
  name: string;
  file: string;
  mountable: boolean;
  reason: string;
}

export interface DesignIndexStats {
  files: number;
  components: number;
  designSystemComponents: number;
  screens: number;
  pages: number;
  themes: number;
  baseTokens: number;
  themeTokens: number;
  tokens: number;
  tokenUsages: number;
}

export interface UtilityMapping {
  utility: string;
  variable: string | null;
}

export interface DesignIndex {
  contract: string;
  sourceRoot: string;
  note: string;
  stats: DesignIndexStats;
  breakpoints: Breakpoint[];
  themes: DesignTheme[];
  tokens: DesignToken[];
  /** Usage sites per variable name — shared by every theme that defines it. */
  usages: Record<string, TokenUsage[]>;
  components: ComponentEntry[];
  screens: ScreenEntry[];
  tailwind: {
    colors: UtilityMapping[];
    spacing: UtilityMapping[];
    radius: UtilityMapping[];
    fontSize: UtilityMapping[];
    shadow: UtilityMapping[];
  };
}

export type Verdict = "PASS" | "WARNING" | "UNKNOWN";

export interface Finding {
  rule: string;
  verdict: Verdict;
  detail: string;
}

export interface ContrastRow {
  foreground: string;
  background: string;
  ratio: number | null;
  /** WCAG AA threshold this row is measured against. */
  threshold: number;
  verdict: Verdict;
}
