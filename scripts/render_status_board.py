#!/usr/bin/env python3
"""Render the roadmap status board from one machine-readable source.

WHY THIS EXISTS
---------------
The status table in `docs/roadmap/ZYLCODE_ROADMAP_V2.md` was hand-edited, and
it showed: three pull requests in one day conflicted on nothing but its column
widths, and branches cut before a verdict kept shipping a stale "2A RE-OPENED"
section that contradicted `main`. A document that two people (or three agents)
must edit to say what is true will eventually say two things at once.

`docs/governance/phase-status.json` is now the source of truth. This script
renders the table between two markers in the roadmap, and `--check` fails when
the committed table differs from what the source file renders — so drift is a
red build, and a merge never has to resolve a percentage-sign by hand.

USAGE
-----
    python scripts/render_status_board.py            # rewrite the board in place
    python scripts/render_status_board.py --check     # exit 1 if it is stale
    python scripts/render_status_board.py --self-test

Exit codes: 0 ok, 1 drift/invalid, 2 unreadable input.
"""

from __future__ import annotations

import argparse
import copy
import json
import sys
from pathlib import Path

BEGIN = "<!-- BEGIN PHASE STATUS BOARD -->"
END = "<!-- END PHASE STATUS BOARD -->"
GENERATED_NOTE = (
    "<!-- Generated from docs/governance/phase-status.json by "
    "scripts/render_status_board.py. Edit the JSON, not this table. -->"
)

STATES = {
    "ACCEPTED": "✅ ACCEPTED",
    "ACCEPTED_WITH_CONDITIONS": "✅ ACCEPTED WITH CONDITIONS",
    "PARTIAL": "⏸ PARTIAL",
    "READY": "⏸ GATE CLEARED",
    "NOT_STARTED": "⏸ NOT STARTED",
    "BLOCKED": "🚫 BLOCKED",
}
REQUIRED = ("id", "title", "state", "rung", "blocks")


def load_json(path: Path) -> dict:
    try:
        with path.open(encoding="utf-8") as fh:
            return json.load(fh)
    except OSError as exc:
        print(f"ERROR: cannot read {path}: {exc}", file=sys.stderr)
        raise SystemExit(2) from exc
    except json.JSONDecodeError as exc:
        print(f"ERROR: {path} is not valid JSON: {exc}", file=sys.stderr)
        raise SystemExit(2) from exc


def validate(data: dict) -> list[str]:
    errors: list[str] = []
    phases = data.get("phases")
    if not isinstance(phases, list) or not phases:
        return ["`phases` is missing, empty, or not a list"]
    ids: set[str] = set()
    for index, phase in enumerate(phases):
        where = f"phases[{index}]"
        if not isinstance(phase, dict):
            errors.append(f"{where}: phase must be an object")
            continue
        missing = [f for f in REQUIRED if not str(phase.get(f) or "").strip()]
        if missing:
            errors.append(f"{where}: missing {', '.join(missing)}")
            continue
        if phase["state"] not in STATES:
            errors.append(
                f"{where}: state {phase['state']!r} is not one of {', '.join(sorted(STATES))}"
            )
        if phase["id"] in ids:
            errors.append(f"{where}: duplicate phase id {phase['id']}")
        ids.add(phase["id"])
        blocks = {b.strip() for b in str(phase["blocks"]).split(",") if b.strip()}
        for target in blocks - {"—"}:
            if target not in {str(p.get("id")) for p in phases if isinstance(p, dict)}:
                errors.append(f"{where}: blocks unknown phase {target}")
    return errors


def _cell(phase: dict) -> str:
    return f"**{phase['title']}**" if phase.get("emphasis") else phase["title"]


def render(data: dict) -> str:
    """Render the marker-free table. Pure: same input, same bytes."""
    rows = []
    for phase in data["phases"]:
        label = f"**{phase['id']}**" if phase.get("emphasis") else phase["id"]
        status = STATES[phase["state"]]
        detail = str(phase.get("detail") or "").strip()
        if detail:
            if phase["state"] == "PARTIAL":
                status = f"{status} — {detail}"
            elif phase["state"] == "READY":
                status = f"{status} — {detail}"
            else:
                status = f"{status} ({detail})"
        if phase.get("emphasis"):
            status = f"**{status}**"
        rung = str(phase["rung"])
        if phase.get("emphasis"):
            rung = f"**{rung}**"
        blocks = str(phase["blocks"])
        if phase.get("emphasis"):
            blocks = f"**{blocks}**"
        rows.append((label, _cell(phase), status, rung, blocks))

    headers = ("Phase", "Title", "Status", "Rung", "Blocks")
    widths = [
        max(len(headers[0]), *(len(r[0]) for r in rows)),
        max(len(headers[1]), *(len(r[1]) for r in rows)),
        max(len(headers[2]), *(len(r[2]) for r in rows)),
        max(len(headers[3]), *(len(r[3]) for r in rows)),
        max(len(headers[4]), *(len(r[4]) for r in rows)),
    ]

    def line(cells) -> str:
        return "| " + " | ".join(c.ljust(w) for c, w in zip(cells, widths)) + " |"

    out = [line(headers), "| " + " | ".join("-" * w for w in widths) + " |"]
    out.extend(line(r) for r in rows)
    return "\n".join(out)


def block(data: dict) -> str:
    # `prettier-ignore` belongs to the generated block, not to the caller: the
    # repository's format gate re-pads markdown tables, which changes the bytes
    # this script renders and makes `--check` fail immediately after a format
    # run. The table is generated, so the formatter must leave it alone.
    return f"{BEGIN}\n{GENERATED_NOTE}\n<!-- prettier-ignore -->\n{render(data)}\n{END}"


def _normalized(text: str) -> str:
    """Compare content, not padding.
   
alignment: the repository's markdown formatter re-pads tables, and a status
    gate that fails on column widths would be a whitespace argument, not a
    truth argument. Blank lines and runs of spaces are therefore ignored; every
    cell's text, order and emphasis still have to match exactly.
    """
    lines = []
    for line in text.splitlines():
        collapsed = " ".join(line.split())
        if collapsed:
            lines.append(collapsed)
    return "\n".join(lines)


def replace_block(document: str, new_block: str) -> str:
    start = document.find(BEGIN)
    end = document.find(END)
    if start == -1 or end == -1 or end < start:
        raise ValueError(f"status-board markers not found ({BEGIN} ... {END})")
    return document[:start] + new_block + document[end + len(END) :]


def _self_test() -> int:
    base = {
        "phases": [
            {"id": "1A", "title": "One", "state": "ACCEPTED", "rung": "R3", "blocks": "1B"},
            {"id": "1B", "title": "Two", "state": "NOT_STARTED", "rung": "—", "blocks": "—"},
        ]
    }
    failures = 0

    def expect(label: str, ok: bool) -> None:
        nonlocal failures
        failures += 0 if ok else 1
        print(f"  {'PASS' if ok else 'FAIL'}  {label}")

    expect("valid data has no errors", not validate(base))
    expect("render is deterministic", render(base) == render(copy.deepcopy(base)))
    expect("table header present", render(base).startswith("| Phase"))

    bad_state = copy.deepcopy(base)
    bad_state["phases"][0]["state"] = "PROBABLY_DONE"
    expect("unknown state rejected", any("state" in e for e in validate(bad_state)))

    missing = copy.deepcopy(base)
    missing["phases"][1].pop("blocks")
    expect("missing field rejected", any("missing" in e for e in validate(missing)))

    ghost_block = copy.deepcopy(base)
    ghost_block["phases"][0]["blocks"] = "99Z"
    expect("unknown blocked phase rejected", any("99Z" in e for e in validate(ghost_block)))

    duplicate = copy.deepcopy(base)
    duplicate["phases"][1]["id"] = "1A"
    expect("duplicate id rejected", any("duplicate" in e for e in validate(duplicate)))

    detail = copy.deepcopy(base)
    detail["phases"][0]["state"] = "PARTIAL"
    detail["phases"][0]["detail"] = "store v2"
    expect("partial detail rendered with em dash", "PARTIAL — store v2" in render(detail))

    document = f"intro\n{block(base)}\noutro\n"
    replaced = replace_block(document, block(base))
    expect("round-trip is a no-op", replaced == document)
    drift = copy.deepcopy(base)
    drift["phases"][0]["state"] = "PARTIAL"
    expect("drift detected", replace_block(document, block(drift)) != document)
    try:
        replace_block("no markers", block(base))
        expect("missing markers raise", False)
    except ValueError:
        expect("missing markers raise", True)

    print(f"self-test: {failures} failure(s)")
    return 1 if failures else 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Render the roadmap status board")
    parser.add_argument("--source", default="docs/governance/phase-status.json")
    parser.add_argument("--target", default="docs/roadmap/ZYLCODE_ROADMAP_V2.md")
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return _self_test()

    data = load_json(Path(args.source))
    errors = validate(data)
    if errors:
        print(f"FAIL: {len(errors)} phase-status error(s)\n")
        for err in errors:
            print(f"  {err}")
        return 1

    target = Path(args.target)
    try:
        document = target.read_text(encoding="utf-8")
    except OSError as exc:
        print(f"ERROR: cannot read {target}: {exc}", file=sys.stderr)
        return 2

    new_block = block(data)
    try:
        updated = replace_block(document, new_block)
    except ValueError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1

    if args.check:
        if _normalized(updated) != _normalized(document):
            print(f"FAIL: {target} status board is stale; run scripts/render_status_board.py")
            return 1
        print(f"OK: status board in {target} matches {args.source}")
        return 0

    if updated == document:
        print(f"OK: {target} already up to date")
        return 0
    target.write_text(updated, encoding="utf-8")
    print(f"OK: rewrote status board in {target}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
