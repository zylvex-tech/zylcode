//! Semantic resolution — imports and cross-file references from a real grammar.
//!
//! [`super::symbols`] extracts definitions with regexes and says so. This module
//! is the other half: [`IMPORT_RESOLVED`] and [`REFERENCE_RESOLVED`] edges
//! produced by a grammar (tree-sitter, through `zylcode-nav`) plus module and
//! import-scope resolution. A regex cannot decide that
//! `use crate::account::Account;` in `ledger.rs` names the `Account` declared in
//! `account.rs`; this can, and every edge records how it was established.
//!
//! ## Fail-closed
//!
//! A language with no grammar is reported as [`UNSUPPORTED_SEMANTIC_INDEX`]; a
//! file the grammar could not parse cleanly is reported as [`PARSE_FAILED`].
//! Neither contributes facts that are then presented as complete: the file is
//! listed in [`SemanticIndex::failures`] and excluded from the coverage
//! denominator's numerator.
//!
//! ## What the coverage number means
//!
//! `resolution_coverage()` is the share of *in-repository* import entries that
//! resolved to a file or symbol here. Imports of registry crates and npm
//! packages are counted separately as `imports_external` and are not failures —
//! they are outside the repository by definition. The denominator is
//! `resolved + unresolved`, so an unresolved import lowers the figure instead of
//! disappearing from it.

use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use zylcode_nav::model::{Evidence, ImportResolution, OccContext, ParseStatus};
use zylcode_nav::queries::attribute;
use zylcode_nav::RepoIndex;

/// An import that resolved to a file or symbol in this repository.
pub const IMPORT_RESOLVED: &str = "IMPORT_RESOLVED";
/// A use of a symbol that resolved to its declaration in this repository.
pub const REFERENCE_RESOLVED: &str = "REFERENCE_RESOLVED";
/// No grammar exists for the file's language.
pub const UNSUPPORTED_SEMANTIC_INDEX: &str = "UNSUPPORTED_SEMANTIC_INDEX";
/// The grammar exists but the file did not parse cleanly.
pub const PARSE_FAILED: &str = "PARSE_FAILED";
/// The grammar parsed the file with error nodes: facts exist but are partial.
///
/// Kept distinct from [`PARSE_FAILED`] because collapsing the two would either
/// hide a real failure behind a common state or overstate what a partial parse
/// supports. Every edge from such a file is marked `from_partial`.
pub const PARSE_PARTIAL: &str = "PARSE_PARTIAL";

/// One resolved import entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedImport {
    /// Importing file, repository-relative.
    pub from_file: String,
    /// 1-based line of the import statement.
    pub line: u32,
    /// Specifier as written (`crate::account`, `./foo`, `react`).
    pub specifier: String,
    /// Name bound in the importing file (after any alias).
    pub local: String,
    /// Name in the source module.
    pub imported: String,
    /// File the import resolved to.
    pub to_file: String,
    /// Symbol the binding resolved to, when it resolved to one.
    pub to_symbol: Option<String>,
    /// Always `DETERMINISTIC` here: a heuristic binding is not emitted as a
    /// resolved import.
    pub evidence: String,
    /// How the binding was established, for audit.
    pub via: String,
    /// True when the importing file parsed with grammar errors, so its fact set
    /// is incomplete even though this edge is real.
    #[serde(default)]
    pub from_partial: bool,
}

/// One reference attributed to a declaration in another file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedReference {
    pub file: String,
    pub line: u32,
    pub name: String,
    /// Symbol id of the declaration this use resolves to.
    pub symbol: String,
    /// How the attribution was made (import binding, module path, scope).
    pub via: String,
    /// True when the referencing file parsed with grammar errors.
    #[serde(default)]
    pub from_partial: bool,
}

/// An import that could not be tied to a repository file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnresolvedImport {
    pub file: String,
    pub line: u32,
    pub specifier: String,
    pub reason: String,
}

/// A file the semantic index refused to pretend about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileFailure {
    pub file: String,
    /// [`UNSUPPORTED_SEMANTIC_INDEX`] or [`PARSE_FAILED`].
    pub code: String,
    pub detail: String,
}

/// Counters behind the coverage figure.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticStats {
    pub files_seen: usize,
    pub files_parsed: usize,
    pub files_partial: usize,
    pub files_failed: usize,
    pub files_unsupported: usize,
    /// Import entries contributed by files that parsed only partially.
    pub imports_from_partial_files: usize,
    /// References attributed inside files that parsed only partially.
    pub references_from_partial_files: usize,
    /// Import entries in files that parsed.
    pub imports_total: usize,
    /// Import entries that resolved to a file or symbol here.
    pub imports_resolved: usize,
    /// Import entries naming a registry crate or npm package.
    pub imports_external: usize,
    /// Import entries that should have resolved here and did not.
    pub imports_unresolved: usize,
    pub references_resolved: usize,
    /// Name-only matches, kept out of `references` on purpose.
    pub references_heuristic: usize,
}

/// The semantic layer's output for one repository.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SemanticIndex {
    pub imports: Vec<ResolvedImport>,
    pub references: Vec<ResolvedReference>,
    pub unresolved: Vec<UnresolvedImport>,
    pub failures: Vec<FileFailure>,
    pub stats: SemanticStats,
}

/// Resolve imports and references for the repository at `root`.
///
/// Fails when `root` is not a directory: an empty index would answer every
/// query with "nothing found", which is a fabricated negative.
pub fn resolve_repository(root: impl AsRef<Path>) -> Result<SemanticIndex> {
    let index = RepoIndex::build(root.as_ref())?;
    Ok(SemanticIndex::from_index(&index))
}

/// The fail-closed decision table for a file's parse status.
///
/// `None` means the file parsed cleanly and nothing is withheld. Kept as a
/// named function so the mapping is unit-tested directly: a directory scan
/// cannot produce every status on demand (tree-sitter recovers from syntax
/// errors, and an unreadable file never reaches the inventory), and a test that
/// pretended otherwise would be testing the fixture, not the policy.
fn classify(status: ParseStatus) -> Option<&'static str> {
    match status {
        ParseStatus::Parsed => None,
        ParseStatus::Partial => Some(PARSE_PARTIAL),
        ParseStatus::Failed => Some(PARSE_FAILED),
        ParseStatus::Unsupported => Some(UNSUPPORTED_SEMANTIC_INDEX),
    }
}

impl SemanticIndex {
    /// Project a navigation index into resolved edges.
    ///
    /// Files are visited in sorted order so two runs produce identical output.
    pub fn from_index(index: &RepoIndex) -> Self {
        let mut out = SemanticIndex::default();
        let mut files: Vec<&String> = index.facts.keys().collect();
        files.sort();

        for file in files {
            let Some(facts) = index.facts.get(file) else {
                continue;
            };
            out.stats.files_seen += 1;

            match facts.status {
                ParseStatus::Parsed => out.stats.files_parsed += 1,
                other => {
                    // Counters first, then one failure row whose code comes from
                    // the decision table, so the table is the only place the
                    // policy is written.
                    let detail = match other {
                        ParseStatus::Partial => {
                            out.stats.files_partial += 1;
                            format!(
                                "grammar reported errors; facts are partial{}",
                                facts
                                    .error
                                    .as_deref()
                                    .map(|e| format!(": {e}"))
                                    .unwrap_or_default()
                            )
                        }
                        ParseStatus::Failed => {
                            out.stats.files_failed += 1;
                            facts
                                .error
                                .clone()
                                .unwrap_or_else(|| "parser failed".to_string())
                        }
                        _ => {
                            out.stats.files_unsupported += 1;
                            format!("no grammar for {}", facts.language.as_str())
                        }
                    };
                    if let Some(code) = classify(other) {
                        out.failures.push(FileFailure {
                            file: file.clone(),
                            code: code.to_string(),
                            detail,
                        });
                    }
                }
            }

            for decl in &facts.imports {
                for entry in &decl.entries {
                    out.stats.imports_total += 1;
                    match &decl.resolution {
                        ImportResolution::Resolved { file: target } => {
                            out.stats.imports_resolved += 1;
                            let binding =
                                index.bindings.get(file).and_then(|m| m.get(&entry.local));
                            let (to_symbol, via) = match binding.map(|b| &b.target) {
                                Some(zylcode_nav::index::BindingTarget::Symbol(id)) => {
                                    (Some(id.clone()), "import binding -> symbol")
                                }
                                Some(zylcode_nav::index::BindingTarget::File(_)) => {
                                    (None, "import binding -> file (glob/namespace)")
                                }
                                _ => (None, "import statement -> file"),
                            };
                            if facts.status == ParseStatus::Partial {
                                out.stats.imports_from_partial_files += 1;
                            }
                            out.imports.push(ResolvedImport {
                                from_file: file.clone(),
                                line: decl.range.start_line,
                                specifier: decl.specifier.clone(),
                                local: entry.local.clone(),
                                imported: entry.imported.clone(),
                                to_file: target.clone(),
                                to_symbol,
                                evidence: Evidence::Deterministic.as_str().to_string(),
                                via: via.to_string(),
                                from_partial: facts.status == ParseStatus::Partial,
                            });
                        }
                        ImportResolution::External { .. } => {
                            out.stats.imports_external += 1;
                        }
                        ImportResolution::Unresolved { reason } => {
                            out.stats.imports_unresolved += 1;
                            out.unresolved.push(UnresolvedImport {
                                file: file.clone(),
                                line: decl.range.start_line,
                                specifier: decl.specifier.clone(),
                                reason: reason.clone(),
                            });
                        }
                    }
                }
            }

            for occ in &facts.occurrences {
                if matches!(occ.ctx, OccContext::Definition | OccContext::Import) {
                    continue;
                }
                let attribution = attribute(index, file, occ);
                match (attribution.symbol, attribution.evidence) {
                    (Some(symbol), Evidence::Deterministic) => {
                        out.stats.references_resolved += 1;
                        if facts.status == ParseStatus::Partial {
                            out.stats.references_from_partial_files += 1;
                        }
                        out.references.push(ResolvedReference {
                            file: file.clone(),
                            line: occ.range.start_line,
                            name: occ.name.clone(),
                            symbol,
                            via: attribution.via,
                            from_partial: facts.status == ParseStatus::Partial,
                        });
                    }
                    (_, Evidence::Heuristic) => out.stats.references_heuristic += 1,
                    _ => {}
                }
            }
        }

        out
    }

    /// Share of in-repository import entries that resolved, as a percentage.
    ///
    /// External package imports are excluded from both sides; unresolved
    /// in-repository imports are in the denominator, so they lower the figure.
    pub fn resolution_coverage(&self) -> f64 {
        let denominator = self.stats.imports_resolved + self.stats.imports_unresolved;
        if denominator == 0 {
            return 0.0;
        }
        (self.stats.imports_resolved as f64 / denominator as f64) * 100.0
    }

    /// References that resolve to `symbol`.
    pub fn references_to(&self, symbol: &str) -> Vec<&ResolvedReference> {
        self.references
            .iter()
            .filter(|r| r.symbol == symbol)
            .collect()
    }

    /// Imports that land in `file`.
    pub fn imports_into(&self, file: &str) -> Vec<&ResolvedImport> {
        self.imports.iter().filter(|i| i.to_file == file).collect()
    }

    /// Failures with the given code.
    pub fn failures_with(&self, code: &str) -> Vec<&FileFailure> {
        self.failures.iter().filter(|f| f.code == code).collect()
    }
}

/// A file-level failure the semantic layer refuses to paper over.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticError {
    /// [`UNSUPPORTED_SEMANTIC_INDEX`] or [`PARSE_FAILED`].
    pub code: String,
    pub file: String,
    pub detail: String,
}

/// Semantics for one explicitly requested file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileSemantics {
    pub file: String,
    /// `PARSED`, `PARTIAL`, or `UNSUPPORTED`.
    pub status: String,
    pub imports: Vec<ResolvedImport>,
    pub references: Vec<ResolvedReference>,
}

/// Resolve one file, or refuse with a fail-closed code.
///
/// A directory scan only inventories languages it has a grammar for, so an
/// unsupported file never appears in [`SemanticIndex::failures`]. A caller can
/// still name one — a CLI argument, an editor buffer, an MCP query — and the
/// answer must be an explicit refusal rather than an empty result that looks
/// like "no imports here".
pub fn resolve_file(
    root: impl AsRef<Path>,
    relative: &str,
) -> Result<FileSemantics, SemanticError> {
    use zylcode_nav::model::Language;

    let path = root.as_ref().join(relative);
    let unsupported = |detail: String| SemanticError {
        code: UNSUPPORTED_SEMANTIC_INDEX.to_string(),
        file: relative.to_string(),
        detail,
    };

    if !Language::from_path(relative).is_supported() {
        return Err(unsupported(format!(
            "no grammar for `{relative}`; supported languages are Rust, TypeScript and JavaScript"
        )));
    }
    if !path.is_file() {
        return Err(unsupported(format!("`{relative}` is not a readable file")));
    }

    let index = RepoIndex::build(root.as_ref()).map_err(|e| SemanticError {
        code: PARSE_FAILED.to_string(),
        file: relative.to_string(),
        detail: format!("cannot index the repository: {e}"),
    })?;

    let key = relative.replace(std::path::MAIN_SEPARATOR, "/");
    let Some(facts) = index.facts.get(&key) else {
        return Err(unsupported(format!(
            "`{relative}` is not in the indexed inventory (ignored, unsupported, or outside the repository)"
        )));
    };

    if facts.status == ParseStatus::Failed {
        return Err(SemanticError {
            code: PARSE_FAILED.to_string(),
            file: key.clone(),
            detail: facts
                .error
                .clone()
                .unwrap_or_else(|| "parser failed".to_string()),
        });
    }

    let whole = SemanticIndex::from_index(&index);
    Ok(FileSemantics {
        file: key.clone(),
        status: match facts.status {
            ParseStatus::Parsed => "PARSED",
            ParseStatus::Partial => "PARTIAL",
            _ => "UNSUPPORTED",
        }
        .to_string(),
        imports: whole
            .imports
            .iter()
            .filter(|i| i.from_file == key)
            .cloned()
            .collect(),
        references: whole
            .references
            .iter()
            .filter(|r| r.file == key)
            .cloned()
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use zylcode_nav::model::ParseStatus;

    #[test]
    fn the_fail_closed_table_names_every_unclean_status() {
        assert_eq!(classify(ParseStatus::Parsed), None);
        assert_eq!(classify(ParseStatus::Partial), Some(PARSE_PARTIAL));
        assert_eq!(classify(ParseStatus::Failed), Some(PARSE_FAILED));
        assert_eq!(
            classify(ParseStatus::Unsupported),
            Some(UNSUPPORTED_SEMANTIC_INDEX)
        );
    }

    #[test]
    fn coverage_excludes_external_imports_and_counts_unresolved_against_it() {
        let index = SemanticIndex {
            stats: SemanticStats {
                imports_resolved: 9,
                imports_unresolved: 1,
                imports_external: 40,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!((index.resolution_coverage() - 90.0).abs() < f64::EPSILON);
    }

    #[test]
    fn an_index_with_nothing_resolvable_reports_zero_rather_than_a_division_error() {
        let index = SemanticIndex::default();
        assert_eq!(index.resolution_coverage(), 0.0);
    }
}
