# `mcp.tools.yaml` — DISPOSITION

**Disposition: REGENERATE.**

The untracked developer-local file is **not** committed. A file regenerated from
the canonical catalogue is committed in its place. The original is preserved
untouched in the primary working tree (see §6).

Authority: owner decision, Option C — controlled consolidation, unit 6.
Requirement: *"No executable product capability may depend exclusively on an
untracked developer-local file."*

---

## 1. Why this file matters

It is not incidental configuration. It is the **live tool surface of the shipped
desktop application**:

| Site | Code |
|---|---|
| `crates/zylcode-mcp/src/lib.rs:73` | `env ZYLCODE_MCP_CONFIG`, default `"mcp.tools.yaml"` |
| `apps/zylcode-desktop/src-tauri/src/main.rs:433` | `register_from_default_location(reg.as_ref())` |
| `apps/zylcode-desktop/src-tauri/src/main.rs:727` | `register_from_default_location(reg.as_ref())` |
| `apps/zylcode-desktop/src/components/McpInspector.tsx:87` | UI: *"No tools registered. Add `mcp.tools.yaml` → hot-reload."* |

It is loaded by `register_from_default_location` → `register_from_config_file` →
`register_from_config`, which wraps each entry in a `DynamicTool`.

It is **untracked and unignored**: `git check-ignore -v mcp.tools.yaml` returns
nothing, and `.gitignore` has no entry. It was simply never added.

---

## 2. Comparison against the canonical catalogue and the executor factory

| Measure | Value |
|---|---|
| Entries in the untracked file | **30** |
| Real executors in `get_real_tool` | **12** |
| Entries that have a real executor | **9** |
| **Entries with no executor** | **21** |
| Executors absent from the file | **3** |

**The 9 that work:** `fs.read`, `fs.write`, `fs.list`, `git.status`, `git.diff`,
`git.commit`, `npm.run`, `search.find`, `search.grep`

**The 21 that do not:** `ai.evaluate`, `ai.train`, `build.debug`, `build.release`,
`code.analyze`, `code.format`, `code.lint`, `db.query`, `docker.build`,
`docker.run`, `docs.generate`, `git.pull`, `git.push`, `k8s.apply`, `k8s.get`,
`npm.install`, `perf.bench`, `security.scan`, `slack.send`, `test.run`,
`test.single`

**The 3 missing:** `cargo.test`, `shell.echo`, `shell.execute`

### 2.1 Two entries reference files that do not exist

| id | command | reality |
|---|---|---|
| `ai.train` | `python train.py --model {{model}}` | `train.py` does not exist in the repository |
| `ai.evaluate` | `python evaluate.py --model {{model}}` | `evaluate.py` does not exist in the repository |

These are the same defect class as the bridge simulator: a tool present in the
catalogue, `enabled: true`, that cannot work.

### 2.2 The `command`/`args` fields are decorative for every working entry

The nine working entries carry `command`/`args` such as:

```yaml
- id: "git.commit"
  command: "git"
  args: ["commit", "-m", "{{message}}"]
```

The real executor **ignores these**. `GitTool` is bound to `git commit` by its
tool id and builds its own argument vector from `message` / `files`. The same is
true for `npm.run` (`npm run` + caller args), `cargo.test`, and the rest.

That is now enforced rather than incidental: the security commit *"bind tool IDs
to permitted operations"* made a caller-supplied program or subcommand a
hard refusal. So the `command` field is schema-compatibility ballast, and a
reader who believes it is the executed command is misled.

---

## 3. What changed underneath this file

Before this consolidation, a `DynamicTool` whose id had no executor returned
`Ok({"status": "unsupported_mock", "echo": params})` — a *successful* result for
a call that executed nothing. The 21 non-executable entries therefore appeared to
work.

That path is gone. `DynamicTool::call` now resolves a real executor or returns
`ToolError::NotImplemented`.

**Consequence: if the untracked file were committed as-is, 21 of its 30 entries
would fail closed at call time.** The application would present a 30-tool
catalogue in which 70% of the tools error on use. That is the fabricated-surface
defect reappearing in configuration form.

---

## 4. Options considered

| Option | Assessment |
|---|---|
| **COMMIT** the file as-is | **Rejected.** Ships 21 entries that now fail closed, plus 2 references to non-existent scripts. Converts a fabricated *implementation* into a fabricated *configuration*. |
| **REGENERATE** from the catalogue | **Chosen.** Every entry then has a real executor by construction, and the file can be checked against the catalogue mechanically. |
| **REPLACE** the mechanism | **Rejected as unnecessary.** `register_from_config_file` works correctly; the loader is not the problem. Replacing it would be churn. |
| **REMOVE** the file | **Rejected.** The product would register zero tools. The UI already tells users to add this file; removing the mechanism without a replacement leaves the desktop app inert. |
| **QUARANTINE** (leave untracked) | **Rejected.** This is the status quo and is precisely the criterion that fails. An untracked file is not shipped, not reviewed, and not reproducible. |

---

## 5. The regenerated file

Committed as `mcp.tools.yaml` at the repository root, derived from
`Catalogue::canonical().executable_ids()`:

* exactly the **12** ids that have a real executor;
* `command: "builtin"` throughout, because the executors are bound by id and no
  caller-supplied program is honoured — the field is present only because
  `McpToolConfig` requires it;
* the 21 unsupported entries retained under a top-level `proposed:` key, which
  the loader does not read, so their schemas are preserved without presenting
  them as executable;
* the two broken script references recorded, not enabled.

Invariant to check mechanically:

```
{ ids under `tools:` }  ==  { id | get_real_tool(id).is_some() }
```

This is asserted by `tool_catalogue::tests::every_executable_entry_has_a_real_executor`
and can be extended to parse the YAML directly.

---

## 6. The original file is preserved, not deleted

The untracked `mcp.tools.yaml` in the primary working tree is **untouched**. Its
30 entries are enumerated in §2 and its content is recorded here in full
comparison form.

**Merge hazard, stated explicitly.** Committing a regenerated `mcp.tools.yaml`
means the primary working tree will show the untracked original as a
conflict-on-checkout. The owner must resolve it deliberately: the 21 entries
belong in `proposed:`, not in `tools:`.

Per owner decision 4, the dirty tree is preserved, not cleaned.

---

## 7. Reproduction

```bash
# untracked and unignored
git check-ignore -v mcp.tools.yaml        # no output
grep -n 'mcp.tools.yaml' .gitignore       # no output

# 30 entries
grep -cE '^\s+- id: "' mcp.tools.yaml

# 12 real executors
sed -n '/pub fn get_real_tool/,/^}/p' crates/zylcode-mcp/src/real_tools.rs \
  | grep -oE '"[a-z0-9._]+"' | tr -d '"' | sort -u | wc -l

# two broken script references
grep -A2 'id: "ai.train"' mcp.tools.yaml ; ls train.py      # No such file

# the file is loaded by the product
grep -n 'mcp.tools.yaml' crates/zylcode-mcp/src/lib.rs \
  apps/zylcode-desktop/src-tauri/src/main.rs
```
