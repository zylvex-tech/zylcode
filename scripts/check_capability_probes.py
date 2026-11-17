#!/usr/bin/env python3
"""Capability probe gate — makes R3 claims executable.

WHY THIS EXISTS
---------------
`docs/capability-registry.json` states what ZylCode can do, but a statement is
not a check. Phase 2A found exactly that gap: `typescript_symbol_index` had an
extractor, a unit test, and no reachable product surface, and nothing in the
repository could have noticed. An audit caught it a year later.

This gate attaches each claimed capability to a **probe** that a machine can
run: a named Rust test, a file that must exist, or a command with an expected
outcome. Probes live in `docs/capability-probes.json` and are validated here.

WHAT THIS ENFORCES
------------------
1. Every probe names a capability that exists in the registry.
2. Probe kinds are from a closed vocabulary, and each kind's fields are
   required and checked (`cargo_test` needs a source file and a real `fn`);
3. A probe that names a transcript must have that transcript on disk;
4. Every capability listed in `policy.must_probe` has at least one probe;
5. `--strict` additionally fails when a GREEN capability at R3+ has no probe,
   unless it is declared in `policy.unprobed` with a reason.

The last rule is the destination, not the starting point: applying it to all 31
GREEN entries today would produce 31 invented probes, which is the failure this
repository keeps recovering from. `must_probe` is the curated core that grows
as each capability gets a real assertion, and the coverage line printed below
makes the remaining gap visible on every run instead of hiding it.

USAGE
-----
    python scripts/check_capability_probes.py
    python scripts/check_capability_probes.py --strict
    python scripts/check_capability_probes.py --self-test

Exit codes: 0 ok, 1 violations, 2 unreadable input.
"""

from __future__ import annotations

import argparse
import copy
import json
import re
import sys
from pathlib import Path

PROBE_KINDS = {"cargo_test", "file", "command", "transcript"}
REQUIRED_FIELDS = {
    "cargo_test": ("package", "test", "source"),
    "file": ("path",),
    "command": ("argv", "outcome"),
    "transcript": ("path",),
}
COMMAND_OUTCOMES = {"exit_zero", "exit_nonzero"}


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


def _source_has_test(source: Path, test_name: str) -> bool:
    """True when `source` declares `fn <name>` for the probe's test.

    The leaf segment is what Rust declares: `intelligence::query::tests::foo`
    is written as `fn foo()`. Requiring the exact leaf keeps a probe from
    pointing at a module path that does not exist.
    """
    try:
        text = source.read_text(encoding="utf-8")
    except OSError:
        return False
    leaf = test_name.rsplit("::", 1)[-1]
    return re.search(rf"\bfn\s+{re.escape(leaf)}\s*\(", text) is not None


def validate(probes: dict, registry: dict, root: Path, strict: bool = False) -> tuple[list[str], list[str]]:
    """Return (errors, warnings). Pure apart from reading probe targets."""
    errors: list[str] = []
    warnings: list[str] = []

    caps = registry.get("capabilities")
    if not isinstance(caps, dict) or not caps:
        return (["registry has no `capabilities` object; probes cannot be bound"], warnings)

    entries = probes.get("probes")
    if not isinstance(entries, dict):
        return (["`probes` is missing or not an object keyed by capability id"], warnings)

    policy = probes.get("policy") or {}
    must_probe = policy.get("must_probe") or []
    if not isinstance(must_probe, list):
        return (["`policy.must_probe` must be a list of capability ids"], warnings)
    unprobed = policy.get("unprobed") or {}

    seen: dict[str, set[str]] = {}

    for cap_id, probe_list in entries.items():
        if cap_id not in caps:
            errors.append(f"probes.{cap_id}: no such capability in the registry")
            continue
        if not isinstance(probe_list, list) or not probe_list:
            errors.append(f"probes.{cap_id}: must be a non-empty list of probes")
            continue
        fingerprints = seen.setdefault(cap_id, set())
        for index, probe in enumerate(probe_list):
            where = f"probes.{cap_id}[{index}]"
            if not isinstance(probe, dict):
                errors.append(f"{where}: probe must be an object")
                continue
            kind = probe.get("kind")
            if kind not in PROBE_KINDS:
                errors.append(f"{where}: kind {kind!r} is not one of {', '.join(sorted(PROBE_KINDS))}")
                continue
            missing = [f for f in REQUIRED_FIELDS[kind] if not probe.get(f)]
            if missing:
                errors.append(f"{where}: {kind} probe missing {', '.join(missing)}")
                continue

            fingerprint = json.dumps(probe, sort_keys=True)
            if fingerprint in fingerprints:
                errors.append(f"{where}: duplicate probe for this capability")
            fingerprints.add(fingerprint)

            if kind == "cargo_test":
                source = root / str(probe["source"])
                if not source.is_file():
                    errors.append(f"{where}: source {probe['source']} does not exist")
                elif not _source_has_test(source, str(probe["test"])):
                    errors.append(
                        f"{where}: {probe['source']} does not declare the test "
                        f"`fn {str(probe['test']).rsplit('::', 1)[-1]}`"
                    )
            elif kind == "file":
                if not (root / str(probe["path"])).is_file():
                    errors.append(f"{where}: file {probe['path']} does not exist")
            elif kind == "transcript":
                if not (root / str(probe["path"])).is_file():
                    errors.append(f"{where}: transcript {probe['path']} does not exist")
            elif kind == "command":
                if probe["outcome"] not in COMMAND_OUTCOMES:
                    errors.append(
                        f"{where}: outcome {probe['outcome']!r} is not one of "
                        f"{', '.join(sorted(COMMAND_OUTCOMES))}"
                    )
                if not isinstance(probe["argv"], list) or not all(
                    isinstance(a, str) and a for a in probe["argv"]
                ):
                    errors.append(f"{where}: argv must be a list of non-empty strings")

            # A transcript claimed by a probe must exist; this is what turns a
            # "we captured it once" note into something a re-run can check.
            transcript = probe.get("transcript")
            if transcript and not (root / str(transcript)).is_file():
                errors.append(f"{where}: transcript {transcript} does not exist")

    for cap_id in must_probe:
        if cap_id not in caps:
            errors.append(f"policy.must_probe: {cap_id} is not a capability")
        elif not entries.get(cap_id):
            errors.append(f"policy.must_probe: {cap_id} has no probe and is required to have one")

    if strict:
        for cap_id, entry in caps.items():
            if not isinstance(entry, dict):
                continue
            status = entry.get("status")
            rung = entry.get("rung")
            if status != "GREEN" or rung not in ("R3", "R4", "R5"):
                continue
            if entries.get(cap_id):
                continue
            declared = unprobed.get(cap_id)
            if declared:
                if not str(declared).strip():
                    errors.append(f"policy.unprobed.{cap_id}: reason must not be blank")
            else:
                errors.append(
                    f"{cap_id}: GREEN at {rung} with no probe and no declared `policy.unprobed` reason"
                )

    if not must_probe:
        warnings.append("policy.must_probe is empty; the gate would accept an empty probe set")

    return errors, warnings


def coverage(probes: dict, registry: dict) -> str:
    caps = (registry.get("capabilities") or {})
    entries = probes.get("probes") or {}
    green_r3 = [
        cid
        for cid, e in caps.items()
        if isinstance(e, dict) and e.get("status") == "GREEN" and e.get("rung") in ("R3", "R4", "R5")
    ]
    probed = [cid for cid in green_r3 if entries.get(cid)]
    total_probes = sum(len(v) for v in entries.values() if isinstance(v, list))
    return (
        f"probe coverage: {len(probed)}/{len(green_r3)} GREEN R3+ capabilities probed "
        f"({total_probes} probe(s)); registry total {len(caps)}"
    )


def _self_test(root: Path) -> int:
    base_registry = {
        "capabilities": {
            "alpha": {"status": "GREEN", "rung": "R3"},
            "beta": {"status": "DISPUTED", "rung": "R2"},
        }
    }
    base_probes = {
        "policy": {"must_probe": ["alpha"], "unprobed": {}},
        "probes": {
            "alpha": [
                {
                    "kind": "cargo_test",
                    "package": "zylcode-core",
                    "test": "intelligence::query::tests::build_repo_query_indexes_typescript_symbols",
                    "source": "crates/zylcode-core/src/intelligence/query.rs",
                }
            ]
        },
    }

    def mutated(fn):
        data = copy.deepcopy(base_probes)
        fn(data)
        return data

    cases = [
        ("untouched", base_probes, False, False),
        ("unknown capability", mutated(lambda d: d["probes"].__setitem__("ghost", [{"kind": "file", "path": "Cargo.toml"}])), False, True),
        ("unknown kind", mutated(lambda d: d["probes"]["alpha"][0].__setitem__("kind", "vibes")), False, True),
        ("missing field", mutated(lambda d: d["probes"]["alpha"][0].pop("source")), False, True),
        ("source missing", mutated(lambda d: d["probes"]["alpha"][0].__setitem__("source", "nope.rs")), False, True),
        ("test absent from source", mutated(lambda d: d["probes"]["alpha"][0].__setitem__("test", "tests::not_a_real_test")), False, True),
        ("transcript missing", mutated(lambda d: d["probes"]["alpha"][0].__setitem__("transcript", "docs/nope.log")), False, True),
        ("must_probe unprobed", mutated(lambda d: d["probes"].pop("alpha")), False, True),
        ("duplicate probe", mutated(lambda d: d["probes"]["alpha"].append(copy.deepcopy(d["probes"]["alpha"][0]))), False, True),
        ("bad command outcome", mutated(lambda d: d["probes"]["alpha"].append({"kind": "command", "argv": ["true"], "outcome": "maybe"})), False, True),
        ("empty must_probe", mutated(lambda d: d["policy"].__setitem__("must_probe", [])), False, False),
    ]

    failures = 0
    for label, data, strict, expect_error in cases:
        errors, _ = validate(data, base_registry, root, strict=strict)
        ok = bool(errors) == expect_error
        failures += 0 if ok else 1
        print(f"  {'PASS' if ok else 'FAIL'}  {label}: {'rejected' if errors else 'accepted'}")

    # Strict mode: GREEN R3 without a probe is an error unless declared.
    strict_registry = {"capabilities": {"alpha": {"status": "GREEN", "rung": "R3"}}}
    empty_probes = {"policy": {"must_probe": []}, "probes": {}}
    errors, _ = validate(empty_probes, strict_registry, root, strict=True)
    ok = any("no probe" in e for e in errors)
    failures += 0 if ok else 1
    print(f"  {'PASS' if ok else 'FAIL'}  strict mode rejects an unprobed R3 claim")

    declared = {"policy": {"must_probe": [], "unprobed": {"alpha": "probe lands with the CLI surface"}}, "probes": {}}
    errors, _ = validate(declared, strict_registry, root, strict=True)
    ok = not errors
    failures += 0 if ok else 1
    print(f"  {'PASS' if ok else 'FAIL'}  strict mode accepts a declared, reasoned gap")

    blank = {"policy": {"must_probe": [], "unprobed": {"alpha": "   "}}, "probes": {}}
    errors, _ = validate(blank, strict_registry, root, strict=True)
    ok = any("must not be blank" in e for e in errors)
    failures += 0 if ok else 1
    print(f"  {'PASS' if ok else 'FAIL'}  strict mode rejects a blank exemption reason")

    total = len(cases) + 3
    print(f"self-test: {total} checks, {failures} failure(s)")
    return 1 if failures else 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Capability probe gate")
    parser.add_argument("--probes", default="docs/capability-probes.json")
    parser.add_argument("--registry", default="docs/capability-registry.json")
    parser.add_argument("--strict", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    root = Path.cwd()
    if args.self_test:
        return _self_test(root)

    registry = load_json(Path(args.registry))
    probes = load_json(Path(args.probes))
    errors, warnings = validate(probes, registry, root, strict=args.strict)

    if errors:
        print(f"FAIL: {len(errors)} capability-probe error(s)\n")
        for err in errors:
            print(f"  {err}")
        print()
        return 1

    for warn in warnings:
        print(f"WARN: {warn}")
    print(f"OK: {coverage(probes, registry)}")
    if not args.strict:
        print("     strict mode is available with --strict once coverage is complete")
    return 0


if __name__ == "__main__":
    sys.exit(main())
