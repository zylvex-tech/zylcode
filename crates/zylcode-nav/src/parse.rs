//! Grammar-based extraction.
//!
//! Every fact here comes from a `tree_sitter` parse tree. There is no regex
//! sweep over source text anywhere in this module: comments, string literals
//! and attribute bodies are not identifier occurrences, and a regex cannot
//! tell them apart.
//!
//! The central construct is the **scope stack**: a stack of qualified scope
//! names, root first (`["crate", "crate::a", "crate::a::main"]`). A symbol's
//! qualified name is `<parent scope>::<name>`, so the symbol's ancestor chain
//! is exactly the prefix list of that qualified name. A bare-name occurrence
//! sitting in scope `S` is attributable to a symbol when the symbol's ancestors
//! are a prefix of `S`'s prefix list. That one rule resolves same-file
//! references, duplicate names across modules, and nested containers without
//! guessing — and when it does not apply, the occurrence is *not* claimed.

use crate::model::*;
use anyhow::Result;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

/// Extraction result for one file, before any cross-file resolution.
#[derive(Debug, Default)]
pub struct ParsedFile {
    pub symbols: Vec<Symbol>,
    pub imports: Vec<ImportDecl>,
    pub exports: Vec<ExportDecl>,
    pub call_sites: Vec<CallSite>,
    pub occurrences: Vec<Occurrence>,
    /// Interned scope names referenced by [`Occurrence::scope_idx`].
    pub scopes: Vec<String>,
    /// Extra qualified module names that resolve to this file (Rust inline
    /// `mod x { .. }` blocks). The path-derived module is separate.
    pub module_aliases: Vec<String>,
    pub status: ParseStatus,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Parse `source` and extract symbols, imports, exports, calls and identifier
/// occurrences.
///
/// `module` is the file's path-derived module path (`crate::intelligence::nav`
/// for Rust, `src::components::shell::SurfaceHost` for TS). `crate_key`
/// disambiguates the same module path appearing in two packages.
pub fn parse(
    source: &str,
    file: &str,
    language: &Language,
    module: &str,
    crate_key: &str,
) -> Result<ParsedFile> {
    if !language.is_supported() {
        return Ok(ParsedFile {
            status: ParseStatus::Unsupported,
            error: Some(format!("no grammar for {}", language.as_str())),
            ..Default::default()
        });
    }

    let mut parser = tree_sitter::Parser::new();
    // `.tsx` is TypeScript *plus JSX*, which the TypeScript grammar cannot
    // parse. Both grammars are already linked; selecting the wrong one marked
    // 94 of this repository's 97 `.tsx` files as partially parsed even though a
    // TSX pass over them reports no error node at all.
    let is_tsx = file.to_ascii_lowercase().ends_with(".tsx");
    let lang: tree_sitter::Language = match language {
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::TypeScript if is_tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        Language::Unsupported(_) => unreachable!("guarded above"),
    };
    if parser.set_language(&lang).is_err() {
        return Ok(ParsedFile {
            status: ParseStatus::Failed,
            error: Some("grammar failed to load".into()),
            ..Default::default()
        });
    }
    // A UTF-8 BOM is not part of any grammar (tree-sitter reports an error on
    // it). Parse the text after it and shift every byte offset back, so ranges
    // still address the file as it exists on disk. The file itself is never
    // rewritten.
    let (text, byte_base) = match source.strip_prefix('\u{feff}') {
        Some(rest) => (rest, '\u{feff}'.len_utf8() as u32),
        None => (source, 0),
    };
    let tree = match parser.parse(text, None) {
        Some(t) => t,
        None => {
            return Ok(ParsedFile {
                status: ParseStatus::Failed,
                error: Some("parser returned no tree".into()),
                ..Default::default()
            });
        }
    };
    let root = tree.root_node();
    let has_error = root.has_error();
    // Name the first construct the grammar could not read. A bare "syntax
    // errors" message makes a grammar gap indistinguishable from a genuinely
    // broken file, which is exactly how 94 mislabelled `.tsx` files went
    // unnoticed.
    let first_error = has_error.then(|| first_error_site(root, text));

    let mut out = ParsedFile {
        status: if has_error {
            ParseStatus::Partial
        } else {
            ParseStatus::Parsed
        },
        error: first_error.map(|(line, col, snippet)| {
            format!(
                "grammar reported syntax errors at {line}:{col} near `{snippet}`; extracted facts are partial"
            )
        }),
        ..Default::default()
    };

    // Initial scope stack: every `::`-prefix of the file's module path.
    let segments: Vec<&str> = module
        .split("::")
        .filter(|s| !s.is_empty() && *s != "crate")
        .collect();
    let mut stack: Vec<String> = Vec::new();
    match language {
        Language::Rust => {
            stack.push("crate".to_string());
            let mut acc = "crate".to_string();
            for seg in segments {
                acc.push_str("::");
                acc.push_str(seg);
                stack.push(acc.clone());
            }
        }
        _ => {
            let mut acc = String::new();
            for seg in segments {
                if !acc.is_empty() {
                    acc.push_str("::");
                }
                acc.push_str(seg);
                stack.push(acc.clone());
            }
            if stack.is_empty() {
                stack.push(file.to_string());
            }
        }
    }
    let module_scope = stack.last().cloned().unwrap_or_default();

    let mut cx = WalkCx {
        file: file.to_string(),
        language: language.clone(),
        crate_key: crate_key.to_string(),
        bytes: text.as_bytes(),
        byte_base,
        stack,
        module_scope,
        shadows: Vec::new(),
        table: Vec::new(),
        table_map: HashMap::new(),
        out: &mut out,
    };
    cx.walk(root);
    Ok(out)
}

/// First error or missing node in the tree, as `(line, column, source snippet)`.
///
/// Prefers a real `ERROR` node (a construct the grammar rejected) over a
/// `MISSING` node (a token the parser inserted), and reports the innermost
/// occurrence so the snippet is the smallest failing construct.
fn first_error_site(root: tree_sitter::Node, text: &str) -> (usize, usize, String) {
    fn seek(node: tree_sitter::Node) -> Option<tree_sitter::Node> {
        if node.is_error() || node.is_missing() {
            return Some(node);
        }
        let mut cursor = node.walk();
        let mut children: Vec<tree_sitter::Node> = node.children(&mut cursor).collect();
        // ERROR nodes sort before MISSING ones at the same depth.
        children.sort_by_key(|c| (!c.is_error(), c.is_missing()));
        for child in children {
            if !child.has_error() && !child.is_missing() && !child.is_error() {
                continue;
            }
            if let Some(found) = seek(child) {
                return Some(found);
            }
        }
        None
    }

    let Some(node) = seek(root) else {
        return (1, 1, String::new());
    };
    let position = node.start_position();
    let raw = node.utf8_text(text.as_bytes()).unwrap_or("");
    let snippet: String = raw
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or(if node.is_missing() { node.kind() } else { "" })
        .chars()
        .take(60)
        .collect();
    (position.row + 1, position.column + 1, snippet)
}

// ---------------------------------------------------------------------------
// Walk context
// ---------------------------------------------------------------------------

/// Node kinds that open a lexical block for shadow analysis.
const BLOCK_KINDS: &[&str] = &[
    "block",
    "statement_block",
    "class_body",
    "for_statement",
    "for_in_statement",
];

/// Node kinds whose parameters bind for the whole node.
const FUNCTION_KINDS: &[&str] = &[
    "function_item",
    "function_declaration",
    "generator_function_declaration",
    "method_definition",
    "arrow_function",
    "closure_expression",
    "function_expression",
];

/// Identifier-ish node kinds worth recording as occurrences.
fn is_identifier_kind(kind: &str) -> bool {
    matches!(
        kind,
        "identifier"
            | "type_identifier"
            | "field_identifier"
            | "property_identifier"
            | "class_identifier"
            | "shorthand_property_identifier"
            | "shorthand_property_identifier_pattern"
    )
}

/// Node kinds that name a *property* rather than a value. Without a receiver
/// these never denote a reference to a module-level symbol, so they are only
/// kept when they are a declaration's own name.
fn is_property_kind(kind: &str) -> bool {
    matches!(
        kind,
        "field_identifier"
            | "property_identifier"
            | "shorthand_property_identifier"
            | "shorthand_property_identifier_pattern"
    )
}

/// What a declaration contributes *besides* the symbol it pushes onto the
/// index: the child scope its body opens, and whether that scope is an inline
/// module (which is also a resolvable module path).
#[derive(Debug, Default)]
struct DeclOutcome {
    new_scope: Option<String>,
    inline_module: bool,
}

struct WalkCx<'a> {
    file: String,
    language: Language,
    crate_key: String,
    bytes: &'a [u8],
    /// Byte offset of the parsed text inside the file (3 when a UTF-8 BOM was
    /// skipped, otherwise 0). Added to every range so offsets stay file-relative.
    byte_base: u32,
    /// Qualified name of each enclosing scope, root first; last is current.
    stack: Vec<String>,
    /// The file's own module scope (last element of the initial stack).
    module_scope: String,
    /// Enclosing shadow scopes: (range, bindings) where a binding is
    /// (name, byte from which it is in scope).
    shadows: Vec<(TextRange, Vec<(String, u32)>)>,
    /// Interned scope table for occurrences.
    table: Vec<String>,
    table_map: HashMap<String, u32>,
    out: &'a mut ParsedFile,
}

impl<'a> WalkCx<'a> {
    // -- primitive helpers -------------------------------------------------

    fn text(&self, node: tree_sitter::Node) -> String {
        node.utf8_text(self.bytes).unwrap_or("").to_string()
    }

    fn range(&self, node: tree_sitter::Node) -> TextRange {
        let sp = node.start_position();
        let ep = node.end_position();
        TextRange {
            start_byte: node.start_byte() as u32 + self.byte_base,
            end_byte: node.end_byte() as u32 + self.byte_base,
            start_line: (sp.row + 1) as u32,
            start_col: (sp.column + 1) as u32,
            end_line: (ep.row + 1) as u32,
            end_col: (ep.column + 1) as u32,
        }
    }

    /// First non-empty line of `node`'s text, trimmed — used for excerpts.
    fn excerpt(&self, node: tree_sitter::Node) -> String {
        self.text(node)
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("")
            .trim()
            .to_string()
    }

    fn named_children<'b>(&self, node: tree_sitter::Node<'b>) -> Vec<tree_sitter::Node<'b>> {
        let mut cur = node.walk();
        node.named_children(&mut cur).collect()
    }

    fn name_node<'b>(&self, node: tree_sitter::Node<'b>) -> Option<tree_sitter::Node<'b>> {
        if let Some(n) = node.child_by_field_name("name") {
            return Some(n);
        }
        self.named_children(node)
            .into_iter()
            .find(|c| is_identifier_kind(c.kind()))
    }

    /// Declaration signature: the header with the body removed.
    fn signature(&self, node: tree_sitter::Node) -> Option<String> {
        let body = node.child_by_field_name("body");
        let end = body.map(|b| b.start_byte()).unwrap_or(node.end_byte());
        let raw = &self.bytes[node.start_byte()..end.min(self.bytes.len())];
        let collapsed = String::from_utf8_lossy(raw)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if collapsed.is_empty() {
            None
        } else {
            Some(collapsed.chars().take(400).collect())
        }
    }

    fn visibility(&self, node: tree_sitter::Node) -> Visibility {
        if self.language != Language::Rust {
            return Visibility::Unknown;
        }
        let mut cur = node.walk();
        for child in node.children(&mut cur) {
            if child.kind() == "visibility_modifier" {
                let t = self.text(child);
                return if t.contains("crate") {
                    Visibility::PubCrate
                } else if t.contains("super") {
                    Visibility::PubSuper
                } else {
                    Visibility::Public
                };
            }
        }
        Visibility::Private
    }

    fn ancestors_have(&self, mut node: Option<tree_sitter::Node>, kinds: &[&str]) -> bool {
        let mut depth = 0;
        while let Some(n) = node {
            if depth > 60 {
                break;
            }
            if kinds.contains(&n.kind()) {
                return true;
            }
            node = n.parent();
            depth += 1;
        }
        false
    }

    fn scope_id(&mut self, scope: &str) -> u32 {
        if let Some(idx) = self.table_map.get(scope) {
            return *idx;
        }
        let idx = self.table.len() as u32;
        self.table.push(scope.to_string());
        self.table_map.insert(scope.to_string(), idx);
        idx
    }

    // -- shadow analysis ---------------------------------------------------

    fn is_shadowed(&self, name: &str, range: TextRange) -> bool {
        for (scope_range, bindings) in &self.shadows {
            if !scope_range.contains(range.start_byte) {
                continue;
            }
            for (bind_name, at) in bindings {
                if bind_name == name && range.start_byte >= *at {
                    return true;
                }
            }
        }
        false
    }

    /// Collect identifiers bound by `pattern`/`name`, without descending into
    /// nested containers (their bindings belong to them).
    fn collect_idents(&self, node: tree_sitter::Node, out: &mut Vec<(String, u32)>, at: u32) {
        if is_identifier_kind(node.kind()) {
            let t = self.text(node);
            if !t.is_empty() {
                out.push((t, at));
                return;
            }
            return;
        }
        for child in self.named_children(node) {
            if FUNCTION_KINDS.contains(&child.kind()) || child.kind() == "class_declaration" {
                continue;
            }
            self.collect_idents(child, out, at);
        }
    }

    fn block_bindings(&self, node: tree_sitter::Node) -> Vec<(String, u32)> {
        let mut out = Vec::new();
        for child in self.named_children(node) {
            match child.kind() {
                "let_declaration" => {
                    if let Some(p) = child.child_by_field_name("pattern") {
                        self.collect_idents(p, &mut out, child.start_byte() as u32);
                    }
                }
                "variable_declaration" | "lexical_declaration" | "const_declaration" => {
                    let mut cur = child.walk();
                    for n in child.named_children(&mut cur) {
                        if n.kind() == "variable_declarator" {
                            if let Some(name) = n.child_by_field_name("name") {
                                self.collect_idents(name, &mut out, child.start_byte() as u32);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        out
    }

    fn param_bindings(&self, node: tree_sitter::Node, at: u32) -> Vec<(String, u32)> {
        let mut out = Vec::new();
        let Some(params) = node.child_by_field_name("parameters") else {
            return out;
        };
        for param in self.named_children(params) {
            if let Some(p) = param
                .child_by_field_name("pattern")
                .or_else(|| param.child_by_field_name("name"))
            {
                self.collect_idents(p, &mut out, at);
            } else {
                let mut cur = param.walk();
                for n in param.named_children(&mut cur) {
                    if matches!(n.kind(), "identifier" | "self" | "receiver") {
                        let t = self.text(n);
                        if !t.is_empty() && t != "self" {
                            out.push((t, at));
                        }
                    }
                }
            }
        }
        out
    }

    // -- the walk ----------------------------------------------------------

    fn walk(&mut self, node: tree_sitter::Node) {
        let kind = node.kind();

        // 1. Lexical scope for shadow analysis.
        let mut pop_shadow = false;
        if BLOCK_KINDS.contains(&kind) {
            let bindings = self.block_bindings(node);
            self.shadows.push((self.range(node), bindings));
            pop_shadow = true;
        } else if FUNCTION_KINDS.contains(&kind) {
            let bindings = self.param_bindings(node, node.start_byte() as u32);
            self.shadows.push((self.range(node), bindings));
            pop_shadow = true;
        }

        // 2. Declaration: pushes the symbol and/or opens a name scope.
        let outcome = self.declare(node);
        let mut pop_container = false;
        let mut inline_module: Option<String> = None;
        if let Some(scope) = outcome.new_scope {
            self.stack.push(scope.clone());
            pop_container = true;
            if outcome.inline_module {
                inline_module = Some(scope);
            }
        }

        // 3. Imports / exports.
        match kind {
            "use_declaration" | "extern_crate_declaration" => {
                if let Some(imp) = self.rust_import(node) {
                    self.out.imports.push(imp);
                }
            }
            "mod_item" if node.child_by_field_name("body").is_none() => {
                if let Some(imp) = self.rust_mod_decl(node) {
                    self.out.imports.push(imp);
                }
            }
            "import_statement" => {
                if let Some(imp) = self.ts_import(node) {
                    self.out.imports.push(imp);
                }
            }
            "export_statement" => self.ts_export(node),
            "variable_declarator" | "lexical_declaration" | "variable_declaration" => {
                if let Some(imp) = self.require_import(node) {
                    self.out.imports.push(imp);
                }
            }
            _ => {}
        }

        // 4. Call sites.
        if kind == "call_expression" || kind == "new_expression" {
            if let Some(site) = self.call_site(node) {
                self.out.call_sites.push(site);
            }
        }

        // 5. Identifier occurrence.
        if is_identifier_kind(kind) {
            if let Some(occ) = self.occurrence(node) {
                self.out.occurrences.push(occ);
            }
        }

        // 6. Recurse.
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.walk(child);
        }

        if let Some(scope) = inline_module {
            self.out.module_aliases.push(scope);
        }
        if pop_container {
            self.stack.pop();
        }
        if pop_shadow {
            self.shadows.pop();
        }
    }

    fn current_scope(&self) -> String {
        self.stack.last().cloned().unwrap_or_default()
    }

    fn exported(&self, node: tree_sitter::Node) -> bool {
        match self.language {
            Language::Rust => matches!(self.visibility(node), Visibility::Public),
            _ => self.ancestors_have(Some(node), &["export_statement"]),
        }
    }

    fn make_symbol(
        &self,
        name: &str,
        name_range: TextRange,
        node: tree_sitter::Node,
        kind: SymbolKind,
    ) -> Option<Symbol> {
        if name.is_empty() || !is_valid_ident(name) {
            return None;
        }
        let parent_scope = self.current_scope();
        let qualified = format!("{parent_scope}::{name}");
        let vis = self.visibility(node);
        let container = if parent_scope == self.module_scope || parent_scope.is_empty() {
            None
        } else {
            Some(parent_scope.clone())
        };
        Some(Symbol {
            id: String::new(),
            file: self.file.clone(),
            language: self.language.clone(),
            qualified_name: qualified,
            name: name.to_string(),
            kind,
            range: self.range(node),
            name_range,
            container,
            signature: self.signature(node),
            visibility: vis,
            exported: self.exported(node),
            module: self.module_scope.clone(),
            crate_key: self.crate_key.clone(),
        })
    }

    /// Produce the symbol (and/or child scope) that `node` declares.
    fn declare(&mut self, node: tree_sitter::Node) -> DeclOutcome {
        let kind = node.kind();
        let mut out = DeclOutcome::default();

        // ---- variable lists: possibly several symbols --------------------
        if matches!(
            kind,
            "lexical_declaration" | "variable_declaration" | "const_declaration"
        ) {
            let children = self.named_children(node);
            for child in children {
                if child.kind() != "variable_declarator" {
                    continue;
                }
                let Some(name_node) = child.child_by_field_name("name") else {
                    continue;
                };
                let name = self.text(name_node);
                let value = child.child_by_field_name("value");
                let is_fn = value
                    .map(|v| {
                        matches!(
                            v.kind(),
                            "arrow_function" | "function_expression" | "generator_function"
                        )
                    })
                    .unwrap_or(false);
                let sym_kind = if is_fn {
                    if self.ancestors_have(Some(child), &["class_body"]) {
                        SymbolKind::Method
                    } else {
                        SymbolKind::Function
                    }
                } else {
                    SymbolKind::Variable
                };
                if let Some(sym) =
                    self.make_symbol(&name, self.range(name_node), child, sym_kind.clone())
                {
                    self.out.symbols.push(sym);
                }
            }
            return out;
        }

        // ---- single-name declarations ------------------------------------
        let sym_kind = match kind {
            "function_item" | "function_declaration" | "generator_function_declaration" => {
                if self.ancestors_have(
                    Some(node),
                    &["impl_item", "trait_item", "class_body", "switch_body"],
                ) {
                    SymbolKind::Method
                } else {
                    SymbolKind::Function
                }
            }
            "method_definition" => SymbolKind::Method,
            "struct_item" => SymbolKind::Struct,
            "enum_item" => SymbolKind::Enum,
            "trait_item" => SymbolKind::Trait,
            "class_declaration" => SymbolKind::Class,
            "interface_declaration" => SymbolKind::Interface,
            "type_alias_declaration" | "type_item" => SymbolKind::TypeAlias,
            "const_item" | "static_item" => SymbolKind::Constant,
            "macro_definition" => SymbolKind::Macro,
            "mod_item" => SymbolKind::Module,
            "public_field_definition" | "field_definition" => SymbolKind::Variable,
            "impl_item" => {
                // Container only: no symbol, so `impl Foo` never collides with
                // `struct Foo` in the qualified-name table.
                if let Some(name) = self.impl_type_name(node) {
                    let scope = self.current_scope();
                    out.new_scope = Some(format!("{scope}::{name}"));
                }
                return out;
            }
            _ => return out,
        };

        let Some(name_node) = self.name_node(node) else {
            return out;
        };
        let name = self.text(name_node);
        let Some(sym) = self.make_symbol(&name, self.range(name_node), node, sym_kind) else {
            return out;
        };
        let qualified = sym.qualified_name.clone();
        self.out.symbols.push(sym);

        // Which containers open a child scope?
        let opens_scope = matches!(
            kind,
            "function_item"
                | "function_declaration"
                | "generator_function_declaration"
                | "method_definition"
                | "class_declaration"
                | "trait_item"
        ) || (kind == "mod_item" && node.child_by_field_name("body").is_some());
        if opens_scope {
            out.new_scope = Some(qualified);
            out.inline_module = kind == "mod_item";
        }
        out
    }

    /// `impl Foo<T>` / `impl Trait for Foo<T>` -> `Foo`.
    fn impl_type_name(&self, node: tree_sitter::Node) -> Option<String> {
        let body = node.child_by_field_name("body");
        let end = body.map(|b| b.start_byte()).unwrap_or(node.end_byte());
        let raw = &self.bytes[node.start_byte()..end.min(self.bytes.len())];
        let header = String::from_utf8_lossy(raw);
        let after = if let Some(idx) = header.rfind(" for ") {
            &header[idx + 5..]
        } else {
            let t = header.trim_start();
            t.strip_prefix("impl").unwrap_or(t)
        };
        let mut depth = 0i32;
        let mut cleaned = String::new();
        for ch in after.chars() {
            match ch {
                '<' => depth += 1,
                '>' => depth -= 1,
                _ => {}
            }
            if depth > 0 {
                continue;
            }
            if ch == '>' && depth == 0 {
                continue;
            }
            cleaned.push(ch);
        }
        if depth != 0 {
            cleaned = after.to_string();
        }
        let cleaned = cleaned.split("where").next().unwrap_or(&cleaned);
        let cleaned = cleaned.split('{').next().unwrap_or(cleaned);
        let seg = cleaned
            .split("::")
            .last()
            .unwrap_or(cleaned)
            .trim()
            .trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
        if seg.is_empty() || !is_valid_ident(seg) {
            None
        } else {
            Some(seg.to_string())
        }
    }

    // -- imports -----------------------------------------------------------

    fn rust_import(&mut self, node: tree_sitter::Node) -> Option<ImportDecl> {
        let range = self.range(node);
        let excerpt = self.excerpt(node);
        let scope = self.current_scope();
        // `pub use` re-exports: the binding is also an export of this module.
        let public = matches!(self.visibility(node), Visibility::Public);

        if node.kind() == "extern_crate_declaration" {
            let name = node
                .child_by_field_name("name")
                .map(|n| self.text(n))
                .or_else(|| {
                    self.named_children(node)
                        .into_iter()
                        .find(|c| c.kind() == "identifier")
                        .map(|c| self.text(c))
                })?;
            if name.is_empty() {
                return None;
            }
            return Some(ImportDecl {
                file: self.file.clone(),
                range,
                kind: ImportKind::ExternCrate,
                specifier: name.clone(),
                entries: vec![ImportEntry {
                    local: name.clone(),
                    imported: name.clone(),
                    module: name,
                    alias: false,
                    glob: false,
                }],
                scope,
                public,
                resolution: ImportResolution::Unresolved {
                    reason: "extern crate; resolved against package metadata".into(),
                },
                excerpt,
            });
        }

        let path = self.named_children(node).into_iter().next()?;
        let tree = self.build_use(path)?;
        let mut raw: Vec<RawEntry> = Vec::new();
        self.eval_use(&tree, &[], &mut raw, &mut Vec::new());
        if raw.is_empty() {
            return None;
        }
        // The specifier is the raw path as written — the per-entry module is
        // what resolution uses, this is for display and edge labelling.
        let specifier = self.text(path);
        let entries = raw
            .into_iter()
            .map(|e| ImportEntry {
                local: e.local,
                imported: e.imported,
                module: e.module,
                alias: e.alias,
                glob: e.glob,
            })
            .collect();
        Some(ImportDecl {
            file: self.file.clone(),
            range,
            kind: ImportKind::Use,
            specifier,
            entries,
            scope,
            public,
            resolution: ImportResolution::Unresolved {
                reason: "not yet resolved".into(),
            },
            excerpt,
        })
    }

    fn rust_mod_decl(&mut self, node: tree_sitter::Node) -> Option<ImportDecl> {
        let name = self.name_node(node).map(|n| self.text(n))?;
        if name.is_empty() {
            return None;
        }
        Some(ImportDecl {
            file: self.file.clone(),
            range: self.range(node),
            kind: ImportKind::ModDecl,
            specifier: name.clone(),
            entries: vec![ImportEntry {
                local: name,
                imported: String::new(),
                module: String::new(),
                alias: false,
                glob: false,
            }],
            scope: self.current_scope(),
            public: false,
            resolution: ImportResolution::Unresolved {
                reason: "child module; resolved from file layout".into(),
            },
            excerpt: self.excerpt(node),
        })
    }

    /// Build a `use` path tree from the syntax tree.
    fn build_use(&self, node: tree_sitter::Node) -> Option<UseTree> {
        match node.kind() {
            "identifier" | "crate" | "super" | "self" => {
                let t = self.text(node);
                (!t.is_empty()).then_some(UseTree::Name(t))
            }
            // `use crate::util::*;` nests the path *inside* the wildcard node
            // (`use_wildcard(scoped_identifier)`), so the path has to be lifted
            // out — otherwise the entry's module comes back empty.
            "use_wildcard" | "*" => {
                let mut segs: Vec<String> = Vec::new();
                for child in self.named_children(node) {
                    self.flatten_segments(child, &mut segs);
                }
                if segs.is_empty() {
                    Some(UseTree::Glob)
                } else {
                    Some(UseTree::Path(segs, Box::new(UseTree::Glob)))
                }
            }
            "use_list" => {
                let items = self
                    .named_children(node)
                    .into_iter()
                    .filter_map(|c| self.build_use(c))
                    .collect::<Vec<_>>();
                (!items.is_empty()).then_some(UseTree::List(items))
            }
            "use_as_clause" => {
                let kids = self.named_children(node);
                if kids.len() < 2 {
                    return None;
                }
                let alias = self.text(kids[kids.len() - 1]);
                let inner = self.build_use(kids[0])?;
                Some(UseTree::As(Box::new(inner), alias))
            }
            "scoped_identifier" | "scoped_use_list" | "scoped_type_identifier" => {
                let mut segs: Vec<String> = Vec::new();
                let mut tail: Option<tree_sitter::Node> = None;
                for c in self.named_children(node) {
                    match c.kind() {
                        "identifier" | "crate" | "super" | "self" | "scoped_identifier" => {
                            self.flatten_segments(c, &mut segs);
                        }
                        _ if tail.is_none() => tail = Some(c),
                        _ => {}
                    }
                }
                match tail {
                    Some(t) => {
                        let inner = self.build_use(t)?;
                        if segs.is_empty() {
                            Some(inner)
                        } else {
                            Some(UseTree::Path(segs, Box::new(inner)))
                        }
                    }
                    None => {
                        if segs.is_empty() {
                            None
                        } else {
                            let last = segs.pop().expect("checked non-empty");
                            Some(UseTree::Path(segs, Box::new(UseTree::Name(last))))
                        }
                    }
                }
            }
            _ => None,
        }
    }

    fn flatten_segments(&self, node: tree_sitter::Node, out: &mut Vec<String>) {
        match node.kind() {
            "identifier" | "crate" | "super" | "self" => {
                let t = self.text(node);
                if !t.is_empty() {
                    out.push(t);
                }
            }
            "scoped_identifier" | "scoped_use_list" | "scoped_type_identifier" => {
                for c in self.named_children(node) {
                    if matches!(
                        c.kind(),
                        "identifier" | "crate" | "super" | "self" | "scoped_identifier"
                    ) {
                        self.flatten_segments(c, out);
                    }
                }
            }
            _ => {}
        }
    }

    fn eval_use(
        &self,
        tree: &UseTree,
        prefix: &[String],
        out: &mut Vec<RawEntry>,
        alias_stack: &mut Vec<String>,
    ) {
        match tree {
            UseTree::Name(n) => out.push(RawEntry {
                module: prefix.join("::"),
                imported: n.clone(),
                local: alias_stack.last().cloned().unwrap_or_else(|| n.clone()),
                alias: !alias_stack.is_empty(),
                glob: false,
            }),
            UseTree::Path(segs, rest) => {
                let mut p = prefix.to_vec();
                p.extend(segs.iter().cloned());
                self.eval_use(rest, &p, out, alias_stack);
            }
            UseTree::List(items) => {
                for item in items {
                    self.eval_use(item, prefix, out, alias_stack);
                }
            }
            UseTree::As(inner, alias) => {
                alias_stack.push(alias.clone());
                self.eval_use(inner, prefix, out, alias_stack);
                alias_stack.pop();
            }
            UseTree::Glob => out.push(RawEntry {
                module: prefix.join("::"),
                imported: String::new(),
                local: "*".to_string(),
                alias: false,
                glob: true,
            }),
        }
    }

    fn ts_import(&mut self, node: tree_sitter::Node) -> Option<ImportDecl> {
        let range = self.range(node);
        let excerpt = self.excerpt(node);
        let mut specifier = String::new();
        let mut entries: Vec<ImportEntry> = Vec::new();

        for child in self.named_children(node) {
            match child.kind() {
                "string" | "string_fragment" => {
                    if specifier.is_empty() {
                        specifier = strip_quotes(&self.text(child));
                    }
                }
                "import_clause" => {
                    for item in self.named_children(child) {
                        match item.kind() {
                            "identifier" | "type_identifier" => entries.push(ImportEntry {
                                local: self.text(item),
                                imported: "default".into(),
                                module: String::new(),
                                alias: false,
                                glob: false,
                            }),
                            "namespace_import" => {
                                let alias = self
                                    .named_children(item)
                                    .into_iter()
                                    .rfind(|n| is_identifier_kind(n.kind()))
                                    .map(|n| self.text(n))?;
                                entries.push(ImportEntry {
                                    local: alias,
                                    imported: String::new(),
                                    module: String::new(),
                                    alias: false,
                                    glob: true,
                                });
                            }
                            "named_imports" => {
                                for spec in self.named_children(item) {
                                    if spec.kind() != "import_specifier" {
                                        continue;
                                    }
                                    let mut names = self
                                        .named_children(spec)
                                        .into_iter()
                                        .filter(|n| is_identifier_kind(n.kind()))
                                        .map(|n| self.text(n))
                                        .collect::<Vec<_>>();
                                    if names.is_empty() {
                                        continue;
                                    }
                                    let imported = names.remove(0);
                                    let local = if names.is_empty() {
                                        imported.clone()
                                    } else {
                                        names.remove(0)
                                    };
                                    let alias = local != imported;
                                    entries.push(ImportEntry {
                                        local,
                                        imported,
                                        module: String::new(),
                                        alias,
                                        glob: false,
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        if specifier.is_empty() {
            return None;
        }
        for e in &mut entries {
            e.module = specifier.clone();
        }
        Some(ImportDecl {
            file: self.file.clone(),
            range,
            kind: ImportKind::Esm,
            specifier,
            entries,
            scope: self.current_scope(),
            public: false,
            resolution: ImportResolution::Unresolved {
                reason: "not yet resolved".into(),
            },
            excerpt,
        })
    }

    /// `const x = require("./y")` — a real CommonJS dependency.
    fn require_import(&mut self, node: tree_sitter::Node) -> Option<ImportDecl> {
        let value = node.child_by_field_name("value")?;
        if value.kind() != "call_expression" {
            return None;
        }
        let func = value.child_by_field_name("function")?;
        if func.kind() != "identifier" || self.text(func) != "require" {
            return None;
        }
        let args = value.child_by_field_name("arguments")?;
        let first = self.named_children(args).into_iter().next()?;
        if !matches!(first.kind(), "string" | "string_fragment") {
            return None;
        }
        let specifier = strip_quotes(&self.text(first));
        if specifier.is_empty() {
            return None;
        }
        let local = node
            .child_by_field_name("name")
            .map(|n| self.text(n))
            .unwrap_or_default();
        let mut entries = Vec::new();
        if !local.is_empty() {
            entries.push(ImportEntry {
                local: local.clone(),
                imported: "default".into(),
                module: specifier.clone(),
                alias: false,
                glob: false,
            });
        }
        Some(ImportDecl {
            file: self.file.clone(),
            range: self.range(node),
            kind: ImportKind::Require,
            specifier,
            entries,
            scope: self.current_scope(),
            public: false,
            resolution: ImportResolution::Unresolved {
                reason: "not yet resolved".into(),
            },
            excerpt: self.excerpt(node),
        })
    }

    fn ts_export(&mut self, node: tree_sitter::Node) {
        let range = self.range(node);
        let excerpt = self.excerpt(node);
        let text = self.text(node);
        let is_default = text.trim_start().starts_with("export default");
        let mut entries: Vec<ExportEntry> = Vec::new();
        let mut from: Option<String> = None;

        for child in self.named_children(node) {
            match child.kind() {
                "string" | "string_fragment" => {
                    if from.is_none() {
                        from = Some(strip_quotes(&self.text(child)));
                    }
                }
                "export_clause" => {
                    for spec in self.named_children(child) {
                        if spec.kind() == "export_wildcard" {
                            entries.push(ExportEntry {
                                exported: "*".into(),
                                local: String::new(),
                                alias: false,
                                glob: true,
                            });
                            continue;
                        }
                        if spec.kind() != "export_specifier" {
                            continue;
                        }
                        let mut names = self
                            .named_children(spec)
                            .into_iter()
                            .filter(|n| is_identifier_kind(n.kind()))
                            .map(|n| self.text(n))
                            .collect::<Vec<_>>();
                        if names.is_empty() {
                            continue;
                        }
                        let local = names.remove(0);
                        let exported = if names.is_empty() {
                            local.clone()
                        } else {
                            names.remove(0)
                        };
                        entries.push(ExportEntry {
                            exported,
                            local,
                            alias: false,
                            glob: false,
                        });
                    }
                }
                "export_wildcard" => entries.push(ExportEntry {
                    exported: "*".into(),
                    local: String::new(),
                    alias: false,
                    glob: true,
                }),
                _ => {}
            }
        }

        if is_default {
            let local = self
                .named_children(node)
                .into_iter()
                .filter(|n| !matches!(n.kind(), "export" | "default" | "string"))
                .find_map(|n| self.name_node(n))
                .map(|n| self.text(n));
            let local = local.unwrap_or_default();
            entries.push(ExportEntry {
                exported: "default".into(),
                local,
                alias: true,
                glob: false,
            });
        }

        for e in &mut entries {
            e.alias = e.exported != e.local;
        }
        // `export * from "./x"` (and any bare re-export) still binds names.
        if from.is_some() && entries.is_empty() {
            entries.push(ExportEntry {
                exported: "*".into(),
                local: String::new(),
                alias: false,
                glob: true,
            });
        }
        if entries.is_empty() {
            return;
        }
        let decl = ExportDecl {
            file: self.file.clone(),
            range,
            entries,
            default: is_default,
            from: from.clone(),
            excerpt: excerpt.clone(),
        };
        self.out.exports.push(decl);

        // A `export ... from` is also a file dependency.
        if let Some(spec) = from {
            self.out.imports.push(ImportDecl {
                file: self.file.clone(),
                range,
                kind: ImportKind::Esm,
                specifier: spec,
                entries: Vec::new(),
                scope: self.current_scope(),
                public: false,
                resolution: ImportResolution::Unresolved {
                    reason: "not yet resolved".into(),
                },
                excerpt,
            });
        }
    }

    // -- calls -------------------------------------------------------------

    fn call_site(&self, node: tree_sitter::Node) -> Option<CallSite> {
        let callee = node
            .child_by_field_name("function")
            .or_else(|| node.child_by_field_name("constructor"))
            .or_else(|| {
                self.named_children(node)
                    .into_iter()
                    .find(|n| n.kind() != "arguments")
            })?;
        let range = self.range(node);
        let callee_range = self.range(callee);
        let kind = callee.kind();

        let (callee_name, callee_path, shape) = match kind {
            "identifier" | "type_identifier" => {
                let n = self.text(callee);
                (n, None, CallShape::Plain)
            }
            "scoped_identifier" | "scoped_type_identifier" | "generic_function" => {
                let inner = if kind == "generic_function" {
                    callee.child_by_field_name("function")?
                } else {
                    callee
                };
                let path = self.text(inner);
                let last = path.rsplit("::").next().unwrap_or(&path).to_string();
                (last, Some(path), CallShape::Scoped)
            }
            "field_expression" => {
                let field = callee.child_by_field_name("field")?;
                let n = self.text(field);
                let receiver = callee
                    .child_by_field_name("value")
                    .map(|v| self.text(v))
                    .unwrap_or_default();
                let path = (!receiver.is_empty()).then(|| format!("{receiver}.{n}"));
                (n, path, CallShape::Member)
            }
            "member_expression" => {
                let prop = callee.child_by_field_name("property")?;
                let n = self.text(prop);
                let obj = callee
                    .child_by_field_name("object")
                    .map(|o| self.text(o))
                    .unwrap_or_default();
                let path = (!obj.is_empty()).then(|| format!("{obj}.{n}"));
                (n, path, CallShape::Member)
            }
            _ => {
                let t = self.text(callee);
                let last = t.rsplit(['.', ':']).next().unwrap_or("").trim().to_string();
                if last.is_empty() || !is_valid_ident(&last) {
                    return None;
                }
                (last, Some(t), CallShape::Scoped)
            }
        };
        if callee_name.is_empty() || !is_valid_ident(&callee_name) {
            return None;
        }

        let enclosing = self.enclosing_symbol(callee_range.start_byte);
        Some(CallSite {
            file: self.file.clone(),
            range,
            callee_range,
            callee_name,
            callee_path,
            shape,
            enclosing,
            excerpt: self.excerpt(node),
        })
    }

    /// Innermost symbol containing `byte` (call attribution for callees).
    fn enclosing_symbol(&self, byte: u32) -> Option<String> {
        self.out
            .symbols
            .iter()
            .filter(|s| s.range.contains(byte))
            .min_by_key(|s| s.range.end_byte.saturating_sub(s.range.start_byte))
            .map(|s| s.qualified_name.clone())
    }

    // -- occurrences -------------------------------------------------------

    fn occurrence(&mut self, node: tree_sitter::Node) -> Option<Occurrence> {
        let name = self.text(node);
        if name.is_empty() || !is_valid_ident(&name) {
            return None;
        }
        let range = self.range(node);
        let is_definition = self.out.symbols.iter().any(|s| s.name_range == range);

        // Receiver of a member access.
        let mut receiver: Option<String> = None;
        if let Some(parent) = node.parent() {
            if matches!(parent.kind(), "field_expression" | "member_expression") {
                let is_tail = parent.child_by_field_name("field").map(|f| f.start_byte())
                    == Some(node.start_byte())
                    || parent
                        .child_by_field_name("property")
                        .map(|p| p.start_byte())
                        == Some(node.start_byte());
                if is_tail {
                    let obj = parent
                        .child_by_field_name("value")
                        .or_else(|| parent.child_by_field_name("object"));
                    if let Some(o) = obj {
                        let t = self.text(o);
                        if !t.is_empty() {
                            receiver = Some(t);
                        }
                    }
                }
            }
        }

        // Property names without a receiver denote a property, never a value
        // reference: keep them only when they are a declaration's own name.
        if is_property_kind(node.kind()) && receiver.is_none() && !is_definition {
            return None;
        }

        // Module-path prefix for scoped references: `crate::agent` before
        // `AgentLoop`, `foo` before `bar`. Deterministically resolvable, unlike
        // a member receiver, which is why the two are recorded separately.
        let mut path_prefix: Option<String> = None;
        if receiver.is_none() {
            if let Some(parent) = node.parent() {
                if matches!(
                    parent.kind(),
                    "scoped_identifier" | "scoped_type_identifier"
                ) {
                    let kids = self.named_children(parent);
                    if kids.last().map(|k| k.start_byte()) == Some(node.start_byte()) {
                        let mut parts: Vec<String> = kids[..kids.len().saturating_sub(1)]
                            .iter()
                            .map(|k| self.text(*k))
                            .filter(|t| !t.is_empty())
                            .collect();
                        // `crate::agent` may itself be one node; take its text.
                        if parts.is_empty() {
                            if let Some(path_node) = parent.child_by_field_name("path") {
                                let t = self.text(path_node);
                                if !t.is_empty() {
                                    parts.push(t);
                                }
                            }
                        }
                        if !parts.is_empty() {
                            path_prefix = Some(parts.join("::"));
                        }
                    }
                }
            }
        }

        let ctx = if is_definition {
            OccContext::Definition
        } else if self.ancestors_have(
            Some(node),
            &["use_declaration", "import_statement", "export_statement"],
        ) {
            OccContext::Import
        } else if self.in_call_callee(node) {
            OccContext::Call
        } else if self.in_type_position(node) {
            OccContext::Type
        } else if self.in_assignment_lhs(node) {
            OccContext::Write
        } else {
            OccContext::Read
        };

        let scope = self.current_scope();
        let scope_idx = self.scope_id(&scope);
        let shadowed = !is_definition && self.is_shadowed(&name, range);
        Some(Occurrence {
            name,
            range,
            ctx,
            shadowed,
            path_prefix,
            scope_idx,
            receiver,
        })
    }

    fn in_call_callee(&self, node: tree_sitter::Node) -> bool {
        let mut parent = node.parent();
        let mut depth = 0;
        while let Some(p) = parent {
            if depth > 8 {
                break;
            }
            if p.kind() == "call_expression" || p.kind() == "new_expression" {
                let f = p
                    .child_by_field_name("function")
                    .or_else(|| p.child_by_field_name("constructor"));
                return match f {
                    Some(f) => {
                        f.start_byte() <= node.start_byte() && node.end_byte() <= f.end_byte()
                    }
                    None => true,
                };
            }
            if matches!(p.kind(), "block" | "statement_block" | "function_item") {
                return false;
            }
            parent = p.parent();
            depth += 1;
        }
        false
    }

    fn in_type_position(&self, node: tree_sitter::Node) -> bool {
        if node.kind() == "type_identifier" {
            return true;
        }
        let mut parent = node.parent();
        let mut depth = 0;
        while let Some(p) = parent {
            if depth > 6 {
                break;
            }
            let k = p.kind();
            if k.ends_with("_type")
                || matches!(
                    k,
                    "type_arguments"
                        | "generic_type"
                        | "scoped_type_identifier"
                        | "type_annotation"
                        | "return_type"
                        | "as_cast"
                        | "predefined_type"
                )
            {
                return true;
            }
            parent = p.parent();
            depth += 1;
        }
        false
    }

    fn in_assignment_lhs(&self, node: tree_sitter::Node) -> bool {
        let mut parent = node.parent();
        let mut depth = 0;
        while let Some(p) = parent {
            if depth > 4 {
                break;
            }
            if matches!(
                p.kind(),
                "assignment_expression" | "augmented_assignment_expression"
            ) {
                return p
                    .child_by_field_name("left")
                    .map(|l| l.start_byte() <= node.start_byte() && node.end_byte() <= l.end_byte())
                    .unwrap_or(false);
            }
            parent = p.parent();
            depth += 1;
        }
        false
    }
}

// ---------------------------------------------------------------------------
// Use-tree representation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum UseTree {
    Name(String),
    Path(Vec<String>, Box<UseTree>),
    List(Vec<UseTree>),
    As(Box<UseTree>, String),
    Glob,
}

#[derive(Debug, Clone)]
struct RawEntry {
    module: String,
    imported: String,
    local: String,
    alias: bool,
    glob: bool,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// An identifier is a plain name (not a path fragment, not a keyword).
fn is_valid_ident(s: &str) -> bool {
    if s.is_empty() || s.len() > 200 {
        return false;
    }
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_alphabetic() || first == '_' || first == '$') {
        return false;
    }
    chars.all(|c| c.is_alphanumeric() || c == '_' || c == '$' || c == '\'')
}

fn strip_quotes(s: &str) -> String {
    let t = s.trim();
    let t = t
        .strip_prefix('"')
        .or_else(|| t.strip_prefix('\''))
        .or_else(|| t.strip_prefix('`'))
        .unwrap_or(t);
    t.strip_suffix('"')
        .or_else(|| t.strip_suffix('\''))
        .or_else(|| t.strip_suffix('`'))
        .unwrap_or(t)
        .to_string()
}

// The declaration's symbol is materialised inside `declare`, so these two are
// only referenced by tests to keep the helpers honest.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_reject_paths_and_digits() {
        assert!(is_valid_ident("helper"));
        assert!(is_valid_ident("_priv"));
        assert!(is_valid_ident("$el"));
        assert!(!is_valid_ident("::"));
        assert!(!is_valid_ident("1abc"));
        assert!(!is_valid_ident(""));
    }

    #[test]
    fn strip_quotes_handles_three_quote_styles() {
        assert_eq!(strip_quotes("\"./foo\""), "./foo");
        assert_eq!(strip_quotes("'./foo'"), "./foo");
        assert_eq!(strip_quotes("`./foo`"), "./foo");
        assert_eq!(strip_quotes("./foo"), "./foo");
    }

    #[test]
    fn parses_rust_symbols_and_calls() {
        let src = "pub fn helper() {}\nfn main() { helper(); }\n";
        let out = parse(src, "a.rs", &Language::Rust, "crate::a", "crates/p").unwrap();
        let names: Vec<&str> = out.symbols.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["helper", "main"]);
        assert_eq!(out.symbols[0].qualified_name, "crate::a::helper");
        assert!(out.symbols[0].exported);
        assert!(!out.symbols[1].exported);
        assert_eq!(out.call_sites.len(), 1);
        assert_eq!(out.call_sites[0].callee_name, "helper");
        assert_eq!(
            out.call_sites[0].enclosing.as_deref(),
            Some("crate::a::main")
        );
        assert_eq!(out.status, ParseStatus::Parsed);
    }

    #[test]
    fn parses_typescript_imports_and_exports() {
        let src = "import { widget as w } from \"./widget\";\nexport function run() { w(); }\n";
        let out = parse(src, "src/a.ts", &Language::TypeScript, "src::a", "apps/x").unwrap();
        assert_eq!(out.imports.len(), 1);
        assert_eq!(out.imports[0].specifier, "./widget");
        assert_eq!(out.imports[0].entries[0].local, "w");
        assert_eq!(out.imports[0].entries[0].imported, "widget");
        assert!(out.imports[0].entries[0].alias);
        assert_eq!(out.symbols[0].name, "run");
        assert!(out.symbols[0].exported);
        assert_eq!(out.symbols[0].qualified_name, "src::a::run");
    }

    #[test]
    fn comments_and_strings_are_not_occurrences() {
        let src = "// helper\nfn a() { let s = \"helper\"; }\n";
        let out = parse(src, "a.rs", &Language::Rust, "crate::a", "p").unwrap();
        let helper_hits = out
            .occurrences
            .iter()
            .filter(|o| o.name == "helper")
            .count();
        assert_eq!(helper_hits, 0, "comment text must not become an occurrence");
    }

    #[test]
    fn rust_use_paths_flatten_correctly() {
        let src = "use crate::ledger::{LedgerEntry, LedgerStore as Store};\nuse super::agent as agent_loop;\nuse crate::util::*;\n";
        let out = parse(src, "a.rs", &Language::Rust, "crate::a", "p").unwrap();
        assert_eq!(out.imports.len(), 3);
        let first = &out.imports[0];
        assert_eq!(first.entries.len(), 2);
        assert_eq!(first.entries[0].module, "crate::ledger");
        assert_eq!(first.entries[0].imported, "LedgerEntry");
        assert_eq!(first.entries[1].local, "Store");
        assert_eq!(first.entries[1].imported, "LedgerStore");
        assert!(first.entries[1].alias);
        let second = &out.imports[1];
        assert_eq!(second.entries[0].local, "agent_loop");
        assert_eq!(second.entries[0].imported, "agent");
        // `super::agent` resolves as item `agent` in module `super`.
        assert_eq!(second.entries[0].module, "super");
        assert!(second.entries[0].alias);
        let third = &out.imports[2];
        assert!(third.entries[0].glob);
        assert_eq!(third.entries[0].module, "crate::util");
    }
}
