//! [`RepoIndex`]: the in-memory navigation index and its incremental build.
//!
//! ## Why a build looks the way it does
//!
//! A query never re-indexes. Everything a query touches — symbols, import
//! bindings, exports, call sites, occurrences — lives in memory from a single
//! build, so `find_references` is a scan of pre-parsed facts, never a reparse.
//!
//! Rebuilds are **per file**: each file's facts are keyed by SHA-256 of its
//! bytes, so editing one file re-parses exactly one file. Files that vanished
//! are dropped (that is how a delete invalidates), and a rename shows up as
//! one removal plus one addition. Nothing is guessed about renames — the hash
//! proves the bytes are the same, the path proves they are not.
//!
//! Persistence is a warm-start cache in `.zylcode/nav-index.json`. It is
//! schema-versioned; a version bump discards it rather than producing facts
//! from an older layout. The cache is a *cache*: `build` always re-scans the
//! working tree first, so on-disk facts can never disagree with what the
//! repository actually contains.

use crate::model::*;
use crate::parse;
use crate::resolve::{self, Layout, ModuleTarget, Resolver};
use crate::scan;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Bumped whenever [`FileFacts`], any cached structure, or the parser's output
/// for unchanged bytes changes shape.
///
/// v2: `.tsx` now selects the TSX grammar and a UTF-8 BOM is skipped, so facts
/// cached by v1 describe different text and must not be reused.
/// v3: partial-parse diagnostics name the failing construct.
pub const SCHEMA_VERSION: u32 = 3;

/// Cache location, repository-relative (gitignored; never an input to queries).
pub const CACHE_RELATIVE: &str = ".zylcode/nav-index.json";

// ---------------------------------------------------------------------------
// What a binding is
// ---------------------------------------------------------------------------

/// Where an import's local name points.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingTarget {
    /// Resolved to a specific symbol in this repository.
    Symbol(String),
    /// Resolved to a file but not to one symbol (glob / namespace import).
    File(String),
    /// Declared in a manifest; outside this repository.
    External(String),
    /// Could not be resolved.
    Unresolved(String),
}

/// One imported local name and where it lands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binding {
    /// Name usable in the importing file (after `as`).
    pub local: String,
    /// Name in the source module.
    pub imported: String,
    pub glob: bool,
    pub target: BindingTarget,
    /// Range of the import statement that created this binding (evidence).
    pub range: TextRange,
    /// The specifier as written.
    pub specifier: String,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

/// Counters the governance record and tests read.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexStats {
    pub files_seen: usize,
    pub files_parsed: usize,
    pub files_reused: usize,
    pub files_removed: usize,
    /// No grammar for the language (reported, not silently dropped).
    pub files_unsupported: usize,
    pub files_failed: usize,
    /// Grammar exists but tree-sitter reported errors; facts are partial.
    pub files_partial: usize,
    pub symbols: usize,
    pub occurrences: usize,
    pub imports: usize,
    pub call_sites: usize,
    /// Exports that could be attributed to a symbol.
    pub exports: usize,
    pub bindings: usize,
    pub build_ms: u128,
    /// True when this build reused at least one cached file.
    pub warm: bool,
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
struct Persisted {
    schema: u32,
    files: BTreeMap<String, FileFacts>,
}

fn cache_path(root: &Path) -> PathBuf {
    root.join(CACHE_RELATIVE.replace('/', std::path::MAIN_SEPARATOR_STR))
}

fn load_cache(path: &Path) -> Option<BTreeMap<String, FileFacts>> {
    let text = std::fs::read_to_string(path).ok()?;
    let parsed: Persisted = serde_json::from_str(&text).ok()?;
    if parsed.schema != SCHEMA_VERSION {
        return None;
    }
    Some(parsed.files)
}

fn write_cache(path: &Path, facts: &HashMap<String, FileFacts>) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let mut files = BTreeMap::new();
    for (k, v) in facts {
        files.insert(k.clone(), v.clone());
    }
    let payload = Persisted {
        schema: SCHEMA_VERSION,
        files,
    };
    let json = serde_json::to_string(&payload).context("serialising navigation cache")?;
    // Write-then-rename so a crash mid-write never leaves a half cache.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json).with_context(|| format!("writing {}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// The index
// ---------------------------------------------------------------------------

/// Deterministic repository navigation index.
#[derive(Debug, Default)]
pub struct RepoIndex {
    pub root: PathBuf,
    pub resolver: Resolver,
    /// file path -> everything parsed from it.
    pub facts: HashMap<String, FileFacts>,
    /// symbol id -> symbol.
    pub symbols: HashMap<String, Symbol>,
    /// file -> symbol ids in declaration order.
    pub symbols_by_file: HashMap<String, Vec<String>>,
    /// (file, bare name) -> symbol ids in declaration order.
    symbols_by_name: HashMap<(String, String), Vec<String>>,
    /// file -> exported name -> symbol id (built, includes re-export chains).
    pub exports: HashMap<String, HashMap<String, String>>,
    /// file -> local name -> where the import lands.
    pub bindings: HashMap<String, HashMap<String, Binding>>,
    /// file -> files pulled in by glob/namespace imports (`use a::*`).
    pub globs: HashMap<String, Vec<String>>,
    pub stats: IndexStats,
}

impl RepoIndex {
    // -- construction ------------------------------------------------------

    /// Build (or warm-start) the index for `root`.
    ///
    /// Fails when `root` is not a directory. Silently indexing a typo'd or
    /// deleted path would produce an empty index, and an empty index answers
    /// every query with "nothing found" — a fabricated negative. Better to
    /// refuse and let the caller report the index as unavailable.
    pub fn build(root: impl AsRef<Path>) -> Result<RepoIndex> {
        let start = Instant::now();
        let root = root.as_ref();
        anyhow::ensure!(
            root.is_dir(),
            "repository root `{}` is not a directory",
            root.display()
        );
        let root = root.to_path_buf();
        let cached = load_cache(&cache_path(&root)).unwrap_or_default();

        let scanned = scan::scan(&root, true).context("scanning repository")?;
        let layout = Layout::discover(&root);

        // Placement needs the complete file set (a crate root is decided by
        // whether `src/lib.rs` exists at all, not by walk order).
        let mut file_set: HashSet<String> = HashSet::new();
        for sf in &scanned {
            if sf.read_error.is_none() {
                file_set.insert(sf.repo_relative.clone());
            }
        }

        let mut index = RepoIndex {
            root,
            resolver: Resolver::new(file_set.clone(), layout),
            ..Default::default()
        };

        let mut current: HashSet<String> = HashSet::new();
        for sf in &scanned {
            index.stats.files_seen += 1;
            let rel = sf.repo_relative.clone();
            if sf.read_error.is_some() {
                index.stats.files_unsupported += 1;
                continue;
            }
            current.insert(rel.clone());

            let (module, crate_key) = resolve::place(&file_set, &index.resolver.layout, &rel);

            // Warm path: identical bytes *and* identical placement means
            // identical facts. If a manifest appeared or vanished, the module
            // path changes too and the cached facts would be stale.
            if let Some(old) = cached.get(&rel) {
                if old.content_hash == sf.content_hash
                    && old.language == sf.language
                    && old.module == module
                    && old.crate_key == crate_key
                {
                    let facts = old.clone();
                    index.stats.files_reused += 1;
                    index.stats.warm = true;
                    index.facts.insert(rel, facts);
                    continue;
                }
            }

            let source = match &sf.bytes {
                Some(bytes) => match std::str::from_utf8(bytes) {
                    Ok(s) => s,
                    Err(_) => {
                        index.stats.files_unsupported += 1;
                        continue;
                    }
                },
                None => {
                    index.stats.files_unsupported += 1;
                    continue;
                }
            };

            match parse::parse(source, &rel, &sf.language, &module, &crate_key) {
                Ok(parsed) => {
                    match parsed.status {
                        ParseStatus::Parsed => index.stats.files_parsed += 1,
                        ParseStatus::Partial => {
                            index.stats.files_parsed += 1;
                            index.stats.files_partial += 1;
                        }
                        ParseStatus::Unsupported => index.stats.files_unsupported += 1,
                        ParseStatus::Failed => index.stats.files_failed += 1,
                    }
                    let facts = FileFacts {
                        file: rel.clone(),
                        language: sf.language.clone(),
                        content_hash: sf.content_hash.clone(),
                        status: parsed.status,
                        error: parsed.error,
                        module,
                        crate_key,
                        module_aliases: parsed.module_aliases,
                        symbols: parsed.symbols,
                        imports: parsed.imports,
                        exports: parsed.exports,
                        call_sites: parsed.call_sites,
                        occurrences: parsed.occurrences,
                        scopes: parsed.scopes,
                    };
                    index.facts.insert(rel, facts);
                }
                Err(e) => {
                    index.stats.files_failed += 1;
                    index.facts.insert(
                        rel.clone(),
                        FileFacts {
                            file: rel,
                            language: sf.language.clone(),
                            content_hash: sf.content_hash.clone(),
                            status: ParseStatus::Failed,
                            error: Some(format!("navigation parse error: {e}")),
                            module,
                            crate_key,
                            module_aliases: Vec::new(),
                            symbols: Vec::new(),
                            imports: Vec::new(),
                            exports: Vec::new(),
                            call_sites: Vec::new(),
                            occurrences: Vec::new(),
                            scopes: Vec::new(),
                        },
                    );
                }
            }
        }

        // Anything in the cache that is gone from disk is gone from the index.
        for key in cached.keys() {
            if !current.contains(key) {
                index.stats.files_removed += 1;
            }
        }

        index.build_module_table();
        index.assign_symbol_ids();
        index.build_exports();
        index.build_bindings();
        index.tally();

        index.stats.build_ms = start.elapsed().as_millis();
        // The cache is an optimisation: failing to write it (read-only tree,
        // full disk) must never fail the index itself.
        let changed =
            !index.stats.warm || index.stats.files_parsed > 0 || index.stats.files_removed > 0;
        if changed {
            if let Err(e) = write_cache(&cache_path(&index.root), &index.facts) {
                tracing::debug!("navigation cache not updated: {e:#}");
            }
        }
        Ok(index)
    }

    // -- derived tables ----------------------------------------------------

    /// Path-derived modules first, inline-module aliases second, so an exact
    /// file always wins a collision. Iteration is sorted for determinism.
    fn build_module_table(&mut self) {
        let keys: Vec<String> = self.sorted_files();
        for key in &keys {
            let f = &self.facts[key];
            self.resolver.add_module(&f.crate_key, &f.module, &f.file);
        }
        for key in &keys {
            let f = &self.facts[key];
            for alias in &f.module_aliases {
                self.resolver.add_module(&f.crate_key, alias, &f.file);
            }
        }
    }

    fn sorted_files(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.facts.keys().cloned().collect();
        keys.sort();
        keys
    }

    /// `<file>#<qualified>`, disambiguated with `@<byte>` only on a real
    /// collision (two symbols with the same qualified name in one file).
    fn assign_symbol_ids(&mut self) {
        for file in self.sorted_files() {
            let mut seen: HashMap<String, u32> = HashMap::new();
            let facts = self.facts.get_mut(&file).expect("key from own map");
            let path = facts.file.clone();
            for sym in facts.symbols.iter_mut() {
                let base = format!("{path}#{}", sym.qualified_name);
                let count = seen.entry(base.clone()).or_insert(0);
                sym.id = if *count == 0 {
                    base
                } else {
                    format!("{base}@{}", sym.range.start_byte)
                };
                *count += 1;
            }
        }

        let mut symbols = HashMap::new();
        let mut by_file: HashMap<String, Vec<String>> = HashMap::new();
        let mut by_name: HashMap<(String, String), Vec<String>> = HashMap::new();
        for file in self.sorted_files() {
            let facts = &self.facts[&file];
            for sym in &facts.symbols {
                by_name
                    .entry((file.clone(), sym.name.clone()))
                    .or_default()
                    .push(sym.id.clone());
                by_file
                    .entry(file.clone())
                    .or_default()
                    .push(sym.id.clone());
                symbols.insert(sym.id.clone(), sym.clone());
            }
        }
        self.symbols = symbols;
        self.symbols_by_file = by_file;
        self.symbols_by_name = by_name;
    }

    /// Exported names per file, including re-export chains and `pub use`.
    fn build_exports(&mut self) {
        // 1. Seed from symbols a module itself exports.
        for file in self.sorted_files() {
            for sym in &self.facts[&file].symbols {
                if !sym.exported {
                    continue;
                }
                self.exports
                    .entry(file.clone())
                    .or_default()
                    .entry(sym.name.clone())
                    .or_insert_with(|| sym.id.clone());
            }
        }

        // 2. Fixed point over export/re-export declarations. Chains and
        //    (`a` re-exports `b` re-exports `c`) settle within a few passes;
        //    the cap guarantees termination on a cyclic re-export.
        for _ in 0..8 {
            let mut changed = false;
            for file in self.sorted_files() {
                let facts = &self.facts[&file];
                for decl in &facts.exports {
                    for entry in &decl.entries {
                        if entry.glob {
                            continue;
                        }
                        if self
                            .exports
                            .get(&file)
                            .map(|m| m.contains_key(&entry.exported))
                            .unwrap_or(false)
                        {
                            continue;
                        }
                        let resolved = if let Some(from) = &decl.from {
                            let target =
                                self.resolve_spec(&file, &facts.crate_key, &facts.scope(), from);
                            match target {
                                Some(target) => self
                                    .exports
                                    .get(&target)
                                    .and_then(|m| m.get(&entry.local).cloned())
                                    .or_else(|| self.symbol_id_by_name(&target, &entry.local)),
                                None => None,
                            }
                        } else {
                            self.exports
                                .get(&file)
                                .and_then(|m| m.get(&entry.local).cloned())
                                .or_else(|| self.symbol_id_by_name(&file, &entry.local))
                        };
                        if let Some(id) = resolved {
                            self.exports
                                .entry(file.clone())
                                .or_default()
                                .entry(entry.exported.clone())
                                .or_insert(id);
                            changed = true;
                        }
                    }
                    // Glob re-exports copy the source module's exports.
                    if let Some(from) = &decl.from {
                        if decl.entries.iter().any(|e| e.glob) {
                            let facts_clone = &self.facts[&file];
                            let target = self.resolve_spec(
                                &file,
                                &facts_clone.crate_key,
                                &facts_clone.scope(),
                                from,
                            );
                            if let Some(target) = target {
                                if let Some(src) = self.exports.get(&target).cloned() {
                                    let map = self.exports.entry(file.clone()).or_default();
                                    for (k, v) in src {
                                        if map.insert(k, v).is_none() {
                                            changed = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // 3. `pub use a::b::C;` re-exports `C` from this module.
                for decl in &facts.imports {
                    if !decl.public {
                        continue;
                    }
                    for entry in &decl.entries {
                        if entry.glob || entry.imported.is_empty() {
                            continue;
                        }
                        if self
                            .exports
                            .get(&file)
                            .map(|m| m.contains_key(&entry.imported))
                            .unwrap_or(false)
                        {
                            continue;
                        }
                        if let Some(id) = self.binding_symbol(facts, decl, entry) {
                            self.exports
                                .entry(file.clone())
                                .or_default()
                                .entry(entry.imported.clone())
                                .or_insert(id);
                            changed = true;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
    }

    fn build_bindings(&mut self) {
        let mut updates: Vec<(String, Binding)> = Vec::new();
        let mut resolutions: Vec<(String, ImportResolution)> = Vec::new();

        for file in self.sorted_files() {
            let facts = &self.facts[&file];
            for decl in &facts.imports {
                let mut resolved_file: Option<String> = None;
                let mut external: Option<String> = None;
                let mut reasons: Vec<String> = Vec::new();

                if decl.entries.is_empty() {
                    // `export ... from "x"` contributes an edge but no binding.
                    match self.resolve_spec(
                        &file,
                        &facts.crate_key,
                        &facts.scope(),
                        &decl.specifier,
                    ) {
                        Some(target) => resolved_file = Some(target),
                        None => reasons.push(format!(
                            "specifier `{}` did not resolve to a repository file",
                            decl.specifier
                        )),
                    }
                }

                for entry in &decl.entries {
                    let target = self.entry_target(facts, decl, entry);
                    match &target {
                        // The binding points at a symbol; the *statement* still
                        // needs a file for the dependency edge. Prefer where the
                        // importer literally looked, and never invent a
                        // self-edge out of a locally declared name.
                        BindingTarget::Symbol(id) => {
                            if resolved_file.is_none() {
                                resolved_file = self.entry_file(facts, decl, entry).or_else(|| {
                                    self.symbols
                                        .get(id)
                                        .map(|s| s.file.clone())
                                        .filter(|f| f != &file)
                                });
                            }
                        }
                        BindingTarget::File(f) => {
                            if resolved_file.is_none() {
                                resolved_file = Some(f.clone());
                            }
                        }
                        BindingTarget::External(p) => {
                            external.get_or_insert_with(|| p.clone());
                        }
                        BindingTarget::Unresolved(r) => reasons.push(r.clone()),
                    }
                    updates.push((
                        file.clone(),
                        Binding {
                            local: entry.local.clone(),
                            imported: entry.imported.clone(),
                            glob: entry.glob,
                            target,
                            range: decl.range,
                            specifier: decl.specifier.clone(),
                        },
                    ));
                }

                let resolution = if let Some(target) = resolved_file {
                    ImportResolution::Resolved { file: target }
                } else if let Some(package) = external {
                    ImportResolution::External { package }
                } else {
                    ImportResolution::Unresolved {
                        reason: if reasons.is_empty() {
                            format!("specifier `{}` did not resolve", decl.specifier)
                        } else {
                            reasons.join("; ")
                        },
                    }
                };
                resolutions.push((file.clone(), resolution));
            }
        }

        // Collecting first keeps `&self` and `&mut self` from overlapping.
        // First import of a name wins: re-importing the same local from a
        // second statement does not silently change what the name means.
        for (file, binding) in updates {
            if binding.glob {
                if let BindingTarget::File(target) = &binding.target {
                    let list = self.globs.entry(file.clone()).or_default();
                    if !list.contains(target) {
                        list.push(target.clone());
                    }
                }
            }
            self.bindings
                .entry(file)
                .or_default()
                .entry(binding.local.clone())
                .or_insert(binding);
        }

        // Apply statement-level resolutions in declaration order per file.
        let mut by_file: HashMap<String, std::collections::VecDeque<ImportResolution>> =
            HashMap::new();
        for (file, res) in resolutions {
            by_file.entry(file).or_default().push_back(res);
        }
        for file in self.sorted_files() {
            let Some(queue) = by_file.remove(&file) else {
                continue;
            };
            let facts = self.facts.get_mut(&file).expect("key from own map");
            for (slot, res) in facts.imports.iter_mut().zip(queue) {
                slot.resolution = res;
            }
        }
    }

    fn tally(&mut self) {
        let mut symbols = 0;
        let mut occurrences = 0;
        let mut imports = 0;
        let mut call_sites = 0;
        for f in self.facts.values() {
            symbols += f.symbols.len();
            occurrences += f.occurrences.len();
            imports += f.imports.len();
            call_sites += f.call_sites.len();
        }
        self.stats.symbols = symbols;
        self.stats.occurrences = occurrences;
        self.stats.imports = imports;
        self.stats.call_sites = call_sites;
        self.stats.exports = self.exports.values().map(|m| m.len()).sum();
        self.stats.bindings = self.bindings.values().map(|m| m.len()).sum();
    }

    // -- lookup helpers ----------------------------------------------------

    /// The file module's scope (used for `self::`/`super::` in re-export paths).
    pub fn file(&self, path: &str) -> Option<&FileFacts> {
        self.facts.get(path)
    }

    pub fn symbol(&self, id: &str) -> Option<&Symbol> {
        self.symbols.get(id)
    }

    /// Exported-first lookup of a bare name inside one file.
    pub fn symbol_id_by_name(&self, file: &str, name: &str) -> Option<String> {
        let ids = self
            .symbols_by_name
            .get(&(file.to_string(), name.to_string()))?;
        let mut fallback: Option<String> = None;
        for id in ids {
            let Some(sym) = self.symbols.get(id) else {
                continue;
            };
            if sym.exported {
                return Some(id.clone());
            }
            if fallback.is_none() {
                fallback = Some(id.clone());
            }
        }
        fallback
    }

    pub fn entry_target(
        &self,
        facts: &FileFacts,
        decl: &ImportDecl,
        entry: &ImportEntry,
    ) -> BindingTarget {
        if decl.kind == ImportKind::ModDecl {
            return self.mod_decl_target(facts, entry);
        }
        let is_rust = matches!(facts.language, Language::Rust);
        let specifier = if entry.module.is_empty() {
            &decl.specifier
        } else {
            &entry.module
        };
        let target = self.resolver.resolve_specifier(
            is_rust,
            &facts.crate_key,
            &decl.scope,
            &facts.file,
            specifier,
        );
        match target {
            ModuleTarget::File(file) => {
                if entry.glob || entry.imported.is_empty() {
                    return BindingTarget::File(file);
                }
                if let Some(id) = self
                    .exports
                    .get(&file)
                    .and_then(|m| m.get(&entry.imported).cloned())
                    .or_else(|| self.symbol_id_by_name(&file, &entry.imported))
                {
                    BindingTarget::Symbol(id)
                } else {
                    BindingTarget::File(file)
                }
            }
            ModuleTarget::External(package) => BindingTarget::External(package),
            ModuleTarget::Unresolved(reason) => BindingTarget::Unresolved(reason),
        }
    }

    /// `mod x;` resolves by file layout alone: `dir/x.rs`, then `dir/x/mod.rs`.
    fn mod_decl_file(&self, facts: &FileFacts, name: &str) -> Option<String> {
        let base = resolve::dir_of(&facts.file);
        let mut candidates: Vec<String> = if base.is_empty() {
            vec![format!("{name}.rs"), format!("{name}/mod.rs")]
        } else {
            vec![format!("{base}/{name}.rs"), format!("{base}/{name}/mod.rs")]
        };

        // Rust 2018 module layout: `src/router.rs` may declare `mod decision;`
        // whose file is `src/router/decision.rs`. Without this, a module file
        // that is a sibling of its own directory never resolves — which is what
        // happened to `crates/zylcode-core/src/router.rs`.
        let file_name = facts.file.rsplit('/').next().unwrap_or("");
        let stem = file_name.strip_suffix(".rs").unwrap_or("");
        if !stem.is_empty() && !matches!(stem, "mod" | "lib" | "main") {
            let prefix = if base.is_empty() {
                String::new()
            } else {
                format!("{base}/")
            };
            candidates.push(format!("{prefix}{stem}/{name}.rs"));
        }

        candidates
            .into_iter()
            .find(|c| self.resolver.files.contains(c))
    }

    fn mod_decl_target(&self, facts: &FileFacts, entry: &ImportEntry) -> BindingTarget {
        let name = &entry.local;
        match self.mod_decl_file(facts, name) {
            // The `mod x;` statement also declares a Module symbol in the
            // *declaring* file; that is what a bare `x` should point at.
            Some(candidate) => self
                .symbol_id_by_name(&facts.file, name)
                .map(BindingTarget::Symbol)
                .unwrap_or(BindingTarget::File(candidate)),
            None => BindingTarget::Unresolved(format!(
                "`mod {name};` has no matching file next to `{}`",
                facts.file
            )),
        }
    }

    /// The file an import *statement* points at, independent of whether the
    /// binding itself resolved to a symbol.
    ///
    /// Module declarations resolve by layout; everything else resolves the raw
    /// specifier first (`crate::a::b`), then falls back to the entry's owning
    /// module — `use crate::ledger::LedgerEntry;` names an item, not a module,
    /// so the edge has to stop one level up at `crate::ledger`.
    fn entry_file(
        &self,
        facts: &FileFacts,
        decl: &ImportDecl,
        entry: &ImportEntry,
    ) -> Option<String> {
        if decl.kind == ImportKind::ModDecl {
            return self.mod_decl_file(facts, &entry.local);
        }
        if let Some(f) = self.resolve_spec(
            &facts.file,
            &facts.crate_key,
            &facts.scope(),
            &decl.specifier,
        ) {
            return Some(f);
        }
        if matches!(facts.language, Language::Rust) && !entry.module.is_empty() {
            if let ModuleTarget::File(f) =
                self.resolver
                    .resolve_rust(&facts.crate_key, &decl.scope, &entry.module)
            {
                return Some(f);
            }
        }
        None
    }

    /// The symbol a `pub use` entry re-exports, if it resolves.
    fn binding_symbol(
        &self,
        facts: &FileFacts,
        decl: &ImportDecl,
        entry: &ImportEntry,
    ) -> Option<String> {
        match self.entry_target(facts, decl, entry) {
            BindingTarget::Symbol(id) => Some(id),
            BindingTarget::File(file) => {
                if entry.imported.is_empty() {
                    None
                } else {
                    self.exports
                        .get(&file)
                        .and_then(|m| m.get(&entry.imported).cloned())
                        .or_else(|| self.symbol_id_by_name(&file, &entry.imported))
                }
            }
            _ => None,
        }
    }

    /// Resolve a raw specifier to a file, for re-export sources.
    fn resolve_spec(
        &self,
        from_file: &str,
        crate_key: &str,
        scope: &str,
        spec: &str,
    ) -> Option<String> {
        let facts = self.facts.get(from_file)?;
        let is_rust = matches!(facts.language, Language::Rust);
        let trimmed = spec.trim();
        let target = if is_rust {
            Some(self.resolver.resolve_rust(crate_key, scope, trimmed))
        } else if trimmed.starts_with('.') || trimmed.starts_with('/') {
            Some(self.resolver.resolve_relative(from_file, trimmed))
        } else if trimmed.is_empty() {
            None
        } else {
            Some(self.resolver.resolve_npm(trimmed))
        };
        match target {
            Some(ModuleTarget::File(f)) => Some(f),
            _ => None,
        }
    }
}

impl FileFacts {
    /// The module scope used for `self::`/`super::` rewriting.
    pub fn scope(&self) -> String {
        self.module.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_version_is_the_cache_contract() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("cache.json");
        std::fs::write(&path, r#"{"schema":999999,"files":{}}"#).expect("write");
        assert!(
            load_cache(&path).is_none(),
            "a future schema must be discarded, never read"
        );
        std::fs::write(
            &path,
            format!(r#"{{"schema":{SCHEMA_VERSION},"files":{{}}}}"#),
        )
        .expect("write");
        assert!(load_cache(&path).is_some());
    }

    #[test]
    fn missing_cache_is_not_an_error() {
        let tmp = tempfile::tempdir().expect("tempdir");
        assert!(load_cache(&tmp.path().join("nope.json")).is_none());
    }
}
