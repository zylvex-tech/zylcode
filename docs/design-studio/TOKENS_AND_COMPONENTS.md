# Tokens and components

## Token model

One token is one custom property in one block of `src/styles/themes.css`:

```json
{
  "name": "--color-primary",
  "value": "#3b82f6",
  "category": "color",
  "theme": "midnight-pro",
  "file": "src/styles/themes.css",
  "line": 72,
  "usages": [{ "componentId": "components/ui/index#Button", "class": "bg-primary", "line": 289 }]
}
```

- `theme: "base"` is the shared scale (spacing, radius, type sizes, weights,
  leading, shadows, transitions, z-index). Every other theme id is a color
  palette.
- The same colour variable is one record per theme, because each theme defines
  its own value. The token explorer sets the theme's values on the page root, so
  a swatch is painted with the value the application would actually use.
- `usages` are the *utility class sites* the indexer could trace to that
  variable. They are the same for every theme's copy of a variable, because the
  class does not change when the theme does.

### What "used by 12 components" means

It means 12 components contain at least one utility that `tailwind.config.js`
binds to that variable. It does not mean the variable reaches the browser in
every state of those components, and it does not include:

- component classes such as `.btn-primary` from `index.css`;
- inline styles;
- utilities built at runtime from strings;
- tokens consumed by CSS the index does not read.

The panel states the count, and its empty state says no utility resolves to the
token rather than implying the token is unused anywhere.

### Token table

`TokenExplorer` shows name, value, and usage count per theme, with filters for
category and free text. Selecting a token opens its value, source line and usage
sites; clicking a usage site opens that component in the canvas.

## Component model

Each component record carries where it is, what it takes, what it renders, and
what it carries:

| Field | Use |
| --- | --- |
| `id`, `file`, `line` | identity and source navigation |
| `kind` | `design-system` (`components/ui`), `component`, `screen` (shell), `page` (`App.tsx`) |
| `props` | inspector controls; `values` drives enums |
| `propsUnknown` | blocks mounting when the props type did not resolve |
| `children` | "renders" list, linked to the child's record |
| `classes`, `classSites` | the exact utilities with their lines |
| `tokens` | which variables those utilities resolve to |
| `a11y` | the counts the accessibility panel reasons over |

### Mountability

The canvas supplies props through `mountPlan`:

| Prop | Decision |
| --- | --- |
| optional enum | first member is used as the initial value |
| optional boolean | `false` |
| required enum / boolean / string / number | a concrete initial value |
| `children` (node) | the text `Design Studio` |
| required callback, array, object, or an unresolved type | **not mountable**, with the prop name in the reason |
| `kind: screen` or `page` | **not mountable** — it composes the shell and needs its state |

A refused component is not hidden. It appears in the list, and its canvas frame
says `NOT MOUNTABLE` with the reason and the source reference. That is the
difference between "we cannot render this yet" and "this component is broken".

### States

States are demonstrated by props, not by a screenshot gallery. Every enum prop
becomes a selector, so `Button`'s five variants, `Badge`'s six and `size`'s
values are reachable and render live. Booleans such as `loading` and `disabled`
toggle the same way. A state that only exists behind application data (an error
banner after a failed fetch) is not synthesised; the component's mount plan will
usually refuse it, and the inspector says so.

### Structure

The inspector's `structure` tab lists the components this one renders and the
components that render it, both clickable. The relation is read from JSX and
resolved through the file's imports, so it is a direct-rendering relation: a
grandchild is not listed as a child.
