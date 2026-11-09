// ---------------------------------------------------------------------------
// Colour contrast (design/contrast.ts)
//
// WCAG 2.1 relative luminance and contrast ratio. Only values that can be read
// as a colour are measured: a token holding a gradient or a keyword returns
// null, and the caller reports UNKNOWN rather than inventing a number.
// ---------------------------------------------------------------------------

export interface Rgb {
  r: number;
  g: number;
  b: number;
  a: number;
}

const HEX = /^#([0-9a-f]{3,8})$/i;
const RGB_FN = /^rgba?\(\s*([\d.]+)[\s,]+([\d.]+)[\s,]+([\d.]+)(?:[\s,/]+([\d.]+%?))?\s*\)$/i;

/** Parse a CSS colour literal. Supports #rgb, #rgba, #rrggbb, #rrggbbaa and rgb()/rgba(). */
export function parseColor(value: string): Rgb | null {
  const trimmed = value.trim();

  const hex = HEX.exec(trimmed);
  if (hex) {
    const digits = hex[1];
    const expand = (s: string) => parseInt(s.length === 1 ? s + s : s, 16);
    if (digits.length === 3)
      return { r: expand(digits[0]), g: expand(digits[1]), b: expand(digits[2]), a: 1 };
    if (digits.length === 4)
      return {
        r: expand(digits[0]),
        g: expand(digits[1]),
        b: expand(digits[2]),
        a: expand(digits[3]) / 255,
      };
    if (digits.length === 6)
      return {
        r: parseInt(digits.slice(0, 2), 16),
        g: parseInt(digits.slice(2, 4), 16),
        b: parseInt(digits.slice(4, 6), 16),
        a: 1,
      };
    if (digits.length === 8)
      return {
        r: parseInt(digits.slice(0, 2), 16),
        g: parseInt(digits.slice(2, 4), 16),
        b: parseInt(digits.slice(4, 6), 16),
        a: parseInt(digits.slice(6, 8), 16) / 255,
      };
    return null;
  }

  const fn = RGB_FN.exec(trimmed);
  if (fn) {
    const alpha =
      fn[4] === undefined
        ? 1
        : fn[4].endsWith("%")
          ? Number(fn[4].slice(0, -1)) / 100
          : Number(fn[4]);
    return { r: Number(fn[1]), g: Number(fn[2]), b: Number(fn[3]), a: alpha };
  }

  return null;
}

/** WCAG relative luminance of an opaque colour. */
export function relativeLuminance(rgb: Rgb): number {
  const channel = (v: number) => {
    const s = v / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * channel(rgb.r) + 0.7152 * channel(rgb.g) + 0.0722 * channel(rgb.b);
}

/** Contrast ratio between two colours, or null when either cannot be parsed. */
export function contrastRatio(foreground: string, background: string): number | null {
  const fg = parseColor(foreground);
  const bg = parseColor(background);
  if (!fg || !bg) return null;
  const l1 = relativeLuminance(fg);
  const l2 = relativeLuminance(bg);
  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);
  return (lighter + 0.05) / (darker + 0.05);
}

/**
 * AA verdict for a ratio. Body text needs 4.5:1; large text (>= 18.66px bold or
 * >= 24px) needs 3:1.
 */
export function contrastVerdict(
  ratio: number | null,
  largeText = false,
): "PASS" | "WARNING" | "UNKNOWN" {
  if (ratio === null) return "UNKNOWN";
  return ratio >= (largeText ? 3 : 4.5) ? "PASS" : "WARNING";
}
