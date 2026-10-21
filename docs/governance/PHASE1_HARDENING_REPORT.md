# Stage 1 repository-hardening report

**Scope:** security hygiene (1A), dependency reproducibility (1B), Rust
formatting enforcement (1C), frontend lint enforcement (1D).
**Base:** `origin/main` @ `a4f6b3e`.
**Delivered:** 5 commits on `hardening/phase1-security-and-reproducibility`,
**not pushed**, working tree clean.

Nothing here claims production readiness. CI gates passing means the gates
run, not that the product is finished.

---

## 1. Original branch, HEAD SHA, working-tree status

| Checkout | Branch | HEAD | State |
|---|---|---|---|
| `C:/Projects/zylcode` (the user's) | `competition/natlas-2026` | `c5c9138` | behind `origin/competition/natlas-2026` by 2, ahead 0 — **untouched, unmodified** |
| `C:/Users/…/Worktrees/zylcode/origin-main-8cdd991f` (this work) | `hardening/phase1-security-and-reproducibility` | `7b1bb0e` | clean |

All work was done in a separate git worktree off `origin/main` precisely so
the live `competition/natlas-2026` checkout was never disturbed. Verified
read-only at the end of the session: still `c5c9138`, still behind 2, no
local changes.

No user work was discarded, reset, or overwritten at any point.

## 2. New branch and resulting HEAD SHA

`hardening/phase1-security-and-reproducibility` @ **`7b1bb0e`**, 5 commits
ahead of `origin/main`, no upstream configured (never pushed).

```
70721f7  security(marketplace): remove credential-shaped literal, make the stub explicit
e9b4655  build: commit Cargo.lock and remediate dompurify advisories
30c46f9  style(desktop): add prettier config and apply formatting
f4a390c  feat(desktop): add eslint 9 flat config and fix the defects it reveals
7b1bb0e  ci: gate formatting, dependency audits, secret scanning, and frontend lint
```

## 3. Files changed and why

| Commit | Files | Why |
|---|---|---|
| `70721f7` | 4 (630+/14−) | Payment-gateway credential literal; natlas smoke fixtures; secret scanner |
| `e9b4655` | 5 (164+/1−) | `Cargo.lock`, dompurify override, advisory register |
| `30c46f9` | 79 (1829+/1174−) | Prettier config + mechanical formatting, isolated as its own commit |
| `f4a390c` | 21 (447+/281−) | ESLint flat config, the defects it revealed, lint-warning register |
| `7b1bb0e` | 1 (93+/3−) | CI gates |

Deliberately split: formatting-only noise (commit 3) is separable from real
defects (commit 4) so a reviewer can skip the former and read the latter as
a defect list. Proven orthogonal — with only commit 3 applied, ESLint still
reports 85 problems (47 errors, 38 warnings); with commit 4 applied, 0
errors and 36 warnings.

## 4. Confirmed and unconfirmed security findings

**Confirmed — `sk_test_...` in non-test source.**
`crates/zylcode-mcp/src/enhanced_plugin_marketplace.rs` constructed
`PaymentGateway { _provider: "stripe", _api_key: "sk_test_..." }`. Evidence
the gateway is inert rather than operational, all checkable in the tree:

- `grep -rn '_payment_gateway\.' crates/` → **no matches**; no field is ever read
- `RevenueManager::process_payment` never calls it — it fabricates
  `TransactionStatus::Completed` and returns `success: true` with a fresh
  UUID. No HTTP client, no provider SDK, no keyring, no env read on the path
- `PaymentGateway::new()` is the only constructor; nothing can supply config

**Disposition:** not wired to a real provider. The inert state is now
unmisreadable: `PaymentGatewayProvider::Unconfigured` and
`PaymentGatewayCredential::Unset` are single-variant enums, so source cannot
hold a credential-shaped literal at that position. `provider()`,
`credential()` and `RevenueManager::payment_gateway()` accessors make the
invariant observable and testable. `process_payment` docs now state that
`success: true` means "the record was written", not "money moved".

**Confirmed — `stub-key-1234` in the smoke suite.** These are genuine
fixtures, not leaks: `redactSecrets` does exact-substring matching, so a
secret is required to test redaction at all. Moved to
`apps/natlas-sdk/test/fixtures.mjs` as `TEST_FIXTURE_API_KEY` with a value
matching no provider prefix. **Rejected** `process.env.TEST_API_KEY`: a
redaction test whose input depends on the developer's shell is not a
redaction test — with the variable unset, CI silently no-ops the assertion.

**Accurate finding, wrong framing.** The assessment described a "liability
in a payment path". There is no payment path. `process_payment` writes an
in-memory record and reports success; nothing settles. That is a
truth-in-advertising problem (now fixed in the docs and types), not a
payment-security exposure.

**Mitigation added:** `scripts/check_hardcoded_secrets.py`, gated in CI.
Matches 13 high-confidence provider credential formats (Stripe, AWS, GitHub,
Slack, Google, npm, PyPI, SendGrid, PEM) and deliberately **not** the words
`api_key`/`secret`/`token`, which appear legitimately in every config schema
in this workspace. Consequence: no allowlist, no suppression, nothing to
except. Verified by `--selftest` and end-to-end (a planted `sk_live_…` is
caught with file:line and pattern, exit 1; removed → exit 0).

**Defect found in the scanner itself and fixed:** once the script became
git-tracked it flagged its own self-test fixtures (11 hits). Fixed by
splitting each synthetic positive across adjacent string literals so no
single line contains a credential-shaped literal, while the concatenated
runtime value the regex is tested against is unchanged. The added `rk_live_`
case also closes a hole where a pattern existed without a selftest case,
which the script's own rules forbid.

## 5. Cargo lockfile resolution

`Cargo.lock` committed: `version = 3`, 598 crates, 6,368 lines.

- `cargo metadata --locked --format-version 1` → **exit 0**. Resolution is
  genuinely locked.
- `version = 3` chosen deliberately so the file stays readable by the
  declared MSRV `rust-version = "1.77"` (v4 needs cargo ≥ 1.78). Not
  regenerated with a newer toolchain just to get v4.
- Correctly out of `.gitignore`, with the reasoning left in `.gitignore` so
  nobody re-adds the entry. `.gitattributes` already marked it `-diff`;
  that attribute is pre-existing and was not added here.
- `pnpm-lock.yaml` is in sync: `pnpm install --frozen-lockfile` exits 0 and
  leaves the file byte-identical.

## 6. Dependency-audit results

```
cargo audit          exit 0   5 warnings, 0 vulnerabilities
pnpm audit --prod    exit 0   No known vulnerabilities found
```

**JavaScript — remediated, not suppressed.** Before: 5 advisories (3 low,
2 moderate) in `dompurify`, reached transitively via `@monaco-editor/react`.
Fixed with a targeted `pnpm.overrides` entry pinning `dompurify` to
`^3.4.17` in the root `package.json`. No other package moved — this is a
security fix, not an unrelated dependency upgrade.

**Rust — documented, not suppressed.** 5 advisories remain, all
`unmaintained` or `unsound`; none is `vulnerable`, which is why plain
`cargo audit` exits 0. `--deny warnings` would fail on non-exploitable
advisories and a gate that fails for a non-reason gets disabled.
`docs/governance/ACCEPTED_ADVISORIES.md` records each with the `grep` that
proves the affected code is unreachable from this repo and the change that
would invalidate the row. There is no `[patch.crates-io]`, no `audit.toml`,
and no `--ignore` flag anywhere. If any of the five were reclassified
`vulnerable`, CI would go red with nobody touching that file.

## 7. Rust formatting results

`cargo fmt --all -- --check` → **exit 0**, both before and after the change.
Now gated in CI as a hard failure on all three OSes: rustfmt is
deterministic (unlike clippy it has no platform-dependent lints) and
`.gitattributes` pins `eol=lf` for `*.rs`, so a CRLF checkout cannot make
it flaky. Adding the gate required adding `rustfmt` to the toolchain
`components` — it is a component, not a separate install.

## 8. Frontend lint and formatting results

```
eslint        exit 0   0 errors, 36 warnings
prettier      clean   --check, no rewrites
```

`pnpm lint` had never once succeeded: the script existed since the project
started but neither eslint nor a config file was committed, so it failed
with "could not find config file". ESLint 9 flat config added.

The decisive line is `react.configs.flat["jsx-runtime"]` — `tsconfig.json`
sets `"jsx": "react-jsx"`, so without it `react-in-jsx-scope` fires on every
JSX element. That mis-configuration produced **1,124 of the first run's
1,209 findings**; the remaining 85 were real, and all 47 errors were fixed.

Rule severities were verified empirically with `eslint --print-config`
rather than assumed: 5 error rules (three inherited from the extended
configs, documented in the config so an upgrade cannot silently change a
severity) and 3 warning rules.

**Two genuine bugs found by the new config, both reproduced not inferred:**

1. `src/hooks/useAuditStream.ts` — a `useEffect` called inside another
   `useEffect`'s callback. React cannot register a hook during the commit
   phase; a probe component in jsdom threw "Invalid hook call".
2. `src/components/ArtifactViewer.tsx` — two `useMemo` after an early
   `return`, so the component called a different number of hooks depending
   on whether any tab was open ("Rendered more hooks than during the
   previous render" when the last tab closes).

Neither had a consumer in the tree and neither was tested, which is why
both survived. Also fixed: 31 unused vars, 7 unescaped entities, 4
`@ts-ignore` → `@ts-expect-error`, 1 empty `catch {}`, and two dead
components (`PackDetail`, 93 lines; `DesktopRequired`).

The 36 residual warnings are itemised by rule and file in
`docs/governance/ACCEPTED_LINT_WARNINGS.md` with the reason each is accepted
*for this codebase* and the change that makes the row stale.

## 9. Rust and frontend test results

**Passing, re-measured on the final tree:**

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, 0 warnings *(measured 09:48; see blockers)* |
| `cargo audit` | exit 0, 5 warnings / 0 vulnerable |
| `pnpm audit --prod` | exit 0, clean |
| secret scan + `--selftest` | exit 0, 13 patterns detect |
| `check_retracted_claims.py` | exit 0 |
| desktop lint / format:check / typecheck / build | exit 0 / clean / 0 / 0 |
| SDK typecheck + smoke | exit 0 / **16 passed, 0 failed** |
| `pnpm install --frozen-lockfile` | exit 0, lockfile unchanged |

**Not passing, and why — stated plainly rather than claimed:**

- **`cargo test --workspace --all-targets` did not execute.** The sandbox
  refuses to let `link.exe` write the test executable into
  `target/debug/deps/` (`LNK1104: cannot open file
  'zylcode_mcp-*.exe'`). `cargo check` and `clippy` are metadata-only and
  pass; the failure is at the link step. An earlier session measured
  `cargo test -p zylcode-mcp --all-targets` at **189 passed, 0 failed** —
  that measurement was real but is **not reproducible now**, and the commit
  message carries that caveat rather than the bare number.
- **Frontend vitest did not execute on the final tree.** The attempt was
  refused by the environment's file-read policy while loading vitest's own
  dependencies from `node_modules`, so it neither passed nor failed. Clean
  runs were measured earlier at **16 files / 108 tests**, and the
  formatting-only parent commit was measured the same. But three of the 18
  changed files (`Forge.tsx`, `ToolRunnerPanel.tsx`,
  `shell/SurfaceHost.tsx`) are under test, so this is genuinely open.
- **Frontend suite is also flaky in this sandbox**, roughly one run in
  three exits non-zero with every executed test still passing. Cause: an
  `EPERM` raised by the host's own fs shim (`node-brokered-fs-shim.cjs`)
  while vitest writes a temp file during config resolution. In one observed
  failure only 15 of 16 files ran (102 of 108 tests), so the flake can
  silently drop a file rather than only fail loudly.

## 10. Pre-existing failures and unresolved blockers

**Pre-existing, unrelated to this work:** the user's `competition/natlas-2026`
checkout is 2 commits behind its upstream. Not touched, not fixed, reported
for visibility.

**Blockers, all environmental rather than code defects:**

1. `cargo test` cannot link — sandbox blocks writes to
   `target/debug/deps/*.exe`. Affects every `cargo test` invocation.
2. `cargo clippy` is intermittent: it passed at 09:48 (exit 0, 0 warnings,
   22.9s) but later runs are blocked writing `target/.rustc_info.json` and
   fingerprint files. Both outcomes recorded; neither was suppressed.
3. Frontend vitest is blocked by the environment's file-read policy, and is
   additionally flaky from the host fs shim described above.

Consequence: **CI's `cargo test --workspace --all-targets` step is
untested.** It was already in the workflow before this work and is left as
is, but it is the single highest-value thing to verify first on a normal
checkout.

## 11. Working tree

**Clean.** `git status --porcelain` is empty on the hardening branch. The
user's `competition/natlas-2026` checkout is unchanged at `c5c9138`. No
branch was pushed, no PR was created, and no `main` was written to.

## 12. Recommended next stage

Ordered by value per unit of risk:

1. **Run the test suites on a normal checkout.** `cargo test --workspace
   --all-targets` and `pnpm --filter zylcode-desktop test`. These are the
   two gates this environment could not execute, and they are the only
   claims in this report that remain unverified. Expect the frontend flake
   to disappear outside this sandbox, since its cause is the host's fs shim.
2. **Then gate the frontend tests in CI.** Not added here precisely because
   nobody has observed the gate pass end-to-end; add it once (1) is green.
3. **Diagnose the vitest EPERM flake properly** if it reproduces outside
   this sandbox. A gate that silently drops a test file is worse than one
   that fails loudly.
4. **Type the Tauri IPC boundary.** All 30 `no-explicit-any` warnings sit on
   the `window.__TAURI__` boundary. Generated bindings (`ts-rs`) or a
   hand-written `.d.ts` would clear the whole class. This is a real API
   decision, not a lint fix, so it belongs in its own change.
5. **Then, and only then, the large-file / god-file refactors.** Not started
   here, as instructed. `Forge.tsx`, `shell/SurfaceHost.tsx` and
   `components/ui/index.tsx` are the obvious candidates; each needs its own
   commit and its own evidence.
6. **Re-audit rather than assume.** Both dependency audits and the secret
   scan are now gated, so a regression is caught mechanically. Re-run
   `cargo audit` and `pnpm audit --prod` after any dependency change and
   before any public claim about the score.

---

*No licensing offer, score improvement, or production-readiness claim is
implied by anything in this report. CI gates passing means the gates run.*
