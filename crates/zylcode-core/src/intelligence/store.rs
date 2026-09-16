//! Repository Intelligence persistence store.
//!
//! SQLite-backed storage for the repository index.
//! Supports incremental invalidation via content hashing.
//! Separate from the Evidence Ledger (Section 10 requirement).

use crate::intelligence::types::{
    ArchitecturalFact, DependencyTarget, EntryPoint, FileNode, GitCommit, IndexMetadata, Language, LanguageSupport,
    Package, Provenance, Symbol, SymbolKind, Visibility,
};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// SQLite-backed repository intelligence store.
pub struct RepoStore {
    conn: Mutex<Connection>,
    repo_root: PathBuf,
}

impl RepoStore {
    /// Open or create a repository intelligence store.
    pub fn new(db_path: &Path, repo_root: &Path) -> Result<Self> {
        let conn = Connection::open(db_path).context("Failed to open repo index DB")?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS repo_metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS files (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL,
                language TEXT NOT NULL,
                role TEXT NOT NULL,
                size INTEGER NOT NULL,
                content_hash TEXT NOT NULL,
                package TEXT,
                support_level TEXT NOT NULL,
                indexed_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS symbols (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                file TEXT NOT NULL,
                line INTEGER NOT NULL,
                end_line INTEGER,
                visibility TEXT NOT NULL,
                parent TEXT,
                language TEXT NOT NULL,
                support_level TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS packages (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                root TEXT NOT NULL,
                language TEXT NOT NULL,
                manifest TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS dependencies (
                from_pkg TEXT NOT NULL,
                to_target TEXT NOT NULL,
                to_kind TEXT NOT NULL,
                kind TEXT NOT NULL,
                version_constraint TEXT NOT NULL,
                features TEXT NOT NULL,
                optional INTEGER NOT NULL,
                FOREIGN KEY (from_pkg) REFERENCES packages(id)
            );

            CREATE TABLE IF NOT EXISTS entry_points (
                path TEXT NOT NULL,
                kind TEXT NOT NULL,
                package TEXT,
                evidence TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS git_commits (
                id TEXT PRIMARY KEY,
                short_id TEXT NOT NULL,
                message TEXT NOT NULL,
                author TEXT NOT NULL,
                timestamp TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS commit_files (
                commit_id TEXT NOT NULL,
                file_id TEXT NOT NULL,
                FOREIGN KEY (commit_id) REFERENCES git_commits(id)
            );

            CREATE TABLE IF NOT EXISTS repo_facts (
                id TEXT PRIMARY KEY,
                fact_type TEXT NOT NULL,
                subject TEXT NOT NULL,
                value TEXT NOT NULL,
                source TEXT NOT NULL,
                source_hash TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                provenance TEXT NOT NULL,
                confidence REAL NOT NULL
            );

            CREATE TABLE IF NOT EXISTS architectural_facts (
                fact TEXT NOT NULL,
                value TEXT NOT NULL,
                evidence TEXT NOT NULL,
                provenance TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_files_language ON files(language);
            CREATE INDEX IF NOT EXISTS idx_files_package ON files(package);
            CREATE INDEX IF NOT EXISTS idx_symbols_file ON symbols(file);
            CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name);
            CREATE INDEX IF NOT EXISTS idx_dependencies_from ON dependencies(from_pkg);
            CREATE INDEX IF NOT EXISTS idx_commit_files_commit ON commit_files(commit_id);
            ",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
            repo_root: repo_root.to_path_buf(),
        })
    }

    /// Store metadata.
    pub fn set_metadata(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO repo_metadata (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    /// Get metadata.
    pub fn get_metadata(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM repo_metadata WHERE key = ?1")?;
        let mut rows = stmt.query_map(params![key], |row| row.get::<_, String>(0))?;
        Ok(rows.next().transpose()?)
    }

    /// Store files (upsert).
    pub fn store_files(&self, files: &[FileNode]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        for file in files {
            conn.execute(
                "INSERT OR REPLACE INTO files (id, path, language, role, size, content_hash, package, support_level, indexed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    file.id,
                    file.path.to_string_lossy(),
                    format!("{:?}", file.language),
                    format!("{:?}", file.role),
                    file.size as i64,
                    file.content_hash,
                    file.package,
                    format!("{:?}", file.support_level),
                    file.indexed_at.to_rfc3339(),
                ],
            )?;
        }
        Ok(())
    }

    /// Get all files.
    pub fn get_files(&self) -> Result<Vec<FileNode>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, path, language, role, size, content_hash, package, support_level, indexed_at FROM files",
        )?;

        let files = stmt
            .query_map([], |row| {
                let lang_str: String = row.get(2)?;
                let role_str: String = row.get(3)?;
                let support_str: String = row.get(7)?;
                let indexed_str: String = row.get(8)?;

                Ok(FileNode {
                    id: row.get(0)?,
                    path: PathBuf::from(row.get::<_, String>(1)?),
                    language: parse_language(&lang_str),
                    role: parse_file_role(&role_str),
                    size: row.get::<_, i64>(4)? as u64,
                    content_hash: row.get(5)?,
                    package: row.get(6)?,
                    symbols: Vec::new(),
                    imports: Vec::new(),
                    indexed_at: DateTime::parse_from_rfc3339(&indexed_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    support_level: parse_support_level(&support_str),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(files)
    }

    /// Get files that changed since a given timestamp.
    pub fn get_changed_files_since(&self, since: &DateTime<Utc>) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id FROM files WHERE indexed_at > ?1")?;

        let ids = stmt
            .query_map(params![since.to_rfc3339()], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ids)
    }

    /// Store symbols.
    pub fn store_symbols(&self, symbols: &[Symbol]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        for sym in symbols {
            conn.execute(
                "INSERT OR REPLACE INTO symbols (id, name, kind, file, line, end_line, visibility, parent, language, support_level)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    sym.id,
                    sym.name,
                    format!("{:?}", sym.kind),
                    sym.file,
                    sym.line as i64,
                    sym.end_line.map(|l| l as i64),
                    format!("{:?}", sym.visibility),
                    sym.parent,
                    format!("{:?}", sym.language),
                    format!("{:?}", sym.support_level),
                ],
            )?;
        }
        Ok(())
    }

    /// Get all symbols.
    pub fn get_symbols(&self) -> Result<Vec<Symbol>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, kind, file, line, end_line, visibility, parent, language, support_level FROM symbols",
        )?;

        let symbols = stmt
            .query_map([], |row| {
                let kind_str: String = row.get(2)?;
                let vis_str: String = row.get(6)?;
                let lang_str: String = row.get(8)?;
                let support_str: String = row.get(9)?;

                Ok(Symbol {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    kind: parse_symbol_kind(&kind_str),
                    file: row.get(3)?,
                    line: row.get::<_, i64>(4)? as u32,
                    end_line: row.get::<_, Option<i64>>(5)?.map(|l| l as u32),
                    visibility: parse_visibility(&vis_str),
                    parent: row.get(7)?,
                    language: parse_language(&lang_str),
                    support_level: parse_support_level(&support_str),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(symbols)
    }

    /// Store packages.
    pub fn store_packages(&self, packages: &[Package]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        for pkg in packages {
            conn.execute(
                "INSERT OR REPLACE INTO packages (id, name, version, root, language, manifest)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    pkg.id,
                    pkg.name,
                    pkg.version,
                    pkg.root.to_string_lossy(),
                    format!("{:?}", pkg.language),
                    pkg.manifest,
                ],
            )?;

            // Clear old dependencies for this package
            conn.execute(
                "DELETE FROM dependencies WHERE from_pkg = ?1",
                params![pkg.id],
            )?;

            // Store dependencies
            for dep in &pkg.dependencies {
                let (to_target, to_kind) = match &dep.to {
                    DependencyTarget::Internal(id) => (id.clone(), "internal".to_string()),
                    DependencyTarget::External { name, registry } => {
                        (format!("{}@{}", name, registry), "external".to_string())
                    }
                };
                conn.execute(
                    "INSERT INTO dependencies (from_pkg, to_target, to_kind, kind, version_constraint, features, optional)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        dep.from,
                        to_target,
                        to_kind,
                        format!("{:?}", dep.kind),
                        dep.version_constraint,
                        serde_json::to_string(&dep.features).unwrap_or_default(),
                        dep.optional as i32,
                    ],
                )?;
            }
        }
        Ok(())
    }

    /// Get all packages.
    pub fn get_packages(&self) -> Result<Vec<Package>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT id, name, version, root, language, manifest FROM packages")?;

        let packages = stmt
            .query_map([], |row| {
                let lang_str: String = row.get(4)?;
                Ok(Package {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    version: row.get(2)?,
                    root: PathBuf::from(row.get::<_, String>(3)?),
                    language: parse_language(&lang_str),
                    manifest: row.get(5)?,
                    files: Vec::new(),
                    dependencies: Vec::new(),
                    dev_dependencies: Vec::new(),
                    build_commands: Vec::new(),
                    test_commands: Vec::new(),
                    entry_points: Vec::new(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(packages)
    }

    /// Store entry points.
    pub fn store_entry_points(&self, entry_points: &[EntryPoint]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM entry_points", [])?;

        for ep in entry_points {
            conn.execute(
                "INSERT INTO entry_points (path, kind, package, evidence)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    ep.path.to_string_lossy(),
                    format!("{:?}", ep.kind),
                    ep.package,
                    serde_json::to_string(&ep.evidence).unwrap_or_default(),
                ],
            )?;
        }
        Ok(())
    }

    /// Store git commits.
    pub fn store_git_commits(&self, commits: &[GitCommit]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        for commit in commits {
            conn.execute(
                "INSERT OR IGNORE INTO git_commits (id, short_id, message, author, timestamp)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    commit.id,
                    commit.short_id,
                    commit.message,
                    commit.author,
                    commit.timestamp.to_rfc3339(),
                ],
            )?;

            // Store commit files
            conn.execute(
                "DELETE FROM commit_files WHERE commit_id = ?1",
                params![commit.id],
            )?;
            for file in &commit.files_changed {
                conn.execute(
                    "INSERT INTO commit_files (commit_id, file_id) VALUES (?1, ?2)",
                    params![commit.id, file],
                )?;
            }
        }
        Ok(())
    }

    /// Get recent git commits.
    pub fn get_git_commits(&self, limit: usize) -> Result<Vec<GitCommit>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, short_id, message, author, timestamp FROM git_commits ORDER BY timestamp DESC LIMIT ?1",
        )?;

        let commits = stmt
            .query_map(params![limit as i64], |row| {
                let ts_str: String = row.get(4)?;
                Ok(GitCommit {
                    id: row.get(0)?,
                    short_id: row.get(1)?,
                    message: row.get(2)?,
                    author: row.get(3)?,
                    timestamp: DateTime::parse_from_rfc3339(&ts_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    files_changed: Vec::new(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(commits)
    }

    /// Store architectural facts.
    pub fn store_architecture(&self, facts: &[ArchitecturalFact]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM architectural_facts", [])?;

        for fact in facts {
            conn.execute(
                "INSERT INTO architectural_facts (fact, value, evidence, provenance)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    fact.fact,
                    fact.value,
                    serde_json::to_string(&fact.evidence).unwrap_or_default(),
                    format!("{:?}", fact.provenance),
                ],
            )?;
        }
        Ok(())
    }

    /// Get architectural facts.
    pub fn get_architecture(&self) -> Result<Vec<ArchitecturalFact>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT fact, value, evidence, provenance FROM architectural_facts")?;

        let facts = stmt
            .query_map([], |row| {
                let evidence_str: String = row.get(2)?;
                let provenance_str: String = row.get(3)?;
                Ok(ArchitecturalFact {
                    fact: row.get(0)?,
                    value: row.get(1)?,
                    evidence: serde_json::from_str(&evidence_str).unwrap_or_default(),
                    provenance: parse_provenance(&provenance_str),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(facts)
    }

    /// Clear the entire index (for full re-scan).
    pub fn clear(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            DELETE FROM files;
            DELETE FROM symbols;
            DELETE FROM packages;
            DELETE FROM dependencies;
            DELETE FROM entry_points;
            DELETE FROM git_commits;
            DELETE FROM commit_files;
            DELETE FROM repo_facts;
            DELETE FROM architectural_facts;
            ",
        )?;
        Ok(())
    }

    /// Get index statistics.
    pub fn stats(&self) -> Result<IndexMetadata> {
        let conn = self.conn.lock().unwrap();
        let file_count: i64 = conn.query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))?;
        let symbol_count: i64 = conn.query_row("SELECT COUNT(*) FROM symbols", [], |r| r.get(0))?;
        let dep_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM dependencies", [], |r| r.get(0))?;

        let repo_id = self.get_metadata("repo_id")?.unwrap_or_default();

        Ok(IndexMetadata {
            repo_id,
            repo_root: self.repo_root.clone(),
            last_full_scan: Utc::now(),
            last_incremental: Utc::now(),
            file_count: file_count as usize,
            symbol_count: symbol_count as usize,
            dependency_count: dep_count as usize,
            version: 1,
        })
    }
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

fn parse_language(s: &str) -> Language {
    match s {
        "Rust" => Language::Rust,
        "TypeScript" => Language::TypeScript,
        "JavaScript" => Language::JavaScript,
        "Json" => Language::Json,
        "Toml" => Language::Toml,
        "Yaml" => Language::Yaml,
        "Markdown" => Language::Markdown,
        "Html" => Language::Html,
        "Css" => Language::Css,
        "Shell" => Language::Shell,
        other => Language::Unknown(other.to_string()),
    }
}

fn parse_file_role(s: &str) -> crate::intelligence::types::FileRole {
    match s {
        "Source" => crate::intelligence::types::FileRole::Source,
        "Test" => crate::intelligence::types::FileRole::Test,
        "Config" => crate::intelligence::types::FileRole::Config,
        "Documentation" => crate::intelligence::types::FileRole::Documentation,
        "BuildScript" => crate::intelligence::types::FileRole::BuildScript,
        "Generated" => crate::intelligence::types::FileRole::Generated,
        "Vendor" => crate::intelligence::types::FileRole::Vendor,
        "Dependency" => crate::intelligence::types::FileRole::Dependency,
        "EntryPoint" => crate::intelligence::types::FileRole::EntryPoint,
        "LockFile" => crate::intelligence::types::FileRole::LockFile,
        "Script" => crate::intelligence::types::FileRole::Script,
        "Ci" => crate::intelligence::types::FileRole::Ci,
        "Asset" => crate::intelligence::types::FileRole::Asset,
        other => crate::intelligence::types::FileRole::Other(other.to_string()),
    }
}

fn parse_support_level(s: &str) -> LanguageSupport {
    match s {
        "Detected" => LanguageSupport::Detected,
        "Parsed" => LanguageSupport::Parsed,
        "SemanticallyIndexed" => LanguageSupport::SemanticallyIndexed,
        _ => LanguageSupport::Detected,
    }
}

fn parse_symbol_kind(s: &str) -> SymbolKind {
    match s {
        "Module" => SymbolKind::Module,
        "Struct" => SymbolKind::Struct,
        "Enum" => SymbolKind::Enum,
        "Trait" => SymbolKind::Trait,
        "Impl" => SymbolKind::Impl,
        "Function" => SymbolKind::Function,
        "Method" => SymbolKind::Method,
        "Constant" => SymbolKind::Constant,
        "TypeAlias" => SymbolKind::TypeAlias,
        "Macro" => SymbolKind::Macro,
        "Class" => SymbolKind::Class,
        "Interface" => SymbolKind::Interface,
        "Type" => SymbolKind::Type,
        "Component" => SymbolKind::Component,
        "Variable" => SymbolKind::Variable,
        other => SymbolKind::Other(other.to_string()),
    }
}

fn parse_visibility(s: &str) -> Visibility {
    match s {
        "Public" => Visibility::Public,
        "PubCrate" => Visibility::PubCrate,
        "PubSuper" => Visibility::PubSuper,
        "Private" => Visibility::Private,
        _ => Visibility::Unknown,
    }
}

fn parse_provenance(s: &str) -> Provenance {
    match s {
        "Observed" => Provenance::Observed,
        "Parsed" => Provenance::Parsed,
        "Derived" => Provenance::Derived,
        "Inferred" => Provenance::Inferred,
        _ => Provenance::Observed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_test_store() -> (TempDir, RepoStore) {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let store = RepoStore::new(&db_path, dir.path()).unwrap();
        (dir, store)
    }

    #[test]
    fn store_and_retrieve_files() {
        let (_dir, store) = make_test_store();

        let files = vec![FileNode {
            id: "src/main.rs".to_string(),
            path: PathBuf::from("src/main.rs"),
            language: Language::Rust,
            role: crate::intelligence::types::FileRole::EntryPoint,
            size: 100,
            content_hash: "abc123".to_string(),
            package: Some("test".to_string()),
            symbols: Vec::new(),
            imports: Vec::new(),
            indexed_at: Utc::now(),
            support_level: LanguageSupport::Parsed,
        }];

        store.store_files(&files).unwrap();
        let retrieved = store.get_files().unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].id, "src/main.rs");
    }

    #[test]
    fn store_and_retrieve_symbols() {
        let (_dir, store) = make_test_store();

        let symbols = vec![Symbol {
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
        }];

        store.store_symbols(&symbols).unwrap();
        let retrieved = store.get_symbols().unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].name, "AgentLoop");
    }

    #[test]
    fn clear_and_rebuild() {
        let (_dir, store) = make_test_store();

        store
            .store_files(&[FileNode {
                id: "old.rs".to_string(),
                path: PathBuf::from("old.rs"),
                language: Language::Rust,
                role: crate::intelligence::types::FileRole::Source,
                size: 10,
                content_hash: "old".to_string(),
                package: None,
                symbols: Vec::new(),
                imports: Vec::new(),
                indexed_at: Utc::now(),
                support_level: LanguageSupport::Parsed,
            }])
            .unwrap();

        assert_eq!(store.get_files().unwrap().len(), 1);

        store.clear().unwrap();
        assert_eq!(store.get_files().unwrap().len(), 0);
    }

    #[test]
    fn metadata_roundtrip() {
        let (_dir, store) = make_test_store();
        store.set_metadata("repo_id", "test-repo").unwrap();
        assert_eq!(
            store.get_metadata("repo_id").unwrap(),
            Some("test-repo".to_string())
        );
    }
}
