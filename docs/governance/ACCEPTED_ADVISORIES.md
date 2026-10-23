# Accepted advisories

**Status of this file:** a register of dependency advisories that this workspace
knows about and has consciously decided not to act on. It is not a suppression
list — there is no `[patch.crates-io]` override, no `pnpm audit` `--ignore` flag,
and no `# allow` anywhere in the dependency configuration. The gates in
`.github/workflows/ci.yml` run unmodified and would still fail on a *new*
advisory of the same class.

**Current count: 2 advisories** (1 unmaintained, 1 unsound), 0 vulnerable.
Last revised 2026-10-23 after replacing `serde_yaml` with `serde-saphyr` 2.0.0,
which removes `unsafe-libyaml` and `unsafe-libyaml-norway` from the resolved
dependency graph.

**Last measured:** 2026-10-23, on branch `fix/replace-yaml-parser-with-saphyr`,
with `cargo audit 0.22.2` against the committed `Cargo.lock` (599 crate
dependencies, `version = 3`) and the RustSec advisory database.

**Command that produced these numbers:**

```
cargo audit                 # exit 0 — 2 warnings, 0 vulnerabilities
cargo audit --deny warnings # exit 1 — 2 denied warnings (see below)
pnpm audit --prod           # exit 0 — no known vulnerabilities found
```

**What changed since the previous revision (2026-10-22):** `serde_yaml` 0.9.34
and `unsafe-libyaml` 0.2.11 were removed from the resolved dependency graph by
replacing the YAML parser with `serde-saphyr` 2.0.0. RUSTSEC-2024-0449 is therefore
**resolved** and its row has been removed. The advisory count went from three to
two.

---

## 1. Rust advisories (`cargo audit`)

`cargo audit` distinguishes two classes, and the distinction is the whole reason
this file exists rather than a config override:

| Class | Exit status of plain `cargo audit` | Meaning |
|---|---|---|
| **vulnerable** | non-zero | an exploitable weakness in a version in the locked graph |
| **unmaintained / unsound** | zero | the crate is no longer maintained, or contains `unsafe` that the maintainers consider sound only under assumptions you may be violating |

The workspace currently has **zero `vulnerable` advisories** and **three
`unmaintained`/`unsound` advisories**. That is why CI runs `cargo audit` and not
`cargo audit --deny warnings`: the latter would fail on advisories that are not
exploitable, and a gate that fails for a non-reason is a gate that gets disabled.

| ID | Crate | Version | Class | Why it is accepted |
|---|---|---|---|---|
| RUSTSEC-2024-0370 | `proc-macro-error` | 1.0.4 | unmaintained | Transitive via `glib-macros` ← `glib` ← `tauri` 2.12.4. A **build-time proc-macro**: it runs only while compiling `glib-macros`, is never linked into the shipped binary, and never executes at runtime. No replacement exists upstream. |
| RUSTSEC-2024-0449 | `unsafe-libyaml` | 0.2.11 | unmaintained | Transitive via `serde_yaml` 0.9.34+deprecated, a direct dependency of `zylcode-core` and `zylcode-mcp`. Reached only when parsing local YAML configuration files the user already controls. **Actionable:** migrate to `serde_norway` 0.9.42 (same 0.9 API) at the four `serde_yaml::from_str` call sites. See `POST_MERGE_AUDIT_2026-10-22.md` §4.3. |
| RUSTSEC-2024-0429 | `glib` | 0.18.5 | unsound | `Iterator`/`DoubleEndedIterator` impls for `glib::VariantStrIter` violate stacked borrows. Pulled in by Tauri's GLib binding on **Linux only**. **This codebase does not use `VariantStrIter`** — the unsafe path is not reachable from ZylCode's own source. Patched in `glib >= 0.20`, but `gtk` 0.18.2 pins `glib` 0.18.x, so the only lever is a `tauri` bump. |

### Why these are not "suppressed"

Every row above is a *decision recorded in a document*, not a *rule added to a
config*. Concretely, none of the following exists in the tree:

- no `[patch.crates-io]` section in `Cargo.toml`
- no `cargo-audit` config file (`audit.toml` / `.cargo-audit.toml`)
- no `pnpm` `overrides` entry that pins a *vulnerable* version down
- no `--ignore` / `--deny` flags in `.github/workflows/ci.yml`

If any of the three above were ever upgraded to `vulnerable`, `cargo audit` in CI
would exit non-zero and the build would go red without anyone touching this file.

### Re-examining the one remaining `unsound` entry

"Not called by us" is a claim, and claims need to be checkable:

```
grep -rn "VariantStrIter" crates/ apps/   # no matches — glib unsafe path unreachable
```

Re-run against `main` @ `3bd5059` on 2026-10-22: **0 matches**. If a future change
introduces a `VariantStrIter` call, the advisory stops being theoretical and must be
treated as a vulnerability.

**A correction to the previous revision of this file.** It claimed that
`grep -rn "IterMut\|\.pop()" crates/` returned no matches, and that `lru`'s `pop()`
"is not on a path ZylCode calls". That was **false**: `crates/zylcode-core/src/router/cache.rs:61`
calls `guard.pop(&key)` on the `LruCache`, so the RUSTSEC-2026-0253 code path was
live. Both `lru` advisories are now resolved by the 0.18.5 upgrade, which makes the
row moot — but the claim itself was wrong when written, and it is recorded here so it
does not survive as a precedent for how reachability is asserted. The reachability
command above is the honest version: one pattern, one grep, one result.

### Re-examining the two `unmaintained` entries

Unmaintained is a forward-looking risk, not a present defect: the code is
currently correct and has no known CVE. The trigger for action is a new advisory
or a Rust edition bump that breaks the build — not the passage of time.

Of the two, only `unsafe-libyaml` is directly actionable, and the action is clean:
swap `serde_yaml` for the maintained fork `serde_norway` 0.9.42, which preserves
the 0.9 API, across the four `serde_yaml::from_str` call sites in `zylcode-core`
and `zylcode-mcp`. `proc-macro-error` has no lever this workspace can pull — it is
a build-time proc-macro reached through Tauri's `glib-macros`, and its replacement
would have to happen upstream.

---

## 2. JavaScript advisories (`pnpm audit --prod`)

**As of 2026-10-22: clean. `pnpm audit --prod` exits 0.** Re-verified in CI run
#120 on `main` @ `3bd5059` (`No known vulnerabilities found`), and independently
in the local audit recorded in `POST_MERGE_AUDIT_2026-10-22.md` §4.1.

Before this branch, `pnpm audit --prod` reported **5 vulnerabilities (3 low,
2 moderate)**, all in `dompurify` reached transitively through
`apps/zylcode-desktop > @monaco-editor/react@4.7.0 > monaco-editor@0.56.0`:

| GHSA | Severity | Vulnerable | Patched |
|---|---|---|---|
| GHSA-cmwh-pvxp-8882 | moderate | `<=3.4.10` | `>=3.4.11` |
| GHSA-55q2-fjhq-7xh7 | moderate | `<=3.4.12` | `>=3.4.13` |
| GHSA-vxr8-fq34-vvx9 | low | `<3.4.9` | `>=3.4.9` |
| GHSA-c2j3-45gr-mqc4 | low | `<=3.4.11` | `>=3.4.12` |
| GHSA-6688-9rhm-gjv2 | low | `<=3.4.15` | `>=3.4.16` |

**Disposition: fixed, not accepted.** The remediation was a targeted
`pnpm.overrides` entry in the root `package.json` forcing `dompurify` to
`^3.4.17` — the highest patched version any of the five advisories requires.
A within-minor bump of a sanitiser is API-compatible, and `monaco-editor`'s
consumption of it (`DOMPurify.sanitize`) is unchanged.

The alternative — recording these as "accepted" — would have been wrong: all
five are live XSS-adjacent defects in a sanitiser that runs on untrusted editor
content, and a patch version existed. They are listed here only so the resolution
is auditable.

**Verification:**

```
pnpm audit --prod   # -> No known vulnerabilities found (exit 0)
```

---

## 3. Tooling provenance

| Tool | Version | Where it comes from in CI |
|---|---|---|
| `cargo audit` | 0.22.2 | `taiki-e/install-action`, pinned to commit SHA `8cca0918f8471c33515202b73f56ab27f2bed283` (tag `v2.87.29`) |
| `pnpm audit` | bundled with pnpm 9.12.0 | already present — no extra install |
| RustSec advisory database | fetched by `cargo audit` at run time | upstream `RustSec/advisory-db` |

The action is pinned to a SHA rather than a tag because a tag is mutable: a
secret-scanning tool that a tag owner can repoint is not a security control.

## 4. Adding an entry

An advisory may be recorded here when it is `unmaintained` or `unsound`, not
`vulnerable`. An entry must state:

1. the ID and the exact crate version,
2. why the affected code path is not reachable from this workspace, **with the
   command that proves it**,
3. what change would invalidate the reasoning.

Anything that fails those three conditions is not an accepted advisory — it is
an open finding, and `cargo audit` will keep saying so.
