# Archived root-level documents (moved 2026-11-07)

These files lived in the repository root until 2026-11-07. They are **historical**: they record
programme status, design notes and research from earlier phases, and several assert completion of
numbering schemes that are not the governing roadmap.

They are kept — deleting them would falsify the record — and they must not be cited as evidence of
current capability. `docs/README_INDEX.md` §4 is the governing quarantine list; this directory is
where that quarantine physically lives.

Every file was moved with `git mv`, so `git log --follow` still resolves its history. The only live
document from that set is `INSTALLATION_GUIDE.md`, which moved to `docs/INSTALLATION_GUIDE.md`.

| File | Original heading |
| ---- | ---------------- |
| `COMPLETE_PROJECT_BLUEPRINT.md` | ZylCode: Complete Research & Implementation Blueprint |
| `FINAL_SUMMARY.md` | 🎉 ZylCode Desktop App - Fixed and Running! |
| `FINAL_WORKING_APPLICATION.md` | 🎉 ZylCode Desktop Application - Fully Working! |
| `FINAL_WORKING_SUMMARY.md` | 🎉 ZylCode Desktop Application - Now Working! |
| `IMPLEMENTATION_STATUS.md` | ZylCode Implementation Status |
| `INSTALLATION_SUCCESS.md` | Binary file INSTALLATION_SUCCESS.md matches |
| `NAVIGATION_GUIDE.md` | ZylCode Navigation Guide |
| `PARALLEL_EXECUTION_MANIFEST.md` | PARALLEL_EXECUTION_MANIFEST.md — Preflight & Coordination |
| `PHASE1_CLOSURE_REPORT.md` | Phase 1 Closure Report |
| `PHASE1_COMPLETE.md` | 🚀 ZylCode Phase 1 Implementation Complete |
| `PHASE1_IMPLEMENTATION_COMPLETE.md` | Phase 1 Implementation: Core Rust Architecture & MCP Bridge |
| `PHASE2_COMPLETE.md` | Phase 2 Implementation Complete: AI Input System & Computer Use System |
| `PHASE2_IMPLEMENTATION_COMPLETE.md` | Phase 2 Implementation Complete: AI Input System & Computer Use System |
| `PHASE2_SUMMARY.md` | 🎉 Phase 2 Implementation Complete! |
| `PHASE4_PROGRESS.md` | Phase 4 Progress: Final Integration & Testing |
| `PROJECT_COMPLETE.md` | 🎉 ZylCode Enhancement Project: COMPLETE |
| `QUICK_REFERENCE.md` | ZylCode Quick Reference Guide |
| `ai-input-implementation.md` | AI Input System Implementation |
| `ai-input-spec.md` | AI Input System Specification |
| `architecture.md` | ZylCode Architecture Design |
| `blueprint.md` | ZylCode: Complete Project Blueprint |
| `competitive-analysis.md` | Competitive Analysis & Strategic Plan for ZylCode |
| `computer-use-implementation.md` | Advanced Computer Use Implementation |
| `computer-use-spec.md` | Advanced Computer Use Specification |
| `deepseek-research.md` | DeepSeek AI Technology Research |
| `enhanced-mcp-bridge.md` | Enhanced MCP Bridge with 100+ Tools |
| `file-upload-implementation.md` | File Upload System Implementation |
| `file-upload-spec.md` | File Upload System Specification |
| `findings-audit.md` | ZylCode Forensic Audit Report |
| `findings.md` | Research Findings Summary |
| `implementation-checklist.md` | ZylCode Implementation Checklist |
| `implementation-roadmap.md` | ZylCode Implementation Roadmap |
| `mcp-bridge-spec.md` | MCP Bridge Specification |
| `openai_codex_research_document.md` | OpenAI Codex: Comprehensive Research Document |
| `openai_codex_summary.md` | OpenAI Codex - Structured Summary |
| `performance-optimization-plan.md` | Performance Optimization Plan for ZylCode |
| `phase1-summary.md` | Phase 1 Implementation Summary |
| `phase2-implementation-plan.md` | Phase 2 Implementation Plan: AI Input System & Computer Use System |
| `phase2-plan.md` | Phase 2 Implementation Plan |
| `phase3-implementation-plan.md` | Phase 3 Implementation Plan: Enhanced MCP Bridge, Skills System & Plugin Marketplace |
| `plugin-marketplace-implementation.md` | Plugin Marketplace Implementation |
| `plugin-marketplace-spec.md` | Plugin Marketplace Specification |
| `project-summary.md` | ZylCode Enhancement Project - Complete Summary |
| `research-driven-strategy.md` | ZylCode: Research-Driven Strategic Implementation |
| `research-findings.md` | AI Coding Ecosystem: Comprehensive Research Findings |
| `skills-system-implementation.md` | Skills System Implementation |
| `skills-system-spec.md` | Skills System Specification |
| `strategic-plan.md` | ZylCode Strategic Plan: Becoming the Market Leader |
| `task_plan.md` | Phase 2A — Repository Intelligence Foundation |

## Added 2026-11-17 — root-level notes triaged

These two files lived untracked in the repository root until this date, so they have no commit
history: they were moved with `mv`, not `git mv`, and their history starts at the commit that added
them here.

| File | Original heading |
| ---- | ---------------- |
| `composio_integration_plan.md` | Composio Integration Plan — ZylCode AI Software Creation OS |
| `MCP_TRUTH_RECONCILIATION.md` | MCP Truth Reconciliation |

**Why quarantined rather than kept live.** `composio_integration_plan.md` is an unimplemented
third-party-integration proposal — the `crates/zylcode-mcp/src/composio_client.rs` and
`crates/zylcode-core/src/user_identity.rs` it specifies do not exist — and it predates Phase 5's
hard rule that the Extension ABI comes before any hard-coded integration.
`MCP_TRUTH_RECONCILIATION.md` is a dated reconciliation written against HEAD `d6ab7c8`. Its
"what exists / what is missing" list was re-checked against the tree on 2026-11-17 and still holds
(no test spawns a real MCP server), but it is an unmaintained snapshot that duplicates
`docs/capability-registry.json`, which is the source of truth. Both are useful as a record of what
was believed and when, and neither may be cited as current capability.

**Deleted in the same sweep, not archived:** `Cargo.lock.bak` and three `Cargo.toml.bak` copies
(superseded pre-`rusqlite`-upgrade backups of tracked files), and
`Phase2B_Completion_Report.md`, which declared Phase 2B "ENGINEERING COMPLETE … CLOSED",
"proven" 3/3 real-server MCP tests (`crates/zylcode-mcp/tests/mcp_real_server.rs` does not exist),
and "no further audit cycles are authorized". That contradicts `docs/governance/phase-status.json`,
where 2B is READY with only the B2 import-closure slice shipped, and it is a builder certifying its
own work. Deleting it removed a false claim rather than a record: the phase's actual state is
recorded in the governance set. `*.bak` and `/screenshots/` are now gitignored.
