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

* **Not the Tauri desktop runtime.** The captures were taken with no Tauri backend
  attached, so the app's own runtime bridge (`src/lib/runtime.ts`) reports
  `BROWSER_PREVIEW`. Backend-dependent panels therefore render their **honest
  controlled state** — the `BLOCKED` badge, the hint *"start `zylcode serve-intel` to
  browse the repository tree"*, and the terminal's `terminal service error: HTTP 501`.
  Those messages are the product telling the truth about a missing backend, not a
  defect we hid.

  **Batches 1 and 2 (assets `01`–`10`) are the ones described above.** **Batch 3
  (`11`–`13`, §8) is the opposite case**: it runs against a live
  `zylcode serve-intel`, so the explorer reads `AVAILABLE`, the terminal executes
  real shell commands, and no error line appears. §8 records that run.
* **Not a mission execution.** `preview-mission-composer.mp4` shows a mission being
  *composed* — the real Mission Composer surface, with real keystrokes in a real
  textarea. The mission is **not run**: `RUN MISSION` requires the desktop backend.
  Per the protocol's scenario catalogue, *"Mission execution — once implemented"* is
  not staged here.
* **Not a repository-intelligence query.** No repository-intelligence scenario is
  staged. The protocol is explicit that this scenario is *"once accepted (currently R2,
  not yet integrated — do not stage this until Phase 2A re-audit passes)"*, and Phase 2A
  is `RE-OPENED`.
* **Not a model run.** No model was invoked; no provider key was used or shown.

The clips deliberately include the failure states. A recording that hides failure is
not evidence.

---

## 2. Shared environment

| Field | Value |
|---|---|
| OS | Microsoft Windows 10.0.26200.9711 (Windows 11) |
| Browser | Chromium 153.0.8010.12 (Playwright build 1243), headless, 1180×740 |
| Driver | `playwright-core` 1.x, local Chromium (no browser download) |
| Node | v22.22.2 |
| Toolchain | `rustc 1.97.1 (8bab26f4f 2026-07-14)` |
| Build | `pnpm --filter zylcode-desktop build` → `tsc && vite build` (470 modules, exit 0) |
| Serve | `python -m http.server 4173` over `apps/zylcode-desktop/dist` |
| Starting commit | `e28ea8f6ba388b453ac5539fa15a8e073013af40` — *Merge pull request #30 from zylvex-tech/release/v0.2.1* (the `v0.2.1` release merge) |
| App version shown | Status bar reads **`v0.2.1`**. The application header shows a separate in-app milestone label, **`v0.3.0-convergence`** — that is a product-milestone string, not the released version, and it is shown here exactly as it renders. |
| Runtime mode | `BROWSER_PREVIEW` (no Tauri IPC, no `zylcode serve-intel`) |
| Model in use | None — no model call occurs in any recorded scenario |

---

## 3. Per-asset checklist

### `01-landing.png` — the workbench

| Field | Value |
|---|---|
| Scenario | The landing workbench: menu bar, explorer, project panel, capability spotlight, bottom panel dock, status bar. |
| Command / action | Load `http://localhost:4173/`, wait for the shell to settle (≈2.2 s), capture the viewport. |
| Expected result (stated before capture) | The shell renders with the project `zylcode` (`C:\Projects\zylcode · main`) marked `AVAILABLE`, missions `0`, and the explorer in its `BLOCKED` state. |
| Evidence | The captured frame itself: the status bar (`Circuit: CLOSED`, `Bridges: 0`, `v0.2.1`), the `AVAILABLE` / `LIMITED` / `COMING SOON` capability labels. |
| Recording steps | Playwright `page.screenshot()` of the 1180×740 viewport. |

### `02-theme-arctic-light.png` / `03-theme-nord.png` — theming

| Field | Value |
|---|---|
| Scenario | The theme control re-themes the whole shell live. |
| Command / action | Select `Arctic Light`, then `Nord`, in the status-bar theme `<select>`; capture after each. |
| Expected result | The entire shell (panels, text, accents) re-colours to the selected theme without a reload. |
| Evidence | The two captures: a light surface and a desaturated dark-blue surface, both with the same layout and the same `v0.2.1` status bar. |
| Recording steps | `selectOption({ label })` on the theme control, then `page.screenshot()`. |

### `04-mission-composer.png` / `05-mission-composer-typed.png` — mission composer

| Field | Value |
|---|---|
| Scenario | The Mission Composer surface: intent textarea, context affordances (`+ Context`, `Files`, `History`, `/ Commands`), autonomy selector, model override, and the `RUN MISSION` action. |
| Command / action | Click `Open mission composer →`; capture the surface; then type a sentence into the intent textarea and capture again. |
| Expected result | The composer replaces the landing surface, the left rail becomes `MISSIONS`, and typed text appears in the intent field. |
| Evidence | `05-mission-composer-typed.png` shows the typed intent verbatim in the textarea. |
| Recording steps | Real `click()` and real `type()` keystrokes (18 ms/char) into the real `<textarea>`. |

### `preview-theming.mp4` — live theming (12.4 s)

| Field | Value |
|---|---|
| Scenario | The shell re-themes live across the theme set. |
| Command / action | Cycle the theme control through `Midnight Pro → Arctic Light → Nord → GitHub Dark → Arctic Light → Midnight Pro`, 1.5 s per theme. |
| Expected result | The full shell visibly re-colours on every selection, including two light-theme passes. |
| Evidence | The clip itself: the light `Arctic Light` surface and the dark surfaces are visibly different renders of the same layout. |
| Recording steps | Playwright `recordVideo` (context-level, 1180×740), then transcoded to H.264 MP4 (`libx264`, `yuv420p`, `+faststart`). |

### `preview-navigation.mp4` — panel and dock navigation (20.6 s)

| Field | Value |
|---|---|
| Scenario | The bottom panel dock and the right workspace switch surfaces. |
| Command / action | Click the bottom dock tabs (`Terminal`, `Output`, `Problems`, `Debug Console`, `Ports`, `Tests`, `Evidence`, `Dev Tools`, `Foundation`, `Delivery`) and the right workspace tabs (`Tasks`, `Changes`, `Evidence`, `AI Agent`). |
| Expected result | The selected tab becomes active and its panel body renders. |
| Evidence | The clip shows the right workspace switching to the `Build Pipeline` and `Deploy Targets` panels. |
| Recording steps | Playwright `recordVideo`, then the same MP4 transcode. |

### `preview-mission-composer.mp4` — composing a mission (8.6 s)

| Field | Value |
|---|---|
| Scenario | Composing a mission: open the composer, type an intent. |
| Command / action | Click `Open mission composer →`; type *"Turn this acceptance criterion into a verified mission."* into the intent textarea (22 ms/char). |
| Expected result | The composer opens and the typed intent appears verbatim in the field. |
| Evidence | The clip shows the sentence typed character by character in the real textarea. |
| Recording steps | Playwright `recordVideo`, then the same MP4 transcode. |

---

## 4. Redaction checklist (applied)

* **No API keys, MCP tokens, cloud credentials or provider secrets** appear in any
  asset. No model was invoked, so no key could be rendered.
* **No SMTP or payment credentials** — not used in these scenarios.
* **No personal information.** No names, emails or locations are shown.
* **No secret environment variables** are displayed; the status bar shows only
  `metrics: n/a`, `Circuit: CLOSED`, `Bridges: 0`.
* **Paths.** The captures show the working directory `C:\Projects\zylcode` and the
  branch `main`. This is the public repository's own checkout path — it exposes no
  internal structure beyond the layout already published in the README, and it is
  retained because the alternative (a doctored path) would misrepresent the capture.
* **No failure was hidden.** The `BLOCKED` badges and the `HTTP 501` terminal error
  are shown as they occurred, per the protocol's *"if a step fails, record the failure
  too"*. Batch 3 (§8) takes the other position deliberately: it shows the product
  *working*, which is why it runs the backend the earlier batches did not have.

---

## 5. Publication checklist

- [x] Starting commit SHA recorded (`e28ea8f6ba388b453ac5539fa15a8e073013af40`) and reachable in the repository.
- [x] Environment matches what is shown (Windows 11, Chromium 153, built frontend, browser preview).
- [x] The action performed is real and reproducible from that commit — build the frontend, serve `dist/`, open it.
- [x] No fabricated or staged-only output presented as live behaviour.
- [x] All secret/PII classes redacted (none were present).
- [x] Capability status label matches the implemented rung: the assets are labelled **DEVELOPER PREVIEW / browser preview**, consistent with the README's early-access status.
- [x] Not a concept or mockup, so no `CONCEPT` overlay is required — but the assets are explicitly labelled as *browser preview* rather than desktop runtime, so the label matches reality.

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

| Field | Value |
|---|---|
| Starting commit | `c7a75d4d6a37c9a7f59c8dcdd914bbced7b5baae` — *chore(release): 0.2.3* |
| Build | `pnpm --filter zylcode-desktop build` → `tsc && vite build` (exit 0) |
| Serve | `python -m http.server 4173` over `apps/zylcode-desktop/dist` |
| Driver | `playwright-core` 1.64.1, local Chromium (Playwright build 1243, no browser download) |
| Viewport | 1180×740, headless, single page, no reload between shots unless stated |
| App version shown | Status bar reads **`v0.2.3`** |
| Runtime mode | `BROWSER_PREVIEW` (no Tauri IPC, no `zylcode serve-intel`) |
| Model in use | None — no model call occurs in any captured scenario |

### Per-asset checklist (batch 2)

#### `06-mission-lifecycle.png` — the workbench on the current build

| Field | Value |
|---|---|
| Scenario | The landing workbench: mission lifecycle strip, project card, next step, capability spotlight, bottom dock, status bar. |
| Command / action | Load `http://localhost:4173/`, wait for `CAPABILITY SPOTLIGHT`, settle 2.5 s, capture the viewport. |
| Expected result (stated before capture) | The lifecycle reads `INTENT → PLAN → EVIDENCE → VERIFIED`, the project card shows `zylcode` `AVAILABLE` with `Missions 0`, the explorer is `BLOCKED`, and the status bar reads `v0.2.3`. |
| Evidence | The frame itself, including the honest `BLOCKED` states and the `terminal service error: HTTP 501` line. |
| Recording steps | Playwright `page.screenshot()` of the 1180×740 viewport. |

#### `07-missions-pipeline.png` — missions surface

| Field | Value |
|---|---|
| Scenario | The missions surface: composer, missions list and the `VERIFY` terminal section. |
| Command / action | Click `Missions` in the activity rail; wait 900 ms; capture. |
| Expected result | The rail's `MISSIONS` panel appears with `New Mission`, the main area becomes `MISSION COMPOSER` (autonomy `Guided — AVAILABLE`) over `MISSIONS` → `No missions yet.` and `VERIFY`. |
| Evidence | The composer's placeholder and autonomy helper text render verbatim; the terminal keeps its `BLOCKED` state. |
| Recording steps | `getByRole("button", { name: "Missions" })` click, then `page.screenshot()`. |

#### `08-provider-fallback.png` — provider fallback chain

| Field | Value |
|---|---|
| Scenario | The right workspace's `Providers` tab, with the environment panel beside it. |
| Command / action | Click `Settings` in the activity rail; click `Providers`; wait 1 s; capture. |
| Expected result | The panel reports its controlled state — `Desktop runtime required — this panel uses the ZylCode desktop engine` — rather than a broken empty list, and the environment panel reads `Detected: BROWSER_PREVIEW`. |
| Evidence | The frame itself: a backend-dependent panel telling the truth about a missing backend, which is the behaviour §1 says to keep visible. |
| Recording steps | Two `click()`s, then `page.screenshot()`. |

#### `09-evidence-center.png` — evidence ledger

| Field | Value |
|---|---|
| Scenario | The `EVIDENCE` rail panel and the `EVIDENCE LEDGER` main surface, both with nothing captured. |
| Command / action | Click `Evidence` in the activity rail; wait 1 s; capture. |
| Expected result | Both surfaces read `start \`zylcode serve-intel\` to read the evidence ledger` with a `BLOCKED` badge and a `Refresh` action — an absent ledger is stated, not implied. |
| Evidence | The frame itself: no surface claims evidence it does not have. |
| Recording steps | `getByRole("button", { name: "Evidence" })` click, then `page.screenshot()`. |

#### `10-theme-arctic-light.png` — live theming on the current build

| Field | Value |
|---|---|
| Scenario | The whole shell re-themes live, without a reload, on the `v0.2.3` build. |
| Command / action | On the landing surface, `selectOption({ label: "Arctic Light" })` on the status-bar theme control; wait 1.2 s; capture. |
| Expected result | The shell turns light — header, cards, rail and composer — while the bottom dock and status bar keep their dark treatment, and the control reads `Arctic Light`. |
| Evidence | The frame itself, alongside `02`/`03` from batch 1 for the same layout in other themes. |
| Recording steps | `getByRole("combobox", { name: "Theme:" }).selectOption(...)`, then `page.screenshot()`. |

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
*documentation* defect rather than a product one: the batch-1/2 recipe served
`dist/` with `python -m http.server`, which answers `POST /api/terminal/exec`
with `501 Unsupported method`. `vite preview` had no proxy at all, so there was
no way to serve the production bundle with a backend behind it —
`apps/zylcode-desktop/vite.config.ts` now shares one proxy table between
`server` and `preview` for exactly that reason.

Batch 3 is the same frontend, built the same way, **with `zylcode serve-intel`
running**. Sections 1 (what these are and are not) and 4 (redaction) apply;
what changes is that nothing is in a controlled failure state.

### Shared environment (batch 3)

| Field | Value |
|---|---|
| Starting commit | `1d2c627` — *feat(desktop): serve the preview bundle through the same backend proxy as dev*, working tree clean (`/api/version` reported `dirty: false`) |
| Build | `pnpm --filter zylcode-desktop build` → `tsc && vite build` (exit 0) |
| Backend | `cargo run -p zylcode-cli -- serve-intel` on `127.0.0.1:17630` — index warm: 669 files, 2949 symbols, 8 packages |
| Serve | `pnpm --filter zylcode-desktop preview` → `vite preview` on `4173`, proxying `/api/*` to `17630` |
| Driver | The Freebuff preview browser (Chromium) at 1180×740, driven with real clicks and real keystrokes |
| App version shown | Status bar reads **`v0.2.3`**; the header shows the in-app milestone label **`v0.3.0-convergence`** |
| Runtime mode | `BROWSER_PREVIEW` in the page, against a live `serve-intel` on loopback |
| Model in use | None — no model call occurs in any captured scenario |

### Per-asset checklist (batch 3)

#### `11-workbench-live-terminal.png` — the workbench with the terminal working

| Field | Value |
|---|---|
| Scenario | The landing workbench with every backend-dependent panel live: explorer, terminal, status bar. |
| Command / action | Load `http://localhost:4173/`, wait for `CAPABILITY SPOTLIGHT`, type `git log --oneline -3` into the terminal input and press Enter; capture the viewport. |
| Expected result (stated before capture) | The explorer reads `AVAILABLE` with a file count instead of `BLOCKED`; the terminal reports `AVAILABLE · powershell · command runner · cwd persists per session` and shows the command with its real output; the status bar reads `v0.2.3`; no `HTTP 501` anywhere on the page. |
| Evidence | The frame itself: three real commit lines ending at `1d2c627`, `672 files` in the explorer, and a page-wide search of `document.body.innerText` for `terminal service error` returning nothing. |
| Recording steps | Real typing into the real `<input>`, then a viewport screenshot. |

#### `12-missions-pipeline-live.png` — the missions surface with the backend up

| Field | Value |
|---|---|
| Scenario | The missions surface: composer, mission history, and the `VERIFY` dock with the live terminal below. |
| Command / action | Click `Missions` in the activity rail; wait for `MISSION COMPOSER`; capture. |
| Expected result | The composer, `MISSIONS` → `No missions yet.` and the `VERIFY` dock all render, with the same real terminal transcript still in the panel. |
| Evidence | The frame itself — the transcript is the same `git log` run captured in `11`, so it is one session, not two staged ones. |
| Recording steps | One `click()`, then a viewport screenshot. |

#### `13-source-control-live.png` — source control reading the real repository

| Field | Value |
|---|---|
| Scenario | The `SOURCE CONTROL` rail panel and main surface, reporting the live git state of this checkout. |
| Command / action | Click `Source Control` in the activity rail; wait for `SOURCE CONTROL`; capture. |
| Expected result | The panel reports `AVAILABLE · via intel service`, the branch `feat/home-mission-pipeline → origin/feat/home-mission-pipeline@1d2c627`, and the untracked files present at that moment — the two captures already written to `docs/media/zylcode/`. |
| Evidence | The frame itself: a backend-dependent panel that previously would have said `start \`zylcode serve-intel\` to see live source-control data` now showing real data. |
| Recording steps | One `click()`, then a viewport screenshot. |

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
