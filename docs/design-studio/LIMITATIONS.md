# Design Studio — what this wave does not do

Stated plainly, because a studio that overstates itself is worse than a smaller
one that does not.

## Not implemented

| Capability | Status | Why it is absent |
| --- | --- | --- |
| AI assistance in the studio | `NOT_IMPLEMENTED` | Any assistant here must resolve to validated operations on real components. A prompt box in a surface that cannot write is decoration. |
| Pixel-diff visual regression | `NOT_IMPLEMENTED` | Baseline comparison diffs computed style properties, not images. Rendering both and diffing pixels needs a headless browser and an image pipeline, which this wave does not add. |
| Editing and writing components | `NOT_IMPLEMENTED` | The studio reads the repository. Prop edits are surface state, and nothing is written back to disk. |
| Drag-and-drop layout composition | `NOT_IMPLEMENTED` | Requires a document model the application does not have. |
| Figma or design-tool import | `NOT_IMPLEMENTED` | No integration exists in the repository. |
| Screenshots of screens | `NOT_IMPLEMENTED` | Screens are not mountable (see below); the studio shows their source and component relationships instead. |

## Partial by design

- **Screens and pages** are indexed, listed and navigable, but not mountable: the
  shell's views are functions of application state (`missions`, `files`,
  `activeTab`, callbacks), which the canvas cannot fabricate without faking the
  app. The inspector shows the reason per component.
- **Accessibility** is static analysis of source plus WCAG contrast from token
  values. No axe, keyboard or screen-reader pass runs in the studio. UNKNOWN rows
  name the pass that would settle them.
- **Token usage** covers Tailwind utilities that the config binds to variables.
  Component classes from `index.css` (`.btn-primary`, `.card`), inline styles and
  runtime-composed class strings are not traced, so a token can be in use and
  still show a lower count than the truth. The empty state says "no utility in
  the indexed components resolves to this token", which is exactly what is known.
- **Prop editing** covers enums, booleans, finite numbers and text (≤200 chars).
  Callbacks, arrays and objects are read-only.
- **Source navigation** depends on the file-content service. In the desktop app
  that is Tauri IPC; in the browser preview it is the proxied HTTP service. If
  neither answers, the pane reports the transport error.
- **Pan** is pointer input and is not exercised by jsdom tests; zoom, viewport
  and selection are.

## Known weaknesses

1. The indexer is regex-free but not type-aware: it does not run a full
   TypeScript program, so a prop type imported from a package (rather than `src`)
   stays `unknown` and blocks mounting.
2. Components conditionally rendered by another component are listed as children
   because JSX is read statically. The studio cannot say "only when variant is
   ghost".
3. `--color-primary` measures about 3.7:1 against white in `midnight-pro`, below
   AA for body text. The panel reports it; the palette is unchanged.
4. The canvas runs component effects. A component that writes to storage or asks
   for IPC will do so when mounted. This is a real render, with the real
   consequences, and it is why the surface says nothing it shows is a mockup.
