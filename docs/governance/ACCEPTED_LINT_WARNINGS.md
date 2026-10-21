# Accepted ESLint warnings

**Status:** the complete inventory of warnings reported by
`pnpm --filter zylcode-desktop lint` on the hardening branch. The CI step runs
plain `eslint` (no `--max-warnings`), so warnings do not fail the build.

**Measured:** 2026-10-21, `eslint 9.39.5` with
`apps/zylcode-desktop/eslint.config.js`, over `src/**/*.{ts,tsx}`.

**Headline numbers:**

```
Before configuration existed:  n/a — `pnpm lint` failed with
                               "could not find config file"; eslint was not
                               even in devDependencies.
First working run:             1209 problems (1171 errors, 38 warnings)
After fixing the JSX-runtime
mis-configuration and the
real defects:                    36 problems (0 errors, 36 warnings)
```

The gap between 1209 and 36 is the point of this file: **1,124 of the first
run's findings were a configuration error, not a code error.**

---

## Why warnings are allowed and errors are not

The rule set is deliberately asymmetric:

| Class | Severity | Rationale |
|---|---|---|
| `react-hooks/rules-of-hooks` | **error** | a violation is a runtime crash, not a preference |
| `@typescript-eslint/no-unused-vars` | **error** | dead code is a maintenance liability and a review cost |
| `no-empty` | **error** | an empty block hides a swallowed error |
| `react/no-unescaped-entities` | **error** | a real, if minor, HTML-correctness defect |
| `@typescript-eslint/ban-ts-comment` | **error** | `@ts-ignore` silently no-ops when the line becomes valid |
| `no-explicit-any` | warning | see §1 below |
| `react-hooks/exhaustive-deps` | warning | see §2 below |
| `react-refresh/only-export-components` | warning | see §3 below |

An error in this configuration is always either a defect or a lie about the
code. A warning is a judgement call, and judgement calls are recorded here
rather than silently tolerated.

---

## §1. `@typescript-eslint/no-explicit-any` — 30 warnings

| File | Count |
|---|---|
| `src/hooks/useTelemetryStream.ts` | 13 |
| `src/hooks/useAuditStream.ts` | 5 |
| `src/components/shell/SurfaceHost.tsx` | 8 |
| `src/components/shell/AgentDock.tsx` | 2 |
| `src/components/TerminalPanel.tsx` | 2 |

(13 + 5 + 8 + 2 + 2 = 30.)

**Why accepted.** Every one of these sits on a Tauri IPC boundary. The desktop
shell reaches Rust commands through `window.__TAURI__`, which TypeScript cannot
resolve statically — hence the `// @ts-expect-error - Tauri module` comments in
the two hook files. The payload shape is defined by Rust structs in
`crates/zylcode-cli` and `apps/zylcode-desktop/src-tauri`, not by a `.d.ts` that
exists in this repository.

Typing them properly would require generated bindings from the Rust side
(`ts-rs` or a hand-written `.d.ts` kept in sync manually). That is a real piece
of work and a real API decision; it is not a lint fix and it does not belong in a
hardening pass.

**What would invalidate this row:** a Tauri command whose payload is statically
known and still typed `any`. Those should be typed one at a time as they are
touched.

---

## §2. `react-hooks/exhaustive-deps` — 2 warnings

| Location | Message |
|---|---|
| `src/App.tsx:216` | missing dependency `verification` |
| `src/components/ArtifactViewer.tsx:87` | missing dependencies `handleApplyPatch`, `handleSave` |

**Why accepted.** Both are *under*-declaring rather than over-declaring, which
is the safe direction: the effect runs too rarely rather than too often.

- `App.tsx:216` — the effect reads `verification` but deliberately re-runs only
  on mission-list changes. Adding `verification` to the deps would make the
  effect fire on every verification update and re-issue work that is already in
  flight. The rule's own suggestion (convert to `useReducer`) is the correct fix
  and is a behaviour change, not a lint fix.
- `ArtifactViewer.tsx:87` — `handleApplyPatch` and `handleSave` are re-created
  every render, so listing them would re-register the keyboard listener on every
  render. The fix is `useCallback` on both, again a behaviour-preserving-but-
  structural change.

**What would invalidate this row:** either file being touched. Fix the hook
contract while you are in there.

---

## §3. `react-refresh/only-export-components` — 4 warnings

| Location |
|---|
| `src/components/ui/index.tsx:29` |
| `src/components/ui/index.tsx:166` |
| `src/components/ui/index.tsx:691` |
| `src/components/shell/RightWorkspace.tsx:15` |

**Why accepted.** A Fast Refresh degradation, not a defect: the module still
works in development, it just falls back to a full reload instead of a hot
patch when one of these files changes.

Three of the four are in `src/components/ui/index.tsx`, which is a **barrel
file** — it exists precisely to re-export from many modules, so "only export
components" is structurally unsatisfiable there. The three are `themes`
(a `ThemeConfig[]` constant), `useTheme` and `useToast` (hooks): all
legitimate, none a component. Splitting it would mean choosing between
losing the single import site or splitting the barrel by concern.

The fourth is `isEmbeddedPreview()` in `src/components/shell/RightWorkspace.tsx`,
a URL probe exported for reuse by the preview host. It is not a component and
the file it lives in is full of them.

**What would invalidate this row:** `ui/index.tsx` being reorganised anyway.

---

## §4. What was fixed rather than accepted

The 1,173 findings that disappeared were not suppressed. For the record:

| Count | Finding | Disposition |
|---|---|---|
| 1,124 | `react/react-in-jsx-scope` | **Configuration error.** `tsconfig.json` sets `"jsx": "react-jsx"`, so `React` is never in scope. Fixed by adding `react.configs.flat["jsx-runtime"]` to `eslint.config.js`. |
| 31 | `@typescript-eslint/no-unused-vars` | Removed the dead imports, destructured props, and locals. |
| 4 | `react-hooks/rules-of-hooks` | **Real defects.** See below. |
| 7 | `react/no-unescaped-entities` | Escaped to `&apos;`. |
| 4 | `@typescript-eslint/ban-ts-comment` | `@ts-ignore` → `@ts-expect-error`. |
| 1 | `no-empty` | Empty `catch {}` in `ProviderSettings.tsx` given an explanatory comment. |
| — | dead `PackDetail` (93 lines) + its now-dead `TRUST_LABEL_STYLE`/`PERMISSION_LABEL` tables | Deleted. `Forge.tsx` net −124 lines, +9. |
| — | dead `DesktopRequired` component in `BottomPanel.tsx` | Deleted — unreferenced anywhere in the tree. |
### The two `rules-of-hooks` defects

These were found *by* the new lint config and are worth recording because they
were live bugs, not style noise:

1. **`src/hooks/useAuditStream.ts`** — a `useEffect` was called *inside* another
   `useEffect`'s callback. React cannot register a hook during the commit phase.
2. **`src/components/ArtifactViewer.tsx`** — two `useMemo` calls sat *after* an
   early `return`, so the component called a different number of hooks depending
   on whether any artifact tab was open.

Both were **reproduced**, not inferred. For (1), a probe component rendering
`useAuditStream()` in jsdom threw:

```
Error: Invalid hook call. Hooks can only be called inside of the body of a
function component.
```

For (2), the failure mode is the same shape ("Rendered more hooks than during the
previous render") triggered by closing the last tab.

Neither hook had a consumer in the tree and neither was covered by a test, which
is why both survived until a lint pass looked at them. Both were fixed by
hoisting, with the original trigger conditions preserved.

---

## Adding an entry

A warning may be recorded here when it is a judgement call rather than a defect.
An entry must state:

1. the rule and the count,
2. why the rule's suggestion is wrong *or* out of scope **for this codebase**,
3. what change would make the row stale.

Everything else stays an error. This file is a register of decisions, not a
place to move findings you do not want to fix.
