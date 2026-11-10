// ---------------------------------------------------------------------------
// Token attribution (design/attribution.ts)
//
// Which token produced the pixels the canvas is showing? The browser has
// already resolved every variable by the time we can read a computed style, so
// the answer is a comparison: computed value against token value, colour by
// exact channel match and lengths within half a pixel.
//
// Anything that does not match stays unattributed. A near miss is not a match,
// and a mismatch is often the interesting finding (a hard-coded hex).
// ---------------------------------------------------------------------------

import { parseColor } from "./contrast";
import { tokenValue } from "./catalog";
import type { DesignIndex, DesignToken } from "./types";

export interface ComputedSnapshot {
  color?: string;
  backgroundColor?: string;
  borderColor?: string;
  fontSize?: string;
  lineHeight?: string;
  borderRadius?: string;
  paddingTop?: string;
  paddingLeft?: string;
  fontFamily?: string;
}

const LENGTH = /^(-?[\d.]+)(px|rem|em|pt)$/i;

/** CSS length in px. `rem` uses the document root size; `em` cannot be resolved here. */
export function lengthToPx(value: string, rootFontSize = 16): number | null {
  const m = LENGTH.exec(value.trim());
  if (!m) return null;
  const n = Number(m[1]);
  if (!Number.isFinite(n)) return null;
  switch (m[2].toLowerCase()) {
    case "px":
      return n;
    case "rem":
      return n * rootFontSize;
    case "pt":
      return (n * 96) / 72;
    default:
      return null;
  }
}

function sameColour(a: string, b: string): boolean {
  const ca = parseColor(a);
  const cb = parseColor(b);
  if (!ca || !cb) return false;
  // A transparent computed value carries no colour information.
  if (ca.a === 0 || cb.a === 0)
    return ca.a === cb.a && ca.r === cb.r && ca.g === cb.g && ca.b === cb.b;
  return (
    Math.round(ca.r) === Math.round(cb.r) &&
    Math.round(ca.g) === Math.round(cb.g) &&
    Math.round(ca.b) === Math.round(cb.b) &&
    Math.abs(ca.a - cb.a) < 0.02
  );
}

function closeEnough(a: number | null, b: number | null, tolerance = 0.5): boolean {
  return a !== null && b !== null && Math.abs(a - b) <= tolerance;
}

/** Read the snapshot the canvas attributes tokens from. */
export function snapshotElement(element: Element, rootFontSize = 16): ComputedSnapshot {
  const styles = window.getComputedStyle(element);
  void rootFontSize;
  return {
    color: styles.color,
    backgroundColor: styles.backgroundColor,
    borderColor: styles.borderColor,
    fontSize: styles.fontSize,
    lineHeight: styles.lineHeight,
    borderRadius: styles.borderRadius,
    paddingTop: styles.paddingTop,
    paddingLeft: styles.paddingLeft,
    fontFamily: styles.fontFamily,
  };
}

/** Tokens whose values explain the computed snapshot, across colour, type, radius and space. */
export function matchTokens(
  index: DesignIndex,
  theme: string,
  snapshot: ComputedSnapshot,
): string[] {
  const matches = new Set<string>();
  const themeTokens = index.tokens.filter((t) => t.theme === theme || t.theme === "base");

  const colourFields: Array<[string | undefined, DesignToken[]]> = [
    [
      snapshot.color,
      themeTokens.filter(
        (t) => t.category === "color" && /^(--color-text|--color-on)/.test(t.name),
      ),
    ],
    [
      snapshot.backgroundColor,
      themeTokens.filter(
        (t) =>
          t.category === "color" &&
          /^--color-(background|surface|primary|secondary|accent|success|warning|error|info)/.test(
            t.name,
          ),
      ),
    ],
    [
      snapshot.borderColor,
      themeTokens.filter((t) => t.category === "color" && /^--color-border/.test(t.name)),
    ],
  ];

  for (const [value, candidates] of colourFields) {
    if (!value) continue;
    for (const token of candidates) {
      if (sameColour(value, token.value)) matches.add(token.name);
    }
  }

  if (snapshot.fontSize) {
    const size = lengthToPx(snapshot.fontSize);
    for (const token of themeTokens) {
      if (token.category !== "typography" || !token.name.startsWith("--text-")) continue;
      if (closeEnough(size, lengthToPx(token.value))) matches.add(token.name);
    }
  }

  if (snapshot.borderRadius) {
    const radius = lengthToPx(snapshot.borderRadius);
    for (const token of themeTokens) {
      if (token.category !== "radius") continue;
      if (closeEnough(radius, lengthToPx(token.value), 1)) matches.add(token.name);
    }
  }

  for (const padding of [snapshot.paddingTop, snapshot.paddingLeft]) {
    if (!padding) continue;
    const px = lengthToPx(padding);
    for (const token of themeTokens) {
      if (token.category !== "spacing") continue;
      if (closeEnough(px, lengthToPx(token.value))) matches.add(token.name);
    }
  }

  if (snapshot.lineHeight && snapshot.fontSize) {
    const lineHeight = lengthToPx(snapshot.lineHeight);
    const fontSize = lengthToPx(snapshot.fontSize);
    for (const token of themeTokens) {
      if (token.category !== "typography" || !token.name.startsWith("--leading-")) continue;
      const unitless = Number(token.value);
      if (!Number.isFinite(unitless) || fontSize === null) continue;
      if (closeEnough(lineHeight, unitless * fontSize, 0.6)) matches.add(token.name);
    }
  }

  return [...matches].sort();
}

/** True when the value is a literal colour that a theme actually defines. */
export function isKnownColour(index: DesignIndex, theme: string, value: string): boolean {
  const token = tokenValue(index, value, theme);
  return token ? parseColor(token.value) !== null : false;
}
