# Windows Reconnection Plan — 2026-10-09

**Goal:** safely reconcile the owner's Windows working tree with GitHub without overwriting local work.

This plan assumes the Windows machine may contain uncommitted work not yet present in GitHub.

---

## 1. Before pulling anything

On the Windows machine, record the current state:

```powershell
git status --short --branch
git branch --show-current
git rev-parse HEAD
```

Also capture a safe snapshot of current work:

```powershell
git stash push -u -m "pre-reconnect-2026-10-09"
```

If the stash is not desired, create a temporary archive instead:

```powershell
mkdir .\reconnect-backup
Copy-Item -Recurse -Force .\* .\reconnect-backup\
```

**Important:** do not delete, reset, or force-clean the working tree without explicit owner approval.

---

## 2. Fetch remote state without merging

```powershell
git fetch --all --prune
```

Then compare branches:

```powershell
git branch -avv
```

Check the remote state explicitly:

```powershell
git rev-parse origin/main
git rev-parse origin/competition/natlas-2026
git rev-parse origin/tester-prep/2026-10-09
```

---

## 3. Compare local and remote state

```powershell
git diff --stat main..origin/main
git diff --stat competition/natlas-2026..origin/competition/natlas-2026
git diff --stat tester-prep/2026-10-09..origin/tester-prep/2026-10-09
```

If local work exists, review it before merging:

```powershell
git stash list
git diff --staged
git diff
```

---

## 4. Reconcile safely

Recommended safe sequence:

```powershell
git checkout main
git pull --ff-only origin main

git checkout competition/natlas-2026
git pull --ff-only origin competition/natlas-2026

git checkout tester-prep/2026-10-09
git pull --ff-only origin tester-prep/2026-10-09
```

If local changes are still needed, re-apply them only after the remote was fetched:

```powershell
git stash list
git stash pop
```

If conflicts appear, resolve them manually and do not overwrite hidden local work without review.

---

## 5. Verification before merge

Run the same validation commands the repo expects:

```powershell
cargo test -p zylcode-core --lib competition::natlas::intent
cargo test -p zylcode-core --test natlas_boundary
cargo test -p zylcode-core --test natlas_runtime
cargo test -p zylcode-core --test natlas_bridge
npm --prefix apps/natlas-sdk run typecheck
```

Do not attempt to merge from `main` to the competition branch until all verification results are recorded and reviewed.

---

## 6. Merge policy

**Never merge automatically.**

The owner should do a deliberate merge only after:
- local branch comparison is reviewed
- stash or backup is preserved
- tests are run
- the branch diff is understood
- approval is explicit

---

## 7. Recovery if there is uncertainty

Use the non-destructive path first:
- stash
- fetch
- compare
- review diffs
- resolve manually
- rerun tests
- then merge

Do not use `git reset --hard`, `git clean -fd`, or force-push as the default recovery method.

---

## 8. Final safe procedure

```powershell
git status --short --branch
git fetch --all --prune
git stash push -u -m "pre-reconnect-2026-10-09"
git checkout main
git pull --ff-only origin main
git checkout competition/natlas-2026
git pull --ff-only origin competition/natlas-2026
git checkout tester-prep/2026-10-09
git pull --ff-only origin tester-prep/2026-10-09
git stash list
```

Only after this should the owner decide whether to merge or keep the branches isolated.
