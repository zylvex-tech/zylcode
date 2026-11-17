#!/usr/bin/env python3
"""Freeze a benchmark question set so it cannot be quietly tuned after scoring.

WHY THIS EXISTS
---------------
Phase 2A's benchmark questions were written by the same person who tuned the
retriever, and eight were added late to lift the average. The verdict's condition
C4 asks for independently authored questions. Independence is a property of
*who wrote them and when*, and neither is observable in a repository that lets
the set change freely between a run and its acceptance.

This script writes a lock file next to the set: the SHA-256 of the exact bytes
scored. `--check` fails when the set has changed since it was frozen. Changing
the questions is still allowed — a benchmark must be able to grow — but it
requires a deliberate re-freeze whose `--reason` is recorded in the lock, so the
edit appears in history as a decision instead of as drift.

USAGE
-----
    python scripts/freeze_question_set.py --set docs/benchmarks/questions.json \
        --reason "initial freeze before scoring"
    python scripts/freeze_question_set.py --set docs/benchmarks/questions.json --check
    python scripts/freeze_question_set.py --self-test

Exit codes: 0 ok, 1 drift/missing lock/invalid argument, 2 unreadable input.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path

LOCK_SUFFIX = ".lock.json"


def lock_path_for(question_set: Path) -> Path:
    return question_set.with_name(question_set.name + LOCK_SUFFIX)


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def count_questions(path: Path) -> int | None:
    """Best-effort count for the lock's metadata; never the basis of the hash."""
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    if isinstance(data, list):
        return len(data)
    if isinstance(data, dict) and isinstance(data.get("questions"), list):
        return len(data["questions"])
    return None


def freeze(question_set: Path, reason: str, force: bool) -> int:
    if not question_set.is_file():
        print(f"ERROR: question set not found: {question_set}", file=sys.stderr)
        return 2
    if not reason.strip():
        print("ERROR: --reason is required; a re-freeze must say why", file=sys.stderr)
        return 1

    lock_file = lock_path_for(question_set)
    current = digest(question_set)
    if lock_file.is_file() and not force:
        previous = json.loads(lock_file.read_text(encoding="utf-8"))
        if previous.get("sha256") == current:
            print(f"OK: already frozen at sha256:{current[:12]}")
            return 0
        print(
            f"ERROR: {question_set} changed since it was frozen at "
            f"sha256:{str(previous.get('sha256'))[:12]}. Re-run with --force to "
            "re-freeze, knowing the previous scores no longer describe this set.",
            file=sys.stderr,
        )
        return 1

    lock = {
        "question_set": question_set.name,
        "sha256": current,
        "questions": count_questions(question_set),
        "frozen_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "reason": reason.strip(),
    }
    lock_file.write_text(json.dumps(lock, indent=2) + "\n", encoding="utf-8")
    print(f"OK: froze {question_set} at sha256:{current[:12]} ({lock_file})")
    return 0


def check(question_set: Path) -> int:
    lock_file = lock_path_for(question_set)
    if not question_set.is_file():
        print(f"ERROR: question set not found: {question_set}", file=sys.stderr)
        return 2
    if not lock_file.is_file():
        print(
            f"FAIL: no freeze lock for {question_set}; scores from an unfrozen set "
            "cannot be trusted as acceptance evidence",
            file=sys.stderr,
        )
        return 1
    lock = json.loads(lock_file.read_text(encoding="utf-8"))
    current = digest(question_set)
    if lock.get("sha256") != current:
        print(
            f"FAIL: {question_set} no longer matches its freeze "
            f"(locked {str(lock.get('sha256'))[:12]}, now {current[:12]})",
            file=sys.stderr,
        )
        return 1
    print(f"OK: {question_set} matches its freeze at sha256:{current[:12]}")
    return 0


def _self_test() -> int:
    failures = 0

    def expect(label: str, ok: bool) -> None:
        nonlocal failures
        failures += 0 if ok else 1
        print(f"  {'PASS' if ok else 'FAIL'}  {label}")

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        qs = root / "questions.json"
        qs.write_text(json.dumps([{"id": "q1"}]), encoding="utf-8")

        expect("check refuses an unfrozen set", check(qs) == 1)
        expect("freeze writes a lock", freeze(qs, "initial freeze", force=False) == 0)
        expect("check accepts a matching set", check(qs) == 0)
        expect("re-freezing unchanged content is a no-op", freeze(qs, "again", force=False) == 0)

        qs.write_text(json.dumps([{"id": "q1"}, {"id": "q2"}]), encoding="utf-8")
        expect("check fails after the set changes", check(qs) == 1)
        expect("re-freeze needs --force", freeze(qs, "added q2", force=False) == 1)
        expect("force re-freeze records the reason", freeze(qs, "added q2", force=True) == 0)
        lock = json.loads(lock_path_for(qs).read_text(encoding="utf-8"))
        expect("lock records the reason", lock["reason"] == "added q2")
        expect("lock records the question count", lock["questions"] == 2)
        expect("check passes after re-freeze", check(qs) == 0)
        expect("blank reason is refused", freeze(qs, "   ", force=True) == 1)

    print(f"self-test: {failures} failure(s)")
    return 1 if failures else 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Freeze a benchmark question set")
    parser.add_argument("--set", default="docs/benchmarks/questions.json")
    parser.add_argument("--reason", default="")
    parser.add_argument("--force", action="store_true")
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return _self_test()
    question_set = Path(args.set)
    if args.check:
        return check(question_set)
    return freeze(question_set, args.reason, args.force)


if __name__ == "__main__":
    sys.exit(main())
