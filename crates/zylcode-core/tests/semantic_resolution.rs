//! D1 acceptance tests: semantic resolution of imports and cross-file references.
//!
//! Phase 2B's D1 row requires `IMPORT_RESOLVED` and `REFERENCE_RESOLVED` to be
//! produced by a real resolver — not a regex — and the fail-closed codes
//! `UNSUPPORTED_SEMANTIC_INDEX` / `PARSE_FAILED` to be produced for input the
//! index cannot honestly resolve. These tests drive the public API.

use std::path::PathBuf;

use zylcode_core::intelligence::semantic::{
    resolve_file, resolve_repository, IMPORT_RESOLVED, PARSE_FAILED, PARSE_PARTIAL,
    REFERENCE_RESOLVED, UNSUPPORTED_SEMANTIC_INDEX,
};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/semantic")
        .join(name)
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

// ---------------------------------------------------------------------------
// Positive fixture: a two-module crate whose imports and references cross files
// ---------------------------------------------------------------------------

#[test]
fn an_import_resolves_to_the_declaring_file_and_symbol() {
    let index = resolve_repository(fixture("positive")).expect("index the positive fixture");

    let imports: Vec<_> = index
        .imports
        .iter()
        .filter(|i| i.from_file.ends_with("ledger.rs"))
        .collect();
    assert_eq!(
        imports.len(),
        1,
        "expected exactly one resolved import from ledger.rs, got {:#?}",
        index.imports
    );

    let ledger = imports[0];
    assert_eq!(IMPORT_RESOLVED, "IMPORT_RESOLVED");
    assert!(
        ledger.to_file.ends_with("account.rs"),
        "`use crate::account::Account` must resolve to account.rs, got {}",
        ledger.to_file
    );
    assert!(
        ledger
            .to_symbol
            .as_deref()
            .unwrap_or_default()
            .contains("Account"),
        "the import must resolve to the Account symbol, got {:?}",
        ledger.to_symbol
    );
    assert_eq!(
        ledger.evidence, "DETERMINISTIC",
        "an import must not be reported as heuristic"
    );
    assert_eq!(ledger.line, 1, "the edge must carry the statement's line");
    assert!(
        ledger.via.contains("symbol"),
        "the edge must say how it resolved, got {:?}",
        ledger.via
    );
    assert_eq!(
        index.resolution_coverage(),
        100.0,
        "every in-repository import in this fixture resolves"
    );
}

#[test]
fn references_resolve_across_files_and_carry_their_via() {
    let index = resolve_repository(fixture("positive")).expect("index the positive fixture");
    let ledger_import = index
        .imports
        .iter()
        .find(|i| i.from_file.ends_with("ledger.rs"))
        .expect("ledger import");
    let symbol = ledger_import
        .to_symbol
        .clone()
        .expect("ledger import resolves to a symbol");

    assert_eq!(REFERENCE_RESOLVED, "REFERENCE_RESOLVED");
    let references = index.references_to(&symbol);
    assert!(
        !references.is_empty(),
        "expected references to {symbol}, got {:#?}",
        index.references
    );
    assert!(
        references.iter().any(|r| r.file.ends_with("ledger.rs")),
        "the type use inside ledger.rs must resolve to the declaration, got {:#?}",
        references
    );
    assert!(
        references.iter().all(|r| !r.via.is_empty() && r.line > 0),
        "every reference must record how it resolved and where"
    );
    assert!(
        references.iter().all(|r| r.symbol == symbol),
        "a reference must point at exactly the symbol it resolved to"
    );
}

// ---------------------------------------------------------------------------
// Negative fixtures: fail closed, and say so
// ---------------------------------------------------------------------------

#[test]
fn an_unsupported_language_is_refused_rather_than_answered_empty() {
    let root = fixture("unsupported");

    // The scan only inventories languages it has a grammar for, so a Python file
    // never appears in the index at all.
    let index = resolve_repository(&root).expect("index the unsupported fixture");
    assert_eq!(index.stats.files_seen, 0);
    assert!(index.imports.is_empty() && index.references.is_empty());

    // Naming it explicitly must produce the fail-closed code, not an empty
    // answer that reads as "this file imports nothing".
    let error = resolve_file(&root, "notes.py").expect_err("notes.py has no grammar");
    assert_eq!(error.code, UNSUPPORTED_SEMANTIC_INDEX);
    assert_eq!(error.file, "notes.py");
    assert!(
        error.detail.contains("no grammar"),
        "the refusal must say why: {}",
        error.detail
    );
}

#[test]
fn a_syntax_error_is_reported_as_partial_not_as_clean() {
    let index = resolve_repository(fixture("parse_failed")).expect("index the broken fixture");

    // tree-sitter recovers from syntax errors, so the honest report is
    // PARSE_PARTIAL, not silence and not "parsed".
    let failures = index.failures_with(PARSE_PARTIAL);
    assert!(
        failures.iter().any(|f| f.file.ends_with("lib.rs")),
        "a file with syntax errors must be reported, got {:#?}",
        index.failures
    );
    assert_eq!(index.stats.files_partial, 1);
    assert_eq!(index.stats.files_parsed, 0);
    assert!(
        index
            .imports
            .iter()
            .filter(|i| i.from_file.ends_with("lib.rs"))
            .all(|i| i.from_partial),
        "every edge from a partially parsed file must carry the label"
    );
}

// A file that cannot be decoded is excluded from the inventory by the scanner.
// It must not appear as an empty file, and naming it must produce a refusal
// rather than a zero-import answer. `PARSE_FAILED` itself is the mapping for a
// hard parser failure; that mapping is unit-tested in `semantic.rs`, because a
// scan cannot manufacture the state on demand.
#[test]
fn an_undecodable_source_file_is_not_silently_treated_as_empty() {
    let root = fixture("invalid_utf8");
    let index = resolve_repository(&root).expect("index the invalid-UTF-8 fixture");

    assert_eq!(
        index.stats.files_seen, 0,
        "the scanner must not invent a file"
    );
    assert!(index.imports.is_empty() && index.references.is_empty());

    let error = resolve_file(&root, "src/lib.rs").expect_err("an undecodable file is not readable");
    assert_eq!(error.file, "src/lib.rs");
    assert!(
        error.detail.contains("not in the indexed inventory"),
        "the refusal must explain the absence: {}",
        error.detail
    );
}

// ---------------------------------------------------------------------------
// This repository
// ---------------------------------------------------------------------------

#[test]
fn this_repository_resolves_its_own_imports() {
    let index = resolve_repository(repo_root()).expect("index this repository");

    println!(
        "semantic resolution: {:.2}% of {} in-repository imports resolved \
         ({} external, {} unresolved); {} references resolved, {} heuristic-only; \
         files: {} parsed, {} partial, {} failed, {} unsupported",
        index.resolution_coverage(),
        index.stats.imports_resolved + index.stats.imports_unresolved,
        index.stats.imports_external,
        index.stats.imports_unresolved,
        index.stats.references_resolved,
        index.stats.references_heuristic,
        index.stats.files_parsed,
        index.stats.files_partial,
        index.stats.files_failed,
        index.stats.files_unsupported,
    );

    assert!(
        index.stats.imports_resolved > 100,
        "the workspace has far more than 100 resolving imports; got {}",
        index.stats.imports_resolved
    );
    assert!(
        index
            .imports
            .iter()
            .any(|i| i.to_file.contains("zylcode-nav")),
        "zylcode-core imports zylcode-nav; that edge must resolve"
    );
    assert!(
        index.stats.references_resolved > 0,
        "cross-file references must resolve in a workspace this size"
    );

    // Fail-closed policy is only useful if it is quiet where it should be.
    // Fixtures are deliberately broken, so they are excluded by path.
    let unexpected: Vec<_> = index
        .failures
        .iter()
        .filter(|f| !f.file.contains("tests/fixtures"))
        .filter(|f| f.code == PARSE_FAILED)
        .collect();
    assert!(
        unexpected.is_empty(),
        "no source file outside the fixtures should fail to parse: {:#?}",
        unexpected
    );

    // A partial parse is reported as PARSE_PARTIAL, and the edges it produces
    // are labelled, never counted silently as complete.
    let partial = index.failures_with(PARSE_PARTIAL);
    assert_eq!(partial.len(), index.stats.files_partial);
    assert!(
        index.imports.iter().filter(|i| i.from_partial).count()
            == index.stats.imports_from_partial_files,
        "every edge from a partial file must carry the label"
    );

    // Coverage must stay above the measured floor, so a regression fails a test
    // rather than shrinking a number nobody reads.
    assert!(
        index.resolution_coverage() >= 95.0,
        "resolution coverage regressed to {:.2}%",
        index.resolution_coverage()
    );
    assert!(
        index.stats.references_resolved > 5_000,
        "far more than 5,000 cross-file references resolve in this workspace; got {}",
        index.stats.references_resolved
    );
}
