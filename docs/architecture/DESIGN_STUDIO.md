# Design Studio

Status: **limited vertical slice**. Project lifecycle, canvas editing with
undo/redo, persistence and standalone HTML export are implemented and tested.
Visual diff, design↔code generation, agent repair and Figma import are **not
implemented**, and no control in the interface pretends they are.

Before this change the Design surface was a `COMING SOON` panel. Product
Architecture v3 §23 defined the workspace and explicitly prohibited a fake Figma
clone; nothing below the panel existed (no component, command, model or test).

## What a user can do

1. **Create** a project by name. It is persisted immediately, before editing, so
   there is no unsaved draft that cannot be reopened.
2. **Edit** it: add text and shape elements, drag them on the canvas, and set
   text, position, size, colour and font size in the inspector.
3. **Undo/redo** every edit. One drag is one undo step; a completed gesture does
   not leave fifty history entries behind.
4. **Save** explicitly (⌘/Ctrl+S or the button), with an `UNSAVED` marker
   visible until the store confirms a new revision.
5. **Reopen** it — including after the application restarts.
6. **Export** a standalone HTML document: inline styles, no scripts, no external
   references, safe to open anywhere.
7. **Rename** and **delete**, with delete requiring a second click (and removing
   the project's export with it).

Switching away from a project with unsaved changes is refused until the user
saves or discards; it is never silently discarded.

## Architecture

| Layer      | File                                                       | Responsibility                                                                      |
| ---------- | ---------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| Document   | `src-tauri/src/design_store.rs`                            | Schema, validation, ids, revisions, atomic writes, export bytes                     |
| IPC        | `src-tauri/src/main.rs` (`design_*`)                       | Thin boundary: resolve the app data directory, call the store                       |
| Model      | `src/features/design/model.ts`                             | The same document shape, validation and ids for the renderer                        |
| Repository | `src/features/design/repository.ts`                        | Desktop (IPC) and browser-preview (`localStorage`) implementations of one interface |
| History    | `src/features/design/history.ts`                           | Snapshot undo/redo, bounded at 50 steps                                             |
| Render     | `src/features/design/export.ts`                            | Standalone HTML; the store writes what the renderer produces                        |
| UI         | `src/features/design/DesignStudio.tsx`, `DesignCanvas.tsx` | Workspace, inspector, canvas, keyboard shortcuts                                    |

### Decisions worth stating

- **The store owns identity, revision and timestamps.** The renderer cannot
  choose an id that is also a path (`^[A-Za-z0-9_-]{1,64}$`), and a save that
  offers a stale revision is refused instead of overwriting work it never saw.
- **Writes are atomic.** A temporary file in the same directory is renamed over
  the project, so a crash leaves the previous readable document.
- **Validation precedes persistence.** A document with a bad colour, a
  non-finite coordinate or an empty name is rejected by the store, so the
  "project that will not open" cannot be written in the first place.
- **One renderer for preview and export.** The Rust side never formats markup;
  duplicating the renderer is how preview and export drift apart.
- **Two storage stories, both labelled.** Desktop projects are JSON files in the
  app data directory. Browser preview (used for development) stores them in
  `localStorage` and the workspace says so verbatim above the canvas. Neither is
  presented as the other.
- **Snapshot history, not inverse operations.** Documents are small; reconstructing
  the past from deltas is where "undo did something unexpected" bugs come from.

## Verification

```text
cargo test -p zylcode-desktop design_store     # 8 passed
cargo test -p zylcode-desktop                  # 33 passed
cargo clippy -p zylcode-desktop --all-targets -- -D warnings
pnpm exec vitest run --coverage                # full frontend suite, gates green
pnpm exec tsc --noEmit
```

Rust tests cover: save → **new store instance** → load (the restart contract),
revision advance, stale-save refusal, rename, delete (including the export),
export bytes, id/path-traversal rejection, validation refusals, and a document
from a newer schema being reported rather than misread.

Frontend tests cover: first-run guidance, create without a name, edit → save →
**remount** → reopen with content intact, undo/redo, one-drag-one-undo, validation
on save, a failed save reported as failure, the unsaved-changes guard, export
(including the HTML containing the current text), rename and two-step delete,
plus the exact IPC command names and argument shapes the Rust side registers.

## Not implemented

- Visual diff (DESIGN ↔ RUNTIME), design↔code generation, agent repair, proof.
- Screens, routes, tokens and component libraries: the document holds one canvas
  of text and shape elements.
- Figma import/export, asset organisation, multi-page projects.
- Concurrent editing: save-time revision checks prevent silent loss, they do not
  merge.

These are named in the workspace footer and in the Design tab note rather than
shown as disabled controls.
