# Canvas and inspector

## The canvas is a real render

`DesignCanvas` loads the selected component through the registry, which resolves
the index's `file` into a module in Vite's graph and takes the named export:

```ts
const modules = import.meta.glob([
  "../components/**/*.tsx",
  "!../components/**/*.test.tsx",
  "!../components/**/__snapshots__/**",
]);
```

Consequences worth knowing:

- The component runs its own effects. A panel that fetches git status will try
  to fetch it in the canvas; the surface it shows is whatever the real transport
  returns (in the browser preview, an honest unavailable state).
- An import error is displayed as `IMPORT FAILED` with the module's message. The
  canvas never falls back to a picture.
- Mounting is cached per component id; switching back is instant.

## Responsive work

- Presets are the widths the brief names: **1440**, **1024**, **768**. A custom
  width input accepts 240–3840; values outside that range are ignored, and the
  next valid value is used.
- The frame's width is set in CSS pixels, so media queries and container
  behaviour respond to it exactly as they would in a window of that width.
- The toolbar reports the active breakpoint from the index (`sm ≥ 640`,
  `md ≥ 768`, …). Below 640 it says `below sm`.
- `compare presets` renders the same component at all three widths side by side,
  scaled down to fit, each labelled with its breakpoint. This is a layout
  comparison, not a diff.

The breakpoints come from `tailwind.config.js`; when the config does not
override `screens`, the index records Tailwind's defaults and marks their
`source` as `tailwind-default` so the studio does not present a framework
default as an application decision.

## Theme switching

The frame carries `data-theme="<id>"`, the same attribute the shell sets on
`<html>`. Every theme block in `themes.css` is an attribute selector, so the
canvas re-resolves the real tokens in place. Switching themes does not reload or
remount the component; it changes the variables underneath it.

## Pan, zoom, selection

| Gesture | Behaviour |
| --- | --- |
| drag on the viewport | pans the frame; a drag over 3px never counts as a click |
| ctrl + wheel | zooms between 25% and 200% |
| `−` / `+` / `reset` | zoom steps, and reset returns to 100% with pan at origin |

Clicking inside the frame selects the element under the pointer. The click is
taken in the **capture** phase and its default is prevented, so the component's
own handlers do not fire while you are inspecting; the footer of the canvas says
so. Selection reports the tag, the class string, the text, the depth from the
frame root, and a computed-style snapshot.

### Token attribution from a real render

Attribution compares what the browser computed with what the theme defines:

- colours by exact channel match (rgb channels and alpha within 0.02);
- font size, padding and radius by length, converted to px, within 0.5px;
- line height by unitless ratio × the computed font size, within 0.6px.

A value that matches nothing stays unattributed, and the inspector says the
value may come from a literal or a component class. Transparent computed
backgrounds are never attributed, because they carry no colour information.

This is how a hard-coded hex becomes visible: it renders, but no token claims it.

## Inspector

Four tabs, one subject at a time:

| Tab | Contents |
| --- | --- |
| `component` | identity, mount status, props with editors, tokens, accessibility findings |
| `element` | the selected rendered element: text, classes, attributed tokens, computed style, baseline comparison |
| `structure` | rendered components, parent components, distinct classes |
| `source` | the indexed file, opened at the indexed line |

### Prop edits are data, not code

There is no `eval` and no `new Function` in the studio. An edit is validated
against the prop's own type before it reaches the canvas:

- enum: must be one of the union's members, otherwise the error names the
  allowed values;
- boolean: `true`/`false` only;
- number: must be finite;
- text: capped at 200 characters;
- anything else: shown read-only with `not editable here — supplied by the app`.

Drafts commit on blur or Enter; Escape restores the last valid value, so a
half-typed string never reaches the component.

## Source navigation

`SourcePane` calls the same service the editor uses (`lib/fileContent`): the
Tauri `file_content` command on the desktop, the proxied `/api/file-content`
service in the browser preview. It windows the file around the indexed line
(12 before, 48 after), highlights it, and reports the range and total.

Failure is a state, not a substitute: `SOURCE UNAVAILABLE` with the transport's
reason. If the indexed line is past the end of the file, the pane says the line
is not there and suggests the index may be stale — which is also what
`design:index:check` would say.

## Baseline comparison (foundation)

`capture baseline` stores the selected element's computed values and its
attributed tokens in `localStorage`, keyed by component, tag and classes. The
next capture is diffed property by property; each row shows the old and new
value, or `unchanged`.

This is **not** a pixel diff and it does not run in CI. It is the smallest honest
step toward visual regression: it proves the studio can capture a render,
remember it, and compare a later render against it. See
[LIMITATIONS.md](./LIMITATIONS.md).

## AI assistance

Not implemented. There is no chat box in this surface, because the requirement
was that any such assistance resolve to real, validated operations. A model
answering "make this prettier" in a panel that cannot write is decoration.
