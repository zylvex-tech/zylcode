#!/usr/bin/env python3
"""Evaluate the Phase 2A audit-condition register against the repository.

WHY THIS EXISTS
---------------
The 2026-11-16 verdict accepted Phase 2A *with six conditions*. Six prose
paragraphs in a report are a memory, not a gate: C1 was closed by a later PR and
nothing in the repository stopped the next edit from quietly re-opening it, and
C2 — "these two capabilities must remain R2/DISPUTED" — had no enforcement at
all beyond a sentence in the registry's `note` field.

`docs/governance/phase2a-conditions.json` records each condition with the
predicate that keeps its stated status true. This script evaluates those
predicates:

* a **CLOSED** condition whose predicate no longer holds fails the gate, so a
  closure cannot rot into a claim;
* an **OPEN** condition that names a `satisfied_by` artefact must point at a real
  file, so "satisfied" is never a word without a referent;
* an **ACCEPTED_AS_EXCEPTION** condition must still have the limitation it
  accepted visible in the code it describes.

What this cannot do is judge whether a rung was *earned*. The checks assert the
recorded state, which is exactly what a register can assert.

USAGE
-----
    python scripts/check_conditions.py
    python scripts/check_conditions.py --json        # machine-readable summary
    python scripts/check_conditions.py --self-test

Exit codes: 0 ok, 1 a condition's predicate does not hold, 2 unreadable input.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

REGISTER = Path("docs/governance/phase2a-conditions.json")
REGISTRY = Path("docs/capability-registry.json")

VALID_STATUSES = {"OPEN", "CLOSED", "ACCEPTED_AS_EXCEPTION"}

# A condition that claims closure must say when and on what evidence. These are
# required so a status field cannot be edited alone.
CLOSED_REQUIRES = ("closed", "evidence")


def load_json(path: Path) -> dict:
    with path.open(encoding="utf-8") as fh:
        return json.load(fh)


def evaluate_check(check: dict, registry: dict, root: Path) -> tuple[bool, str]:
    """Run one predicate. Returns (held, detail)."""
    kind = check.get("kind")

    if kind == "capability_state":
        capability = check["capability"]
        entry = (registry.get("capabilities") or {}).get(capability)
        if entry is None:
            return False, f"{capability} is not in the registry"
        expected = {k: check[k] for k in ("status", "rung") if k in check}
        wrong = {
            key: (entry.get(key), want)
            for key, want in expected.items()
            if entry.get(key) != want
        }
        if wrong:
            detail = ", ".join(f"{k} is {got!r}, condition requires {want!r}" for k, (got, want) in wrong.items())
            return False, f"{capability}: {detail}"
        return True, f"{capability} is {entry.get('status')}/{entry.get('rung')} as required"

    if kind == "capability_not":
        capability = check["capability"]
        entry = (registry.get("capabilities") or {}).get(capability)
        if entry is None:
            return False, f"{capability} is not in the registry"
        if entry.get("status") == check["status"] and check["status"] == "GREEN":
            # A GREEN entry below R3 is a model violation (capability model 4.2),
            # which is what C3 was raised about.
            return False, f"{capability} is GREEN again (rung {entry.get('rung')}); C3 required it not to be"
        return True, f"{capability} is {entry.get('status')}/{entry.get('rung')}, not GREEN"

    if kind == "file_contains":
        path = root / check["path"]
        if not path.is_file():
            return False, f"{check['path']} does not exist"
        text = path.read_text(encoding="utf-8", errors="replace")
        missing = [needle for needle in check["needles"] if needle not in text]
        if missing:
            return False, f"{check['path']} no longer states: {missing}"
        return True, f"{check['path']} still states {check['needles']}"

    if kind == "frozen_sets":
        directory = root / check["directory"]
        if not directory.is_dir():
            return True, f"{check['directory']} does not exist yet (no set to freeze)"
        sets = sorted(
            p
            for p in directory.glob("*.json")
            if not p.name.endswith(".lock.json")
        )
        unlocked = [p.name for p in sets if not p.with_name(p.name + ".lock.json").is_file()]
        if unlocked:
            return False, f"question set(s) without a freeze lock: {unlocked}"
        return True, f"{len(sets)} question set(s) under {check['directory']}, all frozen"

    return False, f"unknown check kind {kind!r}"


def evaluate(register: dict, registry: dict, root: Path) -> list[dict]:
    """Evaluate every condition. Pure enough to test with synthetic inputs."""
    results: list[dict] = []
    seen: set[str] = set()
    for condition in register.get("conditions") or []:
        cid = str(condition.get("id"))
        problems: list[str] = []
        if cid in seen:
            problems.append(f"duplicate condition id {cid}")
        seen.add(cid)

        status = condition.get("status")
        if status not in VALID_STATUSES:
            problems.append(f"status {status!r} is not one of {sorted(VALID_STATUSES)}")
        if status == "CLOSED":
            for field in CLOSED_REQUIRES:
                if not str(condition.get(field) or "").strip():
                    problems.append(f"CLOSED without `{field}`")

        satisfied_by = condition.get("satisfied_by")
        if satisfied_by:
            if status == "OPEN":
                problems.append("an OPEN condition cannot name satisfied_by")
            elif not (root / satisfied_by).exists():
                problems.append(f"satisfied_by names {satisfied_by}, which does not exist")
        if status == "CLOSED" and condition.get("checks") == []:
            problems.append("CLOSED with no predicate: nothing would notice it re-opening")

        details: list[str] = []
        for check in condition.get("checks") or []:
            held, detail = evaluate_check(check, registry, root)
            details.append(detail)
            if not held:
                problems.append(detail)

        results.append(
            {
                "id": cid,
                "title": condition.get("title", ""),
                "status": status,
                "ok": not problems,
                "problems": problems,
                "details": details,
            }
        )
    return results


def _self_test() -> int:
    import tempfile

    failures = 0

    def expect(label: str, ok: bool) -> None:
        nonlocal failures
        failures += 0 if ok else 1
        print(f"  {'PASS' if ok else 'FAIL'}  {label}")

    registry = {
        "capabilities": {
            "bench": {"status": "DISPUTED", "rung": "R2"},
            "loop": {"status": "PARTIAL", "rung": "R2"},
        }
    }
    good = {
        "conditions": [
            {
                "id": "C2",
                "status": "CLOSED",
                "closed": "2026-11-18",
                "evidence": "invariant",
                "checks": [
                    {
                        "kind": "capability_state",
                        "capability": "bench",
                        "status": "DISPUTED",
                        "rung": "R2",
                    }
                ],
            },
            {
                "id": "C3",
                "status": "CLOSED",
                "closed": "2026-11-17",
                "evidence": "downgraded",
                "checks": [{"kind": "capability_not", "capability": "loop", "status": "GREEN"}],
            },
            {
                "id": "C4",
                "status": "OPEN",
                "satisfied_by": None,
                "checks": [{"kind": "frozen_sets", "directory": "docs/benchmarks"}],
            },
        ]
    }

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        results = evaluate(good, registry, root)
        expect("a satisfied register passes", all(r["ok"] for r in results))

        promoted = {"capabilities": {"bench": {"status": "GREEN", "rung": "R3"}, "loop": {"status": "PARTIAL", "rung": "R2"}}}
        results = evaluate(good, promoted, root)
        c2 = next(r for r in results if r["id"] == "C2")
        expect("promoting the C2 entry fails its closure predicate", not c2["ok"])

        regressed = {"capabilities": {"bench": {"status": "DISPUTED", "rung": "R2"}, "loop": {"status": "GREEN", "rung": "R3"}}}
        results = evaluate(good, regressed, root)
        c3 = next(r for r in results if r["id"] == "C3")
        expect("a GREEN/R2 conflict fails C3", not c3["ok"])

        (root / "docs/benchmarks").mkdir(parents=True)
        (root / "docs/benchmarks/set.json").write_text("[]", encoding="utf-8")
        results = evaluate(good, registry, root)
        c4 = next(r for r in results if r["id"] == "C4")
        expect("an unfrozen question set fails C4's scope", not c4["ok"])
        (root / "docs/benchmarks/set.json.lock.json").write_text("{}", encoding="utf-8")
        results = evaluate(good, registry, root)
        c4 = next(r for r in results if r["id"] == "C4")
        expect("a frozen question set satisfies C4's scope", c4["ok"])

        open_with_claim = {
            "conditions": [
                {
                    "id": "C4",
                    "status": "OPEN",
                    "satisfied_by": "docs/benchmarks/set.json",
                    "checks": [],
                }
            ]
        }
        results = evaluate(open_with_claim, registry, root)
        expect("an OPEN condition cannot claim satisfaction", not results[0]["ok"])

        undocumented = {
            "conditions": [{"id": "C2", "status": "CLOSED", "checks": []}]
        }
        results = evaluate(undocumented, registry, root)
        expect(
            "a closure without date, evidence or predicate is refused",
            len(results[0]["problems"]) >= 3,
        )

    print(f"self-test: {failures} failure(s)")
    return 1 if failures else 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Check the Phase 2A condition register")
    parser.add_argument("--register", default=str(REGISTER))
    parser.add_argument("--registry", default=str(REGISTRY))
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return _self_test()

    try:
        register = load_json(Path(args.register))
        registry = load_json(Path(args.registry))
    except OSError as exc:
        print(f"ERROR: cannot read input: {exc}", file=sys.stderr)
        return 2
    except json.JSONDecodeError as exc:
        print(f"ERROR: invalid JSON: {exc}", file=sys.stderr)
        return 2

    results = evaluate(register, registry, Path.cwd())

    if args.json:
        print(json.dumps(results, indent=2))
    else:
        counts: dict[str, int] = {}
        for result in results:
            counts[result["status"]] = counts.get(result["status"], 0) + 1
            mark = "OK  " if result["ok"] else "FAIL"
            print(f"{mark} {result['id']}: {result['title']} [{result['status']}]")
            for detail in result["details"]:
                print(f"       - {detail}")
            for problem in result["problems"]:
                print(f"       ! {problem}")
        summary = ", ".join(f"{k}={v}" for k, v in sorted(counts.items()))
        print(f"\n{len(results)} condition(s): {summary}")

    failed = [r for r in results if not r["ok"]]
    if failed:
        print(
            f"FAIL: {len(failed)} condition(s) no longer hold: {[r['id'] for r in failed]}",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
