#!/usr/bin/env python3
"""Capability-registry validation gate.

WHY THIS EXISTS
---------------
`docs/capability-registry.json` is a **public claim surface**: it is the
repository's statement of what ZylCode can do. `docs/governance/ZYLCODE_CAPABILITY_MODEL.md`
section 5 makes `rung` a required field, and section 4.2 forbids granting
`GREEN` below `R3`. On 2026-11-11 the registry was found with 23 of 37 entries
missing `rung` entirely -- a required field, absent, with no gate to notice.
That is the same failure mode as the retracted-claim backlog: a rule in a
document that nothing enforces.

This script enforces the shape, not the truth. It cannot decide whether an
entry's `rung` is *earned*; that is the auditor's job. It can insist that every
entry states one, that the statement is in vocabulary, that a disputed entry
carries its dispute record, that the summary matches the entries, and that a
`GREEN` entry below `R3` is an **explicitly declared** inconsistency rather
than a silent one.

USAGE
-----
    python scripts/check_capability_registry.py
    python scripts/check_capability_registry.py --path docs/capability-registry.json

Exit codes:
    0  registry is structurally valid
    1  at least one error
    2  registry could not be read or parsed
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from pathlib import Path

STATUS_VOCABULARY = {
    "ABSENT",
    "STUB",
    "PARTIAL",
    "FUNCTIONAL",
    "GREEN",
    "EXTERNAL_BLOCKER",
    "DISPUTED",
    "REMOVED",
}

RUNG_ORDER = ["R0", "R1", "R2", "R3", "R4", "R5"]

# Fields the capability model lists as required, but that the registry does not
# yet carry on every entry. These are reported as WARN, not ERROR: completing
# them is a documentation programme, not a gate, and inventing values to
# silence a check is exactly what this repository keeps being burned by.
SOFT_FIELDS = ["entry_points", "scope", "evidence_commit", "verified_at", "limitations"]


def load(path: Path) -> dict:
    try:
        with path.open(encoding="utf-8") as fh:
            return json.load(fh)
    except OSError as exc:
        print(f"ERROR: cannot read {path}: {exc}", file=sys.stderr)
        raise SystemExit(2) from exc
    except json.JSONDecodeError as exc:
        print(f"ERROR: {path} is not valid JSON: {exc}", file=sys.stderr)
        raise SystemExit(2) from exc


def main() -> int:
    parser = argparse.ArgumentParser(description="Capability registry validation gate")
    parser.add_argument("--path", default="docs/capability-registry.json")
    parser.add_argument("--quiet", action="store_true", help="suppress the WARN section")
    args = parser.parse_args()

    data = load(Path(args.path))
    errors: list[str] = []
    warnings: list[str] = []

    caps = data.get("capabilities")
    if not isinstance(caps, dict) or not caps:
        print("ERROR: `capabilities` is missing or empty", file=sys.stderr)
        return 1

    for cap_id, entry in caps.items():
        where = f"capabilities.{cap_id}"
        if not isinstance(entry, dict):
            errors.append(f"{where}: entry is not an object")
            continue

        status = entry.get("status")
        rung = entry.get("rung")

        # Required field 1: status, in vocabulary.
        if status is None:
            errors.append(f"{where}: missing `status`")
        elif status not in STATUS_VOCABULARY:
            errors.append(
                f"{where}: status {status!r} is outside the vocabulary "
                f"({', '.join(sorted(STATUS_VOCABULARY))})"
            )

        # Required field 2: rung, in vocabulary. This is the field the
        # 2026-11-11 pass found missing on 23 entries.
        if rung is None:
            errors.append(f"{where}: missing `rung` (required by ZYLCODE_CAPABILITY_MODEL section 5)")
        elif rung not in RUNG_ORDER:
            errors.append(f"{where}: rung {rung!r} is not one of {', '.join(RUNG_ORDER)}")

        # Evidence must exist and say something.
        evidence = entry.get("evidence")
        if evidence is None:
            errors.append(f"{where}: missing `evidence`")
        elif not isinstance(evidence, str) or not evidence.strip():
            errors.append(f"{where}: `evidence` is empty")

        # A dispute must carry its record; it is history, not a label.
        if status == "DISPUTED":
            for field in ("disputed_by", "dispute_reason"):
                if not str(entry.get(field) or "").strip():
                    errors.append(f"{where}: status DISPUTED without `{field}`")

        # Section 4.2: GREEN requires R3+. A violation is only allowed when the
        # entry declares it and gives a reason, so it cannot be introduced by
        # quietly lowering a rung.
        if status == "GREEN" and rung in RUNG_ORDER and RUNG_ORDER.index(rung) < RUNG_ORDER.index("R3"):
            if not entry.get("rung_conflict"):
                errors.append(
                    f"{where}: status GREEN at rung {rung} violates ZYLCODE_CAPABILITY_MODEL section 4.2 "
                    "(GREEN requires R3+). Declare `rung_conflict: true` with a reason, or fix the status/rung."
                )
            elif not str(entry.get("rung_conflict_reason") or "").strip():
                errors.append(f"{where}: `rung_conflict` declared without `rung_conflict_reason`")

        missing_soft = [f for f in SOFT_FIELDS if not entry.get(f)]
        if missing_soft:
            warnings.append(f"{where}: no {'/'.join(missing_soft)}")

    # The summary must be computed from the entries, not maintained by hand.
    summary = data.get("summary") or {}
    actual = Counter(str(e.get("status")) for e in caps.values())
    if summary:
        if summary.get("total") != len(caps):
            errors.append(f"summary.total is {summary.get('total')}, entries are {len(caps)}")
        for status in STATUS_VOCABULARY:
            claimed = summary.get(status, 0)
            if claimed != actual.get(status, 0):
                errors.append(
                    f"summary.{status} is {claimed}, entries compute {actual.get(status, 0)}"
                )

    if errors:
        print(f"FAIL: {len(errors)} capability-registry error(s)\n")
        for err in errors:
            print(f"  {err}")
        print()
        return 1

    if warnings and not args.quiet:
        print(f"WARN: {len(warnings)} entr(ies) incomplete against ZYLCODE_CAPABILITY_MODEL section 5")
        print("      (not gated: completing these is a documentation programme, not a shape check)\n")
        for warn in warnings[:5]:
            print(f"  {warn}")
        if len(warnings) > 5:
            print(f"  ... and {len(warnings) - 5} more")
        print()

    rungs = Counter(str(e.get("rung")) for e in caps.values())
    print(
        f"OK: {len(caps)} capabilities valid - "
        + ", ".join(f"{k} {v}" for k, v in sorted(rungs.items()))
        + f"; statuses "
        + ", ".join(f"{k} {v}" for k, v in sorted(actual.items()))
        + (" (re-run without --quiet to list incomplete entries)" if warnings and args.quiet else "")
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
