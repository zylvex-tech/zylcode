# The indexer

`apps/zylcode-desktop/scripts/design-index.mjs` reads the application's source
and writes `src/design/generated/design-index.json`. It runs with Node, uses the
TypeScript compiler API (already a dev dependency) rather than regular
expressions, and has no other dependency.

```bash
pnpm --filter zylcode-desktop design:index        # write the artefact
pnpm --filter zylcode-desktop design:index:check  # fail if it is stale
```

## Inputs

| Input | What is taken from it |
| --- | --- |
| `src/**/*.tsx` (excluding `*.test.tsx`, `src/design/`) | exported components, props, class usage, JSX structure, a11y signals |
| `src/styles/themes.css` | every custom property, its block and its line |
| `tailwind.config.js` | utility → variable mappings, breakpoints |
| `src/components/shell/SurfaceHost.tsx` | the shell's screen ids, from its own `case` labels |

`src/design/` is excluded because the studio consumes the index; indexing its
own panels would make the surface's component list grow every time a panel is
added, which is noise, not coverage.

## Components

A component is an exported function declaration or an exported arrow/function
expression that returns JSX (`componentNameFromVariable`). For each one the
index records:

- `id` — `components/ui/index#Button`: module path without `src/`, without the
  extension, then `#` and the export name. Unique per module, so a component can
  be traced back and never collides with another module's export.
- `file` and `line` — the declaration's own line, which is what source
  navigation opens.
- `props` — name, the type text exactly as written, `required`, the union's
  literal members when the type is one, and a `kind` the inspector can act on.
- `propsUnknown` — true when the props type could not be resolved. Such a
  component is not mountable, because the canvas cannot know what it needs.
- `classes`, `classSites`, `tokens` — every `className` literal in the
  component's subtree, with the line it appears on, and the tokens those
  utilities resolve to.
- `children` — the components it renders, resolved through the file's imports
  and the index's own names.
- `a11y` — counts of ARIA attributes, interactive elements, click handlers and
  click handlers on non-interactive elements.

### Props resolution

`variant?: "primary" | "ghost"` is an enum. `status: CapabilityStatus` is an
enum too, but only if `CapabilityStatus` is an exported string-literal union
somewhere in `src` — the indexer builds that symbol table in a first pass. A type
that does not resolve (a React context value, a domain object, a callback) stays
`unknown` with `values: null`, and the inspector offers no control for it rather
than inventing options.

`interface X extends React.ButtonHTMLAttributes<...>` is recorded in `extends`
and its inherited members are **not** expanded. They are spread onto the DOM
element by the component, so the canvas can pass them through; treating them as
props the inspector must supply would be wrong.

## Tokens

`themes.css` is parsed into blocks: `:root` becomes `theme: "base"` and each
`[data-theme="…"]` becomes `theme: <id>`. Every declaration keeps its value and
line. The category comes from the name prefix (`--color-*`, `--space-*`,
`--radius-*`, `--text-*`/`--font-*`/`--leading-*`, `--shadow-*`,
`--transition-*`, `--z-*`), not from comment headings, because a comment is not
structure.

## Usage attribution in the index

A class name becomes a token only when `tailwind.config.js` says that utility is
bound to that variable:

```
bg-surface      → colors["surface"]      → var(--color-surface)
p-4             → spacing[4]             → var(--space-4)
rounded-lg      → borderRadius["lg"]     → var(--radius-lg)
text-sm         → fontSize["sm"]         → var(--text-sm)
shadow-md       → boxShadow["md"]        → var(--shadow-md)
```

Color prefixes are tried before type scale, so `text-sm` (a size) and
`text-primary` (a color) both land correctly. A utility that no mapping explains
— `flex`, `p-[13px]`, a component class like `btn-primary` — is listed in
`classes` but contributes no token usage. This is why `Button`'s token list is
short: its colors live in `.btn-primary` in `index.css`, not in Tailwind
utilities.

Theme tokens share a variable name, so the same usage site appears on each
theme's copy of that token. `stats.tokenUsages` counts **distinct** sites.

## Determinism

The artefact has sorted components, sorted tokens and no timestamp, so a rerun
on unchanged source produces a byte-identical file. That is what makes
`design:index:check` usable as a test: it compares the committed file with a
fresh pass and fails with the file name and the command to fix it.

## What the indexer deliberately cannot tell you

- Whether a prop is required *at runtime* (TypeScript is not a runtime check).
- Which branch of a conditional renders (`variant === "ghost" ? … : …` is one
  set of classes, not two components).
- Whether a class is applied by a helper function in another module.
- Anything about `src/components/ui/index.css` component classes; only Tailwind
  utility mappings are traced.
- Whether the source still matches a *running* app — the freshness test covers
  the repository, not a stale dev server.
