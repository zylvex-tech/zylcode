# N-ATLAS Challenge — Official Rubric (verified 2026-10-07)

Source of truth: **https://ncair.nitda.gov.ng/naic/** (NCAIR / NITDA / FMCIDE / ONDI).
Verified by fetch on 2026-10-07. Quoted criteria are verbatim from the official page.

> This document records the *actual* judging rules so the build is framed to win — not to our
> assumptions. Where a requirement is human-dependent (beta testers, CAC, team profile), it is
> flagged. Nothing here is fabricated.

## Challenge shape
- **Build-only.** "Every valid submission must demonstrate three things: (1) a working technical
  build, (2) genuine N-ATLAS integration, (3) real-world validation … Not simulated or hypothetical
  results."
- **Deadline:** 12 October 2026, 23:59 WAT.
- **Our problem statement:** **PS1 — Developer Infrastructure** ("Develop infrastructure, documentation
  and tools that enable developers to integrate and extend N-ATLAS efficiently").
- **Our track:** **Innovation & Enterprise** (Zylvex Technologies Limited — registered company).
  Requirements: CAC certificate, valid ID, 2–5 team members, ≥50% Nigeria-based.

## The three things every submission MUST show
1. **A working technical build** — "Software, deployed system, dataset, model or another functioning
   technical artefact relevant to your problem statement."
2. **Genuine N-ATLAS integration** — "The project must use the N-ATLAS model, API, ASR service or
   training pipeline as required by your problem statement."
3. **Real-world validation** — "Evidence of testing with actual users, real data or live benchmarks.
   Not simulated or hypothetical results."

### Disqualification rule (load-bearing)
> "Can I wrap a different foundation model (e.g. GPT-4)? No. Submissions that wrap general-purpose
> models instead of N-ATLAS will be disqualified. The core requirement is genuine, demonstrable
> integration with N-ATLAS."

This **validates ZylCode's architecture choice**: N-ATLAS is a first-class competition provider behind
the dedicated `NatlasTransport` seam, and is **deliberately NOT** added to the product `router.rs`
`ModelProvider` enum (which has three silent synthetic-degradation paths — the fabrication hazard).

## Seven mandatory submission components
1. **Working Artefact** — repository, deployed app, published model, live API or dataset.
2. **N-ATLAS Integration Evidence** — documentation showing how the solution integrates with N-ATLAS.
3. **Real-World Validation** — testing with real users, data or live benchmarks.
4. **Technical Documentation** — architecture, setup, usage.
5. **Video Demonstration** — 3–5 minute video, end-to-end.
6. **Team Profile** — names, affiliations, roles.
7. **Endorsement / Registration** — institutional letter (Track A) or CAC certificate / ID (Track B).

### PS1-specific validation requirement
> "Developer Infrastructure — At least 2 external beta testers."

**This is the one hard requirement ZylCode cannot self-satisfy.** It needs the owner to recruit
≥2 external beta testers. We will build the beta-test harness and a ready-to-send invite + feedback
template, but the testers themselves are a human action (see SUBMISSION_CHECKLIST.md).

## Six evaluation criteria (panel: technical AI experts, sector specialists, ONDI + NCAIR)
1. **Working Artefact & Technical Rigour** — quality, completeness, soundness.
2. **N-ATLAS Integration** — depth and correctness of integration.
3. **Real-World Validation** — real users / data / benchmarks.
4. **Impact Potential** — social, economic, ecosystem impact.
5. **Scalability & Sustainability** — realistic pathway to scale and continued operation.
6. **Team Capability** — expertise, diversity, delivery credibility.

(No numeric weights published; all six are assessed by the panel.)

## What this means for "flagship"
- Lead with **sovereign, Nigerian-built AI in developer infrastructure** — the exact national-infra
  narrative the programme exists to surface.
- Prove **genuine N-ATLAS integration** with reproducible evidence (our promotion gate).
- Satisfy **real-world validation** via ≥2 external beta testers + live benchmarks.
- Ship a **3–5 min end-to-end video** and complete technical docs.
- Honour the **Apache-2.0 / N-ATLaS licence** (attribution to Awarri + FMCIDE, ≤1000 users).
