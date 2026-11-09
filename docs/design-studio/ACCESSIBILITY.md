# Accessibility in the Design Studio

The studio answers two questions, and keeps their evidence apart.

## 1. What can be read from the source?

For the selected component, `a11yFindings` derives rules from the counts the
indexer recorded: `<img>` without `alt`, click handlers on elements that are not
interactive, presence of `aria-label`/`aria-labelledby`/`htmlFor`, explicit
`tabIndex`, and whether the component has interactive elements at all.

| Rule | PASS | WARNING | UNKNOWN |
| --- | --- | --- | --- |
| `image-alt` | no unlabelled `<img>` | at least one `<img>` without `alt` | component renders no image and nothing interactive |
| `click-target` | every click handler sits on a button/link/input | handler on a `div`/`span`/`li`/`p` | no click handlers |
| `accessible-name` | `aria-label`/`aria-labelledby`/`htmlFor` present | never asserted from absence | no explicit labelling in source — a text child may still name the control |
| `focus-order` | never asserted | never asserted | tab order is a property of the rendered tree |

Absence of ARIA is not a defect: `<button>Save</button>` has an accessible name
from its text. That is precisely why `accessible-name` is UNKNOWN rather than
WARNING when there is no explicit label, and why `focus-order` is always UNKNOWN
from static analysis.

## 2. What does the palette actually measure?

`contrastMatrix` computes WCAG 2.1 contrast from the token values themselves:

| Pair | Threshold | Why |
| --- | --- | --- |
| text-primary / background | 4.5:1 | body text (1.4.3) |
| text-secondary / background | 4.5:1 | body text |
| text-muted / background | 4.5:1 | body text |
| text-primary / surface | 4.5:1 | body text on a panel |
| text-muted / surface | 4.5:1 | body text on a panel |
| primary-foreground / primary | 4.5:1 | button label |
| secondary-foreground / secondary | 4.5:1 | button label |
| border / background | 3:1 | UI boundary (1.4.11) |

Only pairs where both tokens exist in the theme are measured. A value that does
not parse as a colour (`linear-gradient(...)`, `color-mix(...)`) yields a `null`
ratio and an `UNKNOWN` verdict — the panel never estimates.

The maths:

```
luminance(c) = 0.2126·R + 0.7152·G + 0.0722·B   (linearised sRGB channels)
ratio        = (Lmax + 0.05) / (Lmin + 0.05)
```

`#f8fafc` on `#0f172a` in `midnight-pro` measures **17.06:1**; that value is
asserted in `catalog.test.ts`, so a palette edit that changes it is visible in
the test rather than only in the panel.

### What the panel found at the time of writing

`midnight-pro` has one pair below AA: white text on `--color-primary` (#3b82f6)
measures about 3.7:1, which is acceptable for large text and not for body text.
The panel reports it as WARNING and names the pair. Fixing the palette is a
product decision and is not part of this wave.

## What this is not

- No axe-core run.
- No keyboard traversal test.
- No screen-reader output.
- No automated colour-blindness simulation.

Those belong to a rendered-browser pass. The panel says which checks are
UNKNOWN, so the missing evidence is visible instead of assumed.
