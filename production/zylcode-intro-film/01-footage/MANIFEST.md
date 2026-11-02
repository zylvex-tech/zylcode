# Footage manifest — recorded 2026-10-31

Source: live ZylCode browser preview (`http://localhost:4173`) against running
`zylcode serve-intel` (:17630), repo on `main` @ `d060b7c`, app `v0.2.3`.
Captured with the Freebuff browser recorder (VP8 WebM, 1280×720 — the recorder caps at 720p;
full-frame PNG stills below are 1920×1080).

## Video takes

| File | Content | Notes |
|---|---|---|
| `take-01-workbench.webm` | Product opening + workbench: home surface, Explorer 677 files AVAILABLE, real terminal `git log --oneline -3` showing `d060b7c Merge pull request #44`, Source Control on `main`, Code Navigation query, mission composer | ~1.35 MB |
| `take-02-agent-execution.webm` | **Hero:** real engineering instruction in the composer → agent mission executed inside the ZylCode terminal (`status: COMPLETED`, `failures: 1`, `integrity: ledger=true graph=true claims=true`) → Evidence surface "✓ chain intact — 33 entries across 9 sessions" | ~0.90 MB |
| `take-03-verification-evidence.webm` | Verification: live `vitest run` in the app terminal → **26/26 tests passed**; Evidence Ledger (failures recorded honestly); MCP Tool Catalogue 20/20; Providers fallback chain with **no key material**; return to home surface | auto-stopped at the 5-minute recorder cap, ~0.97 MB |

## Full-resolution stills (1920×1080 PNG)

| File | Frame |
|---|---|
| `stills/01-workspace-tools-providers.png` | Workspace + MCP Tool Catalogue + Providers fallback chain |
| `stills/02-code-navigation-deterministic.png` | Code navigation: 8 DETERMINISTIC / 0 heuristic + live index 675 files · 2949 symbols · 8 packages |
| `stills/03-agent-execution-completed.png` | Terminal: mission `status: COMPLETED`, `integrity: ledger=true graph=true claims=true` |
| `stills/04-evidence-ledger-chain.png` | Evidence Ledger: 33 entries · 9 sessions · hash chain intact, incl. real failures |
| `stills/05-mission-composer.png` | Mission composer with a real instruction, autonomy ladder, terminal result |

## Verified before capture

- `zylcode mission` (First Mission fixture, isolated workdir): COMPLETED, exit 0, **5 s**
- targeted `vitest run`: 26/26 passing, **46 s** (inside the 120 s terminal timeout)
- `/api/version` → `branch main`, `head_commit d060b7c`, `app_version 0.2.3`
- No credential, key or token appears in any frame; providers render `key required` only.

## Known gaps for the assembly stage

- Takes are 720p → upscale to 1080p at assembly; stills carry the crisp hero frames.
- Not yet captured: animated diagrams (mission lifecycle, architecture, evidence ladder),
  PR #44 approval + CI checks, closing brand card.