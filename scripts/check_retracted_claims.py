#!/usr/bin/env python3
"""Retracted-claim guard.

WHY THIS EXISTS
---------------
ZylCode has a documented history of publishing claims that were measured, found
unsupported, and removed -- only for the same claim to reappear later in a new
document. The canonical case is `156 tools`:

  docs/PHASE1C_COMPLETION_REPORT.md:59
      Removed unsupported claims (SOC 2, ISO 27001, <100ms, 156 tools)

A retracted claim is more dangerous on re-entry than one that was never made,
because reviewers who remember the retraction assume it stuck. A line in a
completion report does not enforce anything. This script does.

USAGE
-----
    python scripts/check_retracted_claims.py            # scan, exit 1 on hit
    python scripts/check_retracted_claims.py --list      # show the rule table
    python scripts/check_retracted_claims.py --baseline  # list pre-existing hits

Exit codes:
    0  no *new* retracted claim found
    1  at least one new (non-baselined) retracted claim found

Baseline
--------
The repository carries a known backlog of pre-existing violations, overwhelmingly
in untracked legacy reports that are already quarantined by docs/README_INDEX.md.
Those are recorded in `scripts/retracted_claims_baseline.txt`.

The point of the baseline is that it can only ever SHRINK. A baselined entry that
no longer reproduces is reported as STALE and fails the run, so the backlog cannot
silently grow back and cannot be quietly abandoned. Delete baseline lines as you
fix the underlying documents.

Adding a rule: append to RULES below. Every rule MUST cite the document that
retracted the claim and the measurement that replaced it. A rule without both is
not admissible -- see ZYLCODE_AGENT_OPERATING_PROTOCOL.md on evidence.
"""

from __future__ import annotations

import argparse
import os
import re
import sys
from pathlib import Path

# --------------------------------------------------------------------------
# Rule table
# --------------------------------------------------------------------------
# Each rule: (label, regex, retraction_reference, measured_reality)
#
# `allow_paths` lists files where the phrase is legitimate because the file IS
# the retraction record. Everything else is a regression.
RULES: list[dict] = [
    {
        "label": "156 tools",
        "pattern": re.compile(r"156\s+tools?\b", re.IGNORECASE),
        "retraction_reference": "docs/PHASE1C_COMPLETION_REPORT.md:59",
        # Measured 2026-09-17:
        #   grep -c 'ToolDefinition {' crates/zylcode-mcp/src/enhanced_bridge.rs  -> 113
        #   grep -o 'id: "[a-z0-9._-]*"' ... | sort -u | wc -l                     -> 112
        #   grep -c 'fn get_.*ToolCategory' ...                                     -> 17
        #   categories referenced in EnhancedMcpBridge::new()                       -> 16
        "measured_reality": "112 distinct tool IDs across 16 categories",
        "allow_paths": {
            "docs/PHASE1C_COMPLETION_REPORT.md",  # the retraction record itself
        },
    },
    {
        "label": "SOC 2",
        "pattern": re.compile(r"\bSOC\s*2\b", re.IGNORECASE),
        "retraction_reference": "docs/PHASE1C_COMPLETION_REPORT.md:59",
        "measured_reality": "no SOC 2 certification is held",
        "allow_paths": {
            "docs/PHASE1C_COMPLETION_REPORT.md",
            "docs/governance/ZYLCODE_PUBLIC_COMMUNICATION_POLICY.md",
            "docs/governance/STATUS_SWEEP_2026-09-16.md",
            "docs/README_INDEX.md",
        },
    },
    {
        "label": "ISO 27001",
        "pattern": re.compile(r"\bISO[\s/]*27001\b", re.IGNORECASE),
        "retraction_reference": "docs/PHASE1C_COMPLETION_REPORT.md:59",
        "measured_reality": "no ISO 27001 certification is held",
        "allow_paths": {
            "docs/PHASE1C_COMPLETION_REPORT.md",
            "docs/governance/ZYLCODE_PUBLIC_COMMUNICATION_POLICY.md",
            "docs/governance/STATUS_SWEEP_2026-09-16.md",
            "docs/README_INDEX.md",
        },
    },
]

# Directories that are never scanned.
SKIP_DIRS = {
    ".git",
    "target",
    "node_modules",
    "dist",
    "build",
    ".pnpm-store",
    ".workbuddy-ai",        # agent workspace -- local, never published
    ".dsh-vision-toolkit",  # third-party scratch, unknown provenance
    "superseded",           # governance/superseded/ is quarantined by policy
}

# Individual files that must never be scanned, because their whole purpose is to
# name the claims. Scanning them produces a self-referential false positive.
SELF_PATHS = {
    "scripts/check_retracted_claims.py",
    "scripts/retracted_claims_baseline.txt",
}

# Only scan prose and config. Source comments are covered separately below.
SCAN_SUFFIXES = {".md", ".txt", ".json", ".yaml", ".yml", ".toml", ".html"}

# Lines that are explicitly about the *absence* of the claim are fine anywhere.
# e.g. "we removed the SOC 2 claim", "no ISO 27001 certification".
NEGATION = re.compile(
    r"\b(?:no|not|never|removed|retracted|unsupported|without|absent|"
    r"drop(?:ped)?|delet(?:e|ed)|struck|withdrawn|lack(?:s|ing)?)\b",
    re.IGNORECASE,
)

# Some phrases legitimately appear because the text is *discussing the claim*
# rather than asserting it -- a retraction record, a regression report, a guard
# rule. Detect the common markers of that register.
DISCUSSION = re.compile(
    r"(?:"
    r"retract|unsupported claim|removed claim|regression|re-?ent(?:er|ry)|"
    r"claim\s*[:|]|measured reality|ground truth|allowlist|allow_paths|"
    r"guard|CI step|check_retracted|must not be cited|quarantin|"
    r"present in \*\*|appears in|found in"
    r")",
    re.IGNORECASE,
)

# Compliance terms are also legitimate when they describe SOMEONE ELSE (a
# competitor profile or a comparison table) or a FUTURE objective (a roadmap
# item to obtain certification). Only *our own* present-tense claim is a defect.
THIRD_PARTY = re.compile(
    r"(?:"
    r"tabnine|github|copilot|cursor|windsurf|amazon|jetbrains|junie|cody|"
    r"sourcegraph|anthropic|openai|google|meta|microsoft|mistral|claude|"
    r"source\s*:|acquired by|"
    r"priority|roadmap|q[1-4]\s*20\d\d|objective|goal|planned|target|"
    r"to obtain|pursue|achieve certification"
    r")",
    re.IGNORECASE,
)

# A markdown table row comparing vendors: contains 2+ pipes and a competitor.
TABLE_ROW = re.compile(r"\|.*\|")

# Only these rules are subject to the THIRD_PARTY excuse. A numeric tool-count
# claim is never someone else's, so "156 tools" always fails.
THIRD_PARTY_EXCUSABLE = {"SOC 2", "ISO 27001"}

# Inline quotation or backtick-wrapping: `156 tools`, "156 tools"
QUOTED = re.compile(r"[`\"'].{0,12}(?:156\s*tools?|SOC\s*2|ISO[\s/]*27001).{0,12}[`\"']", re.IGNORECASE)


def should_skip_dir(name: str) -> bool:
    return name in SKIP_DIRS


def normalise(path: Path, root: Path) -> str:
    """POSIX-style path relative to root, for allowlist comparison."""
    return path.relative_to(root).as_posix()


def is_discussion(line: str, window: str, rule: dict) -> bool:
    """True when the line is about the claim's status, not asserting it.

    `window` is the surrounding block (the line plus a few lines either side),
    because retraction records often state the claim on one line and the
    retraction on the next.
    """
    if NEGATION.search(line):
        return True
    if DISCUSSION.search(line):
        return True
    if QUOTED.search(line):
        return True
    # Compliance terms describing a third party or a future objective.
    if rule["label"] in THIRD_PARTY_EXCUSABLE and THIRD_PARTY.search(line):
        return True
    # A comparison table row naming another vendor.
    if (
        rule["label"] in THIRD_PARTY_EXCUSABLE
        and TABLE_ROW.search(line)
        and THIRD_PARTY.search(window)
    ):
        return True
    # The claim is being listed *inside* a retraction context block.
    if DISCUSSION.search(window) and NEGATION.search(window):
        return True
    return False


def scan(root: Path) -> list[tuple[str, int, str, dict]]:
    """Return (relpath, lineno, line, rule) for every non-allowlisted hit."""
    hits: list[tuple[str, int, str, dict]] = []

    for dirpath, dirnames, filenames in os.walk(root):
        # Prune in place so os.walk does not descend.
        dirnames[:] = [d for d in dirnames if not should_skip_dir(d)]

        for fname in filenames:
            fpath = Path(dirpath) / fname
            if fpath.suffix.lower() not in SCAN_SUFFIXES:
                continue

            rel = normalise(fpath, root)

            # Never scan our own data files: the baseline contains every claim
            # string by construction, and the guard's own source documents them.
            if rel in SELF_PATHS:
                continue

            try:
                text = fpath.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue

            lines = text.splitlines()

            for idx, line in enumerate(lines):
                for rule in RULES:
                    if rel in rule["allow_paths"]:
                        break
                    if not rule["pattern"].search(line):
                        continue

                    # Build a small context block: 3 lines either side.
                    lo = max(0, idx - 3)
                    hi = min(len(lines), idx + 4)
                    window = "\n".join(lines[lo:hi])

                    if is_discussion(line, window, rule):
                        continue

                    hits.append((rel, idx + 1, line.strip(), rule))

    return hits


def main() -> int:
    parser = argparse.ArgumentParser(description="Retracted-claim guard")
    parser.add_argument(
        "--root",
        default=".",
        help="repository root (default: current directory)",
    )
    parser.add_argument(
        "--list",
        action="store_true",
        help="print the rule table and exit",
    )
    parser.add_argument(
        "--baseline",
        action="store_true",
        help="print every current hit as a sorted baseline file body, then exit 0",
    )
    args = parser.parse_args()

    if args.list:
        for rule in RULES:
            print(f"  claim      : {rule['label']}")
            print(f"  retracted  : {rule['retraction_reference']}")
            print(f"  reality    : {rule['measured_reality']}")
            print(f"  allowlist  : {', '.join(sorted(rule['allow_paths'])) or '(none)'}")
            print()
        return 0

    root = Path(args.root).resolve()
    if not (root / ".git").exists():
        print(f"ERROR: {root} is not a git repository root", file=sys.stderr)
        return 2

    hits = scan(root)

    # --baseline: emit the canonical file body for the current state.
    if args.baseline:
        for rel, lineno, _line, rule in sorted(
            hits, key=lambda h: (h[3]["label"], h[0], h[1])
        ):
            print(f"{rule['label']}|{rel}|{lineno}")
        return 0

    baseline_path = root / "scripts" / "retracted_claims_baseline.txt"
    baseline: set[str] = set()
    if baseline_path.exists():
        for raw in baseline_path.read_text(encoding="utf-8").splitlines():
            entry = raw.strip()
            if entry and not entry.startswith("#"):
                baseline.add(entry)

    def key(rel: str, lineno: int, rule: dict) -> str:
        return f"{rule['label']}|{rel}|{lineno}"

    new_hits = [h for h in hits if key(h[0], h[1], h[3]) not in baseline]
    seen = {key(h[0], h[1], h[3]) for h in hits}
    stale = sorted(b for b in baseline if b not in seen)

    # Stale baseline entries mean the backlog shrank (good) but the file is now
    # inaccurate, or a line was renumbered (needs re-baselining).
    if stale:
        print(
            f"FAIL: {len(stale)} baseline entr(ies) no longer reproduce. "
            "Either the claim was fixed (delete the line) or the file changed "
            "(re-run --baseline).\n"
        )
        for entry in stale[:20]:
            print(f"  STALE: {entry}")
        if len(stale) > 20:
            print(f"  ... and {len(stale) - 20} more")
        print()

    if new_hits:
        print("FAIL: NEW retracted claim(s) found (not in the baseline).\n")
        for rel, lineno, line, rule in new_hits:
            print(f"  {rel}:{lineno}")
            print(f"    claim     : {rule['label']}")
            print(f"    retracted : {rule['retraction_reference']}")
            print(f"    reality   : {rule['measured_reality']}")
            print(f"    line      : {line[:160]}")
            print()
        print(
            f"{len(new_hits)} new occurrence(s). Remove the claim, rephrase it to "
            "state the\nabsence, or -- if it is a legitimate reference to the "
            "retraction -- add the file\nto that rule's allow_paths with a "
            "justification. Do NOT add it to the baseline:\nthe baseline is for "
            "the pre-existing backlog only."
        )
        return 1

    if stale:
        return 1

    total = len(hits)
    if total:
        print(
            f"OK: no NEW retracted claims "
            f"({total} pre-existing baselined occurrence(s), "
            f"{len(baseline)} baseline line(s))."
        )
    else:
        print(f"OK: no retracted claims found ({len(RULES)} rule(s) checked).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
