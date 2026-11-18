#!/usr/bin/env python3
"""Render the public facts block from files that already know the numbers.

WHY THIS EXISTS
---------------
Every count in a hand-written document is a copy of a fact that lives somewhere
else, and copies drift. This repository has paid for that repeatedly:

- `README.md` said probe coverage was "5 of 28" after it was 6 of 29, because a
  merged capability added a probe and nothing regenerated the sentence;
- `README.md` pinned `main` at a commit that stopped being `main` two merges
  later;
- the roadmap status table was hand-maintained until three PRs in one day
  conflicted only on its column widths (fixed by `render_status_board.py`).

`phase-status.json` and `capability-registry.json` are already the source of
truth for their own claims, and `docs/capability-probes.json` for probes. This
script renders the numbers derived from them into `README.md` between markers,
and `--check` fails when the committed text differs — so a stale count is a red
build instead of a wrong sentence.

WHAT IS GENERATED, AND WHAT IS NOT
----------------------------------
Generated (computed, therefore unarguable): capability totals, probe coverage,
phase counts. Declared (human, therefore dated): the status date, which comes
from `phase-status.json`'s `updated` field. The script never invents a date and
never reads the wall clock — a generator that says "today" produces a different
document every day and a drift check that always fails.

USAGE
-----
    python scripts/render_facts.py            # rewrite the block in README.md
    python scripts/render_facts.py --check     # exit 1 if it is stale
    python scripts/render_facts.py --self-test

Exit codes: 0 ok, 1 drift/invalid, 2 unreadable input.
"""

from __future__ import annotations

import argparse
import copy
import json
import sys
from pathlib import Path

BEGIN = "<!-- BEGIN PUBLIC FACTS -->"
END = "<!-- END PUBLIC FACTS -->"
GENERATED_NOTE = (
    "<!-- Generated from docs/governance/phase-status.json, "
    "docs/capability-registry.json and docs/capability-probes.json by "
    "scripts/render_facts.py. Edit those files, not this block. -->"
)


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


def probe_coverage(probes: dict, registry: dict) -> tuple[int, int, int]:
    """(probed, green_r3_plus, total_probes).

    Deliberately the same rule as `check_capability_probes.coverage()`: a
    capability counts as probed only when it is GREEN at R3 or above *and* has at
    least one probe. Two implementations of one rule can disagree, so the number
    the gate prints and the number the README states come from the same inputs
    and the same definition; the self-test pins the definition.
    """
    caps = registry.get("capabilities") or {}
    entries = probes.get("probes") or {}
    green_r3 = [
        cid
        for cid, entry in caps.items()
        if isinstance(entry, dict)
        and entry.get("status") == "GREEN"
        and entry.get("rung") in ("R3", "R4", "R5")
    ]
    probed = [cid for cid in green_r3 if entries.get(cid)]
    total_probes = sum(len(v) for v in entries.values() if isinstance(v, list))
    return (len(probed), len(green_r3), total_probes)


def facts(phases: dict, registry: dict, probes: dict) -> dict:
    """Every value the block states, as data. Pure."""
    summary = registry.get("summary") or {}
    total = summary.get("total") or len(registry.get("capabilities") or {})
    green = summary.get("GREEN", 0)
    partial = summary.get("PARTIAL", 0)
    disputed = summary.get("DISPUTED", 0)
    probed, green_r3, total_probes = probe_coverage(probes, registry)

    state_counts: dict[str, int] = {}
    for phase in phases.get("phases") or []:
        state = str(phase.get("state"))
        state_counts[state] = state_counts.get(state, 0) + 1

    return {
        "updated": str(phases.get("updated") or "undated"),
        "total": total,
        "green": green,
        "partial": partial,
        "disputed": disputed,
        "probed": probed,
        "green_r3": green_r3,
        "total_probes": total_probes,
        "phases_total": len(phases.get("phases") or []),
        "phases_accepted": state_counts.get("ACCEPTED", 0)
        + state_counts.get("ACCEPTED_WITH_CONDITIONS", 0),
    }


def render(values: dict) -> str:
    """Render the marker-free block. Deterministic: same input, same bytes."""
    return "\n".join(
        [
            f"**Project status: {values['updated']}** · pre-release",
            "",
            f"- **Capabilities:** {values['green']} GREEN / {values['partial']} PARTIAL "
            f"/ {values['disputed']} DISPUTED of {values['total']} "
            f"(`docs/capability-registry.json`)",
            f"- **Probe coverage:** {values['probed']} of {values['green_r3']} GREEN R3+ "
            f"capabilities have an executable probe ({values['total_probes']} probes); "
            f"the gap is printed on every gate run rather than hidden",
            f"- **Phases:** {values['phases_accepted']} of {values['phases_total']} accepted "
            f"or accepted-with-conditions (`docs/governance/phase-status.json`)",
        ]
    )


def block(values: dict) -> str:
    # `prettier-ignore` belongs to the generated block: the repository's format
    # gate re-pads markdown lists, which would change these bytes and make
    # `--check` fail immediately after a format run.
    return f"{BEGIN}\n{GENERATED_NOTE}\n<!-- prettier-ignore -->\n{render(values)}\n{END}"


def _normalized(text: str) -> str:
    """Compare content, not whitespace.

    A truth gate that fails on a reflowed line is a formatting argument wearing
    a truth argument's clothes. Blank lines and runs of spaces are ignored; every
    word, number and list item still has to match exactly.
    """
    lines = [" ".join(line.split()) for line in text.splitlines()]
    return "\n".join(line for line in lines if line)


def replace_block(document: str, new_block: str) -> str:
    start = document.find(BEGIN)
    end = document.find(END)
    if start == -1 or end == -1 or end < start:
        raise ValueError(f"facts markers not found ({BEGIN} ... {END})")
    return document[:start] + new_block + document[end + len(END) :]


def _self_test() -> int:
    registry = {
        "capabilities": {
            "a": {"status": "GREEN", "rung": "R3"},
            "b": {"status": "GREEN", "rung": "R2"},
            "c": {"status": "DISPUTED", "rung": "R2"},
        },
        "summary": {"total": 3, "GREEN": 2, "PARTIAL": 0, "DISPUTED": 1},
    }
    probes = {
        "probes": {
            "a": [{"kind": "file", "path": "Cargo.toml"}],
            "b": [{"kind": "file", "path": "Cargo.toml"}],
        }
    }
    phases = {
        "updated": "2026-11-17",
        "phases": [
            {"id": "1A", "state": "ACCEPTED"},
            {"id": "2A", "state": "ACCEPTED_WITH_CONDITIONS"},
            {"id": "2B", "state": "READY"},
        ],
    }
    failures = 0

    def expect(label: str, ok: bool) -> None:
        nonlocal failures
        failures += 0 if ok else 1
        print(f"  {'PASS' if ok else 'FAIL'}  {label}")

    values = facts(phases, registry, probes)
    expect("green R3 count ignores a probed GREEN R2 entry", values["green_r3"] == 1)
    expect("probed count is 1 (b is not R3+)", values["probed"] == 1)
    expect("probe total counts every probe", values["total_probes"] == 2)
    expect(
        "accepted counts ACCEPTED_WITH_CONDITIONS as accepted",
        values["phases_accepted"] == 2,
    )
    expect("render is deterministic", render(values) == render(copy.deepcopy(values)))
    expect("state date comes from the source file", values["updated"] == "2026-11-17")

    document = f"intro\n{block(values)}\noutro\n"
    expect("round-trip is a no-op", replace_block(document, block(values)) == document)

    drifted = copy.deepcopy(phases)
    drifted["updated"] = "2026-12-01"
    expect(
        "drift detected",
        replace_block(document, block(facts(drifted, registry, probes))) != document,
    )
    expect(
        "whitespace-only difference is not drift",
        _normalized(block(values).replace(" ", "  ")) == _normalized(block(values)),
    )
    try:
        replace_block("no markers", block(values))
        expect("missing markers raise", False)
    except ValueError:
        expect("missing markers raise", True)

    print(f"self-test: {failures} failure(s)")
    return 1 if failures else 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Render the public facts block")
    parser.add_argument("--phases", default="docs/governance/phase-status.json")
    parser.add_argument("--registry", default="docs/capability-registry.json")
    parser.add_argument("--probes", default="docs/capability-probes.json")
    parser.add_argument("--target", default="README.md")
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return _self_test()

    phases = load_json(Path(args.phases))
    registry = load_json(Path(args.registry))
    probes = load_json(Path(args.probes))
    if not phases.get("phases"):
        print(f"FAIL: {args.phases} has no phases", file=sys.stderr)
        return 1
    if not str(phases.get("updated") or "").strip():
        print(
            f"FAIL: {args.phases} has no `updated` date. The generated block states a "
            "date, and inventing one would make the document wrong by default.",
            file=sys.stderr,
        )
        return 1

    new_block = block(facts(phases, registry, probes))
    target = Path(args.target)
    try:
        document = target.read_text(encoding="utf-8")
    except OSError as exc:
        print(f"ERROR: cannot read {target}: {exc}", file=sys.stderr)
        return 2

    try:
        updated = replace_block(document, new_block)
    except ValueError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1

    if args.check:
        if _normalized(updated) != _normalized(document):
            print(f"FAIL: {target} public facts are stale; run scripts/render_facts.py")
            return 1
        print(f"OK: public facts in {target} match the registry, probes and phase status")
        return 0

    if updated == document:
        print(f"OK: {target} already up to date")
        return 0
    target.write_text(updated, encoding="utf-8")
    print(f"OK: rewrote the public facts block in {target}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
