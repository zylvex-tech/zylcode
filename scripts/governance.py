#!/usr/bin/env python3
"""Run every governance gate in this repository from one command.

WHY THIS EXISTS
---------------
The gates grew one incident at a time: a retracted claim came back, so
`check_retracted_claims.py`; the status board conflicted on whitespace, so
`render_status_board.py --check`; a condition was closed in prose, so
`check_conditions.py`. Each is wired into CI separately, and nothing listed them
together — so a contributor working locally had to read `ci.yml` to discover that
there were eleven of them, and the ones added later were the ones nobody ran.

This runner is the local mirror of the governance steps in `.github/workflows/ci.yml`.
It deliberately does **not** reimplement any check: it executes the same scripts
with the same arguments, and reports a table. Two implementations of a rule can
disagree, which is the failure mode the whole `docs/governance/` directory exists
to prevent.

The gates are cheap (seconds, no build) except the optional ones:

    python scripts/governance.py                 # all fast gates
    python scripts/governance.py --with-rust     # + cargo fmt/clippy/test
    python scripts/governance.py --with-node     # + prettier format check
    python scripts/governance.py --list          # what would run, nothing executed

`check_branch_protection.py` needs an admin-scoped token, so it is listed but not
run by default: on a contributor's machine it would report UNKNOWN every time,
and a gate that is usually unknown is a gate people learn to ignore. Run it with
`--with-network` when you have the token.

USAGE
-----
Exit codes: 0 all gates passed, 1 at least one failed, 2 a gate could not run.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

PY = sys.executable or "python"

# (name, argv, why). Every command is the one CI runs.
FAST_GATES: list[tuple[str, list[str], str]] = [
    (
        "retracted claims",
        [PY, "scripts/check_retracted_claims.py"],
        "a claim measured and removed must not re-enter the repo",
    ),
    (
        "capability registry",
        [PY, "scripts/check_capability_registry.py"],
        "registry shape: rung present, status in vocabulary, summary matches",
    ),
    (
        "capability registry (self-test)",
        [PY, "scripts/check_capability_registry.py", "--self-test"],
        "the shape gate's own fixtures",
    ),
    (
        "capability probes",
        [PY, "scripts/check_capability_probes.py"],
        "every must-probe capability names a real test, file or command",
    ),
    (
        "capability probes (self-test)",
        [PY, "scripts/check_capability_probes.py", "--self-test"],
        "the probe gate's own fixtures",
    ),
    (
        "status board",
        [PY, "scripts/render_status_board.py", "--check"],
        "docs/roadmap matches docs/governance/phase-status.json",
    ),
    (
        "public facts",
        [PY, "scripts/render_facts.py", "--check"],
        "README counts match the registry, probes and phase status",
    ),
    (
        "conditions register",
        [PY, "scripts/check_conditions.py"],
        "Phase 2A conditions still hold in the state the register claims",
    ),
    (
        "conditions register (self-test)",
        [PY, "scripts/check_conditions.py", "--self-test"],
        "the condition evaluator's own fixtures",
    ),
    (
        "flaky register",
        [PY, "scripts/check_flaky_register.py"],
        "every #[ignore] is a declared quarantine with a reason",
    ),
    (
        "flaky register (self-test)",
        [PY, "scripts/check_flaky_register.py", "--self-test"],
        "the quarantine gate's own fixtures",
    ),
    (
        "tool catalogue counts",
        [PY, "scripts/check_tool_catalogue_counts.py"],
        "27 definitions / 12 executors / 20 mapped agree with source and docs",
    ),
    (
        "question-set scope (C4)",
        [PY, "scripts/freeze_question_set.py", "--check-all", "docs/benchmarks"],
        "no question set may be scored and then edited",
    ),
    (
        "question-set freeze (self-test)",
        [PY, "scripts/freeze_question_set.py", "--self-test"],
        "the freeze lock's own fixtures",
    ),
    (
        "secret scan",
        [PY, "scripts/check_hardcoded_secrets.py"],
        "high-confidence provider credentials never enter the tree",
    ),
]

NETWORK_GATES: list[tuple[str, list[str], str]] = [
    (
        "branch protection",
        [PY, "scripts/check_branch_protection.py"],
        "live main protection matches docs/governance/branch-protection.json",
    )
]

RUST_GATES: list[tuple[str, list[str], str]] = [
    ("cargo fmt", ["cargo", "fmt", "--all", "--", "--check"], "formatting is enforced, not advisory"),
    (
        "cargo clippy",
        ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
        "warnings are errors",
    ),
    ("cargo test", ["cargo", "test", "--workspace", "--all-targets"], "the suite, as CI runs it"),
]

NODE_GATES: list[tuple[str, list[str], str]] = [
    ("prettier", ["pnpm", "run", "format:check"], "markdown/JSON formatting"),
]


def run(gate: tuple[str, list[str], str], root: Path) -> tuple[str, str, str]:
    """(status, name, detail)."""
    name, argv, _why = gate
    try:
        completed = subprocess.run(argv, cwd=root, capture_output=True, text=True)
    except FileNotFoundError:
        return "UNAVAILABLE", name, f"{argv[0]} is not on PATH"
    output = (completed.stdout or "") + (completed.stderr or "")
    if completed.returncode == 0:
        return "PASS", name, ""
    tail = "\n".join(output.strip().splitlines()[-12:])
    if completed.returncode == 2:
        return "UNKNOWN", name, tail
    return "FAIL", name, tail


def main() -> int:
    parser = argparse.ArgumentParser(description="Run the repository governance gates")
    parser.add_argument("--with-network", action="store_true", help="include branch-protection drift")
    parser.add_argument("--with-rust", action="store_true", help="include cargo fmt/clippy/test")
    parser.add_argument("--with-node", action="store_true", help="include prettier format:check")
    parser.add_argument("--list", action="store_true", help="print the gates without running them")
    args = parser.parse_args()

    root = Path(__file__).resolve().parent.parent
    gates = list(FAST_GATES)
    if args.with_network:
        gates += NETWORK_GATES
    if args.with_rust:
        gates += RUST_GATES
    if args.with_node:
        gates += NODE_GATES

    if args.list:
        for name, argv, why in gates:
            print(f"{name:34} {' '.join(argv)}")
            print(f"{'':34} {why}")
        return 0

    results = []
    for gate in gates:
        status, name, detail = run(gate, root)
        mark = {"PASS": "ok  ", "FAIL": "FAIL", "UNKNOWN": "??  ", "UNAVAILABLE": "--  "}[status]
        print(f"{mark} {name}")
        if detail:
            for line in detail.splitlines():
                print(f"       {line}")
        results.append((status, name))

    failed = [name for status, name in results if status == "FAIL"]
    unknown = [name for status, name in results if status in ("UNKNOWN", "UNAVAILABLE")]
    passed = len(results) - len(failed) - len(unknown)
    print(f"\n{passed} passed, {len(failed)} failed, {len(unknown)} not evaluated")
    if failed:
        print(f"FAILED: {failed}", file=sys.stderr)
        return 1
    if unknown:
        print(f"NOT EVALUATED (not a pass): {unknown}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
