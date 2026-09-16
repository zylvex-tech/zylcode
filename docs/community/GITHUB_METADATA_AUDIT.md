# GitHub Public Metadata Audit — PUBLIC-FOUNDATION-01

**Track:** `PUBLIC-FOUNDATION-01` (GitHub Public Foundation)
**Status:** Report delivered. **Remote settings not modified** — see "Why no changes were applied" below.

---

## What this audit covers

The GitHub repository metadata: About description, Topics, Website field, license metadata, README
license statement, homepage, releases, tags, Discussions, Issues, Projects, SECURITY/Contributing
links, and stale public claims.

## Limitation — read access from the sandbox

> The build/agent environment has **no authenticated GitHub CLI** (`gh auth login` required) and no API
> token. The live GitHub metadata **could not be read** during this audit. The values below are the
> **target/recommended** state from `DEEPSEEK_PUBLIC_FOUNDATION_PROMPT_V1.md` Workstream A, **not** a
> reading of the current repo. The owner must verify each against the live repository at
> `github.com/zylvex-tech/zylcode/settings`.

This is the same discipline the governance package enforces for capability claims: do not assert a
current state you have not observed.

---

## Recommended metadata (apply via repo Settings → General / Social)

### About description (target)

> Evidence-first autonomous software creation environment. Plan, build, run, inspect, repair, verify,
> and ship software from one project.

Remove any of the following unless independently supported at the appropriate rung: `formally verified`,
`zero token waste`, `best`, `fastest`, `superior to competitors`, `production-grade`.

### Topics (reconcile — do not remove useful existing ones without justification)

```
ai
coding-agent
developer-tools
rust
tauri
autonomous-agents
mcp
ide
llm
software-engineering
local-first
open-source
```

### Website field

Do **not** point GitHub at a placeholder/broken ZylCode page. Set it to the canonical ZylCode product
URL only once that page is live and verified (see `zylforge.com` rebuild track).

---

## Audit checklist for the owner (verify against live repo)

- [ ] About description communicates the target text accurately; no unsupported superlatives.
- [ ] Topics reconciled to the list above; no stale/duplicate topics.
- [ ] License metadata matches the README license statement (no mismatch — a known class of defect).
- [ ] Homepage / Website field points to a real, verified URL or is empty (not a placeholder).
- [ ] Releases / tags: any "released" or "available" claim matches an actual tagged, published artifact.
- [ ] Discussions enabled (preferred categories: Announcements, Ideas, Show and Tell, Help,
      Architecture, Extensions) — or, if GitHub's model differs, enabled with a note on manual steps.
- [ ] Issues / Projects: no phantom or auto-generated issues presenting fake engineering work.
- [ ] SECURITY.md and CONTRIBUTING.md linked (both now present at repo root, this batch).
- [ ] No stale public claim anywhere (README, releases, About) above the implemented rung.

---

## Why no changes were applied from the sandbox

GitHub repository metadata is set via the GitHub web UI or API, **not** via a git commit. This
environment cannot authenticate to GitHub, so the audit reports the target state and the verification
checklist for the owner to apply. This is correct behaviour: a contribution must not fabricate that it
performed an action it could not perform.

(PF-02 — the OSS community foundation files — *is* committable and is delivered in the same batch as
documentation.)
