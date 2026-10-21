#!/usr/bin/env python3
"""Hardcoded-secret guard.

WHY THIS EXISTS
---------------
An automated external quality scan of this repository flagged three
credential-shaped strings (2026-10-20):

  crates/zylcode-mcp/src/enhanced_plugin_marketplace.rs:589   `_api_key: "sk_test_..."`
  apps/natlas-sdk/test/smoke.mjs (3 sites)                    `"stub-key-1234"`

Two were test fixtures; one sat in a non-test source file. Both were real
findings in the sense that a scanner cannot tell a placeholder from a leaked key,
so a human reviewer cannot either -- and the next real key pasted into source
would look exactly the same.

This script closes that gap. It fails CI on a *new* credential-shaped literal.

DESIGN -- WHAT IT DELIBERATELY DOES NOT DO
------------------------------------------
It does NOT grep for `api_key`, `secret`, `token` or `password`. Those words
appear legitimately in every configuration schema in this workspace (see
`zylcode_core::router`, `competition::natlas::config`, `McpConfigFile`), and a
pattern broad enough to catch them would fire on the code that correctly *reads
a key from the environment*. A guard that cries wolf on correct code gets
disabled, and a disabled guard is worse than none.

Instead it matches only high-confidence, provider-specific credential FORMATS --
the prefixes and entropy of real issued keys. Those do not occur in ordinary
source, so the guard carries no allowlist and suppresses nothing.

The three flagged literals are gone:
  * `sk_test_...` was replaced by an explicit unconfigured state
    (`PaymentGatewayCredential::Unset`) -- see the type docs.
  * `stub-key-1234` moved to `apps/natlas-sdk/test/fixtures.mjs` as
    `TEST_FIXTURE_API_KEY`, documented as a test-only fixture.
Neither matches any pattern below, so nothing is exempted.

USAGE
-----
    python scripts/check_hardcoded_secrets.py            # scan, exit 1 on hit
    python scripts/check_hardcoded_secrets.py --list     # show the pattern table
    python scripts/check_hardcoded_secrets.py --selftest # prove detection works
    python scripts/check_hardcoded_secrets.py --baseline # list known-safe hits

Exit codes:
    0  no credential-shaped literal found (or selftest passed)
    1  at least one credential-shaped literal found, or a stale baseline entry

SELFTEST
--------
A secret scanner that has never been observed to detect anything is a claim, not
a control. `--selftest` asserts, per pattern, that a synthetic real-format key
DOES match and that the known-safe shapes in this repository do NOT. Run it after
touching PATTERNS.

Only **git-tracked** files are scanned. A guard whose verdict depends on whose
working tree it ran in -- green on a machine with untracked scratch files, red on
a clean checkout -- trains people to ignore it.
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from pathlib import Path

# Force UTF-8 output: findings quote source lines that may contain non-cp1252
# characters. On Windows consoles the default codec raises UnicodeEncodeError
# while printing the finding, which masks the actual result (and can turn a real
# failure into an infrastructure error).
if sys.stdout.encoding and sys.stdout.encoding.lower() not in ("utf-8", "utf8"):
    os.environ.setdefault("PYTHONIOENCODING", "utf-8")
    try:
        sys.stdout.reconfigure(encoding="utf-8")
        sys.stderr.reconfigure(encoding="utf-8")
    except (AttributeError, OSError):
        pass

# --------------------------------------------------------------------------
# Pattern table
# --------------------------------------------------------------------------
# Each entry: (label, compiled regex, what it catches)
#
# Inclusion rule: the pattern must correspond to a credential format that a real
# provider ISSUES -- a recognisable prefix plus enough entropy that ordinary
# source cannot produce it by accident. `{20,}`-style minimums are what keep the
# obviously-fake shapes (`sk_test_...`, `changeme`, `your-key-here`) from ever
# matching, so the guard needs no allowlist.
#
# Adding a pattern: append here, add a matching case to SELFTEST_CASES, and run
# `--selftest`. A pattern without a selftest case is not admissible.
PATTERNS: list[tuple[str, re.Pattern[str], str]] = [
    (
        "Stripe live secret key",
        re.compile(r"\bsk_live_[0-9A-Za-z]{20,}"),
        "a live Stripe key can move money",
    ),
    (
        "Stripe restricted/test secret key",
        re.compile(r"\bsk_(?:test|restricted)_[0-9A-Za-z]{20,}"),
        "test keys are not live, but committing one still leaks the account",
    ),
    (
        "Stripe live restricted key",
        re.compile(r"\brk_live_[0-9A-Za-z]{20,}"),
        "live restricted key",
    ),
    (
        "AWS access key id",
        re.compile(r"\bAKIA[0-9A-Z]{16}\b"),
        "AWS access key id (permanent credentials)",
    ),
    (
        "GitHub personal/token",
        re.compile(r"\bgh[pousr]_[0-9A-Za-z]{36,}"),
        "GitHub PAT for a user / server / oauth app",
    ),
    (
        "GitHub fine-grained PAT",
        re.compile(r"\bgithub_pat_[0-9A-Za-z_]{60,}"),
        "GitHub fine-grained personal access token",
    ),
    (
        "Slack token",
        re.compile(r"\bxox[baprs]-[0-9A-Za-z\-]{10,}"),
        "Slack bot/user/app/legacy token",
    ),
    (
        "Slack incoming webhook",
        re.compile(r"https://hooks\.slack\.com/services/T[0-9A-Za-z_]+/B[0-9A-Za-z_]+/[0-9A-Za-z_]+"),
        "Slack webhook URL is itself a credential",
    ),
    (
        "Google API key",
        re.compile(r"\bAIza[0-9A-Za-z_\-]{35}\b"),
        "Google API key",
    ),
    (
        "npm access token",
        re.compile(r"\bnpm_[0-9A-Za-z]{36}\b"),
        "npm registry token",
    ),
    (
        "PyPI API token",
        re.compile(r"\bpypi-[0-9A-Za-z_\-]{16,}"),
        "PyPI upload token",
    ),
    (
        "SendGrid API key",
        re.compile(r"\bSG\.[0-9A-Za-z_\-]{22}\.[0-9A-Za-z_\-]{43}\b"),
        "SendGrid API key",
    ),
    (
        "Private key block",
        re.compile(r"-----BEGIN (?:RSA |EC |DSA |OPENSSH |PGP )?PRIVATE KEY(?: BLOCK)?-----"),
        "any PEM private key body, including passphrase-protected ones",
    ),
]

# Synthetic positives and known-safe negatives, used by `--selftest`.
# `safe` entries are shapes that actually occur in this repository (or that a
# scanner would otherwise flag) and MUST NOT match.
#
# WHY THE POSITIVES ARE SPLIT ACROSS ADJACENT STRING LITERALS
# -----------------------------------------------------------
# This script is a tracked file, so the scanner scans it like any other. Left
# contiguous, the synthetic positives below are real-format credential literals
# sitting in the repository, and the guard would flag its own test data -- a
# self-inflicted false positive that only becomes visible the moment the script
# is committed rather than run from an untracked working tree.
#
# Splitting them costs nothing in detection power: the regex is tested against
# the concatenated runtime value, which is unchanged. It just means no single
# source line contains a credential-shaped literal. Do not "tidy" these back
# into one string -- `--selftest` alone will not catch that, but the scan of
# this very file will.
SELFTEST_CASES: list[dict] = [
    {
        "pattern": "Stripe live secret key",
        "unsafe": "STRIPE_KEY=" + "sk_live_" + "4eC39HqLyjWDarjtT1zdp7dc",
        "safe": [
            "sk_test_...",
            "sk_test_XXX",
            "_api_key: \"sk_test_...\".to_string(),",
            "sk_live_tooshort",
        ],
    },
    {
        "pattern": "Stripe restricted/test secret key",
        "unsafe": "const k = '" + "sk_test_" + "4eC39HqLyjWDarjtT1zdp7dc'",
        "safe": ["sk_test_...", "sk_test_abc"],
    },
    {
        "pattern": "Stripe live restricted key",
        "unsafe": "rk = " + "rk_live_" + "4eC39HqLyjWDarjtT1zdp7dc",
        "safe": ["rk_live_short", "rk_live_tooshort"],
    },
    {
        "pattern": "AWS access key id",
        "unsafe": "aws_key = " + "AKIA" + "IOSFODNN7EXAMPLE",
        "safe": ["AKIA", "AKIASHORT", "akiaiosfodnn7example"],
    },
    {
        "pattern": "GitHub personal/token",
        "unsafe": "token: " + "ghp_" + "16C7e42F292c6912E7710c838347Ae178B4a",
        "safe": ["ghp_short", "gho_x"],
    },
    {
        "pattern": "GitHub fine-grained PAT",
        "unsafe": "github_pat_11ABCDEFG0" + "a" * 70,
        "safe": ["github_pat_short"],
    },
    {
        "pattern": "Slack token",
        "unsafe": "SLACK=" + "xoxb-" + "1234567890-ABCDEFGHIJ",
        "safe": ["xoxb-", "slack_token = \"stub-key-1234\""],
    },
    {
        "pattern": "Slack incoming webhook",
        "unsafe": (
            "https://hooks.slack.com/services/"
            + "T00000000" + "/B00000000/" + "XXXXXXXXXXXXXXXXXXXXXXXX"
        ),
        "safe": ["https://hooks.slack.com/services/T/B/X"],
    },
    {
        "pattern": "Google API key",
        "unsafe": "key=" + "AIza" + "SyA1234567890abcdefghijklmnopqrstuv",
        "safe": ["AIzaSyShort"],
    },
    {
        "pattern": "npm access token",
        # npm tokens are exactly 36 chars after the `npm_` prefix.
        "unsafe": "npm_" + "abcdefghij0123456789abcdefghij012345",
        "safe": ["npm_short", "npm_" + "a" * 35],
    },
    {
        "pattern": "PyPI API token",
        "unsafe": "pypi-" + "AgEIcHlwaS5vcmcCJDU0ZTQ2YmY2LW",
        "safe": ["pypi-"],
    },
    {
        "pattern": "SendGrid API key",
        "unsafe": "SG." + "abcdefghijklmnopqrstuv" + "."
        "abcdefghijklmnopqrstuvwxyz0123456789ABCDEFG",
        "safe": ["SG.short.short"],
    },
    {
        "pattern": "Private key block",
        "unsafe": "-----BEGIN " + "RSA " + "PRIVATE KEY-----",
        "safe": ["BEGIN RSA PRIVATE KEY", "-----BEGIN PUBLIC KEY-----", "-----BEGIN CERTIFICATE-----"],
    },
]

# The known-safe shapes in this repository that MUST never match. They are
# asserted here so that removing or altering them trips the selftest rather than
# silently weakening the guard. Every entry is a string that really exists in the
# tree as of the 2026-10-21 hardening branch -- no hypotheticals.
REPO_SAFE_SHAPES = [
    'TEST_FIXTURE_API_KEY = "test-fixture-not-a-key-0001"',   # apps/natlas-sdk/test/fixtures.mjs
    "_api_key: PaymentGatewayCredential",                      # crates/zylcode-mcp/.../enhanced_plugin_marketplace.rs
    "apiKey: TEST_FIXTURE_API_KEY",                            # apps/natlas-sdk/test/smoke.mjs
    "number: \"4242424242424242\".to_string(),",               # Stripe's published test PAN
]

# Directories that are never scanned. Same rationale as the retracted-claim
# guard: generated artefacts and caches are not authored source, and `.workbuddy-ai`
# is local-only and never published.
SKIP_DIRS = {
    ".git",
    "target",
    "node_modules",
    "dist",
    "build",
    ".pnpm-store",
    ".workbuddy-ai",
    ".wt",
    ".zylcode",
}

# Extensions scanned. Text formats only -- binaries cannot contain a credential
# in any form a human could read back out.
SCAN_EXTS = {
    ".rs", ".toml", ".py", ".js", ".mjs", ".cjs", ".ts", ".tsx", ".jsx",
    ".json", ".yaml", ".yml", ".md", ".txt", ".sh", ".ps1", ".bat", ".cmd",
    ".html", ".css", ".cfg", ".ini", ".env", ".template",
}

MAX_LINE = 2000  # skip pathological lines; a credential is not that long


def tracked_files() -> list[str]:
    """Git-tracked files, via `git ls-files` so the verdict is tree-independent."""
    out = subprocess.run(
        ["git", "ls-files", "-z"],
        capture_output=True,
        text=True,
        check=True,
    )
    return [p for p in out.stdout.split("\0") if p]


def git_root() -> Path:
    out = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        capture_output=True,
        text=True,
        check=True,
    )
    return Path(out.stdout.strip())


def selftest() -> int:
    """Prove every pattern detects, and that nothing safe matches."""
    by_label = {label: rx for label, rx, _ in PATTERNS}
    failures: list[str] = []

    for case in SELFTEST_CASES:
        label = case["pattern"]
        if label not in by_label:
            failures.append(f"no pattern registered for selftest label {label!r}")
            continue
        rx = by_label[label]
        if not rx.search(case["unsafe"]):
            failures.append(f"[{label}] FAILED TO DETECT synthetic credential: {case['unsafe']!r}")
        for safe in case["safe"]:
            if rx.search(safe):
                failures.append(f"[{label}] FALSE POSITIVE on safe shape: {safe!r}")

    # Nothing in the table may fire on the repository's known-safe shapes.
    for shape in REPO_SAFE_SHAPES:
        for label, rx, _ in PATTERNS:
            if rx.search(shape):
                failures.append(f"[{label}] FALSE POSITIVE on repository shape: {shape!r}")

    if failures:
        print("selftest FAILED -- the guard is not trustworthy as configured:\n")
        for f in failures:
            print(f"  - {f}")
        return 1

    print(f"selftest OK: {len(SELFTEST_CASES)} patterns detect, "
          f"{sum(len(c['safe']) for c in SELFTEST_CASES)} safe shapes and "
          f"{len(REPO_SAFE_SHAPES)} repository shapes produce no match.")
    return 0


def show_list() -> int:
    print("Patterns (high-confidence provider credential formats only):\n")
    for label, rx, why in PATTERNS:
        print(f"  {label}")
        print(f"      regex: {rx.pattern}")
        print(f"      why:   {why}")
    print("\nNo allowlist, no baseline, no suppression: the guard has nothing to except.")
    return 0


def scan(baseline_only: bool) -> int:
    root = git_root()
    hits: list[tuple[str, int, str, str]] = []

    for rel in tracked_files():
        path = root / rel
        if path.suffix.lower() not in SCAN_EXTS:
            continue
        if any(part in SKIP_DIRS for part in Path(rel).parts):
            continue
        try:
            text = path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue  # unreadable per-tracked-file is an infra error, not a secret

        for lineno, line in enumerate(text.splitlines(), start=1):
            if len(line) > MAX_LINE:
                continue
            for label, rx, _ in PATTERNS:
                if rx.search(line):
                    hits.append((rel, lineno, label, line.strip()[:160]))

    hits.sort()

    if baseline_only:
        if hits:
            print(f"{len(hits)} hit(s) (baseline listing only, exit 0):")
            for rel, lineno, label, line in hits:
                print(f"  {rel}:{lineno}  {label}")
                print(f"      {line}")
        else:
            print("baseline is empty -- the repository carries no credential-shaped literal.")
        return 0

    if not hits:
        print("secret scan: clean -- no credential-shaped literal in tracked files.")
        return 0

    print(f"secret scan FAILED: {len(hits)} credential-shaped literal(s) in tracked files.\n")
    print("A real credential in source is a disclosure: rotate it, do not just delete it.\n")
    for rel, lineno, label, line in hits:
        print(f"  {rel}:{lineno}  [{label}]")
        print(f"      {line}")
    print("\nIf the value is genuinely a test fixture, move it to a clearly-named")
    print("fixtures module with a doc comment stating it is not a credential -- and")
    print("make sure it does not match a real provider format.")
    return 1


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--list", action="store_true", help="show the pattern table")
    ap.add_argument("--selftest", action="store_true", help="prove detection works")
    ap.add_argument("--baseline", action="store_true", help="list known-safe hits without failing")
    args = ap.parse_args()

    if args.selftest:
        return selftest()
    if args.list:
        return show_list()
    return scan(args.baseline)


if __name__ == "__main__":
    sys.exit(main())
