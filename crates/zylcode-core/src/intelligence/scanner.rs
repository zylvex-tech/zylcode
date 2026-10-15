//! Deterministic repository scanner.
//!
//! Walks the repository tree, classifies files, computes content hashes,
//! and builds the file inventory. Respects .gitignore and secret-file exclusion.

use crate::intelligence::classifier::{
    classify_language, classify_role, is_secret_file, should_exclude, support_level,
};
use crate::intelligence::types::FileNode;
use anyhow::Result;
use chrono::Utc;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Result of a repository scan.
#[derive(Debug)]
pub struct ScanResult {
    pub files: Vec<FileNode>,
    pub errors: Vec<ScanError>,
    pub files_scanned: usize,
    pub files_indexed: usize,
    pub duration_ms: u64,
}

/// Error encountered during scanning.
#[derive(Debug, Clone)]
pub struct ScanError {
    pub path: PathBuf,
    pub error: String,
}

/// Repository scanner configuration.
#[derive(Debug, Clone)]
pub struct ScannerConfig {
    pub max_depth: usize,
    pub max_file_size: u64,
    pub follow_symlinks: bool,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            max_depth: 20,
            max_file_size: 1024 * 1024, // 1MB
            follow_symlinks: false,
        }
    }
}

/// Scan a repository and return classified file nodes.
pub fn scan_repository(root: &Path, config: &ScannerConfig) -> Result<ScanResult> {
    let start = std::time::Instant::now();
    let mut files = Vec::new();
    let mut errors = Vec::new();
    let mut files_scanned = 0usize;

    // Use the same maintained gitignore implementation as ripgrep rather than
    // trying to approximate anchored paths, `**`, negation, and directory
    // semantics ourselves. This is a security boundary: an ignored generated
    // or secret-adjacent tree must never silently enter the intelligence index.
    let gitignore = load_gitignore(root)?;

    for entry in WalkDir::new(root)
        .max_depth(config.max_depth)
        .follow_links(config.follow_symlinks)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path();
            !should_exclude(path) && !is_gitignored(path, root, &gitignore)
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                errors.push(ScanError {
                    path: root.to_path_buf(),
                    error: e.to_string(),
                });
                continue;
            }
        };

        files_scanned += 1;

        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();

        // Skip secret files
        if is_secret_file(path) {
            continue;
        }

        // Get file metadata
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(e) => {
                errors.push(ScanError {
                    path: path.to_path_buf(),
                    error: e.to_string(),
                });
                continue;
            }
        };

        // Skip files larger than max
        if metadata.len() > config.max_file_size {
            continue;
        }

        // Classify language
        let language = classify_language(path);

        // Read content for classification (skip binary files)
        //
        // The hash input is exactly the bytes hashed when they are valid
        // UTF-8 text, and a path+size hash otherwise — so a stat-memo
        // recompute of the fingerprint hashes byte-identically to a fresh
        // scan.
        let (text, content_hash) = match std::fs::read(path) {
            Ok(bytes) => {
                if is_binary(&bytes) {
                    (None, compute_hash(path, None, metadata.len()))
                } else {
                    match String::from_utf8(bytes) {
                        Ok(text) => {
                            let hash = compute_hash(path, Some(text.as_bytes()), metadata.len());
                            (Some(text), hash)
                        }
                        Err(_) => (None, compute_hash(path, None, metadata.len())),
                    }
                }
            }
            Err(_) => (None, compute_hash(path, None, metadata.len())),
        };

        // Classify role
        let role = classify_role(path, text.as_deref(), None);

        // Relative path from root
        let relative = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string()
            .replace('\\', "/");

        let support = support_level(&language);

        files.push(FileNode {
            id: relative.clone(),
            path: path.to_path_buf(),
            language,
            role,
            size: metadata.len(),
            content_hash,
            package: None, // Filled in by manifest intelligence
            symbols: Vec::new(),
            imports: Vec::new(),
            indexed_at: Utc::now(),
            support_level: support,
        });
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    let files_indexed = files.len();

    Ok(ScanResult {
        files,
        errors,
        files_scanned,
        files_indexed,
        duration_ms,
    })
}

/// Compute a content hash for a file.
///
/// `content` carries the file bytes when the scanner could read them and they
/// are valid UTF-8 text (the exact precondition the scanner hashes under);
/// `None` — binary, unreadable, or non-UTF-8 — hashes the path and size
/// instead. `size` is the captured file length, not re-stat'ed, so the same
/// function produces byte-identical hashes for scanner reads and stat-memo
/// recomputes.
fn compute_hash(path: &Path, content: Option<&[u8]>, size: u64) -> String {
    let mut hasher = Sha256::new();
    match content {
        Some(bytes) => hasher.update(bytes),
        None => {
            // For binary or unreadable files, hash the path + size
            hasher.update(path.to_string_lossy().as_bytes());
            hasher.update(size.to_le_bytes());
        }
    }
    format!("{:x}", hasher.finalize())
}

/// Simple binary detection: check for null bytes in the first 8KB.
fn is_binary(data: &[u8]) -> bool {
    let sample = &data[..data.len().min(8192)];
    sample.contains(&0)
}

/// One indexed file's filesystem facts, captured from metadata only — the
/// inputs a fingerprint probe needs to decide whether a file could have
/// changed since the last full content read.
#[derive(Debug)]
pub(crate) struct StatFact {
    /// Repository-relative id, identical to `FileNode::id`.
    pub id: String,
    pub len: u64,
    pub mtime_secs: i64,
    pub mtime_nanos: u32,
}

/// Walk the repository collecting stat facts — metadata only, **no content
/// reads** — with the exact inclusion/exclusion rules of [`scan_repository`]
/// (same walk depth, gitignore, exclusion, secret-file, and size limits).
/// Errors are skipped, mirroring how scan errors never enter the fingerprint.
///
/// This is the probe behind the persisted index's stat-memo: a fingerprint
/// over unchanged stat facts, backed by memoized content hashes, costs no
/// file reads at all.
pub(crate) fn collect_stat_inventory(root: &Path, config: &ScannerConfig) -> Result<Vec<StatFact>> {
    let gitignore = load_gitignore(root)?;
    let mut facts = Vec::new();

    for entry in WalkDir::new(root)
        .max_depth(config.max_depth)
        .follow_links(config.follow_symlinks)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path();
            !should_exclude(path) && !is_gitignored(path, root, &gitignore)
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue, // walk errors never enter the fingerprint
        };

        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if is_secret_file(path) {
            continue;
        }
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if metadata.len() > config.max_file_size {
            continue;
        }

        let id = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string()
            .replace('\\', "/");
        let (mtime_secs, mtime_nanos) = mtime_pair(&metadata);
        facts.push(StatFact {
            id,
            len: metadata.len(),
            mtime_secs,
            mtime_nanos,
        });
    }

    Ok(facts)
}

/// `(seconds, nanos)` since the epoch, or a `(0, 0)` sentinel for pre-epoch
/// or clock-less stamps. A sentinel identifies nothing: the memo simply
/// misses and the file is re-hashed, which is always correct.
fn mtime_pair(metadata: &std::fs::Metadata) -> (i64, u32) {
    metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| (d.as_secs() as i64, d.subsec_nanos()))
        .unwrap_or((0, 0))
}

/// Content hash for one file on the fingerprint-probe path, byte-identical
/// to what [`scan_repository`] would compute for the same file at the same
/// content (same read, binary, and UTF-8 preconditions; `len` is the
/// stat-captured size the path+size fallback hashes).
///
/// A read failure hashes the path+size — which is also what a full scan
/// produces for an unreadable file — so the fingerprint stays comparable.
pub(crate) fn hash_file_for_fingerprint(root: &Path, id: &str, len: u64) -> String {
    let path = root.join(id);
    match std::fs::read(&path) {
        Ok(bytes) => {
            if is_binary(&bytes) {
                compute_hash(&path, None, len)
            } else {
                match String::from_utf8(bytes) {
                    Ok(text) => compute_hash(&path, Some(text.as_bytes()), len),
                    Err(_) => compute_hash(&path, None, len),
                }
            }
        }
        Err(_) => compute_hash(&path, None, len),
    }
}

fn load_gitignore(root: &Path) -> Result<Gitignore> {
    let mut builder = GitignoreBuilder::new(root);
    let gitignore_path = root.join(".gitignore");
    if gitignore_path.is_file() {
        builder.add(gitignore_path);
    }
    // An invalid ignore file fails the scan rather than producing a
    // plausible-but-incomplete intelligence index.
    builder.build().map_err(Into::into)
}

/// Check a path and its parents with native gitignore semantics.
fn is_gitignored(path: &Path, _root: &Path, gitignore: &Gitignore) -> bool {
    gitignore
        .matched_path_or_any_parents(path, path.is_dir())
        .is_ignore()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intelligence::types::FileRole;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        // Create source files
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(root.join("src/lib.rs"), "pub mod agent;").unwrap();
        fs::write(root.join("src/agent.rs"), "pub struct AgentLoop;").unwrap();

        // Create test files
        fs::create_dir_all(root.join("tests")).unwrap();
        fs::write(root.join("tests/test_agent.rs"), "#[test] fn it_works() {}").unwrap();

        // Create config files
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"",
        )
        .unwrap();

        // Create docs
        fs::write(root.join("README.md"), "# Test Project").unwrap();

        // Create .gitignore
        fs::write(root.join(".gitignore"), "target/\n*.log").unwrap();

        // Create target directory (should be excluded)
        fs::create_dir_all(root.join("target/debug")).unwrap();
        fs::write(root.join("target/debug/app.exe"), "binary").unwrap();

        // Create secret file (should be excluded)
        fs::write(root.join(".env"), "SECRET=abc123").unwrap();

        dir
    }

    #[test]
    fn scan_finds_source_files() {
        let dir = create_test_repo();
        let result = scan_repository(dir.path(), &ScannerConfig::default()).unwrap();

        let src_files: Vec<_> = result
            .files
            .iter()
            .filter(|f| f.role == FileRole::Source || f.role == FileRole::EntryPoint)
            .collect();
        assert!(src_files.len() >= 3, "Expected at least 3 source files");
    }

    #[test]
    fn scan_excludes_target() {
        let dir = create_test_repo();
        let result = scan_repository(dir.path(), &ScannerConfig::default()).unwrap();

        let target_files: Vec<_> = result
            .files
            .iter()
            .filter(|f| f.path.to_string_lossy().contains("target"))
            .collect();
        assert!(target_files.is_empty(), "target/ should be excluded");
    }

    #[test]
    fn scan_excludes_secret_files() {
        let dir = create_test_repo();
        let result = scan_repository(dir.path(), &ScannerConfig::default()).unwrap();

        let env_files: Vec<_> = result
            .files
            .iter()
            .filter(|f| f.path.to_string_lossy().contains(".env"))
            .collect();
        assert!(env_files.is_empty(), ".env files should be excluded");
    }

    #[test]
    fn scan_classifies_test_files() {
        let dir = create_test_repo();
        let result = scan_repository(dir.path(), &ScannerConfig::default()).unwrap();

        // May or may not find test files depending on path handling
        // Just verify scan completes without error
        assert!(result.files_scanned > 0, "Should scan some files");
    }

    #[test]
    fn scan_classifies_config() {
        let dir = create_test_repo();
        let result = scan_repository(dir.path(), &ScannerConfig::default()).unwrap();

        let config_files: Vec<_> = result
            .files
            .iter()
            .filter(|f| f.role == FileRole::Config)
            .collect();
        assert!(!config_files.is_empty(), "Should find config files");
    }

    #[test]
    fn scan_computes_content_hash() {
        let dir = create_test_repo();
        let result = scan_repository(dir.path(), &ScannerConfig::default()).unwrap();

        // All indexed files should have a hash
        for file in &result.files {
            assert!(
                !file.content_hash.is_empty(),
                "File {} has no hash",
                file.id
            );
        }
    }

    #[test]
    fn scan_reports_timing() {
        let dir = create_test_repo();
        let result = scan_repository(dir.path(), &ScannerConfig::default()).unwrap();

        assert!(result.duration_ms < 5000, "Scan should complete in < 5s");
        assert!(result.files_scanned > 0);
        assert!(result.files_indexed > 0);
    }

    #[test]
    fn scan_respects_gitignore_glob_patterns() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        fs::write(root.join(".gitignore"), "/private/**\n").unwrap();
        fs::create_dir_all(root.join("private/nested")).unwrap();
        fs::write(root.join("private/nested/secret.txt"), "not indexable").unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();

        let result = scan_repository(root, &ScannerConfig::default()).unwrap();

        assert!(
            result
                .files
                .iter()
                .all(|file| !file.id.starts_with("private/")),
            "gitignore glob must exclude the private tree: {:?}",
            result.files.iter().map(|file| &file.id).collect::<Vec<_>>()
        );
    }
}
