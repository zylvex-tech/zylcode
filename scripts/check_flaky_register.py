#!/usr/bin/env python3
"""Keep every `#[ignore]` a declared decision rather than a quiet absence.

WHY THIS EXISTS
---------------
An ignored test produces no result. Nothing in CI notices its absence, so the
default outcome of adding `#[ignore]` is that a red test becomes an invisible one
with a green build on top. The repository already had four of them and no single
place saying so.

`docs/governance/flaky-tests.json` is that place. This script derives the ignored
set from source and compares:

* every `#[ignore = "..."]` in `crates/` must appear in `quarantined` with the
  same test name, file and reason;
* every `quarantined` entry must exist in source with that attribute, so a
  deleted test cannot keep a quarantine slot warm;
* every `reported_but_not_quarantined` entry must still exist and must **not** be
  ignored — a reported flake that was silently quarantined fails here.

USAGE
-----
    python scripts/check_flaky_register.py
    python scripts/check_flaky_register.py --self-test

Exit codes: 0 ok, 1 register disagrees with source, 2 unreadable input.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

REGISTER = Path("docs/governance/flaky-tests.json")
SEARCH_ROOT = Path("crates")

# Anchored to a line start: prose in a doc comment that mentions `#[ignore]`
# is a description, not an attribute, and an unanchored pattern counts it as one
# (which the first version of this script did, reporting a phantom entry).
IGNORE_RE = re.compile(r'^[ \t]*#\[ignore(?:\s*=\s*"(?P<reason>[^"]*)")?\]', re.MULTILINE)
FN_RE = re.compile(r"(?:async\s+)?fn\s+(?P<name>\w+)")


def ignored_tests(root: Path) -> list[dict]:
    """Every `#[ignore]` attribute in Rust source, with its function and reason."""
    found: list[dict] = []
    for path in sorted((root / SEARCH_ROOT).rglob("*.rs")):
        text = path.read_text(encoding="utf-8", errors="replace")
        for match in IGNORE_RE.finditer(text):
            after = text[match.end() :]
            fn = FN_RE.search(after)
            found.append(
                {
                    "file": path.relative_to(root).as_posix(),
                    "test": fn.group("name") if fn else "",
                    "reason": match.group("reason") or "",
                }
            )
    return found


def declared_tests(root: Path) -> list[dict]:
    """Every entry in the register, from both sections."""
    data = json.loads((root / REGISTER).read_text(encoding="utf-8"))
    entries = []
    for section in ("quarantined", "reported_but_not_quarantined"):
        for entry in data.get(section) or []:
            entries.append({**entry, "section": section})
    return entries


def compare(root: Path) -> list[str]:
    source = ignored_tests(root)
    declared = declared_tests(root)
    problems: list[str] = []

    quarantined = {e["test"]: e for e in declared if e["section"] == "quarantined"}
    for actual in source:
        entry = quarantined.get(actual["test"])
        if entry is None:
            problems.append(
                f"`{actual['test']}` is ignored in {actual['file']} ({actual['reason']!r}) "
                "and is not declared in flaky-tests.json"
            )
            continue
        if entry.get("file") != actual["file"]:
            problems.append(
                f"`{actual['test']}` is ignored in {actual['file']}, register says {entry.get('file')}"
            )
        if (entry.get("reason") or "") != actual["reason"]:
            problems.append(
                f"`{actual['test']}` ignore reason is {actual['reason']!r}, register says {entry.get('reason')!r}"
            )

    source_names = {a["test"] for a in source}
    for entry in declared:
        path = root / str(entry.get("file") or "")
        if not path.is_file():
            problems.append(f"`{entry['test']}` names a file that does not exist: {entry.get('file')}")
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        if not re.search(rf"fn\s+{re.escape(str(entry['test']))}\b", text):
            problems.append(f"`{entry['test']}` is not defined in {entry['file']}")
        if entry["section"] == "quarantined":
            if entry["test"] not in source_names:
                problems.append(
                    f"`{entry['test']}` is declared quarantined but carries no #[ignore] attribute"
                )
        elif entry["test"] in source_names:
            problems.append(
                f"`{entry['test']}` is listed as reported-but-running but is now ignored; "
                "move it to `quarantined` with a reason"
            )
    return problems


def _self_test() -> int:
    import tempfile

    failures = 0

    def expect(label: str, ok: bool) -> None:
        nonlocal failures
        failures += 0 if ok else 1
        print(f"  {'PASS' if ok else 'FAIL'}  {label}")

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "crates/x/src").mkdir(parents=True)
        source = root / "crates/x/src/lib.rs"
        source.write_text(
            'mod tests {\n    #[ignore = "needs a network"]\n    async fn live_probe() {}\n\n'
            "    #[test]\n    fn flaky_but_running() {}\n}\n",
            encoding="utf-8",
        )
        register = root / REGISTER
        register.parent.mkdir(parents=True)
        register.write_text(
            json.dumps(
                {
                    "quarantined": [
                        {
                            "test": "live_probe",
                            "file": "crates/x/src/lib.rs",
                            "reason": "needs a network",
                        }
                    ],
                    "reported_but_not_quarantined": [
                        {"test": "flaky_but_running", "file": "crates/x/src/lib.rs"}
                    ],
                }
            ),
            encoding="utf-8",
        )

        found = ignored_tests(root)
        expect("an ignore attribute is found with its reason", found[0]["reason"] == "needs a network")
        expect("the function name is found", found[0]["test"] == "live_probe")
        expect("a matching register passes", compare(root) == [])

        register.write_text(
            json.dumps({"quarantined": [], "reported_but_not_quarantined": []}),
            encoding="utf-8",
        )
        problems = compare(root)
        expect(
            "an undeclared ignore fails",
            any("not declared" in p for p in problems),
        )

        register.write_text(
            json.dumps(
                {
                    "quarantined": [],
                    "reported_but_not_quarantined": [
                        {"test": "live_probe", "file": "crates/x/src/lib.rs"}
                    ],
                }
            ),
            encoding="utf-8",
        )
        expect(
            "a silently quarantined reported flake fails",
            any("is now ignored" in p for p in compare(root)),
        )

        register.write_text(
            json.dumps(
                {
                    "quarantined": [
                        {"test": "live_probe", "file": "crates/x/src/lib.rs", "reason": "other"}
                    ],
                    "reported_but_not_quarantined": [],
                }
            ),
            encoding="utf-8",
        )
        expect(
            "a reason that drifts from the source fails",
            any("ignore reason is" in p for p in compare(root)),
        )

    print(f"self-test: {failures} failure(s)")
    return 1 if failures else 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Check the flaky/quarantine register")
    parser.add_argument("--root", default=".")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return _self_test()

    try:
        problems = compare(Path(args.root))
    except (OSError, json.JSONDecodeError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2

    if problems:
        for problem in problems:
            print(f"FAIL: {problem}", file=sys.stderr)
        return 1

    data = json.loads((Path(args.root) / REGISTER).read_text(encoding="utf-8"))
    quarantined = len(data.get("quarantined") or [])
    reported = len(data.get("reported_but_not_quarantined") or [])
    print(
        f"OK: {quarantined} quarantined test(s) declared with reasons, "
        f"{reported} reported-but-running test(s) still in the default suite"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
