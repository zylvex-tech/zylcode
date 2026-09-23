//! Repository Intelligence — types, scanner, manifest parsing, symbol graph,
//! dependency graph, entry-point discovery, architectural fingerprinting,
//! git/change intelligence, persistence, and query API.
//!
//! This module enables ZylCode to construct, persist, query, update, and verify
//! a structured model of a software repository.

pub mod architecture;
pub mod classifier;
pub mod context;
pub mod dependency;
pub mod entry_points;
pub mod git;
pub mod manifest;
pub mod persisted;
pub mod query;
pub mod scanner;
pub mod store;
pub mod symbols;
pub mod types;
