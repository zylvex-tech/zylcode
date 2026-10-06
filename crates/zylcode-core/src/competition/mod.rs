//! Competition namespace — **N-ATLAS 2026 branch only**.
//!
//! # Why this namespace exists
//!
//! The N-ATLAS competition work must not contaminate normal ZylCode product
//! development. Everything competition-specific lives under this module so it
//! is trivially separable: deleting `competition/` and one `pub mod` line in
//! `lib.rs` removes the entire surface, and nothing else in the workspace
//! changes.
//!
//! # What it is allowed to do
//!
//! * Reuse existing ZylCode infrastructure (evidence graph, claim store, the
//!   factory task graph) **unchanged**.
//! * Add new, self-contained code that adapts to an external provider
//!   boundary.
//!
//! # What it must never do
//!
//! * Modify the existing provider router to make room for N-ATLAS. The
//!   integration is an *additional* boundary, not a surgery on `router.rs`.
//! * Represent an unavailable external service as available. If N-ATLAS access
//!   is not proven, the state is [`natlas::NatlasStatus::BlockedNatlasAccess`]
//!   and that is the honest answer.
//!
//! # Status vocabulary used throughout
//!
//! | Word | Means |
//! |---|---|
//! | `IMPLEMENTED` | Code exists on this branch. |
//! | `TESTED` | Exercised by a committed test that ran. |
//! | `RUNTIME_VERIFIED` | Exercised against the **real** N-ATLAS service. |
//! | `BLOCKED` | Cannot be reached for a stated reason. Never "not done". |
//! | `PROPOSED` | Specified; nothing built. |
//!
//! **`TESTED` against a mock is not `RUNTIME_VERIFIED`.** No code in this
//! namespace claims the latter, because the N-ATLAS service has never been
//! reached.

pub mod natlas;
