# Language coverage

What ZylCode's repository intelligence actually does per language, as opposed to
what "multi-language support" sounds like. The table is machine-checked:
`crates/zylcode-core/src/intelligence/classifier.rs` parses it in a unit test and
fails if a `Support` value here disagrees with `support_level()`.

`Support` values mean:

- **Parsed** — the file is read and classified; structure (packages, roles,
  hashes) is extracted.
- **Detected** — the file is recognised and indexed by path, but not parsed.
- **symbols** in the last column means `extract_symbols()` returns definitions
  for that language; `none` means it returns an empty list, which is honest and
  not a missing feature hidden behind a parse count.

| Language   | Extensions  | Support  | Symbols | Notes                                               |
| ---------- | ----------- | -------- | ------- | --------------------------------------------------- |
| Rust       | .rs         | Parsed   | symbols | Regex extraction: structs, enums, traits, fns, impl |
| TypeScript | .ts, .tsx   | Parsed   | symbols | Classes, interfaces, types, functions, variables    |
| JavaScript | .js, .jsx   | Parsed   | symbols | Same extractor as TypeScript                        |
| Json       | .json       | Parsed   | none    | Parsed as a document, no definitions to extract     |
| Toml       | .toml       | Parsed   | none    | Manifests are read by `manifest.rs`, not as symbols |
| Yaml       | .yml, .yaml | Parsed   | none    | Same: configuration, not code                       |
| Markdown   | .md         | Detected | none    | Indexed by path; content is untrusted text          |
| Html       | .html       | Detected | none    | Indexed by path                                     |
| Css        | .css        | Detected | none    | Indexed by path                                     |
| Shell      | .sh         | Detected | none    | Indexed by path; entry points detected by role      |

## What this means for a claim

`typescript_symbol_index` is only true because a TypeScript symbol is returned by
a product surface — `zylcode repo-symbols <name>`. The extractor existing is not
the capability; reaching it is. `docs/capability-probes.json` binds that claim to
a process-level test (`repo_symbols_returns_a_typescript_symbol`).

## The honest gap

Ten languages are recognised; three produce symbols. Python, Go, Java, C/C++,
Ruby and C# are **not** recognised at all — they classify as
`Language::Unknown("<ext>")`, which is `Detected`. Adding a language means, in
this order: extensions → `support_level` → extraction → a product surface → a
probe. Anything less is a parse count wearing a capability's clothes.

The next language should be chosen from what users' repositories actually
contain, not from a list of popular languages; no such measurement exists yet,
and inventing one would repeat the problem this document exists to avoid.
