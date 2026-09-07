# Verification Ladder

ZylCode's verification system provides increasing levels of assurance for AI-generated code changes. Each rung adds a layer of verification, with higher rungs gating access to more sensitive operations.

## Rungs

### Rung 1: Lint + Type-Check
**Status:** Planned (Phase 10, Track A)

Fast, deterministic checks that catch the most common errors:
- Rust: `cargo clippy`, `cargo check`
- TypeScript: `tsc --noEmit`, ESLint
- Python: `mypy`, `ruff`

**Gate:** Must pass before any code change is proposed to the user.

### Rung 2: Property-Based Tests
**Status:** Planned (Phase 10, Track A)

Automatically generated test cases that exercise invariants across random inputs:
- Rust: `proptest` or `quickcheck`
- TypeScript: `fast-check`

**Gate:** Required for file-write operations and multi-file changes.

### Rung 3: Formal Specification
**Status:** Planned (Phase 12)

Lightweight formal specs that encode business rules as checkable properties:
- Alloy for data structure invariants
- TLA+ for concurrent protocol verification

**Gate:** Required for payment flows, authentication, and data migration logic.

### Rung 4: Full Formal Verification
**Status:** Planned (Phase 12)

Machine-checked proofs of correctness:
- Dafny for algorithmic correctness
- Z3 for constraint satisfaction

**Gate:** Required for cryptographic operations and financial calculations.

## Routing

The verification rung for a given task is determined by `check_permission()` in `crates/zylcode-core/src/router/decision.rs`. The function is pure and side-effect-free:

```rust
fn check_permission(agent_id: &str, tool_id: &str, context: &SessionContext) -> Decision
```

Task categories map to minimum rungs:

| Task Category | Minimum Rung |
|---|---|
| Read-only exploration | 1 |
| Single-file edits | 1 |
| Multi-file refactors | 2 |
| File writes | 2 |
| Payment/auth logic | 3 |
| Crypto operations | 4 |

## Status

| Rung | Implementation | Tests | Documentation |
|---|---|---|---|
| 1 | Planned | Planned | This file |
| 2 | Planned | Planned | This file |
| 3 | Planned | Planned | This file |
| 4 | Planned | Planned | This file |
