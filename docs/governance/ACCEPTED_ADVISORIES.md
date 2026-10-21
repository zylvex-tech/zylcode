# Accepted advisories

**Status of this file:** a register of dependency advisories that this workspace
knows about and has consciously decided not to act on. It is not a suppression
list — there is no `[patch.crates-io]` override, no `pnpm audit` `--ignore` flag,
and no `# allow` anywhere in the dependency configuration. The gates in
`.github/workflows/ci.yml` run unmodified and would still fail on a *new*
advisory of the same class.

**Last measured:** 2026-10-21, on branch
`hardening/phase1-security-and-reproducibility` at HEAD
`a4f6b3e2` + the Phase-1 changes, with `cargo audit 0.22.2` against the
committed `Cargo.lock` (598 crate dependencies, `version = 3`) and the RustSec
advisory database (`Loaded 1348 security advisories`).

**Command that produced these numbers:**

```
cargo audit                 # exit 0 — 5 warnings, 0 vulnerabilities
cargo audit --deny warnings # exit 1 — 5 denied warnings (see below)
pnpm audit --prod           # exit 0 — no known vulnerabilities found
```

---

## 1. Rust advisories (`cargo audit`)

`cargo audit` distinguishes two classes, and the distinction is the whole reason
this file exists rather than a config override:

| Class | Exit status of plain `cargo audit` | Meaning |
|---|---|---|
| **vulnerable** | non-zero | an exploitable weakness in a version in the locked graph |
| **unmaintained / unsound** | zero | the crate is no longer maintained, or contains `unsafe` that the maintainers consider sound only under assumptions you may be violating |

The workspace currently has **zero `vulnerable` advisories** and **five
`unmaintained`/`unsound` advisories**. That is why CI runs `cargo audit` and not
`cargo audit --deny warnings`: the latter would fail on advisories that are not
exploitable, and a gate that fails for a non-reason is a gate that gets disabled.

| ID | Crate | Version | Class | Why it is accepted |
|---|---|---|---|---|
| RUSTSEC-2024-0370 | `proc-macro-error` | 1.0.4 | unmaintained | Transitive via `syn`/`quote`-derived macros. No replacement exists upstream; the code is a stable, widely-vendored macro helper. |
| RUSTSEC-2024-0449 | `unsafe-libyaml` | 0.2.11 | unmaintained | Transitive via `serde_yaml` 0.9. Only reached when parsing YAML MCP configuration, and only from a local file the user already trusts. |
| RUSTSEC-2024-0429 | `glib` | 0.18.5 | unsound | `Iterator`/`DoubleEndedIterator` impls for `glib::VariantStrIter` violate stacked borrows. Pulled in by Tauri's GLib binding. **This codebase does not use `VariantStrIter`** — the unsafe path is not reachable from ZylCode's own source. |
| RUSTSEC-2026-0002 | `lru` | 0.12.5 | unsound | `IterMut` invalidates an internal pointer under Stacked Borrows. The only `LruCache` in the workspace is `SpeculativeCache` in `crates/zylcode-core/src/router/cache.rs`; `IterMut` is not called on it. |
| RUSTSEC-2026-0253 | `lru` | 0.12.5 | unsound | `LruCache::pop()` lacks panic safety. Same cache, same assessment: `pop()` is not on a path ZylCode calls. |

### Why these are not "suppressed"

Every row above is a *decision recorded in a document*, not a *rule added to a
config*. Concretely, none of the following exists in the tree:

- no `[patch.crates-io]` section in `Cargo.toml`
- no `cargo-audit` config file (`audit.toml` / `.cargo-audit.toml`)
- no `pnpm` `overrides` entry that pins a *vulnerable* version down
- no `--ignore` / `--deny` flags in `.github/workflows/ci.yml`

If any of the five above were ever upgraded to `vulnerable`, `cargo audit` in CI
would exit non-zero and the build would go red without anyone touching this file.

### Re-examining the two `unsound` entries

"Not called by us" is a claim, and claims need to be checkable:

```
grep -rn "VariantStrIter" crates/          # no matches — glib unsafe path unreachable
grep -rn "IterMut\|\.pop()" crates/        # no matches on the LruCache in
                                           # crates/zylcode-core/src/router/cache.rs
```

Both were run on 2026-10-21 and returned nothing. The only `LruCache` in the
workspace is `SpeculativeCache` in `crates/zylcode-core/src/router/cache.rs`, and
it is reached through `get`/`put`/eviction only.

Re-run both before relying on this row. If a future change introduces either
call, the advisory stops being theoretical and must be treated as a
vulnerability.

### Re-examining the three `unmaintained` entries

Unmaintained is a forward-looking risk, not a present defect: the code is
currently correct and has no known CVE. The trigger for action is a new advisory
or a Rust edition bump that breaks the build — not the passage of time.

---

## 2. JavaScript advisories (`pnpm audit --prod`)

**As of 2026-10-21: clean. `pnpm audit --prod` exits 0.**

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
