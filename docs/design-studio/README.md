# Design Studio

The Design Studio is the desktop shell's `design` surface. It is a **view over
the application's own source**, not a parallel design system: every token,
component, prop and usage it shows was read out of `apps/zylcode-desktop/src`
by the indexer, and the canvas mounts the real component modules.

Status vocabulary for this surface, using the repository's fixed terms:

| Capability | Status | Where it is proven |
| --- | --- | --- |
| Token explorer (values, categories, usage trace) | `VERIFIED_RUNTIME` | `src/design/TokenExplorer.tsx`, `catalog.test.ts`, `design-index.test.ts` |
| Component explorer (props, source, structure) | `VERIFIED_RUNTIME` | `src/design/ComponentExplorer.tsx`, `Inspector.tsx` |
| Canvas mounting real components | `VERIFIED_RUNTIME` | `DesignCanvas.test.tsx`, `DesignStudio.test.tsx` |
| Viewport switching (1440/1024/768 + custom) and breakpoint reporting | `VERIFIED_RUNTIME` | `DesignCanvas.test.tsx` |
| Zoom and pan | `VERIFIED_RUNTIME` (zoom), `VERIFIED_LOCAL_ONLY` (pan gestures) | zoom is asserted; drag panning is pointer input, not covered by jsdom |
| Theme switching on the canvas | `VERIFIED_RUNTIME` | `DesignStudio.test.tsx` |
| Element selection with computed styles | `VERIFIED_RUNTIME` | `DesignCanvas.test.tsx` |
| Token attribution from computed values | `VERIFIED_LOCAL_ONLY` | `attribution.test.ts` against fixtures |
| Accessibility verdicts (PASS/WARNING/UNKNOWN) | `PARTIAL` | static rules + WCAG contrast are tested; no axe, keyboard or screen-reader pass |
| Source navigation | `VERIFIED_RUNTIME` (browser service), `VERIFIED_LOCAL_ONLY` (Tauri `file_content`) | `DesignStudio.test.tsx` with a mocked service |
| Baseline comparison | `PARTIAL` | property diff of computed styles in `baseline.test.ts`; no pixel diff |
| Screen/page inspection | `PARTIAL` | screens are listed and their source opens; they are not mountable |
| AI assistance in the studio | `NOT_IMPLEMENTED` | — |

"Verified" above means verified by the named automated test. Nothing in this
document claims a desktop-window check of the Design Studio.

## Data flow

```
apps/zylcode-desktop/src/**/*.tsx ─┐
src/styles/themes.css ─────────────┼─→ scripts/design-index.mjs ─→ src/design/generated/design-index.json
tailwind.config.js ────────────────┘                                        │
                                                                            ▼
                          types.ts · catalog.ts · contrast.ts · attribution.ts
                                                                            │
                        ┌───────────────┬───────────────┬───────────────────┘
                        ▼               ▼               ▼
                  TokenExplorer   ComponentExplorer   DesignCanvas ⇄ Inspector
                                                          │
                                                    AccessibilityPanel
```

The JSON artefact is committed. `design:index:check` fails when it no longer
matches the source, so a stale index is a red test rather than a quiet lie.

## Why an artefact instead of reading files at runtime

The renderer has no filesystem, and a runtime scan of ~50 modules on every
interaction would be slower than the thing it inspects. The index is produced
once, deterministically, and consumed by both the UI and the tests.

## Boundaries

- The studio **reads** the repository. It does not write components, tokens or
  files; prop edits live in surface state, and the baseline store lives in
  `localStorage`.
- There is no second token catalogue. Token values come from
  `src/styles/themes.css`; if that file changes, the index and the tests move
  with it.
- No screenshots stand in for rendering. The canvas imports modules through
  Vite's graph (`import.meta.glob`), which is why an import failure surfaces as
  `IMPORT FAILED` with the module's error text.

## Documents

| Document | Covers |
| --- | --- |
| [INDEXER.md](./INDEXER.md) | extraction rules, determinism, `--check`, what stays unknown |
| [TOKENS_AND_COMPONENTS.md](./TOKENS_AND_COMPONENTS.md) | token model, attribution, component model, mountability |
| [CANVAS_AND_INSPECTOR.md](./CANVAS_AND_INSPECTOR.md) | canvas, responsive, selection, props, source navigation, comparison |
| [ACCESSIBILITY.md](./ACCESSIBILITY.md) | verdict semantics and the contrast maths |
| [TESTING.md](./TESTING.md) | acceptance coverage and the commands that produce it |
| [LIMITATIONS.md](./LIMITATIONS.md) | what this wave does not do, stated plainly |
