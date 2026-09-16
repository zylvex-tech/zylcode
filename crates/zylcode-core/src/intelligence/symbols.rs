//! Symbol extraction from source files.
//!
//! Phase 2A uses regex-based extraction for Rust and TypeScript/JavaScript.
//! This provides DEFINITION_INDEXED support level.
//! Full reference resolution (IMPORT_RESOLVED, REFERENCE_RESOLVED)
//! may be added in a later phase with tree-sitter.
//!
//! Architecture MUST NOT pretend regex extraction is semantic indexing.

use crate::intelligence::types::{Language, Symbol, SymbolKind, Visibility};
use regex::Regex;
use std::path::Path;

/// Extract symbols from a source file.
pub fn extract_symbols(
    file_id: &str,
    _path: &Path,
    content: &str,
    language: &Language,
) -> Vec<Symbol> {
    match language {
        Language::Rust => extract_rust_symbols(file_id, content),
        Language::TypeScript | Language::JavaScript => extract_ts_symbols(file_id, content),
        _ => Vec::new(),
    }
}

/// Extract Rust symbols using regex patterns.
fn extract_rust_symbols(file_id: &str, content: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();

    // Regex patterns for Rust items
    let patterns: Vec<(Regex, SymbolKind, bool)> = vec![
        // pub struct Name
        (
            Regex::new(r"(?m)^(?:pub\s+)?struct\s+(\w+)").unwrap(),
            SymbolKind::Struct,
            true,
        ),
        // pub enum Name
        (
            Regex::new(r"(?m)^(?:pub\s+)?enum\s+(\w+)").unwrap(),
            SymbolKind::Enum,
            true,
        ),
        // pub trait Name
        (
            Regex::new(r"(?m)^(?:pub\s+)?trait\s+(\w+)").unwrap(),
            SymbolKind::Trait,
            true,
        ),
        // impl Name
        (
            Regex::new(r"(?m)^impl(?:<[^>]*>)?\s+(\w+)").unwrap(),
            SymbolKind::Impl,
            false,
        ),
        // pub fn name / pub async fn name
        (
            Regex::new(r"(?m)^(?:pub\s+)?(?:async\s+)?fn\s+(\w+)").unwrap(),
            SymbolKind::Function,
            true,
        ),
        // pub const NAME
        (
            Regex::new(r"(?m)^(?:pub\s+)?const\s+(\w+)").unwrap(),
            SymbolKind::Constant,
            true,
        ),
        // pub type Name
        (
            Regex::new(r"(?m)^(?:pub\s+)?type\s+(\w+)").unwrap(),
            SymbolKind::TypeAlias,
            true,
        ),
        // mod name
        (
            Regex::new(r"(?m)^mod\s+(\w+)").unwrap(),
            SymbolKind::Module,
            false,
        ),
        // macro_rules! name
        (
            Regex::new(r"(?m)^macro_rules!\s+(\w+)").unwrap(),
            SymbolKind::Macro,
            false,
        ),
    ];

    for (regex, kind, check_pub) in &patterns {
        for cap in regex.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
            if name.is_empty() {
                continue;
            }

            let line = content[..cap.get(0).unwrap().start()]
                .chars()
                .filter(|c| *c == '\n')
                .count() as u32
                + 1;

            let visibility = if *check_pub {
                let full_match = cap.get(0).unwrap().as_str();
                if full_match.starts_with("pub") {
                    Visibility::Public
                } else {
                    Visibility::Private
                }
            } else {
                Visibility::Unknown
            };

            let qualified_name = format!(
                "{}::{}",
                file_id.replace('/', "::").replace(".rs", ""),
                name
            );

            symbols.push(Symbol {
                id: qualified_name,
                name,
                kind: kind.clone(),
                file: file_id.to_string(),
                line,
                end_line: None,
                visibility,
                parent: None, // Could be enhanced with impl context
                language: Language::Rust,
                support_level: crate::intelligence::types::LanguageSupport::Parsed,
            });
        }
    }

    // Detect methods inside impl blocks
    let impl_regex = Regex::new(r"(?m)^impl(?:<[^>]*>)?\s+(\w+)").unwrap();
    let method_regex = Regex::new(r"(?m)(?:pub\s+)?(?:async\s+)?fn\s+(\w+)").unwrap();

    for impl_cap in impl_regex.captures_iter(content) {
        let impl_name = impl_cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let impl_start = impl_cap.get(0).unwrap().end();

        // Find the matching closing brace
        let impl_body = find_impl_body(&content[impl_start..]);
        let impl_line = content[..impl_cap.get(0).unwrap().start()]
            .chars()
            .filter(|c| *c == '\n')
            .count() as u32
            + 1;

        for method_cap in method_regex.captures_iter(impl_body) {
            let method_name = method_cap
                .get(1)
                .map(|m| m.as_str())
                .unwrap_or("")
                .to_string();
            if method_name.is_empty() || method_name == "new" || method_name == "default" {
                continue;
            }

            let method_line = impl_body[..method_cap.get(0).unwrap().start()]
                .chars()
                .filter(|c| *c == '\n')
                .count() as u32
                + impl_line;

            let full_match = method_cap.get(0).unwrap().as_str();
            let visibility = if full_match.trim_start().starts_with("pub") {
                Visibility::Public
            } else {
                Visibility::Private
            };

            let parent_id = format!(
                "{}::{}",
                file_id.replace('/', "::").replace(".rs", ""),
                impl_name
            );
            let method_id = format!("{}::{}", parent_id, method_name);

            symbols.push(Symbol {
                id: method_id,
                name: method_name,
                kind: SymbolKind::Method,
                file: file_id.to_string(),
                line: method_line,
                end_line: None,
                visibility,
                parent: Some(parent_id),
                language: Language::Rust,
                support_level: crate::intelligence::types::LanguageSupport::Parsed,
            });
        }
    }

    symbols
}

/// Find the body of an impl block by brace matching.
fn find_impl_body(content: &str) -> &str {
    let mut depth = 0;
    let mut started = false;
    let mut end = 0;
    for (i, ch) in content.char_indices() {
        match ch {
            '{' => {
                depth += 1;
                if !started {
                    started = true;
                }
            }
            '}' => {
                depth -= 1;
                if depth == 0 && started {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    &content[..end]
}

/// Extract TypeScript/JavaScript symbols using regex patterns.
fn extract_ts_symbols(file_id: &str, content: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();

    let patterns: Vec<(Regex, SymbolKind)> = vec![
        // export class Name / class Name
        (
            Regex::new(r"(?m)^(?:export\s+)?(?:default\s+)?class\s+(\w+)").unwrap(),
            SymbolKind::Class,
        ),
        // export interface Name / interface Name
        (
            Regex::new(r"(?m)^(?:export\s+)?interface\s+(\w+)").unwrap(),
            SymbolKind::Interface,
        ),
        // export type Name / type Name
        (
            Regex::new(r"(?m)^(?:export\s+)?type\s+(\w+)").unwrap(),
            SymbolKind::Type,
        ),
        // export function Name / function Name / export async function Name
        (
            Regex::new(r"(?m)^(?:export\s+)?(?:default\s+)?(?:async\s+)?function\s+(\w+)").unwrap(),
            SymbolKind::Function,
        ),
        // export const Name = ... (arrow function or component)
        (
            Regex::new(r"(?m)^export\s+(?:default\s+)?(?:const|let)\s+(\w+)\s*(?::\s*\w+)?\s*=")
                .unwrap(),
            SymbolKind::Variable,
        ),
        // React component detection: export const Name: React.FC
        (
            Regex::new(r"(?m)^export\s+(?:default\s+)?(?:const|let)\s+(\w+)\s*:\s*React\.FC")
                .unwrap(),
            SymbolKind::Component,
        ),
    ];

    for (regex, kind) in &patterns {
        for cap in regex.captures_iter(content) {
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
            if name.is_empty() {
                continue;
            }

            let line = content[..cap.get(0).unwrap().start()]
                .chars()
                .filter(|c| *c == '\n')
                .count() as u32
                + 1;

            let full_match = cap.get(0).unwrap().as_str();
            let visibility = if full_match.starts_with("export") {
                Visibility::Public
            } else {
                Visibility::Private
            };

            let module_name = file_id
                .replace('/', "::")
                .replace(".ts", "")
                .replace(".tsx", "")
                .replace(".js", "")
                .replace(".jsx", "");
            let qualified_name = format!("{}::{}", module_name, name);

            symbols.push(Symbol {
                id: qualified_name,
                name,
                kind: kind.clone(),
                file: file_id.to_string(),
                line,
                end_line: None,
                visibility,
                parent: None,
                language: Language::TypeScript,
                support_level: crate::intelligence::types::LanguageSupport::Parsed,
            });
        }
    }

    symbols
}

/// Find symbols by name (fuzzy matching).
pub fn find_symbols_by_name<'a>(symbols: &'a [Symbol], query: &str) -> Vec<&'a Symbol> {
    let query_lower = query.to_lowercase();
    symbols
        .iter()
        .filter(|s| {
            s.name.to_lowercase().contains(&query_lower)
                || s.id.to_lowercase().contains(&query_lower)
        })
        .collect()
}

/// Find symbol by exact ID.
pub fn find_symbol_by_id<'a>(symbols: &'a [Symbol], id: &str) -> Option<&'a Symbol> {
    symbols.iter().find(|s| s.id == id)
}

/// Get all symbols defined in a specific file.
pub fn symbols_in_file<'a>(symbols: &'a [Symbol], file_id: &str) -> Vec<&'a Symbol> {
    symbols.iter().filter(|s| s.file == file_id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_rust_struct() {
        let content = r#"
pub struct AgentLoop {
    ledger: Arc<dyn LedgerStore>,
}

struct PrivateHelper;
"#;
        let symbols = extract_rust_symbols("src/agent.rs", content);
        let structs: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Struct)
            .collect();
        assert_eq!(structs.len(), 2);
        assert_eq!(structs[0].name, "AgentLoop");
        assert_eq!(structs[0].visibility, Visibility::Public);
        assert_eq!(structs[1].name, "PrivateHelper");
        assert_eq!(structs[1].visibility, Visibility::Private);
    }

    #[test]
    fn extract_rust_enum() {
        let content = r#"
pub enum ExecutionState {
    Planned,
    Approved,
    Started,
}
"#;
        let symbols = extract_rust_symbols("src/ledger.rs", content);
        let enums: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Enum)
            .collect();
        assert_eq!(enums.len(), 1);
        assert_eq!(enums[0].name, "ExecutionState");
    }

    #[test]
    fn extract_rust_trait() {
        let content = r#"
pub trait LedgerStore: Send + Sync {
    async fn append(&self, entry: LedgerEntry) -> Result<()>;
}
"#;
        let symbols = extract_rust_symbols("src/ledger.rs", content);
        let traits: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Trait)
            .collect();
        assert_eq!(traits.len(), 1);
        assert_eq!(traits[0].name, "LedgerStore");
    }

    #[test]
    fn extract_rust_function() {
        let content = r#"
pub fn new(workspace: PathBuf) -> Self {
    Self { workspace }
}

async fn call_model(&self, prompt: &str) -> Result<AgentDecision> {
    todo!()
}
"#;
        let symbols = extract_rust_symbols("src/agent.rs", content);
        let fns: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Function)
            .collect();
        assert_eq!(fns.len(), 2);
        assert_eq!(fns[0].name, "new");
        assert_eq!(fns[1].name, "call_model");
    }

    #[test]
    fn extract_rust_impl_methods() {
        let content = r#"
impl AgentLoop {
    pub fn new(workspace: PathBuf) -> Self {
        Self { workspace }
    }

    pub async fn run(&mut self) -> Result<AgentState> {
        todo!()
    }

    async fn private_helper(&self) -> Result<()> {
        todo!()
    }
}
"#;
        let symbols = extract_rust_symbols("src/agent.rs", content);
        let methods: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Method)
            .collect();
        // run + private_helper (new is skipped by filter)
        assert!(
            methods.len() >= 2,
            "Expected at least 2 methods, got {}",
            methods.len()
        );
        let method_names: Vec<&str> = methods.iter().map(|m| m.name.as_str()).collect();
        assert!(method_names.contains(&"run"), "Should find 'run' method");
        assert!(
            method_names.contains(&"private_helper"),
            "Should find 'private_helper' method"
        );
    }

    #[test]
    fn extract_ts_class() {
        let content = r#"
export class AgentEngine {
    constructor() {}
}

class InternalHelper {}
"#;
        let symbols = extract_ts_symbols("src/engine.ts", content);
        let classes: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Class)
            .collect();
        assert_eq!(classes.len(), 2);
        assert_eq!(classes[0].name, "AgentEngine");
        assert_eq!(classes[0].visibility, Visibility::Public);
        assert_eq!(classes[1].name, "InternalHelper");
        assert_eq!(classes[1].visibility, Visibility::Private);
    }

    #[test]
    fn extract_ts_interface() {
        let content = r#"
export interface ToolSchema {
    id: string;
    description: string;
}
"#;
        let symbols = extract_ts_symbols("src/tools.ts", content);
        let interfaces: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Interface)
            .collect();
        assert_eq!(interfaces.len(), 1);
        assert_eq!(interfaces[0].name, "ToolSchema");
    }

    #[test]
    fn extract_ts_function() {
        let content = r#"
export function processIntent(prompt: string): Promise<Result> {
    return Promise.resolve({} as Result);
}

async function internalHelper() {}
"#;
        let symbols = extract_ts_symbols("src/intent.ts", content);
        let fns: Vec<_> = symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Function)
            .collect();
        assert_eq!(fns.len(), 2);
    }

    #[test]
    fn find_symbols_by_name_query() {
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
                support_level: crate::intelligence::types::LanguageSupport::Parsed,
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
                support_level: crate::intelligence::types::LanguageSupport::Parsed,
            },
        ];

        let results = find_symbols_by_name(&symbols, "agent");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "AgentLoop");
    }

    #[test]
    fn symbols_in_file_query() {
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
                support_level: crate::intelligence::types::LanguageSupport::Parsed,
            },
            Symbol {
                id: "agent::AgentState".to_string(),
                name: "AgentState".to_string(),
                kind: SymbolKind::Enum,
                file: "src/agent.rs".to_string(),
                line: 20,
                end_line: None,
                visibility: Visibility::Public,
                parent: None,
                language: Language::Rust,
                support_level: crate::intelligence::types::LanguageSupport::Parsed,
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
                support_level: crate::intelligence::types::LanguageSupport::Parsed,
            },
        ];

        let results = symbols_in_file(&symbols, "src/agent.rs");
        assert_eq!(results.len(), 2);
    }
}
