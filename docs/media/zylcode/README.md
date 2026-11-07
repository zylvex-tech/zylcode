# ZylCode product preview — capture record

This directory holds the screenshots and short screen recordings referenced by the
repository [README](../../README.md). Every file here is a **real capture of the real
frontend** — no mockup, no generated animation, no staged output.

They follow [`docs/community/DEMO_RECORDING_PROTOCOL.md`](../community/DEMO_RECORDING_PROTOCOL.md)
("Record **real** software. Never generate promotional animation pretending to be product
operation."). This file is the per-recording checklist for each asset.

---

## 1. What these captures are — and what they are not

**They are:** the production React/TypeScript frontend of the ZylCode desktop
application (`apps/zylcode-desktop`), built with `pnpm --filter zylcode-desktop build`
and driven in a real Chromium browser.

**They are not:**

- **Not the Tauri desktop runtime.** The captures were taken with no Tauri backend
  attached, so the app's own runtime bridge (`src/lib/runtime.ts`) reports
  `BROWSER_PREVIEW`. Backend-dependent panels therefore render their **honest
  controlled state** — the `BLOCKED` badge, the hint _"start `zylcode serve-intel` to
  browse the repository tree"_, and the terminal's `terminal service error: HTTP 501`.
  Those messages are the product telling the truth about a missing backend, not a
  defect we hid.

  **Batches 1 and 2 (assets `01`–`10`) are the ones described above.** **Batch 3
  (`11`–`13`, §8) is the opposite case**: it runs against a live
  `zylcode serve-intel`, so the explorer reads `AVAILABLE`, the terminal executes
  real shell commands, and no error line appears. §8 records that run.

- **Not a mission execution.** `preview-mission-composer.mp4` shows a mission being
  _composed_ — the real Mission Composer surface, with real keystrokes in a real
  textarea. The mission is **not run**: `RUN MISSION` requires the desktop backend.
  Per the protocol's scenario catalogue, _"Mission execution — once implemented"_ is
  not staged here.
- **Not a repository-intelligence query.** No repository-intelligence scenario is
  staged. The protocol is explicit that this scenario is _"once accepted (currently R2,
  not yet integrated — do not stage this until Phase 2A re-audit passes)"_, and Phase 2A
  is `RE-OPENED`.
- **Not a model run.** No model was invoked; no provider key was used or shown.

The clips deliberately include the failure states. A recording that hides failure is
not evidence.

---

## 2. Shared environment

| Field             | Value                                                                                                                                                                                                                               |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| OS                | Microsoft Windows 10.0.26200.9711 (Windows 11)                                                                                                                                                                                      |
| Browser           | Chromium 153.0.8010.12 (Playwright build 1243), headless, 1180×740                                                                                                                                                                  |
| Driver            | `playwright-core` 1.x, local Chromium (no browser download)                                                                                                                                                                         |
| Node              | v22.22.2                                                                                                                                                                                                                            |
| Toolchain         | `rustc 1.97.1 (8bab26f4f 2026-07-14)`                                                                                                                                                                                               |
| Build             | `pnpm --filter zylcode-desktop build` → `tsc && vite build` (470 modules, exit 0)                                                                                                                                                   |
| Serve             | `python -m http.server 4173` over `apps/zylcode-desktop/dist`                                                                                                                                                                       |
| Starting commit   | `e28ea8f6ba388b453ac5539fa15a8e073013af40` — _Merge pull request #30 from zylvex-tech/release/v0.2.1_ (the `v0.2.1` release merge)                                                                                                  |
| App version shown | Status bar reads **`v0.2.1`**. The application header shows a separate in-app milestone label, **`v0.3.0-convergence`** — that is a product-milestone string, not the released version, and it is shown here exactly as it renders. |
| Runtime mode      | `BROWSER_PREVIEW` (no Tauri IPC, no `zylcode serve-intel`)                                                                                                                                                                          |
| Model in use      | None — no model call occurs in any recorded scenario                                                                                                                                                                                |

---

## 3. Per-asset checklist

### `01-landing.png` — the workbench

| Field                                   | Value                                                                                                                                                  |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Scenario                                | The landing workbench: menu bar, explorer, project panel, capability spotlight, bottom panel dock, status bar.                                         |
| Command / action                        | Load `http://localhost:4173/`, wait for the shell to settle (≈2.2 s), capture the viewport.                                                            |
| Expected result (stated before capture) | The shell renders with the project `zylcode` (`C:\Projects\zylcode · main`) marked `AVAILABLE`, missions `0`, and the explorer in its `BLOCKED` state. |
| Evidence                                | The captured frame itself: the status bar (`Circuit: CLOSED`, `Bridges: 0`, `v0.2.1`), the `AVAILABLE` / `LIMITED` / `COMING SOON` capability labels.  |
| Recording steps                         | Playwright `page.screenshot()` of the 1180×740 viewport.                                                                                               |

### `02-theme-arctic-light.png` / `03-theme-nord.png` — theming

| Field            | Value                                                                                                                              |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Scenario         | The theme control re-themes the whole shell live.                                                                                  |
| Command / action | Select `Arctic Light`, then `Nord`, in the status-bar theme `<select>`; capture after each.                                        |
| Expected result  | The entire shell (panels, text, accents) re-colours to the selected theme without a reload.                                        |
| Evidence         | The two captures: a light surface and a desaturated dark-blue surface, both with the same layout and the same `v0.2.1` status bar. |
| Recording steps  | `selectOption({ label })` on the theme control, then `page.screenshot()`.                                                          |

### `04-mission-composer.png` / `05-mission-composer-typed.png` — mission composer

| Field            | Value                                                                                                                                                                                |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Scenario         | The Mission Composer surface: intent textarea, context affordances (`+ Context`, `Files`, `History`, `/ Commands`), autonomy selector, model override, and the `RUN MISSION` action. |
| Command / action | Click `Open mission composer →`; capture the surface; then type a sentence into the intent textarea and capture again.                                                               |
| Expected result  | The composer replaces the landing surface, the left rail becomes `MISSIONS`, and typed text appears in the intent field.                                                             |
| Evidence         | `05-mission-composer-typed.png` shows the typed intent verbatim in the textarea.                                                                                                     |
| Recording steps  | Real `click()` and real `type()` keystrokes (18 ms/char) into the real `<textarea>`.                                                                                                 |

### `preview-theming.mp4` — live theming (12.4 s)

| Field            | Value                                                                                                                              |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Scenario         | The shell re-themes live across the theme set.                                                                                     |
| Command / action | Cycle the theme control through `Midnight Pro → Arctic Light → Nord → GitHub Dark → Arctic Light → Midnight Pro`, 1.5 s per theme. |
| Expected result  | The full shell visibly re-colours on every selection, including two light-theme passes.                                            |
| Evidence         | The clip itself: the light `Arctic Light` surface and the dark surfaces are visibly different renders of the same layout.          |
| Recording steps  | Playwright `recordVideo` (context-level, 1180×740), then transcoded to H.264 MP4 (`libx264`, `yuv420p`, `+faststart`).             |

### `preview-navigation.mp4` — panel and dock navigation (20.6 s)

| Field            | Value                                                                                                                                                                                                                          |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Scenario         | The bottom panel dock and the right workspace switch surfaces.                                                                                                                                                                 |
| Command / action | Click the bottom dock tabs (`Terminal`, `Output`, `Problems`, `Debug Console`, `Ports`, `Tests`, `Evidence`, `Dev Tools`, `Foundation`, `Delivery`) and the right workspace tabs (`Tasks`, `Changes`, `Evidence`, `AI Agent`). |
| Expected result  | The selected tab becomes active and its panel body renders.                                                                                                                                                                    |
| Evidence         | The clip shows the right workspace switching to the `Build Pipeline` and `Deploy Targets` panels.                                                                                                                              |
| Recording steps  | Playwright `recordVideo`, then the same MP4 transcode.                                                                                                                                                                         |

### `preview-mission-composer.mp4` — composing a mission (8.6 s)

| Field            | Value                                                                                                                                    |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Scenario         | Composing a mission: open the composer, type an intent.                                                                                  |
| Command / action | Click `Open mission composer →`; type _"Turn this acceptance criterion into a verified mission."_ into the intent textarea (22 ms/char). |
| Expected result  | The composer opens and the typed intent appears verbatim in the field.                                                                   |
| Evidence         | The clip shows the sentence typed character by character in the real textarea.                                                           |
| Recording steps  | Playwright `recordVideo`, then the same MP4 transcode.                                                                                   |

---

## 4. Redaction checklist (applied)

- **No API keys, MCP tokens, cloud credentials or provider secrets** appear in any
  asset. No model was invoked, so no key could be rendered.
- **No SMTP or payment credentials** — not used in these scenarios.
- **No personal information.** No names, emails or locations are shown.
- **No secret environment variables** are displayed; the status bar shows only
  `metrics: n/a`, `Circuit: CLOSED`, `Bridges: 0`.
- **Paths.** The captures show the working directory `C:\Projects\zylcode` and the
  branch `main`. This is the public repository's own checkout path — it exposes no
  internal structure beyond the layout already published in the README, and it is
  retained because the alternative (a doctored path) would misrepresent the capture.
- **No failure was hidden.** The `BLOCKED` badges and the `HTTP 501` terminal error
  are shown as they occurred, per the protocol's _"if a step fails, record the failure
  too"_. Batch 3 (§8) takes the other position deliberately: it shows the product
  _working_, which is why it runs the backend the earlier batches did not have.

---

## 5. Publication checklist

- [x] Starting commit SHA recorded (`e28ea8f6ba388b453ac5539fa15a8e073013af40`) and reachable in the repository.
- [x] Environment matches what is shown (Windows 11, Chromium 153, built frontend, browser preview).
- [x] The action performed is real and reproducible from that commit — build the frontend, serve `dist/`, open it.
- [x] No fabricated or staged-only output presented as live behaviour.
- [x] All secret/PII classes redacted (none were present).
- [x] Capability status label matches the implemented rung: the assets are labelled **DEVELOPER PREVIEW / browser preview**, consistent with the README's early-access status.
- [x] Not a concept or mockup, so no `CONCEPT` overlay is required — but the assets are explicitly labelled as _browser preview_ rather than desktop runtime, so the label matches reality.

---

## 6. Reproducing these captures

```bash
# 1. Build the frontend exactly as captured
pnpm --filter zylcode-desktop build

# 2. Serve the production bundle
cd apps/zylcode-desktop/dist && python -m http.server 4173

# 3. Drive it with a real browser (Playwright, pointing at a local Chromium)
#    load http://localhost:4173/, then screenshot / record as described per asset above.
```

Every asset in this directory can be regenerated from the starting commit with the
steps above. Nothing here is hand-edited after capture.

---

## 7. Batch 2 — 2026-10-29 (the `v0.2.3` build)

`06`–`10` are the same kind of capture taken from a later commit, so the README's
front face shows the current product instead of the `v0.2.1` one. Sections 1
(what these are and are not) and 4 (redaction) apply to them unchanged; this
section records what differs.

### Shared environment (batch 2)

| Field             | Value                                                                                 |
| ----------------- | ------------------------------------------------------------------------------------- |
| Starting commit   | `c7a75d4d6a37c9a7f59c8dcdd914bbced7b5baae` — _chore(release): 0.2.3_                  |
| Build             | `pnpm --filter zylcode-desktop build` → `tsc && vite build` (exit 0)                  |
| Serve             | `python -m http.server 4173` over `apps/zylcode-desktop/dist`                         |
| Driver            | `playwright-core` 1.64.1, local Chromium (Playwright build 1243, no browser download) |
| Viewport          | 1180×740, headless, single page, no reload between shots unless stated                |
| App version shown | Status bar reads **`v0.2.3`**                                                         |
| Runtime mode      | `BROWSER_PREVIEW` (no Tauri IPC, no `zylcode serve-intel`)                            |
| Model in use      | None — no model call occurs in any captured scenario                                  |

### Per-asset checklist (batch 2)

#### `06-mission-lifecycle.png` — the workbench on the current build

| Field                                   | Value                                                                                                                                                                                    |
| --------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Scenario                                | The landing workbench: mission lifecycle strip, project card, next step, capability spotlight, bottom dock, status bar.                                                                  |
| Command / action                        | Load `http://localhost:4173/`, wait for `CAPABILITY SPOTLIGHT`, settle 2.5 s, capture the viewport.                                                                                      |
| Expected result (stated before capture) | The lifecycle reads `INTENT → PLAN → EVIDENCE → VERIFIED`, the project card shows `zylcode` `AVAILABLE` with `Missions 0`, the explorer is `BLOCKED`, and the status bar reads `v0.2.3`. |
| Evidence                                | The frame itself, including the honest `BLOCKED` states and the `terminal service error: HTTP 501` line.                                                                                 |
| Recording steps                         | Playwright `page.screenshot()` of the 1180×740 viewport.                                                                                                                                 |

#### `07-missions-pipeline.png` — missions surface

| Field            | Value                                                                                                                                                                               |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Scenario         | The missions surface: composer, missions list and the `VERIFY` terminal section.                                                                                                    |
| Command / action | Click `Missions` in the activity rail; wait 900 ms; capture.                                                                                                                        |
| Expected result  | The rail's `MISSIONS` panel appears with `New Mission`, the main area becomes `MISSION COMPOSER` (autonomy `Guided — AVAILABLE`) over `MISSIONS` → `No missions yet.` and `VERIFY`. |
| Evidence         | The composer's placeholder and autonomy helper text render verbatim; the terminal keeps its `BLOCKED` state.                                                                        |
| Recording steps  | `getByRole("button", { name: "Missions" })` click, then `page.screenshot()`.                                                                                                        |

#### `08-provider-fallback.png` — provider fallback chain

| Field            | Value                                                                                                                                                                                                            |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Scenario         | The right workspace's `Providers` tab, with the environment panel beside it.                                                                                                                                     |
| Command / action | Click `Settings` in the activity rail; click `Providers`; wait 1 s; capture.                                                                                                                                     |
| Expected result  | The panel reports its controlled state — `Desktop runtime required — this panel uses the ZylCode desktop engine` — rather than a broken empty list, and the environment panel reads `Detected: BROWSER_PREVIEW`. |
| Evidence         | The frame itself: a backend-dependent panel telling the truth about a missing backend, which is the behaviour §1 says to keep visible.                                                                           |
| Recording steps  | Two `click()`s, then `page.screenshot()`.                                                                                                                                                                        |

#### `09-evidence-center.png` — evidence ledger

| Field            | Value                                                                                                                                                               |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Scenario         | The `EVIDENCE` rail panel and the `EVIDENCE LEDGER` main surface, both with nothing captured.                                                                       |
| Command / action | Click `Evidence` in the activity rail; wait 1 s; capture.                                                                                                           |
| Expected result  | Both surfaces read `start \`zylcode serve-intel\` to read the evidence ledger`with a`BLOCKED`badge and a`Refresh` action — an absent ledger is stated, not implied. |
| Evidence         | The frame itself: no surface claims evidence it does not have.                                                                                                      |
| Recording steps  | `getByRole("button", { name: "Evidence" })` click, then `page.screenshot()`.                                                                                        |

#### `10-theme-arctic-light.png` — live theming on the current build

| Field            | Value                                                                                                                                                            |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Scenario         | The whole shell re-themes live, without a reload, on the `v0.2.3` build.                                                                                         |
| Command / action | On the landing surface, `selectOption({ label: "Arctic Light" })` on the status-bar theme control; wait 1.2 s; capture.                                          |
| Expected result  | The shell turns light — header, cards, rail and composer — while the bottom dock and status bar keep their dark treatment, and the control reads `Arctic Light`. |
| Evidence         | The frame itself, alongside `02`/`03` from batch 1 for the same layout in other themes.                                                                          |
| Recording steps  | `getByRole("combobox", { name: "Theme:" }).selectOption(...)`, then `page.screenshot()`.                                                                         |

### Reproducing batch 2

```bash
pnpm --filter zylcode-desktop build                       # exit 0
cd apps/zylcode-desktop/dist && python -m http.server 4173
cd <a scratch dir> && npm install playwright-core@1       # not a repo dependency
# drive http://localhost:4173/ at 1180x740 with the locators named per asset above
```

The driver lives outside the repository on purpose: the repo gains no
playwright dependency for a documentation task, and every step above is stated
in the checklist rather than hidden in a script.

---

## 8. Batch 3 — 2026-10-31 (the first capture with a live backend)

`06` and `07` are the README's front-face images, and both carried the line
`terminal service error: HTTP 501`. That line was real, and it was a
_documentation_ defect rather than a product one: the batch-1/2 recipe served
`dist/` with `python -m http.server`, which answers `POST /api/terminal/exec`
with `501 Unsupported method`. `vite preview` had no proxy at all, so there was
no way to serve the production bundle with a backend behind it —
`apps/zylcode-desktop/vite.config.ts` now shares one proxy table between
`server` and `preview` for exactly that reason.

Batch 3 is the same frontend, built the same way, **with `zylcode serve-intel`
running**. Sections 1 (what these are and are not) and 4 (redaction) apply;
what changes is that nothing is in a controlled failure state.

### Shared environment (batch 3)

| Field             | Value                                                                                                                                                    |
| ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Starting commit   | `1d2c627` — _feat(desktop): serve the preview bundle through the same backend proxy as dev_, working tree clean (`/api/version` reported `dirty: false`) |
| Build             | `pnpm --filter zylcode-desktop build` → `tsc && vite build` (exit 0)                                                                                     |
| Backend           | `cargo run -p zylcode-cli -- serve-intel` on `127.0.0.1:17630` — index warm: 669 files, 2949 symbols, 8 packages                                         |
| Serve             | `pnpm --filter zylcode-desktop preview` → `vite preview` on `4173`, proxying `/api/*` to `17630`                                                         |
| Driver            | The Freebuff preview browser (Chromium) at 1180×740, driven with real clicks and real keystrokes                                                         |
| App version shown | Status bar reads **`v0.2.3`**; the header shows the in-app milestone label **`v0.3.0-convergence`**                                                      |
| Runtime mode      | `BROWSER_PREVIEW` in the page, against a live `serve-intel` on loopback                                                                                  |
| Model in use      | None — no model call occurs in any captured scenario                                                                                                     |

### Per-asset checklist (batch 3)

#### `11-workbench-live-terminal.png` — the workbench with the terminal working

| Field                                   | Value                                                                                                                                                                                                                                                                           |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Scenario                                | The landing workbench with every backend-dependent panel live: explorer, terminal, status bar.                                                                                                                                                                                  |
| Command / action                        | Load `http://localhost:4173/`, wait for `CAPABILITY SPOTLIGHT`, type `git log --oneline -3` into the terminal input and press Enter; capture the viewport.                                                                                                                      |
| Expected result (stated before capture) | The explorer reads `AVAILABLE` with a file count instead of `BLOCKED`; the terminal reports `AVAILABLE · powershell · command runner · cwd persists per session` and shows the command with its real output; the status bar reads `v0.2.3`; no `HTTP 501` anywhere on the page. |
| Evidence                                | The frame itself: three real commit lines ending at `1d2c627`, `672 files` in the explorer, and a page-wide search of `document.body.innerText` for `terminal service error` returning nothing.                                                                                 |
| Recording steps                         | Real typing into the real `<input>`, then a viewport screenshot.                                                                                                                                                                                                                |

#### `12-missions-pipeline-live.png` — the missions surface with the backend up

| Field            | Value                                                                                                                                      |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Scenario         | The missions surface: composer, mission history, and the `VERIFY` dock with the live terminal below.                                       |
| Command / action | Click `Missions` in the activity rail; wait for `MISSION COMPOSER`; capture.                                                               |
| Expected result  | The composer, `MISSIONS` → `No missions yet.` and the `VERIFY` dock all render, with the same real terminal transcript still in the panel. |
| Evidence         | The frame itself — the transcript is the same `git log` run captured in `11`, so it is one session, not two staged ones.                   |
| Recording steps  | One `click()`, then a viewport screenshot.                                                                                                 |

#### `13-source-control-live.png` — source control reading the real repository

| Field            | Value                                                                                                                                                                                                                                               |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Scenario         | The `SOURCE CONTROL` rail panel and main surface, reporting the live git state of this checkout.                                                                                                                                                    |
| Command / action | Click `Source Control` in the activity rail; wait for `SOURCE CONTROL`; capture.                                                                                                                                                                    |
| Expected result  | The panel reports `AVAILABLE · via intel service`, the branch `feat/home-mission-pipeline → origin/feat/home-mission-pipeline@1d2c627`, and the untracked files present at that moment — the two captures already written to `docs/media/zylcode/`. |
| Evidence         | The frame itself: a backend-dependent panel that previously would have said `start \`zylcode serve-intel\` to see live source-control data` now showing real data.                                                                                  |
| Recording steps  | One `click()`, then a viewport screenshot.                                                                                                                                                                                                          |

### Reproducing batch 3

```bash
pnpm --filter zylcode-desktop build                       # exit 0
cargo run -p zylcode-cli -- serve-intel                   # 127.0.0.1:17630
pnpm --filter zylcode-desktop preview                     # 4173, proxies /api/* to 17630
# drive http://localhost:4173/ at 1180x740 with the actions named per asset above
```

The difference from batch 2 is one process (`serve-intel`) and one config line
(`preview.proxy`). Everything else — the build, the port, the viewport, the
locator-per-asset discipline — is unchanged, and no asset here is edited after
capture.
---

## 9. Batch 4 — 2026-11-06 (the provider boundary, `fc69e4c`)

Batch 4 exists because the model layer changed: three commits (`ebd5098`, `a202a08`,
`fc69e4c`) replaced the fake line-chunker with real per-provider streaming, a neutral
`StreamChunk`, request-aborting cancellation and a capability table. These five assets
are the evidence a reader needs to check that story against the running product.

### Shared environment (batch 4)

| Field             | Value                                                                                                                                                                                                                                                  |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Starting commit   | `fc69e4c` on `fix/r05-input-validation`; `/api/version` reported `dirty: true` (untracked scratch files and pre-existing untracked frontend work — none appear in a frame)                                                                             |
| Build             | `pnpm --filter zylcode-desktop build` → `tsc && vite build` (exit 0)                                                                                                                                                                                   |
| Backend           | `cargo run -p zylcode-cli -- serve-intel` on `127.0.0.1:17630` — index warm: 764 files, 3,187 symbols, 8 packages. The explorer in these frames reads `788 files`: the capture scratch directory existed in the checkout while the browser was driven. |
| Serve             | `pnpm --filter zylcode-desktop preview` → `vite preview` on `4173`, proxying `/api/*` to `17630`                                                                                                                                                       |
| Driver            | `playwright-core` 1.64.1 driving local Chromium build 1243 at 1440×900, `deviceScaleFactor: 2`, real clicks and real keystrokes                                                                                                                        |
| App version shown | Status bar reads **`v0.2.3`**                                                                                                                                                                                                                          |
| Runtime mode      | `BROWSER_PREVIEW` in the page, against a live `serve-intel` on loopback                                                                                                                                                                                |
| Model in use      | None — no model call occurs in any captured scenario                                                                                                                                                                                                   |

Why a separate Chromium rather than the panel's built-in preview browser: that browser
draws its own product badge over the app's top-right logo, which would put a fabricated
UI element in the frame. The packaged Chromium is the same engine with no overlay.

### Per-asset checklist (batch 4)

#### `14-workbench-live.png` — the workbench, live, used as the README hero

| Field            | Value                                                                                                                                                                                                                                            |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Scenario         | The landing workbench with every backend-dependent surface reporting real state.                                                                                                                                                                 |
| Command / action | Load `http://localhost:4173/`, wait for `CAPABILITY SPOTLIGHT`, drag the bottom-panel separator up ~250 px so the terminal is legible, scroll the explorer to `crates` so no scratch directory is visible, type `git log --oneline -4`, capture. |
| Expected result  | Explorer `AVAILABLE` with a real file count and real tree; terminal shows the four newest commits ending `fc69e4c`; status bar `v0.2.3` and `40 entries · chain intact`; no `HTTP 501` or `BLOCKED` anywhere on the page.                        |
| Evidence         | The frame itself: commits `fc69e4c`, `a202a08`, `ebd5098`, `afdf68b`; the ledger counter; the live capability spotlight.                                                                                                                         |

#### `15-mission-intent.png` — a real intent in the Mission Composer

| Field            | Value                                                                                                                                                       |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Scenario         | Missions surface with a genuine instruction typed into the composer, repository and terminal visible beside it.                                             |
| Command / action | Click `Missions` in the activity rail, wait for `MISSION COMPOSER`, click the textarea, type the intent character by character, capture without submitting. |
| Expected result  | The composer holds the typed intent; the surface does **not** claim a mission ran.                                                                          |
| Evidence         | The frame itself, cross-checked against the `/api/missions` traffic for this session, which shows no mission was created.                                   |

#### `16-terminal-provider-tests.png` — tool execution, not a mock

| Field            | Value                                                                                                                                              |
| ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| Scenario         | The provider test suite running in the integrated terminal, with its individual test names on screen.                                              |
| Command / action | With the terminal focused, type `cargo test -p zylcode-core --lib providers::` and press Enter; wait ~45 s for the run to finish; capture.         |
| Expected result  | The transcript shows the streaming, transport and cancellation test names and `81 passed; 0 failed`.                                               |
| Evidence         | The frame itself, plus the matching `/api/terminal/exec` request, plus the same suite reproduced locally — `providers::` → 81 passed on this head. |

#### `17-provider-chain.png` — the provider surface and the tool catalogue

| Field            | Value                                                                                                                                                                                                                             |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Scenario         | Dev Tools / Providers: the live fallback chain as reported by `/api/providers`, next to the in-process tool catalogue.                                                                                                            |
| Command / action | Open the bottom panel's `Providers` tab, wait for the chain to render, capture.                                                                                                                                                   |
| Expected result  | Provider entries with model, `key required` state, enabled/disabled and measured outcome; tool catalogue reads `20/20 executors bound`.                                                                                           |
| Evidence         | The frame itself, matched against the `/api/providers` response for this session. Read-only by design in browser preview: editing keys and testing a connection need the Tauri runtime, so no edit control is shown working here. |

#### `18-evidence-ledger.png` — verification, failures included

| Field            | Value                                                                                                           |
| ---------------- | --------------------------------------------------------------------------------------------------------------- |
| Scenario         | The evidence rail and ledger over the real `.zylcode/` store.                                                   |
| Command / action | Click `Evidence` in the activity rail, wait for the ledger, capture the full-height rail.                       |
| Expected result  | `40 entries · 13 sessions · chain intact`, with `Verified`, `Recorded`, `Approved` **and `Failed`** rows.       |
| Evidence         | The frame itself. The `Failed` rows stay on purpose: a ledger that only showed successes would not be evidence. |

### Redaction (batch 4)

- No API keys appear anywhere; provider entries show `key required`, never a key value.
- No absolute path carrying the local user profile: the explorer was left on
  `crates`/`apps`/`docs` and the terminal ran repository-relative commands only.
- The `.capture/` scratch directory used to drive the browser is not framed.
- Terminal history holds only the two commands named above; no secret was exported into
  that shell.

### Reproducing batch 4

```bash
pnpm --filter zylcode-desktop build                       # exit 0
cargo run -p zylcode-cli -- serve-intel                   # 127.0.0.1:17630
pnpm --filter zylcode-desktop preview                     # 4173, proxies /api/* to 17630
# drive http://localhost:4173/ at 1440x900, deviceScaleFactor 2, with the actions named per asset
```

Nothing in these assets is edited after capture: no crop retouching, no compositing, no
text drawn on top. The only processing is the PNG encode itself.
