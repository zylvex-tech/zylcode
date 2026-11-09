# Testing the Design Studio

## Commands

```bash
pnpm --filter zylcode-desktop design:index        # rebuild the artefact
pnpm --filter zylcode-desktop design:index:check  # fail if it is stale
pnpm --filter zylcode-desktop test                # index check, then vitest
pnpm --filter zylcode-desktop test:coverage       # vitest with thresholds
pnpm --filter zylcode-desktop typecheck
pnpm --filter zylcode-desktop lint
pnpm --filter zylcode-desktop build
```

`test` runs `design:index:check` first, so a stale index fails the suite even
when every assertion passes. Run Vitest from the app directory; running it from
the repository root does not pick up the app's config.

## Acceptance coverage

| # | Requirement | Test |
| --- | --- | --- |
| 1 | Real components discovered from `src/` | `design-index.test.ts` — every module in the build graph produced a record; `DesignStudio.test.tsx` — the list shows `Badge` with its source |
| 2 | Themes and tokens discovered | `design-index.test.ts` — 8 distinct themes, >100 tokens, every token cites its file and line; `DesignStudio.test.tsx` — 8 theme tabs, token values in the detail pane |
| 3 | Components traced to tokens | `catalog.test.ts` — `componentsUsingToken`; `DesignStudio.test.tsx` — usage sites for `--color-primary`, clicking one opens the component |
| 4 | Canvas renders real components | `DesignCanvas.test.tsx` — mounts `Button` and asserts `btn-primary`; `DesignStudio.test.tsx` — the same through the surface |
| 5 | Viewport switching | `DesignCanvas.test.tsx` — preset click reports 1440, custom 900 accepted, 100 rejected; `DesignStudio.test.tsx` — frame width becomes 768px |
| 6 | Theme switching | `DesignStudio.test.tsx` — frame `data-theme` follows the selector |
| 7 | Element selection and inspector | `DesignCanvas.test.tsx` — click selects `button` with class, depth and computed snapshot; `Inspector` renders attributed tokens |
| 8 | Source navigation resolves | `DesignStudio.test.tsx` — source pane opens the indexed line; unavailable service shows the reason |
| 9 | Safe prop edits | `catalog.test.ts` — enum/boolean/number/text rules; `DesignStudio.test.tsx` — selecting `ghost` changes the rendered class |
| 10 | Accessibility derived | `catalog.test.ts` — verdict rules including UNKNOWN cases; `DesignStudio.test.tsx` — contrast rows and the summary line |
| 11 | Responsive inspection | `DesignCanvas.test.tsx` — active breakpoint at 800px is `md`, preset comparison lists all three widths |
| 12 | Baseline comparison foundation | `baseline.test.ts` — capture, diff, storage round-trip, corrupt storage; `DesignStudio.test.tsx` — capture and clear through the inspector |
| 13 | App intact | `SurfaceHost.test.tsx` — the `design` surface renders the studio, `extensions` keeps its placeholder; full frontend suite below |

Gesture-only behaviour (drag pan, ctrl+wheel) is not asserted: jsdom has no
layout, so those are exercised by hand and reported as unverified.

## Results from the implementation run

Measured on Windows, Node 24, 2026-11-09, on `feat/design-studio-foundation`:

| Check | Result |
| --- | --- |
| `vitest run --coverage` | 74 files, 835 tests passed, exit 0 |
| Coverage (statements/branches/functions/lines) | 95.32 / 89.19 / 84.71 / 95.32 — above the configured 70/83/80/70 floors |
| `src/design` coverage | 84.21 / 80.67 / 47.85 / 84.21 |
| `tsc --noEmit` | exit 0 |
| `eslint src` | exit 0 |
| `prettier --check "src/**/*.{ts,tsx,css,json}"` | clean |
| `design:index:check` | 66 components, 342 tokens, 8 themes, 1577 usage sites |
| `vite build` | exit 0 |

The Rust workspace is unchanged by this branch; its checks are recorded in the
pull request.

## What is not covered

- No browser or Tauri-window run of the surface: the studio's rendering is
  asserted in jsdom, which sees the DOM but not layout, fonts or real colour
  resolution. A screenshot of the canvas in the running desktop app is not part
  of this evidence and is not claimed.
- No interaction test for pan gestures.
- No visual-regression baseline in CI; the comparison is manual and per-element.
