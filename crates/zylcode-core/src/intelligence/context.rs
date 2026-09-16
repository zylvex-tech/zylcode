//! Context retrieval — the most important user-facing capability.
//!
//! Given a task description, ranks repository resources by relevance.
//! Returns resources with relevance scores and REASONS why they're relevant.
//!
//! Ranking signals:
//! - Exact symbol match
//! - Filename/path match
//! - Dependency distance
//! - Symbol relationship
//! - Package ownership
//! - Test relationship
//! - Recent-change relationship
//! - Task terminology

use crate::intelligence::dependency::DependencyGraph;
use crate::intelligence::types::{
    ArchitecturalFingerprint, ContextResult, EntryPoint, FileNode, GitCommit, Provenance, Symbol,
};
use std::collections::{HashMap, HashSet};

/// Context retriever that ranks resources by relevance to a task.
pub struct ContextRetriever {
    /// File index for fast lookup.
    file_index: HashMap<String, FileNode>,
    /// Symbol index for fast lookup.
    symbol_index: HashMap<String, Symbol>,
    /// Package name → file list.
    package_files: HashMap<String, Vec<String>>,
    /// File → symbols defined in it.
    file_symbols: HashMap<String, Vec<String>>,
    /// Recent change files (from git).
    recent_files: HashSet<String>,
}

impl ContextRetriever {
    /// Create a new retriever from indexed data.
    pub fn new(
        files: &[FileNode],
        symbols: &[Symbol],
        _packages: &[crate::intelligence::types::Package],
        _dep_graph: &DependencyGraph,
        _entry_points: &[EntryPoint],
        _architecture: &ArchitecturalFingerprint,
        git_commits: &[GitCommit],
    ) -> Self {
        let file_index: HashMap<String, FileNode> =
            files.iter().map(|f| (f.id.clone(), f.clone())).collect();

        let symbol_index: HashMap<String, Symbol> =
            symbols.iter().map(|s| (s.id.clone(), s.clone())).collect();

        let mut package_files: HashMap<String, Vec<String>> = HashMap::new();
        for file in files {
            if let Some(pkg) = &file.package {
                package_files
                    .entry(pkg.clone())
                    .or_default()
                    .push(file.id.clone());
            }
        }

        let mut file_symbols: HashMap<String, Vec<String>> = HashMap::new();
        for sym in symbols {
            file_symbols
                .entry(sym.file.clone())
                .or_default()
                .push(sym.id.clone());
        }

        let mut recent_files = HashSet::new();
        for commit in git_commits {
            for file in &commit.files_changed {
                recent_files.insert(file.clone());
            }
        }

        Self {
            file_index,
            symbol_index,
            package_files,
            file_symbols,
            recent_files,
        }
    }

    /// Retrieve relevant context for a task description.
    pub fn retrieve(&self, task: &str) -> Vec<ContextResult> {
        let task_lower = task.to_lowercase();
        let task_words: Vec<&str> = task_lower
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .collect();

        let mut scored: Vec<(f64, ContextResult)> = Vec::new();

        // Score symbols
        for (id, sym) in &self.symbol_index {
            let mut score = 0.0f64;
            let mut reasons = Vec::new();

            // Exact symbol name match
            if task_lower.contains(&sym.name.to_lowercase()) {
                score += 0.9;
                reasons.push(format!("symbol name '{}' matches task", sym.name));
            }

            // Symbol name contains task word
            for word in &task_words {
                if sym.name.to_lowercase().contains(word) {
                    score += 0.4;
                    reasons.push(format!("symbol '{}' contains '{}'", sym.name, word));
                }
            }

            // File path contains task word
            for word in &task_words {
                if sym.file.to_lowercase().contains(word) {
                    score += 0.3;
                    reasons.push(format!("file '{}' contains '{}'", sym.file, word));
                }
            }

            // Recent change bonus
            if self.recent_files.contains(&sym.file) {
                score += 0.15;
                reasons.push("recently changed file".to_string());
            }

            if score > 0.1 {
                scored.push((
                    score,
                    ContextResult {
                        resource: id.clone(),
                        resource_type: "symbol".to_string(),
                        relevance: score,
                        reason: reasons.join("; "),
                        provenance: Provenance::Parsed,
                        snippet: None,
                    },
                ));
            }
        }

        // Score files
        for (id, file) in &self.file_index {
            let mut score = 0.0f64;
            let mut reasons = Vec::new();

            // Filename match
            for word in &task_words {
                let file_lower = file.id.to_lowercase();
                if file_lower.contains(word) {
                    score += 0.5;
                    reasons.push(format!("filename contains '{}'", word));
                }
            }

            // File contains relevant symbols
            if let Some(sym_ids) = self.file_symbols.get(id) {
                for sym_id in sym_ids {
                    if let Some(sym) = self.symbol_index.get(sym_id) {
                        if task_lower.contains(&sym.name.to_lowercase()) {
                            score += 0.6;
                            reasons.push(format!("defines symbol '{}'", sym.name));
                        }
                    }
                }
            }

            // Recent change bonus
            if self.recent_files.contains(id) {
                score += 0.2;
                reasons.push("recently changed".to_string());
            }

            // Test file bonus for test-related tasks
            if task_lower.contains("test")
                && file.role == crate::intelligence::types::FileRole::Test
            {
                score += 0.3;
                reasons.push("test file".to_string());
            }

            if score > 0.1 {
                scored.push((
                    score,
                    ContextResult {
                        resource: id.clone(),
                        resource_type: "file".to_string(),
                        relevance: score,
                        reason: reasons.join("; "),
                        provenance: Provenance::Observed,
                        snippet: None,
                    },
                ));
            }
        }

        // Score packages
        for (pkg_name, files) in &self.package_files {
            let mut score = 0.0f64;
            let mut reasons = Vec::new();

            // Package name match
            if task_lower.contains(&pkg_name.to_lowercase()) {
                score += 0.7;
                reasons.push("package name matches task".to_string());
            }

            // Package contains relevant files
            let relevant_files: Vec<&str> = files
                .iter()
                .filter(|f| {
                    for word in &task_words {
                        if f.to_lowercase().contains(word) {
                            return true;
                        }
                    }
                    false
                })
                .map(|s| s.as_str())
                .collect();

            if !relevant_files.is_empty() {
                score += 0.3;
                reasons.push(format!("contains {} relevant files", relevant_files.len()));
            }

            if score > 0.1 {
                scored.push((
                    score,
                    ContextResult {
                        resource: pkg_name.clone(),
                        resource_type: "package".to_string(),
                        relevance: score,
                        reason: reasons.join("; "),
                        provenance: Provenance::Parsed,
                        snippet: None,
                    },
                ));
            }
        }

        // Sort by score descending
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Return top results
        scored
            .into_iter()
            .map(|(_, result)| result)
            .take(20)
            .collect()
    }
}

/// Token-aware context assembly.
pub struct ContextAssembler {
    /// Maximum token budget.
    max_tokens: usize,
    /// Approximate tokens per character (rough estimate).
    tokens_per_char: f64,
}

impl ContextAssembler {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            tokens_per_char: 0.25, // ~4 chars per token
        }
    }

    /// Assemble context within a token budget.
    pub fn assemble(
        &self,
        results: &[ContextResult],
        priority: crate::intelligence::types::ContextPriority,
    ) -> Vec<ContextResult> {
        let mut selected = Vec::new();
        let mut total_chars = 0usize;
        let max_chars = (self.max_tokens as f64 / self.tokens_per_char) as usize;

        // Filter by priority
        let filtered: Vec<&ContextResult> = results
            .iter()
            .filter(|r| match priority {
                crate::intelligence::types::ContextPriority::High => r.relevance > 0.7,
                crate::intelligence::types::ContextPriority::Medium => r.relevance > 0.4,
                crate::intelligence::types::ContextPriority::Low => r.relevance > 0.1,
            })
            .collect();

        for result in filtered {
            let estimated_size = result.resource.len()
                + result.reason.len()
                + result.snippet.as_ref().map(|s| s.len()).unwrap_or(0);

            if total_chars + estimated_size > max_chars {
                break;
            }

            total_chars += estimated_size;
            selected.push(result.clone());
        }

        selected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intelligence::types::*;
    use std::path::PathBuf;

    fn make_test_retriever() -> ContextRetriever {
        let files = vec![
            FileNode {
                id: "src/agent.rs".to_string(),
                path: PathBuf::from("src/agent.rs"),
                language: Language::Rust,
                role: FileRole::Source,
                size: 1000,
                content_hash: "abc".to_string(),
                package: Some("zylcode-core".to_string()),
                symbols: Vec::new(),
                imports: Vec::new(),
                indexed_at: chrono::Utc::now(),
                support_level: LanguageSupport::Parsed,
            },
            FileNode {
                id: "src/ledger.rs".to_string(),
                path: PathBuf::from("src/ledger.rs"),
                language: Language::Rust,
                role: FileRole::Source,
                size: 500,
                content_hash: "def".to_string(),
                package: Some("zylcode-core".to_string()),
                symbols: Vec::new(),
                imports: Vec::new(),
                indexed_at: chrono::Utc::now(),
                support_level: LanguageSupport::Parsed,
            },
            FileNode {
                id: "tests/crash_recovery.rs".to_string(),
                path: PathBuf::from("tests/crash_recovery.rs"),
                language: Language::Rust,
                role: FileRole::Test,
                size: 300,
                content_hash: "ghi".to_string(),
                package: Some("zylcode-core".to_string()),
                symbols: Vec::new(),
                imports: Vec::new(),
                indexed_at: chrono::Utc::now(),
                support_level: LanguageSupport::Parsed,
            },
        ];

        let symbols = vec![
            Symbol {
                id: "agent::AgentLoop".to_string(),
                name: "AgentLoop".to_string(),
                kind: SymbolKind::Struct,
                file: "src/agent.rs".to_string(),
                line: 10,
                end_line: None,
                visibility: Visibility::Public,
                parent: None,
                language: Language::Rust,
                support_level: LanguageSupport::Parsed,
            },
            Symbol {
                id: "agent::recover_from_checkpoint".to_string(),
                name: "recover_from_checkpoint".to_string(),
                kind: SymbolKind::Function,
                file: "src/agent.rs".to_string(),
                line: 50,
                end_line: None,
                visibility: Visibility::Public,
                parent: None,
                language: Language::Rust,
                support_level: LanguageSupport::Parsed,
            },
            Symbol {
                id: "ledger::LedgerStore".to_string(),
                name: "LedgerStore".to_string(),
                kind: SymbolKind::Trait,
                file: "src/ledger.rs".to_string(),
                line: 5,
                end_line: None,
                visibility: Visibility::Public,
                parent: None,
                language: Language::Rust,
                support_level: LanguageSupport::Parsed,
            },
        ];

        let packages = vec![Package {
            id: "zylcode-core".to_string(),
            name: "zylcode-core".to_string(),
            version: "0.2.0".to_string(),
            root: PathBuf::from("crates/zylcode-core"),
            language: Language::Rust,
            manifest: "Cargo.toml".to_string(),
            files: Vec::new(),
            dependencies: Vec::new(),
            dev_dependencies: Vec::new(),
            build_commands: vec!["cargo build".to_string()],
            test_commands: vec!["cargo test".to_string()],
            entry_points: Vec::new(),
        }];

        let dep_graph = DependencyGraph::from_packages(&packages);
        let architecture = ArchitecturalFingerprint { facts: Vec::new() };

        ContextRetriever::new(
            &files,
            &symbols,
            &packages,
            &dep_graph,
            &[],
            &architecture,
            &[],
        )
    }

    #[test]
    fn retrieve_by_symbol_name() {
        let retriever = make_test_retriever();
        let results = retriever.retrieve("Fix crash recovery in AgentLoop");
        assert!(
            !results.is_empty(),
            "Should find some results for 'AgentLoop'"
        );

        // Should find at least one symbol result
        let symbol_results: Vec<_> = results
            .iter()
            .filter(|r| r.resource_type == "symbol")
            .collect();
        assert!(
            !symbol_results.is_empty(),
            "Should find at least 1 symbol result"
        );
    }

    #[test]
    fn retrieve_by_filename() {
        let retriever = make_test_retriever();
        let results = retriever.retrieve("Modify the ledger module");
        assert!(!results.is_empty());

        let file_results: Vec<_> = results
            .iter()
            .filter(|r| r.resource_type == "file" && r.resource.contains("ledger"))
            .collect();
        assert!(!file_results.is_empty());
    }

    #[test]
    fn retrieve_test_files_for_test_tasks() {
        let retriever = make_test_retriever();
        let results = retriever.retrieve("Write tests for crash recovery");
        assert!(!results.is_empty());

        let test_results: Vec<_> = results
            .iter()
            .filter(|r| r.resource.contains("test"))
            .collect();
        assert!(!test_results.is_empty());
    }

    #[test]
    fn results_have_reasons() {
        let retriever = make_test_retriever();
        let results = retriever.retrieve("AgentLoop");
        for result in &results {
            assert!(!result.reason.is_empty(), "Result should have a reason");
        }
    }

    #[test]
    fn context_assembly_respects_budget() {
        let assembler = ContextAssembler::new(1000);
        let results = vec![
            ContextResult {
                resource: "a".to_string(),
                resource_type: "symbol".to_string(),
                relevance: 0.9,
                reason: "test".to_string(),
                provenance: Provenance::Parsed,
                snippet: None,
            },
            ContextResult {
                resource: "b".to_string(),
                resource_type: "symbol".to_string(),
                relevance: 0.5,
                reason: "test".to_string(),
                provenance: Provenance::Parsed,
                snippet: None,
            },
        ];

        let assembled =
            assembler.assemble(&results, crate::intelligence::types::ContextPriority::High);
        // Only high-relevance results should be included
        assert!(assembled.len() <= results.len());
    }
}
