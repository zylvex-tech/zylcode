# Building and distributing ZylCode installers — the honest runbook

Status: the bundler is **configured** (`apps/zylcode-desktop/src-tauri/tauri.conf.json`:
NSIS `currentUser` install on Windows, DMG on macOS, DEB/RPM on Linux,
`createUpdaterArtifacts: true`), but **no signed installer has ever been
produced or published by this repository.** This document is the procedure for
the first real one, plus the two external blockers that exist today.

## What is already configured

| Piece | State |
|---|---|
| Bundler targets (`all`: NSIS, DMG, DEB, RPM) | configured |
| NSIS per-user install (no elevation) | configured |
| Updater artifacts (`.zip`/`.nsis.zip` + signature placeholders) | configured |
| Updater plugin runtime (`plugins.updater` endpoint + key) | **not configured** — deliberately: an updater that has no public key and no endpoint would not verify anything and might establish a false "auto-updates" impression |
| Code signing (Authenticode / macOS identity) | **not configured** — no certificate exists |

## Building locally (Windows, the actual development host)

```powershell
cd apps/zylcode-desktop
pnpm build                      # frontend (tsc + vite)
cargo tauri build --bundles nsis
# artifacts land in src-tauri/target/release/bundle/nsis/
```

For macOS/Linux targets, build on the target OS (or in CI): the bundle step is
platform-native.

## Enabling the updater (after a key exists)

1. Generate a signing pair (keep the private key out of the repo):

   ```bash
   cargo tauri signer generate -w ~/.tauri/zylcode.key
   ```

2. Put the **public** key in `tauri.conf.json` under `plugins.updater.pubkey`,
   add the plugin (`tauri-plugin-updater`) to the app, and point
   `endpoints` at a real update manifest URL.
3. Set `TAURI_SIGNING_PRIVATE_KEY` (+ password) in the release environment,
   not in source control.

## Blocked externally (recorded, not hidden)

* **Remote CI**: GitHub Actions for the organization is disabled by the billing
  lock, so release.yml cannot produce artifacts yet. Local builds above are the
  substitute evidence until billing is restored.
* **Code-signing certificates**: absent. An unsigned NSIS/DMG build installs
  with OS warnings; that is acceptable for controlled distribution to beta
  testers provided the README says so plainly — it does not clear ZylCode for
  general public distribution claims.
