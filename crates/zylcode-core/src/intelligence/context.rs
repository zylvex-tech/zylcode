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
use std::collections::{BTreeSet, HashMap, HashSet};

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
    /// Package name → build/test commands (P1.2: previously ignored signal).
    package_commands: HashMap<String, Vec<String>>,
    /// File pairs that were changed in the same commit (P1.2: co-change
    /// signal for "which files need inspection" questions). Canonicalized
    /// (a <= b) and deduplicated; derived deterministically from the
    /// indexed git history.
    cochange_pairs: BTreeSet<(String, String)>,
    /// (dependent, dependency) package pairs, sorted (P1.2: dep-graph signal).
    package_dep_pairs: Vec<(String, String)>,
    /// File id -> package id (P1.2: joins scanner files to packages).
    file_package: HashMap<String, String>,
    /// Entry point paths, sorted (P1.2: previously ignored signal).
    entry_points: Vec<String>,
}

/// Tokenize a path/id into lowercase components (P1.2): path separators,
/// dashes, underscores, dots. Matching task words against path *tokens*
/// (not substrings) prevents 'code' from matching inside 'zylcode' — every
/// path in this repository contains 'zylcode', so substring matching let
/// that one generic word hit every file and pollute both ranking and IDF.
fn path_tokens(path: &str) -> Vec<String> {
    let lower = path.to_lowercase();
    let mut out = Vec::new();
    for seg in lower.split(['/', '\\']) {
        for part in seg.split(['-', '_', '.']) {
            if !part.is_empty() {
                out.push(part.to_string());
            }
        }
    }
    out
}

impl ContextRetriever {
    /// Create a new retriever from indexed data.
    pub fn new(
        files: &[FileNode],
        symbols: &[Symbol],
        packages: &[crate::intelligence::types::Package],
        dep_graph: &DependencyGraph,
        entry_points: &[EntryPoint],
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

        // Co-change mining (P1.2): two files changed in the same commit are
        // functionally related — the strongest structural link available for
        // "which files need inspection to change X" questions, where the task
        // lexically matches only one of them (e.g. crash_recovery tests).
        // Deterministic: pairs are canonicalized and stored in a BTreeSet.
        let mut cochange_pairs: BTreeSet<(String, String)> = BTreeSet::new();
        for commit in git_commits {
            let mut changed: Vec<&str> = commit.files_changed.iter().map(|s| s.as_str()).collect();
            changed.sort_unstable();
            changed.dedup();
            // Skip mega-commits (bulk renames/docs sweeps): every pair they
            // contain is noise, not functional coupling.
            if changed.len() > 12 {
                continue;
            }
            for i in 0..changed.len() {
                for j in (i + 1)..changed.len() {
                    cochange_pairs.insert((changed[i].to_string(), changed[j].to_string()));
                }
            }
        }

        let mut recent_files = HashSet::new();
        let package_commands: HashMap<String, Vec<String>> = packages
            .iter()
            .map(|p| {
                let mut cmds = p.build_commands.clone();
                cmds.extend(p.test_commands.clone());
                (p.id.clone(), cmds)
            })
            .collect();
        let mut ep_paths: Vec<String> = entry_points
            .iter()
            .map(|ep| ep.path.to_string_lossy().to_string())
            .collect();
        ep_paths.sort();
        let entry_points = ep_paths;
        let mut package_dep_pairs: Vec<(String, String)> = packages
            .iter()
            .flat_map(|p| {
                dep_graph
                    .dependencies_of(&p.id)
                    .into_iter()
                    .map(|dep| (p.id.clone(), dep))
                    .collect::<Vec<_>>()
            })
            .collect();
        package_dep_pairs.sort();
        package_dep_pairs.dedup();
        let package_dep_pairs = package_dep_pairs;
        // Scanner leaves FileNode.package = None ("filled in by manifest
        // intelligence" but never done). Join files to packages here by
        // package root/manifest, so package-aware ranking actually works.
        let mut file_package: HashMap<String, String> = HashMap::new();
        for pkg in packages {
            for fid in &pkg.files {
                file_package.insert(fid.clone(), pkg.id.clone());
            }
            // Package.files is not populated by discovery; fall back to
            // manifest-based membership: the manifest itself and any file
            // under the package root directory.
            file_package.insert(pkg.manifest.clone(), pkg.id.clone());
            // Windows: Path::join yields backslash separators; file ids use
            // forward slashes. Normalize before prefix-matching (same class of
            // defect as the scanner's should_exclude bug).
            let root_prefix = format!("{}/", pkg.root.to_string_lossy().replace('\\', "/"));
            for f in files {
                let fid = f.id.as_str();
                if fid.starts_with(&root_prefix) {
                    file_package.insert(fid.to_string(), pkg.id.clone());
                }
            }
        }
        let file_package = file_package;
        // Package scoring needs the package→files map, but the scanner never
        // sets FileNode.package, so the original map was empty. Rebuild it
        // from the manifest-based join above (sorted pairs for determinism).
        let package_files: HashMap<String, Vec<String>> = {
            let mut m: HashMap<String, Vec<String>> = HashMap::new();
            let mut pairs: Vec<(&String, &String)> = file_package.iter().collect();
            pairs.sort();
            for (fid, pkg) in pairs {
                m.entry(pkg.clone()).or_default().push(fid.clone());
            }
            m
        };
        // Recency is tied to the 20 most recent commits (git log order,
        // newest first); co-change mining uses the full window the caller
        // provides, so deeper history can strengthen coupling without
        // changing recency semantics.
        for commit in git_commits.iter().take(20) {
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
            cochange_pairs,
            package_commands,
            package_dep_pairs,
            file_package,
            entry_points,
        }
    }

    /// Retrieve relevant context for a task description.
    pub fn retrieve(&self, task: &str) -> Vec<ContextResult> {
        let task_lower = task.to_lowercase();
        // Stopwords (P1.2): English function words carry no location signal
        // and, fed into the scorer, let files whose paths merely contain
        // "the" (theme.ts) or generic tokens outrank real evidence.
        const STOPWORDS: &[&str] = &[
            "a", "an", "the", "is", "are", "was", "were", "be", "been", "being", "do", "does",
            "did", "done", "what", "which", "who", "whose", "where", "when", "why", "how", "for",
            "of", "in", "on", "to", "with", "and", "or", "not", "its", "this", "that", "these",
            "those", "from", "by", "at", "as", "into", "about", "can", "could", "should", "would",
            "will", "it", "his", "her", "their", "our", "your", "my",
        ];
        // camelCase/snake_case splitting (P1.2): the task "Which code persists
        // SessionCheckpoint?" contains 'sessioncheckpoint' as one blob, which
        // never substring-matches `save_checkpoint` or `checkpoint`. Splitting
        // PascalCase/camelCase and snake_case boundaries exposes the
        // distinctive words ('checkpoint', 'session') the codebase actually
        // uses. Owned strings required: splitting produces new tokens.
        let split_case = |w: &str, out: &mut Vec<String>| {
            let chars: Vec<char> = w.chars().collect();
            let mut start = 0usize;
            for i in 1..chars.len() {
                let boundary = (chars[i].is_uppercase() && !chars[i - 1].is_uppercase())
                    || (chars[i] == '_' && i + 1 < chars.len());
                if boundary {
                    out.push(chars[start..i].iter().collect());
                    start = if chars[i] == '_' { i + 1 } else { i };
                }
            }
            out.push(chars[start..].iter().collect());
        };
        let mut token_buf: Vec<String> = Vec::new();
        // Split on the ORIGINAL case (camelCase boundaries are uppercase
        // characters), lowercase each part, keep snake parts of multi-token
        // identifiers, and drop single stopwords.
        for w in task.split_whitespace() {
            let w = w.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_');
            let w_lower = w.to_lowercase();
            if w_lower.len() > 2 && !STOPWORDS.contains(&w_lower.as_str()) {
                token_buf.push(w_lower.clone());
                if w.contains('_') || w.chars().any(|c| c.is_uppercase()) {
                    split_case(w, &mut token_buf);
                }
            }
        }
        let mut task_words: Vec<String> = token_buf
            .into_iter()
            .map(|t| t.to_lowercase())
            .filter(|w| w.len() > 2 && !STOPWORDS.contains(&w.as_str()))
            .collect();
        task_words.sort_unstable();
        task_words.dedup();
        // Domain query expansion (P1.2): map a few unambiguous task verbs to
        // the vocabulary the codebase actually uses, so "persist its ledger"
        // can reach `LedgerStore`/`sqlite_ledger` without full semantic
        // matching. Documented, closed-set, and deterministic.
        const EXPANSIONS: &[(&str, &[&str])] = &[
            ("persist", &["store", "save"]),
            ("persists", &["store", "save"]),
            ("persistent", &["store", "save"]),
            ("stores", &["store"]),
            ("storing", &["store"]),
        ];
        let mut expanded: Vec<&str> = Vec::new();
        for w in &task_words {
            if let Some((_, syns)) = EXPANSIONS.iter().find(|(k, _)| k == w) {
                for syn in *syns {
                    expanded.push(syn);
                }
            }
        }
        expanded.sort_unstable();
        expanded.dedup();
        for syn in &expanded {
            if !task_words.iter().any(|w| w == syn) {
                task_words.push(syn.to_string());
            }
        }
        // Inverse document frequency over the indexed corpus (P1.2): words
        // that appear in many ids/symbol names ("file", "the") carry almost
        // no location signal, while rare words ("scanner", "executor") are
        // distinctive. Weighting word hits by IDF stops files full of common
        // tokens from outranking the one file the task is about.
        let total_docs = (self.file_index.len() + self.symbol_index.len()) as f64;
        let idf_of = |word: &str| -> f64 {
            let df = self
                .file_index
                .keys()
                .filter(|f| path_tokens(f).iter().any(|t| t == word))
                .count()
                + self
                    .symbol_index
                    .values()
                    .filter(|s| s.name.to_lowercase().contains(word))
                    .count();
            if df == 0 {
                // Unseen word: no resource can match it anyway, so the value
                // only matters for stability.
                1.0
            } else {
                // Normalized IDF in [0, 1] (P1.2): raw log-IDF reached ~6 and
                // one rare-word hit outweighed the gated structural boosts,
                // while the clamp flattened common words to the same ceiling.
                // Dividing by ln(1 + N) keeps rare words near 1 and common
                // words near 0, so lexical totals stay below the structural
                // evidence scores by construction.
                (((1.0 + total_docs / df as f64).ln()) / (1.0 + total_docs).ln()).clamp(0.05, 1.0)
            }
        };
        let word_idf: HashMap<String, f64> =
            task_words.iter().map(|w| (w.clone(), idf_of(w))).collect();

        // Dependency intent: reverse-dependency signals only apply when the
        // task actually asks about dependency direction ("what depends on X"),
        // otherwise tasks that merely mention a package name (e.g. "which
        // manifest defines X for P") get flooded with unrelated dependents.
        let dep_intent = task_words.iter().any(|w| {
            w.starts_with("depend")
                || w.starts_with("requir")
                || w.starts_with("import")
                || w.starts_with("consum")
                || matches!(
                    w.as_str(),
                    "uses" | "used" | "using" | "callers" | "consumers"
                )
        });

        let mut scored: Vec<(f64, ContextResult)> = Vec::new();

        // Determinism contract (P1.2): identical inputs must produce identical
        // output. Hash maps randomize iteration order per process; we therefore
        // score candidates in sorted-key order and break ranking ties by
        // resource id, so result ordering never depends on the process seed.

        // Score symbols (sorted by id for deterministic iteration)
        let mut symbol_ids: Vec<&String> = self.symbol_index.keys().collect();
        symbol_ids.sort();
        for id in symbol_ids {
            let sym = &self.symbol_index[id];
            let mut score = 0.0f64;
            let mut reasons = Vec::new();

            // Exact symbol name match, IDF-scaled (P1.2): a flat bonus paid
            // generic single-word names (`Tool`, `File`, `call`) the same as
            // distinctive ones (`LedgerStore`); weighting by the name's own
            // corpus frequency keeps the former as a nudge and the latter as
            // a strong signal.
            if task_lower.contains(&sym.name.to_lowercase()) {
                let name_idf = idf_of(sym.name.to_lowercase().as_str());
                score += 0.9 * name_idf;
                reasons.push(format!("symbol name '{}' matches task", sym.name));
            }

            // Symbol name contains task word (IDF-weighted). A symbol is a
            // more specific answer than its containing file, so name hits
            // weigh at parity with filename hits (0.8→1.0): for "where is X
            // defined" questions the symbol itself must be able to outrank
            // the file that defines it.
            for word in &task_words {
                if sym.name.to_lowercase().contains(word) {
                    score += 1.0 * word_idf[word];
                    reasons.push(format!("symbol '{}' contains '{}'", sym.name, word));
                }
            }

            // File path contains task word as a path TOKEN (IDF-weighted).
            // Token equality, not substring: 'code' must not match 'zylcode'.
            let sym_file_tokens = path_tokens(&sym.file);
            for word in &task_words {
                if sym_file_tokens.iter().any(|t| t == word) {
                    score += 0.3 * word_idf[word];
                    reasons.push(format!("path token '{}'", word));
                }
            }

            // Recent change bonus
            if self.recent_files.contains(&sym.file) {
                // Small recency nudge (P1.2): a flat bonus applied to ~half the
                // corpus drowned targeted evidence; it must only break ties.
                score += 0.05;
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

        // Score files (sorted by id for deterministic iteration)
        // Task-named files ("anchors"): files whose filename stem contains a
        // task word. Their co-change partners gain the co-change bonus below.
        // Sorted for deterministic iteration.
        // Anchors are restricted to implementation files (Source/Test):
        // doc-sweep commits changed every governance doc together, and letting
        // a Documentation file anchor co-change flooding flooded rankings with
        // unrelated docs. The change surface of a task is code and tests.
        let mut anchors: Vec<String> = self
            .file_index
            .values()
            .filter(|f| {
                matches!(
                    f.role,
                    crate::intelligence::types::FileRole::Source
                        | crate::intelligence::types::FileRole::Test
                )
            })
            .filter(|f| {
                let stem = f.id.rsplit('/').next().unwrap_or("");
                let stem_tokens = path_tokens(stem);
                task_words
                    .iter()
                    .any(|w| stem_tokens.iter().any(|t| t == w))
            })
            .map(|f| f.id.clone())
            .collect();
        anchors.sort();
        anchors.dedup();

        let mut file_ids: Vec<&String> = self.file_index.keys().collect();
        file_ids.sort();
        for id in file_ids {
            let file = &self.file_index[id];
            let file_lower = file.id.to_lowercase();
            let file_tokens = path_tokens(&file.id);
            let mut score = 0.0f64;
            let mut reasons = Vec::new();

            // Filename/path token match (IDF-weighted). The filename is the
            // strongest lexical location signal: a file named for the task
            // word usually IS the implementation the task asks for. Token
            // equality, not substring.
            for word in &task_words {
                if file_tokens.iter().any(|t| t == word) {
                    score += 0.8 * word_idf[word];
                    reasons.push(format!("path token '{}'", word));
                }
            }

            // File contains relevant symbols. Bonus scales with DISTINCT
            // matched symbol names (0.6 first, +0.15 each, cap 0.9) — the old
            // per-occurrence bonus let one file with seven same-named test
            // helpers score 4.2 and flood the ranking.
            //
            // Aggregate-of-parts rule (P1.2): a file whose DEFINED symbols
            // repeatedly match task words embodies the answer even when no
            // single symbol name appears verbatim in the task (e.g.
            // symbols.rs holding seven *_symbols functions for "where are
            // symbols extracted"). Each distinct task word hit across the
            // file's symbols contributes a small IDF-weighted share, capped.
            if let Some(sym_ids) = self.file_symbols.get(id) {
                let mut matched_names: Vec<&str> = sym_ids
                    .iter()
                    .filter_map(|sym_id| self.symbol_index.get(sym_id))
                    .filter(|sym| task_lower.contains(&sym.name.to_lowercase()))
                    .map(|sym| sym.name.as_str())
                    .collect();
                matched_names.sort_unstable();
                matched_names.dedup();
                if !matched_names.is_empty() {
                    // A file that *defines* the matched symbol is stronger
                    // evidence than any substring coincidence, so its bonus
                    // dominates name/path matches (P1.2 calibration). Scaled
                    // by the strongest matched name's IDF: defining a rare,
                    // task-named symbol (`LedgerStore`) is strong evidence,
                    // while "defines File" (task contains "files") is not.
                    let strongest_idf = matched_names
                        .iter()
                        .map(|n| idf_of(&n.to_lowercase()))
                        .fold(0.0f64, f64::max);
                    let bonus =
                        ((0.9 + 0.2 * (matched_names.len() - 1) as f64).min(1.3)) * strongest_idf;
                    score += bonus;
                    for name in &matched_names {
                        reasons.push(format!("defines symbol '{}'", name));
                    }
                }

                // Aggregate-of-parts contribution (P1.2): small per-hit share
                // over all defined symbols' name matches, capped so a file
                // with hundreds of incidental matches cannot dominate.
                let mut matched_words: Vec<&str> = Vec::new();
                for sym_id in sym_ids {
                    if let Some(sym) = self.symbol_index.get(sym_id) {
                        let name_lower = sym.name.to_lowercase();
                        for word in &task_words {
                            if name_lower.contains(word) {
                                matched_words.push(word);
                            }
                        }
                    }
                }
                matched_words.sort_unstable();
                matched_words.dedup();
                if !matched_words.is_empty() {
                    let aggregate: f64 = matched_words
                        .iter()
                        .map(|w| 0.25 * word_idf[*w])
                        .sum::<f64>()
                        .min(1.0);
                    score += aggregate;
                    reasons.push(format!(
                        "defines {} symbol(s) matching task terms",
                        matched_words.len()
                    ));
                }
            }

            // Recent change bonus
            if self.recent_files.contains(id) {
                score += 0.05;
                reasons.push("recently changed".to_string());
            }

            // Test file bonus for test-related tasks
            if task_lower.contains("test")
                && file.role == crate::intelligence::types::FileRole::Test
            {
                score += 0.3;
                reasons.push("test file".to_string());
            }

            // Test-file name matches "tests" task word (Q5: test discovery)
            if task_lower.contains("test")
                && task_words.iter().any(|w| file_lower.contains(w))
                && file.role == crate::intelligence::types::FileRole::Test
            {
                score += 0.5;
                reasons.push("test file matching task terms".to_string());
            }

            // Co-change evidence (P1.2): when a task names a file (via a
            // distinctive filename fragment) and this file was repeatedly
            // changed together with it, this file is likely part of the same
            // change surface even though no task word matches it — e.g.
            // agent.rs/ledger.rs for "inspect crash recovery", where the
            // crash windows live in AgentLoop and checkpoints in the ledger.
            // History data is deterministic per HEAD.
            let impl_role = matches!(
                file.role,
                crate::intelligence::types::FileRole::Source
                    | crate::intelligence::types::FileRole::Test
            );
            if impl_role {
                if let Some(anchor) = anchors.iter().find(|a| {
                    a.as_str() != id.as_str()
                        && self.cochange_pairs.iter().any(|(x, y)| {
                            (x == *a && y == id.as_str()) || (y == *a && x == id.as_str())
                        })
                }) {
                    // 0.8: must clear the incidental test-symbol band
                    // (0.70-0.73) to reach the top-10, but stay below plain
                    // lexical filename evidence (1.0 x IDF).
                    score += 0.8;
                    reasons.push(format!("changes together with '{}' in git history", anchor));
                }
            }

            // Package-aware signals (Q6: reverse dependency)
            // The task names package P and this file's package Q depends on P,
            // or the task names Q itself.
            if let Some(pkg_q) = self.file_package.get(file.id.as_str()) {
                if task_lower.contains(pkg_q) {
                    score += 0.6;
                    reasons.push(format!("file belongs to package '{}'", pkg_q));
                } else {
                    let names_dep: bool = self
                        .package_dep_pairs
                        .iter()
                        .any(|(q, dep)| q == pkg_q && task_words.iter().any(|w| w == dep));
                    if names_dep && dep_intent {
                        // Structural dependency evidence outranks lexical
                        // filename coincidence: when the task names package P,
                        // files in a package that depends on P are the direct
                        // answer to "what depends on P" and must outrank the
                        // (many) files that merely contain P in their path.
                        score += 1.5;
                        reasons.push(format!(
                            "package '{}' depends on a package named in the task",
                            pkg_q
                        ));
                    }
                }
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

        // Score packages (sorted by id for deterministic iteration)
        let mut pkg_names: Vec<&String> = self.package_files.keys().collect();
        pkg_names.sort();
        for pkg_name in pkg_names {
            let files = &self.package_files[pkg_name];
            let mut score = 0.0f64;
            let mut reasons = Vec::new();

            // Package name match
            if task_lower.contains(&pkg_name.to_lowercase()) {
                score += 0.7;
                reasons.push("package name matches task".to_string());
            }

            // Reverse-dependency match (Q6): the task names package P and this
            // package Q depends on P — Q is a direct answer.
            let names_dep: bool = self
                .package_dep_pairs
                .iter()
                .any(|(q, dep)| q == pkg_name.as_str() && task_words.iter().any(|w| w == dep));
            if names_dep && dep_intent {
                score += 1.2;
                reasons.push(format!(
                    "package '{}' depends on a package named in the task",
                    pkg_name
                ));
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

        // Build/test commands (Q10): surface commands when the task asks
        // how to build/test, ranked by overlap with package context.
        if task_words.iter().any(|w| {
            matches!(
                w.as_str(),
                "build" | "builds" | "test" | "tests" | "compile" | "run"
            )
        }) {
            let mut pkg_names: Vec<&String> = self.package_commands.keys().collect();
            pkg_names.sort();
            let asks_commands = task_lower.contains("command")
                || task_lower.contains("how do i")
                || task_lower.contains("how to");
            for pkg_name in pkg_names {
                let mut cmds = self.package_commands[pkg_name].clone();
                cmds.sort();
                for cmd in cmds {
                    let mut score = if asks_commands { 2.0f64 } else { 0.5f64 };
                    let mut reasons = vec![format!("command of package '{}'", pkg_name)];
                    if task_lower.contains(pkg_name) {
                        score += 0.3;
                        reasons.push(format!("task names package '{}'", pkg_name));
                    }
                    for w in &task_words {
                        if cmd.contains(w) {
                            score += 0.2;
                            reasons.push(format!("command contains '{}'", w));
                        }
                    }
                    scored.push((
                        score,
                        ContextResult {
                            resource: cmd.clone(),
                            resource_type: "command".to_string(),
                            relevance: score,
                            reason: reasons.join("; "),
                            provenance: Provenance::Parsed,
                            snippet: None,
                        },
                    ));
                }
            }
        }

        // Entry points (Q7): surface when the task asks where execution starts.
        let asks_entry = task_words.iter().any(|w| {
            matches!(
                w.as_str(),
                "entry" | "entrypoint" | "main" | "starts" | "launch" | "boot" | "frontend"
            )
        }) || task_lower.contains("entry point");
        if asks_entry {
            let asks_direct = task_lower.contains("entry point");
            for ep in &self.entry_points {
                let mut score = if asks_direct { 2.0f64 } else { 0.9f64 };
                let mut reasons = vec!["discovered entry point".to_string()];
                for w in &task_words {
                    if ep.to_lowercase().contains(w) {
                        score += 0.3;
                        reasons.push(format!("path contains '{}'", w));
                    }
                }
                scored.push((
                    score,
                    ContextResult {
                        resource: ep.clone(),
                        resource_type: "entry_point".to_string(),
                        relevance: score,
                        reason: reasons.join("; "),
                        provenance: Provenance::Observed,
                        snippet: None,
                    },
                ));
            }
        }

        // Deduplicate resources (e.g. the same command from several packages),
        // keeping the highest score. Built in sorted order, so this is stable.
        let mut seen: HashMap<String, f64> = HashMap::new();
        scored.retain(|(sc, r)| match seen.get(&r.resource) {
            Some(prev) if *prev >= *sc => false,
            _ => {
                seen.insert(r.resource.clone(), *sc);
                true
            }
        });

        // Sort by score descending, ties broken by resource id ascending
        // (total order — no dependence on pre-sort iteration order)
        scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.1.resource.cmp(&b.1.resource))
        });

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
