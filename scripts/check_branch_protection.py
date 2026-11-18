#!/usr/bin/env python3
"""Compare the live `main` branch protection against the recorded configuration.

WHY THIS EXISTS
---------------
Branch protection lives in GitHub settings, not in git. That has two consequences
this repository has already paid for:

* the settings can be relaxed for a merge and not restored, and no diff shows it;
* a required status check can stop matching (a renamed CI job) and the branch is
  then protected by four contexts instead of five, silently.

`docs/governance/branch-protection.json` records the intended configuration and
the date it was read. This script reads the live configuration and reports every
difference, so "protected" is a checkable statement rather than an assumption.

It reports **unknown** separately from **drift**: a token without admin scope
cannot read protection at all (HTTP 403), and a tool that called that "drift"
would be crying wolf on every run.

USAGE
-----
    python scripts/check_branch_protection.py
    python scripts/check_branch_protection.py --record   # rewrite the recorded file
    python scripts/check_branch_protection.py --self-test

Exit codes: 0 matches, 1 drift, 2 could not read the live configuration.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

RECORDED = Path("docs/governance/branch-protection.json")


def flatten(live: dict) -> dict:
    """Reduce the API payload to the fields the record states."""

    def enabled(key: str) -> bool:
        value = live.get(key)
        return bool(value.get("enabled")) if isinstance(value, dict) else bool(value)

    reviews = live.get("required_pull_request_reviews") or {}
    checks = live.get("required_status_checks") or {}
    return {
        "required_status_checks": {
            "strict": bool(checks.get("strict")),
            "contexts": sorted(checks.get("contexts") or []),
        },
        "required_pull_request_reviews": {
            "dismiss_stale_reviews": bool(reviews.get("dismiss_stale_reviews")),
            "require_code_owner_reviews": bool(reviews.get("require_code_owner_reviews")),
            "require_last_push_approval": bool(reviews.get("require_last_push_approval")),
            "required_approving_review_count": int(
                reviews.get("required_approving_review_count") or 0
            ),
        },
        "required_signatures": enabled("required_signatures"),
        "enforce_admins": enabled("enforce_admins"),
        "required_linear_history": enabled("required_linear_history"),
        "allow_force_pushes": enabled("allow_force_pushes"),
        "allow_deletions": enabled("allow_deletions"),
        "block_creations": enabled("block_creations"),
        "required_conversation_resolution": enabled("required_conversation_resolution"),
        "lock_branch": enabled("lock_branch"),
        "allow_fork_syncing": enabled("allow_fork_syncing"),
    }


def diff(expected: dict, actual: dict, prefix: str = "") -> list[str]:
    """Every differing leaf, as `path: expected X, live Y`."""
    out: list[str] = []
    for key, want in expected.items():
        path = f"{prefix}{key}"
        if isinstance(want, dict):
            got = actual.get(key)
            if not isinstance(got, dict):
                out.append(f"{path}: recorded a table, live is {got!r}")
            else:
                out.extend(diff(want, got, prefix=f"{path}."))
            continue
        got = actual.get(key)
        if isinstance(want, list):
            if sorted(got or []) != sorted(want):
                missing = sorted(set(want) - set(got or []))
                extra = sorted(set(got or []) - set(want))
                out.append(f"{path}: missing {missing}, unexpected {extra}")
            continue
        if got != want:
            out.append(f"{path}: recorded {want!r}, live {got!r}")
    return out


def fetch(repository: str, branch: str) -> tuple[dict | None, str]:
    """Read the live protection. Returns (payload, error)."""
    try:
        completed = subprocess.run(
            ["gh", "api", f"repos/{repository}/branches/{branch}/protection"],
            capture_output=True,
            text=True,
            timeout=60,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        return None, f"could not run gh: {exc}"
    if completed.returncode != 0:
        message = (completed.stderr or completed.stdout).strip()
        if "403" in message or "404" in message:
            return None, (
                "cannot read branch protection (HTTP 403/404): the token needs admin "
                "scope on the repository. This is UNKNOWN, not drift.\n" + message
            )
        return None, message or f"gh exited {completed.returncode}"
    try:
        return json.loads(completed.stdout), ""
    except json.JSONDecodeError as exc:
        return None, f"gh returned unreadable JSON: {exc}"


def _self_test() -> int:
    failures = 0

    def expect(label: str, ok: bool) -> None:
        nonlocal failures
        failures += 0 if ok else 1
        print(f"  {'PASS' if ok else 'FAIL'}  {label}")

    live = {
        "required_status_checks": {"strict": True, "contexts": ["b", "a"]},
        "required_pull_request_reviews": {
            "dismiss_stale_reviews": False,
            "require_code_owner_reviews": False,
            "require_last_push_approval": False,
            "required_approving_review_count": 1,
        },
        "required_signatures": {"enabled": False},
        "enforce_admins": {"enabled": True},
        "required_linear_history": {"enabled": False},
        "allow_force_pushes": {"enabled": False},
        "allow_deletions": {"enabled": False},
        "block_creations": {"enabled": False},
        "required_conversation_resolution": {"enabled": True},
        "lock_branch": {"enabled": False},
        "allow_fork_syncing": {"enabled": False},
    }
    flat = flatten(live)
    expect("contexts are sorted for comparison", flat["required_status_checks"]["contexts"] == ["a", "b"])
    expect("admin enforcement reads the nested object", flat["enforce_admins"] is True)
    expect("a matching record has no drift", diff(flat, flat) == [])

    relaxed = flatten({**live, "required_pull_request_reviews": {**live["required_pull_request_reviews"], "required_approving_review_count": 0}})
    expect(
        "a relaxed approval count is reported",
        any("required_approving_review_count" in line for line in diff(flat, relaxed)),
    )

    fewer = flatten({**live, "required_status_checks": {"strict": True, "contexts": ["a"]}})
    expect(
        "a dropped required check is reported",
        any("missing" in line for line in diff(flat, fewer)),
    )

    print(f"self-test: {failures} failure(s)")
    return 1 if failures else 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Check live branch protection against the record")
    parser.add_argument("--recorded", default=str(RECORDED))
    parser.add_argument("--repository", default="")
    parser.add_argument("--branch", default="")
    parser.add_argument("--record", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return _self_test()

    try:
        recorded = json.loads(Path(args.recorded).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        print(f"ERROR: cannot read {args.recorded}: {exc}", file=sys.stderr)
        return 2

    repository = args.repository or recorded.get("repository") or ""
    branch = args.branch or recorded.get("branch") or "main"
    if not repository:
        print("ERROR: no repository recorded and none passed with --repository", file=sys.stderr)
        return 2

    live, error = fetch(repository, branch)
    if live is None:
        print(f"UNKNOWN: {error}", file=sys.stderr)
        return 2

    flat = flatten(live)
    if args.record:
        recorded["protection"] = flat
        recorded["recorded_at"] = "recorded-by-check-script"
        Path(args.recorded).write_text(json.dumps(recorded, indent=2) + "\n", encoding="utf-8")
        print(f"OK: recorded the live protection for {repository}@{branch} in {args.recorded}")
        return 0

    problems = diff(recorded.get("protection") or {}, flat)
    if problems:
        print(f"DRIFT: {repository}@{branch} differs from the recorded configuration:")
        for problem in problems:
            print(f"  - {problem}")
        print(
            "\nEither restore the intended setting or update "
            f"{args.recorded} with a reason. A relaxed setting left in place is a "
            "policy change nobody voted for."
        )
        return 1

    print(
        f"OK: {repository}@{branch} matches {args.recorded} "
        f"(recorded {recorded.get('recorded_at')})"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
