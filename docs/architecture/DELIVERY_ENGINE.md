# DELIVERY_ENGINE.md — ZylCode Delivery Engine

**Status: PARTIAL — release workflow configured, CI blocked externally**
**Rung: R1 (OBSERVED)**
**Phase: 13**
**Governing docs:** `ZYLCODE_PRODUCT_CONSTITUTION_V2.md` §11 · `ZYLCODE_ARCHITECTURE_V2.md` §3.7

> ⚠️ A release workflow exists (Tauri action, NSIS/DMG/DEB/RPM configured) but has never
> completed successfully through CI. The installer path has an incident history.
> **R1 — observed, not verified.**

---

## 1. Responsibility

Understand how to **finish the job**.

Git · PRs · CI/CD · Windows installers · macOS bundles · Linux packages · Docker · web
deployment · Android AAB/APK · Play Store · iOS IPA/TestFlight/App Store · release evidence.

---

## 2. Why Delivery Deserves Its Own Engine

The installer incident is the argument.

Distribution was handled as scattered shell scripts and workflow YAML. The result:

- installers that could not be produced reliably
- download URLs that referenced artifacts that did not exist
- documentation that described a release process that had never completed
- a CI path blocked by an external billing issue with no fallback
- no release evidence to inspect

None of these is a hard engineering problem. They are the predictable outcome of treating
delivery as an afterthought. **Delivery is a first-class engineering concern**, so it gets a
first-class engine.

---

## 3. The Governing Rule

> **Nothing ships unproven.**

The Delivery Engine **cannot publish** an artifact whose required proofs are below threshold.

```
Release candidate
      │
      ▼
Required proofs (per release kind)
      │
      ├── insufficient → BLOCKED (explicit, with the missing proofs named)
      │
      └── sufficient   → package → sign → publish → release evidence bundle
```

**Fail closed.** A release that cannot meet its proof threshold is blocked, and the block names
exactly what is missing. It is never "published anyway".

---

## 4. Release Kinds and Required Proofs

| Release kind | Required proofs (minimum) |
|---|---|
| **Development build** | compiles; unit tests |
| **Internal alpha** | + integration tests; installs on a clean machine |
| **Public beta** | + browser/runtime verification; packaging verified per platform |
| **Stable release** | + R4 reproducibility; security scan; accessibility; upgrade path from previous stable |
| **Store release (Play / App Store)** | + device-lab verification (Phases 10/11); store compliance checks |

Thresholds are declared per project, not hard-coded, and are recorded as Project decisions.

---

## 5. Distribution Targets

```
Git              commits, branches, tags, PRs
CI/CD            pipeline orchestration
Windows          NSIS installer, MSI, portable
macOS            .app bundle, DMG, notarisation
Linux            .deb, .rpm, AppImage, flatpak
Docker           images, registries
Web              static hosting, serverless, container
Android          APK, AAB, Play Store tracks
iOS              IPA, TestFlight, App Store
```

Each target is a **provider** (Extension Platform, Phase 5), not a core special case. This is
why Phase 5 precedes Phase 13.

---

## 6. Release Evidence Bundle

Every release produces a bundle, stored as **release-lifetime artifacts** (never
garbage-collected):

```
release-evidence/
├── manifest.json          version, commit, date, publisher
├── proofs.json            proof graph for the release
├── artifacts/             installers, packages, images (hashed)
├── test-reports/          unit, integration, browser, visual
├── verification/          device-lab and platform verification outputs
├── security/              scan results, dependency audit
├── changelog.md           generated from commits/PRs
└── reproduction.md        how a third party reproduces this release
```

**`reproduction.md` is mandatory.** A release whose provenance cannot be reproduced is not a
release — it is a file someone uploaded.

---

## 7. Interfaces

```
zylcode release plan  --kind <kind>
zylcode release check --kind <kind>       # reports missing proofs, non-destructively
zylcode release build --target <target>
zylcode release publish --target <target>
zylcode release evidence <version>
```

Plus the UI Ship surface.

---

## 8. CI Fallback

`EXTERNAL_BLOCKER` (e.g. a CI billing failure) must not stop delivery entirely.

| Requirement | Detail |
|---|---|
| **Local release path** | the full pipeline must be runnable locally, not only in CI |
| **Parity** | local and CI produce equivalent artifacts, verified by hash |
| **Explicit status** | a release produced locally is marked as such in its evidence bundle |
| **No silent downgrade** | a blocked CI does not lower the proof threshold |

> This is the direct lesson of the CI incident: when the remote channel is unavailable, local
> reproduction becomes the **only** evidence — which makes it **more** important, not less.

---

## 9. Benchmark / Acceptance

| # | Criterion | Method |
|---|---|---|
| 1 | A tagged release produced end-to-end | transcript |
| 2 | Complete release evidence bundle | bundle inspection |
| 3 | A third party reproduces the release | independent run, hash comparison |
| 4 | A release with insufficient proofs is **blocked**, naming what is missing | negative test |
| 5 | Local and CI artifacts are hash-equivalent | hash comparison |
| 6 | Installer installs on a **clean** machine | captured transcript |
| 7 | Release artifacts are never garbage-collected | GC test |

**Rung target: R3, R4 preferred.**

---

## 10. Anti-Requirements

- Do not publish an artifact below its proof threshold.
- Do not reference download URLs that have not been verified to resolve.
- Do not treat delivery as shell scripts scattered across workflows.
- Do not special-case a distribution target in core.
- Do not silently lower thresholds when CI is unavailable.
- Do not garbage-collect release evidence.
- Do not describe a release as shipped before its evidence bundle exists.
