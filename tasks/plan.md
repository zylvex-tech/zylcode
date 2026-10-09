# Implementation Plan: Phase 2A Remediation and Delivery Readiness

## Overview

Restore the repository-intelligence foundation to an auditable state before
building later delivery surfaces. The work keeps the existing in-progress
handoff intact, verifies the real repository rather than a synthetic fixture,
and separates external GitHub Actions billing from code-level failures.

## Architecture Decisions

- Follow the governing Phase 2A remediation order; do not begin blocked Phase 2B
  or later delivery-engine product features.
- Preserve the current dirty tree until each change is understood and tested;
  do not overwrite another agent's work.
- Treat GitHub Actions billing as an external blocker. Local builds and tests
  remain required evidence.
- Keep release workflow repair separate from feature work and only enable tag
  releases; a push to `main` must not create a release.

## Task List

### Phase 1: Establish the baseline

- [ ] Task 1: Inventory the active handoff and identify each modified or
  untracked module's intent.
- [ ] Task 2: Run the focused repository-intelligence and workspace test paths;
  record real failures without suppressing them.
- [ ] Checkpoint: baseline failures, ownership boundaries, and current build
  status are captured.

### Phase 2: Phase 2A evidence and reachability

- [ ] Task 3: Verify real-repository exclusion, metric scope, and deterministic
  benchmark assertions.
- [ ] Task 4: Verify the named CLI, HTTP, Tauri, and desktop entry points and
  correct any missing wiring.
- [ ] Checkpoint: repository intelligence is testable and reachable at its
  stated scope, with captured local evidence.

### Phase 3: Delivery readiness

- [ ] Task 5: Repair the release workflow so it is valid, tag-triggered, and
  cannot publish from ordinary pushes.
- [ ] Task 6: Run local quality gates and record GitHub Actions billing as an
  external blocker if it remains unresolved.
- [ ] Checkpoint: local verification passes and remote CI can be rerun once the
  organization billing lock is removed.

## Risks and Mitigations

| Risk | Mitigation |
| --- | --- |
| Existing dirty work has unknown ownership | Inspect before editing; keep work isolated and avoid destructive operations. |
| GitHub billing blocks remote evidence | Run the same quality gates locally; report the remote blocker accurately. |
| Governance conflicts with visible UI ambitions | Implement only the accepted remediation path and record later-phase work as deferred. |

## Open Questions

- GitHub Actions billing must be restored by an organization owner before remote
  checks and release artifact builds can run.
