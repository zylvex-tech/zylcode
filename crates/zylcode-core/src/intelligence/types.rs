//! Canonical Repository Intelligence types.
//!
//! Every object has a stable identifier and provenance classification.
//! Relationships are modeled as graph edges, not string references.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Provenance classification
// ---------------------------------------------------------------------------

/// How a repository fact was established.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provenance {
    /// Directly observed from the filesystem (file exists, size, mtime).
    Observed,
    /// Extracted by parsing structured content (manifest, AST).
    Parsed,
    /// Computed from other facts (dependency transitive closure).
    Derived,
    /// Inferred heuristically (framework detection from keywords).
    Inferred,
}

/// Language support levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanguageSupport {
    /// Extension detected but no semantic indexing.
    Detected,
    /// File parsed for symbols/structure.
    Parsed,
    /// Full semantic indexing with references resolved.
    SemanticallyIndexed,
}

// ---------------------------------------------------------------------------
// Stable identifiers
// ---------------------------------------------------------------------------

/// Unique identifier for a repository (based on canonical path + git identity).
pub type RepoId = String;

/// Unique identifier for a workspace member.
pub type WorkspaceId = String;

/// Unique identifier for a package/crate.
pub type PackageId = String;

/// Unique identifier for a file (relative path from repo root).
pub type FileId = String;

/// Unique identifier for a symbol (qualified path, e.g. `crate::agent::AgentLoop`).
pub type SymbolId = String;

/// Unique identifier for a git commit.
pub type CommitId = String;

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// The top-level repository model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: RepoId,
    pub root: PathBuf,
    pub git_identity: Option<GitIdentity>,
    pub workspaces: Vec<WorkspaceId>,
    pub packages: Vec<PackageId>,
    pub entry_points: Vec<EntryPoint>,
    pub architecture: ArchitecturalFingerprint,
    pub indexed_at: DateTime<Utc>,
    pub total_files: usize,
    pub total_symbols: usize,
}

/// Git repository identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitIdentity {
    pub head: CommitId,
    pub branch: Option<String>,
    pub dirty: bool,
    pub remote_url: Option<String>,
}

/// A workspace (e.g. Rust workspace, pnpm workspace).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub root: PathBuf,
    pub workspace_type: WorkspaceType,
    pub members: Vec<PackageId>,
    pub manifest_path: FileId,
}

/// Type of workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceType {
    Cargo,
    Pnpm,
    Npm,
    Yarn,
}

/// A package/crate within a workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub id: PackageId,
    pub name: String,
    pub version: String,
    pub root: PathBuf,
    pub language: Language,
    pub manifest: FileId,
    pub files: Vec<FileId>,
    pub dependencies: Vec<Dependency>,
    pub dev_dependencies: Vec<Dependency>,
    pub build_commands: Vec<String>,
    pub test_commands: Vec<String>,
    pub entry_points: Vec<EntryPoint>,
}

/// A file in the repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    pub id: FileId,
    pub path: PathBuf,
    pub language: Language,
    pub role: FileRole,
    pub size: u64,
    pub content_hash: String,
    pub package: Option<PackageId>,
    pub symbols: Vec<SymbolId>,
    pub imports: Vec<SymbolId>,
    pub indexed_at: DateTime<Utc>,
    pub support_level: LanguageSupport,
}

/// Language classification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Json,
    Toml,
    Yaml,
    Markdown,
    Html,
    Css,
    Shell,
    Unknown(String),
}

/// Role of a file in the repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileRole {
    Source,
    Test,
    Config,
    Documentation,
    BuildScript,
    Generated,
    Vendor,
    Dependency,
    EntryPoint,
    LockFile,
    Script,
    Ci,
    Asset,
    Other(String),
}

/// A symbol extracted from source code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
    pub file: FileId,
    pub line: u32,
    pub end_line: Option<u32>,
    pub visibility: Visibility,
    pub parent: Option<SymbolId>,
    pub language: Language,
    pub support_level: LanguageSupport,
}

/// Kind of symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    // Rust
    Module,
    Struct,
    Enum,
    Trait,
    Impl,
    Function,
    Method,
    Constant,
    TypeAlias,
    Macro,
    // TypeScript/JavaScript
    Class,
    Interface,
    Type,
    Component,
    Variable,
    // Generic
    Other(String),
}

/// Visibility of a symbol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    PubCrate,
    PubSuper,
    Private,
    Unknown,
}

/// A dependency edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub from: PackageId,
    pub to: DependencyTarget,
    pub kind: DependencyKind,
    pub version_constraint: String,
    pub features: Vec<String>,
    pub optional: bool,
}

/// Target of a dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyTarget {
    /// Internal package in the same repository.
    Internal(PackageId),
    /// External package from a registry.
    External { name: String, registry: String },
}

/// Kind of dependency relationship.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyKind {
    Normal,
    Dev,
    Build,
}

/// An entry point (binary, library, test, build script, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPoint {
    pub path: PathBuf,
    pub kind: EntryPointKind,
    pub package: Option<PackageId>,
    pub evidence: Vec<String>,
}

/// Kind of entry point.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryPointKind {
    Main,
    Lib,
    Binary(String),
    Test,
    BuildScript,
    TauriApp,
    ReactBootstrap,
    CliEntry,
    WorkspaceScript(String),
}

/// Architectural fingerprint of the repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitecturalFingerprint {
    pub facts: Vec<ArchitecturalFact>,
}

/// A single architectural fact with evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitecturalFact {
    pub fact: String,
    pub value: String,
    pub evidence: Vec<String>,
    pub provenance: Provenance,
}

/// A git commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommit {
    pub id: CommitId,
    pub short_id: String,
    pub message: String,
    pub author: String,
    pub timestamp: DateTime<Utc>,
    pub files_changed: Vec<FileId>,
}

/// A change affecting a file or symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitChange {
    pub commit: CommitId,
    pub file: FileId,
    pub change_type: ChangeType,
    pub lines_added: u32,
    pub lines_removed: u32,
}

/// Type of file change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
    Renamed { from: FileId },
}

/// A repository fact with provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryFact {
    pub id: String,
    pub fact_type: String,
    pub subject: String,
    pub value: String,
    pub source: String,
    pub source_hash: String,
    pub timestamp: DateTime<Utc>,
    pub provenance: Provenance,
    pub confidence: f64,
}

/// Query result with relevance and provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextResult {
    pub resource: String,
    pub resource_type: String,
    pub relevance: f64,
    pub reason: String,
    pub provenance: Provenance,
    pub snippet: Option<String>,
}

/// Token budget priority levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextPriority {
    High,
    Medium,
    Low,
}

/// Repository index metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMetadata {
    pub repo_id: RepoId,
    pub repo_root: PathBuf,
    pub last_full_scan: DateTime<Utc>,
    pub last_incremental: DateTime<Utc>,
    pub file_count: usize,
    pub symbol_count: usize,
    pub dependency_count: usize,
    pub version: u32,
}
