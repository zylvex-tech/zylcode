# EXTENSION_PLATFORM.md — ZylCode Extension Platform

**Status: PROPOSED**
**Rung: R0 (CLAIMED)**
**Phase: 5**
**Governing docs:** `ZYLCODE_PRODUCT_CONSTITUTION_V2.md` §7 · `ZYLCODE_ARCHITECTURE_V2.md` §4.1

> ⚠️ **This system does not exist.**
> An MCP bridge and a plugin-marketplace module exist, but there is **no stable package ABI**.
> Do not confuse "there is a plugin marketplace module" with "there is an extension platform".
> A marketplace without an ABI sells packages nothing can load.

---

## 1. The Rule

> **Extension ABI first. Marketplace second.**

If extensibility is deferred, core accumulates hard-coded integrations that must later be
redesigned as extensions — at a cost that grows with every integration added. By Phase 9 the
redesign would touch half the product.

---

## 2. Why This Is Phase 5, Not Phase 9

| Deferred to Phase 9 | Consequence |
|---|---|
| Android integration (Phase 10) | built into core, must be extracted later |
| Mac worker (Phase 11) | built into core, must be extracted later |
| Deployment providers (Phase 13) | built into core, must be extracted later |
| Verification providers (Phase 12) | built into core, must be extracted later |

Four systems, each of which would need extraction. Defining the ABI first costs far less.

---

## 3. Package Structure

```
zylcode-package/
├── manifest.json           identity, version, compatibility, permissions
├── tools/                  RealTool implementations
├── mcp/                    MCP server declarations
├── skills/                 skill definitions
├── agents/                 agent definitions
├── commands/               user-facing commands
├── hooks/                  lifecycle hooks
├── providers/              model providers
├── panels/                 UI panels
├── runtimes/               execution backends
├── templates/              project/scaffold templates
├── design_libraries/       design systems
└── verification/           verification providers
```

---

## 4. Manifest

```json
{
  "id": "com.example.android-lab",
  "name": "Android Device Lab",
  "version": "1.0.0",
  "engine": { "min": "0.3.0", "max": "0.4.x" },
  "publisher": { "id": "...", "name": "...", "signature": "..." },
  "permissions": ["filesystem:read", "process:spawn", "network:outbound"],
  "contributes": {
    "runtimes":    ["android-emulator"],
    "tools":       ["android.install_apk", "android.launch", "android.logcat"],
    "commands":    ["android build", "android test"],
    "panels":      ["android-device-lab"],
    "verification":["android-runtime"]
  },
  "entry": { "kind": "native|wasm|process", "path": "..." }
}
```

### 4.1 Manifest requirements

| Field | Why |
|---|---|
| `engine` range | Prevents a package loading against an ABI it was not built for |
| `permissions` | Undeclared capability is **denied** |
| `publisher.signature` | Identity, required for the marketplace (Phase 15) |
| `contributes` | The complete list of capability points; anything not listed is not available |
| `entry.kind` | Determines the isolation model |

---

## 5. Contribution Points

| Point | What it provides | Consumed by |
|---|---|---|
| `tools` | Agent-callable actions with risk levels | Agent Kernel |
| `mcp` | MCP servers | MCP bridge |
| `skills` | Skill definitions | Agent Kernel |
| `agents` | Agent definitions | Agent Kernel |
| `commands` | User commands | CLI / UI |
| `hooks` | Lifecycle hooks | Core |
| `providers` | Model providers | Model Platform |
| `panels` | UI surfaces | Shell |
| `runtimes` | Execution backends | Execution Engine |
| `templates` | Project scaffolds | Project System |
| `design_libraries` | Design systems | Vision Studio |
| `verification` | Verification providers | Proof Engine |

**Each contribution point is a versioned contract.** Changing one is an ABI break and requires a
major version bump.

---

## 6. Permission Model

Fail-closed. Undeclared capability is denied.

| Permission | Grants |
|---|---|
| `filesystem:read` / `filesystem:write` | scoped to declared paths |
| `process:spawn` | execute external processes |
| `network:outbound` | egress (off by default — local-first) |
| `project:read` / `project:write` | Project state access |
| `ledger:append` | write evidence |
| `artifact:create` | publish artifacts |

**Rules**

1. Permissions are declared in the manifest **and** granted by the user. Both are required.
2. Granted permissions are recorded as Project decisions.
3. A permission escalation on update requires re-approval. An update that silently widens
   permissions is rejected.
4. `network:outbound` is the highest-scrutiny permission: ZylCode is local-first, and a package
   requesting egress is a decision the user must make explicitly.

---

## 7. Isolation

| Entry kind | Isolation | Use |
|---|---|---|
| `wasm` | strongest | pure computation, portable |
| `process` | strong | language-agnostic, OS-level sandbox |
| `native` | weakest | trusted first-party only |

**Default recommendation: `process`.** Native loading is reserved for first-party packages
because it shares the address space and cannot be sandboxed meaningfully.

---

## 8. Lifecycle

```
install → validate manifest → check engine compatibility
        → display permissions → user grants
        → register contributions → activate

update  → validate → check compatibility → diff permissions
        → if widened: re-approval required
        → swap atomically

disable → deactivate → unregister contributions (state retained)
remove  → deactivate → unregister → delete (state purged on request)
```

**Atomicity:** a failed update leaves the previous version active. Never a half-installed state.

---

## 9. Interfaces

```
zylcode ext install <package>
zylcode ext list
zylcode ext inspect <package>
zylcode ext permissions <package>
zylcode ext disable|enable|remove <package>
```

Plus the UI Extensions surface.

---

## 10. Benchmark / Acceptance

| # | Criterion | Method |
|---|---|---|
| 1 | Reference package in a **separate repository** contributes a tool, an agent and a UI panel | zero core modification, demonstrated |
| 2 | Undeclared permission is denied | negative test with output |
| 3 | Widened permissions on update force re-approval | test with output |
| 4 | Failed update leaves the old version active | fault-injection test |
| 5 | Engine version mismatch is rejected with a clear message | test with output |
| 6 | Package removal purges contributions | state dump before/after |

**Rung target: R3.** Criterion 1 must be demonstrated from an **out-of-tree** repository — a
package living inside the core repo proves nothing.

---

## 11. Marketplace (Phase 15 — later)

Deliberately deferred until the ABI is stable:

signed packages · permission review · versioning · compatibility · ratings · publisher identity ·
update mechanism · security review.

> Selling packages for an ABI that is still moving produces packages that break. The ABI is the
> product; the marketplace is distribution.

---

## 12. Anti-Requirements

- Do not build the marketplace before the ABI is stable.
- Do not allow undeclared capability.
- Do not permit silent permission widening on update.
- Do not load untrusted native code.
- Do not special-case first-party integrations in core — if a first-party integration needs a
  path core does not offer, the ABI is incomplete, not the integration.
