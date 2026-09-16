# Extension Contribution Guide

> **STATUS: PROPOSED — extension ABI not yet implemented.**
> This guide describes the *intended* extension contribution flow. Until the Extension Platform ABI
> exists (Phase 5, `ZYLCODE_MASTER_EXECUTION_PLAN.md`), contributions are limited to what the current
> system supports (the MCP bridge and the plugin marketplace module), which are **not** a stable package
> ABI. Do not build an extension assuming a contract that does not exist.

## What an extension is (target state)

A package expressing capability without modifying core:

```
ZylCode Package
├── manifest        (identity, version, compatibility, permissions)
├── tools
├── mcp_servers
├── skills
├── agents
├── commands
├── hooks
├── model_providers
├── ui_panels
├── runtimes
├── templates
├── design_libraries
└── verification_providers
```

The rule: anything that would otherwise be a hard-coded integration must be expressible as a package.
Marketplace comes later. ABI first.

## Before contributing an extension (when the ABI lands)

1. Read `docs/architecture/EXTENSION_PLATFORM.md`.
2. Declare **permissions** explicitly in the manifest. Undeclared capability is denied.
3. State the **compatibility** range; do not claim compatibility the ABI does not guarantee.
4. Include a verification provider or hook only if it can be tested against the real system, not a fixture.

## What you can do today

- Use the existing MCP bridge to connect an external tool (see `zylcode mcp-bridge`).
- Report gaps where a hard-coded integration should become a package once Phase 5 lands.

## Prohibited

- **Claiming a stable extension ABI exists.** It does not. Mark PROPOSED.
- **Shipping an extension that modifies core** to work around the missing ABI. Wait for it.
- **Fabricating a marketplace listing or download count.** None exists yet.

This file is intentionally conservative. When Phase 5 is accepted, this guide is expanded to a full
contribution walkthrough and its PROPOSED marker is removed.
