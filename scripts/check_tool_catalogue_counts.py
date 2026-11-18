#!/usr/bin/env python3
"""Check the tool-catalogue counts the docs state against the source that produces them.

WHY THIS EXISTS
---------------
The registry entry `tool_system_executable` says "27 catalogued definitions / 12
real executors". The truth table says "20 real executors (12 general-purpose + 8
repository-intelligence `nav.*`)". Both can be true at once, but nothing in the
repository made that explicit: a reader comparing the two documents sees 12 in one
place and 20 in another and cannot tell whether one of them is stale.

The two numbers are different measurements:

* **definitions** — `ToolDefinition`s the bridge catalogues. They carry a schema
  and no executor. These must never be registered as executable tools.
* **executors** — ids `get_real_tool` actually binds, which is 12 general-purpose
  executors plus the 8 `nav.*` ids, hence 20.

This script derives all four counts from source and compares them with the
machine-readable marker in `docs/governance/TOOL_CATALOGUE_TRUTH_TABLE.md` and the
figures in the registry's evidence string, so a number that moves in one place
and not the others fails the gate instead of becoming a footnote discrepancy.

USAGE
-----
    python scripts/check_tool_catalogue_counts.py
    python scripts/check_tool_catalogue_counts.py --self-test

Exit codes: 0 ok, 1 a figure disagrees, 2 unreadable input.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

BRIDGE = Path("crates/zylcode-mcp/src/enhanced_bridge.rs")
REAL_TOOLS = Path("crates/zylcode-mcp/src/real_tools.rs")
NAV_TOOLS = Path("crates/zylcode-mcp/src/nav_tools.rs")
TRUTH_TABLE = Path("docs/governance/TOOL_CATALOGUE_TRUTH_TABLE.md")
REGISTRY = Path("docs/capability-registry.json")

MARKER_RE = re.compile(r"<!--\s*tool-counts:(?P<body>[^>]*)-->")
ID_RE = re.compile(r'"([a-z][a-z0-9_]*\.[a-z][a-z0-9_]*)"')
NAV_LEN_RE = re.compile(r"pub const NAV_TOOL_IDS:\s*\[&str;\s*(\d+)\]")


def count_definitions(text: str) -> int:
    """Catalogued `ToolDefinition`s: one `id: "..."` field per definition."""
    return sum(1 for line in text.splitlines() if re.match(r'^\s*id:\s*"', line))


def count_general_executors(text: str) -> int:
    """Ids bound in `get_real_tool` before the `nav.*` catch-all arm."""
    start = text.find("pub fn get_real_tool")
    if start == -1:
        raise ValueError("real_tools.rs no longer defines get_real_tool")
    body = text[start:]
    guard = body.find("id if crate::nav_tools::is_nav_tool")
    if guard == -1:
        raise ValueError("real_tools.rs no longer guards the nav.* catch-all arm")
    return len(set(ID_RE.findall(body[:guard])))


def count_nav_executors(text: str) -> int:
    match = NAV_LEN_RE.search(text)
    if not match:
        raise ValueError("nav_tools.rs no longer declares NAV_TOOL_IDS with a length")
    return int(match.group(1))


def parse_marker(text: str) -> dict[str, int]:
    match = MARKER_RE.search(text)
    if not match:
        raise ValueError(
            "the truth table has no `<!-- tool-counts: ... -->` marker; a count with no "
            "declared source is what this gate exists to prevent"
        )
    values: dict[str, int] = {}
    for pair in match.group("body").split():
        key, _, value = pair.partition("=")
        values[key] = int(value)
    return values


def compare(root: Path) -> list[str]:
    """Every disagreement, as human-readable lines. Empty list means consistent."""
    definitions = count_definitions((root / BRIDGE).read_text(encoding="utf-8"))
    general = count_general_executors((root / REAL_TOOLS).read_text(encoding="utf-8"))
    nav = count_nav_executors((root / NAV_TOOLS).read_text(encoding="utf-8"))
    total = general + nav

    observed = {
        "definitions": definitions,
        "general_executors": general,
        "nav_executors": nav,
        "total_executors": total,
    }
    declared = parse_marker((root / TRUTH_TABLE).read_text(encoding="utf-8"))

    problems: list[str] = []
    for key, value in observed.items():
        if key not in declared:
            problems.append(f"truth-table marker does not declare `{key}` (source says {value})")
        elif declared[key] != value:
            problems.append(
                f"truth-table marker says {key}={declared[key]}, source says {value}"
            )

    registry = json.loads((root / REGISTRY).read_text(encoding="utf-8"))
    evidence = str(
        ((registry.get("capabilities") or {}).get("tool_system_executable") or {}).get(
            "evidence", ""
        )
    )
    if f"{definitions} catalogued definitions" not in evidence:
        problems.append(
            f"registry evidence must state `{definitions} catalogued definitions`; it says: {evidence!r}"
        )
    if f"{general} real executors" not in evidence:
        problems.append(
            f"registry evidence must state `{general} real executors`; it says: {evidence!r}"
        )
    if f"{total} mapped" not in evidence:
        problems.append(
            f"registry evidence must state `{total} mapped` (general + nav) so 12 and 20 cannot be read as a contradiction"
        )
    return problems


def _self_test() -> int:
    failures = 0

    def expect(label: str, ok: bool) -> None:
        nonlocal failures
        failures += 0 if ok else 1
        print(f"  {'PASS' if ok else 'FAIL'}  {label}")

    bridge = '\n'.join(['id: "a.b".to_string(),', '  id: "c.d".to_string(),', "// id: not a field"] * 1)
    expect("definitions count id fields only", count_definitions(bridge) == 2)

    real = (
        "pub fn get_real_tool(tool_id: &str) -> Option<Box<dyn RealTool>> {\n"
        "    match tool_id {\n"
        '        "fs.read" | "fs.write" => x,\n'
        '        "git.status" => y,\n'
        "        id if crate::nav_tools::is_nav_tool(id) => z,\n"
        "        _ => None,\n"
        "    }\n"
        "}\n"
    )
    expect("general executors stop at the nav guard", count_general_executors(real) == 3)
    expect(
        "nav length is read from the const",
        count_nav_executors("pub const NAV_TOOL_IDS: [&str; 8] = [") == 8,
    )
    expect(
        "marker parses",
        parse_marker("<!-- tool-counts: definitions=27 total_executors=20 -->")
        == {"definitions": 27, "total_executors": 20},
    )
    try:
        parse_marker("no marker")
        expect("missing marker raises", False)
    except ValueError:
        expect("missing marker raises", True)

    print(f"self-test: {failures} failure(s)")
    return 1 if failures else 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Check tool-catalogue counts")
    parser.add_argument("--root", default=".")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return _self_test()

    try:
        problems = compare(Path(args.root))
    except (OSError, ValueError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2

    if problems:
        for problem in problems:
            print(f"FAIL: {problem}", file=sys.stderr)
        return 1

    print(
        "OK: tool-catalogue counts agree — the truth-table marker and the registry "
        "evidence both state the numbers derived from enhanced_bridge.rs, "
        "real_tools.rs and nav_tools.rs"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
