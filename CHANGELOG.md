# Changelog

All notable changes to ZylCode. Dates are the merge date on `main`.

This file starts at 0.2.4; earlier releases are described in the GitHub release
notes and in `README.md` history. A version is only listed here once it is on
`main`; it is not a promise that the version has been tagged or published.

## [0.2.4] — unreleased

### Added

- **Repository symbols from the CLI** (`zylcode repo-symbols <name>`): resolves a
  symbol through the persisted index the agent uses. This is the product surface
  that makes `typescript_symbol_index` a capability rather than an extractor.
  (#74, #76)
- **Mission evidence verification** (`zylcode missions verify`): file citations
  record a SHA-256 at citation time; verification re-reads the file and exits
  non-zero when the bytes changed or the file is gone. Non-hashable evidence
  (test, command, human) is reported as unverifiable rather than counted as green.
  (#76)
- **Prompt-injection scan** (`zylcode trust-scan <path>`): reports
  instruction-shaped repository text and hidden unicode with line numbers; exits
  non-zero on high-severity findings. Repository text is treated as data. (#76)
- **Capability probe gate** (`scripts/check_capability_probes.py`): binds registry
  claims to a named test, file, command or transcript, and fails CI when a
  required capability has no probe. Coverage is printed every run (currently
  5 of 28 GREEN R3+ capabilities). (#76)
- **Generated roadmap status board** (`scripts/render_status_board.py` +
  `docs/governance/phase-status.json`): the status table is rendered, and CI fails
  on drift. (#76)
- **Benchmark question freeze** (`scripts/freeze_question_set.py`): a hash and a
  reason pin a question set before scoring. (#76)
- **Desktop Mission Engine panel**: eleven-state mission surface over five Tauri
  commands. (#75)
- **Cross-process project-store lock**: the CLI and desktop can no longer
  interleave writes to `.zylcode/projects.json`. (#76)

### Changed

- **Repository indexing now extracts TypeScript and JavaScript symbols** in the
  canonical query path, not only Rust. (#74)
- `remote_ci` in the capability registry is re-stated as PARTIAL/R2: CI is green
  on Ubuntu, macOS and Windows, and the recorded billing blocker no longer
  matched observed reality. (#77)
- `release_installers` evidence now says the packaging path is unrehearsed for
  this head rather than blocked by billing. (#77)
- `README.md` front page brought back in line with `main` and the registry:
  status date and commit, capability counts (31 GREEN / 3 DISPUTED / 3 PARTIAL),
  Phase 2A acceptance, Phase 2B status, and test/benchmark figures. (#77)

### Fixed

- A relative `cwd` for terminal commands resolves against the workspace root
  (pre-existing behaviour, now covered by a test that was flaky on CI runners).
  (#75)

### Known gaps in this version

- Phase 2A conditions C2–C6 are open; C1 is closed in code but the registry entry
  has not been promoted with its evidence.
- Phase 3A: project knowledge graph, desktop wiring and evidence bytes in export
  bundles are missing.
- Phase 3B: missions are not yet persisted inside the Project System, and ledger
  evidence has no replay tooling.
- Phase 2B has a contract and no engine.
- No signed installers; the tagged packaging matrix has not been rehearsed for
  this head.

## Earlier releases

| Version | Date       | Headline                                      |
| ------- | ---------- | --------------------------------------------- |
| 0.2.3   | 2026-10-29 | Provider boundary, real wire parsing          |
| 0.2.2   | 2026-10-27 | —                                             |
| 0.2.1   | 2026-10-25 | —                                             |
| 0.2.0   | 2026-08-31 | Zero-alloc core, speculative LRU, dynamic MCP |
| 0.1.0   | 2026-08-31 | First pre-release                             |
