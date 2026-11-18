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
carries its dispute record, that a rung has provenance, that the summary is
present and agrees with the entries, and that a `GREEN` entry below `R3` is an
**explicitly declared** inconsistency rather than a silent one.

USAGE
-----
    python scripts/check_capability_registry.py
    python scripts/check_capability_registry.py --path docs/capability-registry.json
    python scripts/check_capability_registry.py --self-test

Exit codes:
    0  registry is structurally valid (or every self-test case behaved)
    1  at least one error
    2  registry could not be read or parsed
"""

from __future__ import annotations

import argparse
import copy
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

# A rung needs a stated basis. Entries that predate the 2026-11-11 backfill
# carry their provenance in one of these instead, and are deliberately not
# required to duplicate it into `rung_basis`.
PROVENANCE_ALTERNATIVES = ("p12_rebaseline", "evidence_doc")


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


def validate(data: dict) -> tuple[list[str], list[str]]:
    """Return (errors, warnings) for a parsed registry. Pure: no I/O, no exit."""
    errors: list[str] = []
    warnings: list[str] = []

    caps = data.get("capabilities")
    if not isinstance(caps, dict) or not caps:
        errors.append("`capabilities` is missing, empty, or not an object keyed by capability id")
        return errors, warnings

    for cap_id, entry in caps.items():
        where = f"capabilities.{cap_id}"

        # Never touch fields before the type is known: an entry that is a list
        # or a string used to reach the summary stage and raise AttributeError,
        # turning a validation failure into a traceback.
        if not isinstance(entry, dict):
            errors.append(f"{where}: entry must be an object, found {type(entry).__name__}")
            continue

        status = entry.get("status")
        rung = entry.get("rung")

        if status is None:
            errors.append(f"{where}: missing `status`")
        elif not isinstance(status, str) or status not in STATUS_VOCABULARY:
            errors.append(
                f"{where}: status {status!r} is outside the vocabulary "
                f"({', '.join(sorted(STATUS_VOCABULARY))})"
            )

        if rung is None:
            errors.append(f"{where}: missing `rung` (required by ZYLCODE_CAPABILITY_MODEL section 5)")
        elif not isinstance(rung, str) or rung not in RUNG_ORDER:
            errors.append(f"{where}: rung {rung!r} is not one of {', '.join(RUNG_ORDER)}")

        # Provenance for the rung. Without it, a backfilled rung is a bare
        # assertion with nothing saying where it came from.
        basis = entry.get("rung_basis")
        if not any(entry.get(field) for field in PROVENANCE_ALTERNATIVES):
            if basis is None:
                errors.append(
                    f"{where}: missing `rung_basis` (no `p12_rebaseline`/`evidence_doc` provenance either)"
                )
            elif not isinstance(basis, str) or not basis.strip():
                errors.append(f"{where}: `rung_basis` is empty")

        evidence = entry.get("evidence")
        if evidence is None:
            errors.append(f"{where}: missing `evidence`")
        elif not isinstance(evidence, str) or not evidence.strip():
            errors.append(f"{where}: `evidence` is empty")

        if status == "DISPUTED":
            for field in ("disputed_by", "dispute_reason"):
                if not str(entry.get(field) or "").strip():
                    errors.append(f"{where}: status DISPUTED without `{field}`")

        # Section 4.2: GREEN requires R3+. A violation is only allowed when the
        # entry declares it and gives a reason, so it cannot be introduced by
        # quietly lowering a rung.
        if (
            status == "GREEN"
            and isinstance(rung, str)
            and rung in RUNG_ORDER
            and RUNG_ORDER.index(rung) < RUNG_ORDER.index("R3")
        ):
            if not entry.get("rung_conflict"):
                errors.append(
                    f"{where}: status GREEN at rung {rung} violates ZYLCODE_CAPABILITY_MODEL section 4.2 "
                    "(GREEN requires R3+). Declare `rung_conflict: true` with a reason, or fix the status/rung."
                )
            elif not str(entry.get("rung_conflict_reason") or "").strip():
                errors.append(f"{where}: `rung_conflict` declared without `rung_conflict_reason`")

        # The nested rebaseline block carries its own rung; an out-of-vocabulary
        # value there is just as wrong as one at the top level.
        rebaseline = entry.get("p12_rebaseline")
        if rebaseline is not None:
            if not isinstance(rebaseline, dict):
                errors.append(f"{where}.p12_rebaseline: must be an object")
            else:
                nested = rebaseline.get("rung")
                if nested is not None and (not isinstance(nested, str) or nested not in RUNG_ORDER):
                    errors.append(
                        f"{where}.p12_rebaseline.rung: {nested!r} is not one of {', '.join(RUNG_ORDER)}"
                    )

        missing_soft = [f for f in SOFT_FIELDS if not entry.get(f)]
        if missing_soft:
            warnings.append(f"{where}: no {'/'.join(missing_soft)}")

    # The summary must exist and be computed from the entries. Deleting it used
    # to pass, which made every count in the public registry unverified.
    summary = data.get("summary")
    if not isinstance(summary, dict):
        errors.append("`summary` is missing or not an object; its counts cannot be verified")
    else:
        if summary.get("total") != len(caps):
            errors.append(f"summary.total is {summary.get('total')}, entries are {len(caps)}")

        actual = Counter(e.get("status") for e in caps.values() if isinstance(e, dict))
        claimed_statuses = {k for k in summary if k in STATUS_VOCABULARY}
        observed_statuses = {s for s in actual if isinstance(s, str)}
        for status in sorted(claimed_statuses | observed_statuses):
            have = actual.get(status, 0)
            if status not in summary:
                errors.append(f"summary has no count for {status}, but entries compute {have}")
            elif summary[status] != have:
                errors.append(f"summary.{status} is {summary[status]}, entries compute {have}")

    return errors, warnings


def _self_test(base_path: Path) -> int:
    """Regression fixtures for every rule, including the four gaps closed on 2026-11-11."""
    base = load(base_path)
    caps = base.get("capabilities")
    if not isinstance(caps, dict) or not caps:
        print("ERROR: self-test needs a registry with `capabilities`", file=sys.stderr)
        return 2

    sample_id = next(iter(caps))
    if not isinstance(caps[sample_id], dict):
        print("ERROR: self-test sample entry is not an object", file=sys.stderr)
        return 2

    def mutated(fn) -> dict:
        data = copy.deepcopy(base)
        fn(data)
        return data

    def set_entry(field, value):
        def apply(data):
            data["capabilities"][sample_id][field] = value

        return apply

    dispossessed = next(
        k
        for k, v in caps.items()
        if isinstance(v, dict) and not any(v.get(f) for f in PROVENANCE_ALTERNATIVES)
    )

    cases = [
        ("untouched registry", base, False),
        ("missing rung", mutated(lambda d: d["capabilities"][sample_id].pop("rung")), True),
        ("rung R9", mutated(set_entry("rung", "R9")), True),
        ("status GREEEN", mutated(set_entry("status", "GREEEN")), True),
        (
            "GREEN at R2 with no declared conflict",
            mutated(lambda d: (d["capabilities"][sample_id].__setitem__("rung", "R2"),
                               d["capabilities"][sample_id].pop("rung_conflict", None))),
            True,
        ),
        (
            "GREEN at R2 declaring a conflict without a reason",
            mutated(lambda d: (d["capabilities"][sample_id].__setitem__("rung", "R2"),
                               d["capabilities"][sample_id].__setitem__("rung_conflict", True),
                               d["capabilities"][sample_id].pop("rung_conflict_reason", None))),
            True,
        ),
        ("blank evidence", mutated(set_entry("evidence", "   ")), True),
        (
            "missing rung_basis on an entry with no other provenance",
            mutated(lambda d: d["capabilities"][dispossessed].pop("rung_basis")),
            True,
        ),
        (
            "blank rung_basis on an entry with no other provenance",
            mutated(lambda d: d["capabilities"][dispossessed].__setitem__("rung_basis", "  ")),
            True,
        ),
        ("non-object entry", mutated(lambda d: d["capabilities"].__setitem__("ghost_cap", "GREEN")), True),
        (
            "nested p12_rebaseline.rung R9",
            mutated(lambda d: d["capabilities"][sample_id].__setitem__(
                "p12_rebaseline", {"date": "2026-11-11", "rung": "R9"})),
            True,
        ),
        ("summary removed", mutated(lambda d: d.pop("summary")), True),
        ("summary total drift", mutated(lambda d: d["summary"].__setitem__("total", 36)), True),
        (
            "summary count drift",
            mutated(lambda d: d["summary"].__setitem__(
                "GREEN", d["summary"].get("GREEN", 0) + 1)),
            True,
        ),
        (
            "capabilities not an object",
            mutated(lambda d: d.__setitem__("capabilities", [])),
            True,
        ),
    ]

    failures = 0
    for label, data, expect_error in cases:
        errors, _ = validate(data)
        ok = bool(errors) == expect_error
        failures += 0 if ok else 1
        print(f"  {'PASS' if ok else 'FAIL'}  {label}: {'rejected' if errors else 'accepted'}")

    # Diagnostics, not just exit codes: a non-object entry must say so.
    errors, _ = validate(mutated(lambda d: d["capabilities"].__setitem__("ghost_cap", "GREEN")))
    if not any("must be an object" in err for err in errors):
        print("  FAIL  non-object entry produced no clear diagnostic")
        failures += 1
    else:
        print("  PASS  non-object entry produces a clear diagnostic")

    total = len(cases) + 1
    print(f"self-test: {total} checks, {failures} failure(s)")
    return 1 if failures else 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Capability registry validation gate")
    parser.add_argument("--path", default="docs/capability-registry.json")
    parser.add_argument("--quiet", action="store_true", help="suppress the WARN section")
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="run the regression fixtures for every rule instead of validating one file",
    )
    args = parser.parse_args()
    path = Path(args.path)

    if args.self_test:
        return _self_test(path)

    data = load(path)
    errors, warnings = validate(data)

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

    caps = data["capabilities"]
    rungs = Counter(str(e.get("rung")) for e in caps.values() if isinstance(e, dict))
    actual = Counter(str(e.get("status")) for e in caps.values() if isinstance(e, dict))
    print(
        f"OK: {len(caps)} capabilities valid - "
        + ", ".join(f"{k} {v}" for k, v in sorted(rungs.items()))
        + "; statuses "
        + ", ".join(f"{k} {v}" for k, v in sorted(actual.items()))
        + (" (re-run without --quiet to list incomplete entries)" if warnings and args.quiet else "")
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
