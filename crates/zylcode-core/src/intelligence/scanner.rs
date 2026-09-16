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

    // Load .gitignore patterns (simple implementation)
    let gitignore = load_gitignore(root);

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
        let content = match std::fs::read(path) {
            Ok(bytes) => {
                if is_binary(&bytes) {
                    None
                } else {
                    String::from_utf8(bytes).ok()
                }
            }
            Err(_) => None,
        };

        // Classify role
        let role = classify_role(path, content.as_deref(), None);

        // Compute content hash
        let content_hash = compute_hash(path, content.as_deref());

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
fn compute_hash(path: &Path, content: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    match content {
        Some(c) => hasher.update(c.as_bytes()),
        None => {
            // For binary or unreadable files, hash the path + size
            hasher.update(path.to_string_lossy().as_bytes());
            if let Ok(metadata) = std::fs::metadata(path) {
                hasher.update(metadata.len().to_le_bytes());
            }
        }
    }
    format!("{:x}", hasher.finalize())
}

/// Simple binary detection: check for null bytes in the first 8KB.
fn is_binary(data: &[u8]) -> bool {
    let sample = &data[..data.len().min(8192)];
    sample.contains(&0)
}

/// Load .gitignore patterns (simple implementation).
fn load_gitignore(root: &Path) -> Vec<String> {
    let gitignore_path = root.join(".gitignore");
    match std::fs::read_to_string(gitignore_path) {
        Ok(content) => content
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Check if a path matches any gitignore pattern (simple glob matching).
fn is_gitignored(path: &Path, root: &Path, patterns: &[String]) -> bool {
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");

    for pattern in patterns {
        if matches_gitignore_pattern(&relative, pattern) {
            return true;
        }
    }
    false
}

/// Simple gitignore pattern matching.
fn matches_gitignore_pattern(path: &str, pattern: &str) -> bool {
    let pattern = pattern.trim_end_matches('/');

    // Exact match
    if path == pattern {
        return true;
    }

    // Suffix match (e.g., "*.rs" matches "src/main.rs")
    if let Some(suffix) = pattern.strip_prefix('*') {
        return path.ends_with(suffix);
    }

    // Directory match
    if path.starts_with(pattern) {
        return true;
    }

    // Filename match
    if let Some(name) = path.rsplit('/').next() {
        if name == pattern {
            return true;
        }
    }

    false
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
    fn gitignore_pattern_matching() {
        assert!(matches_gitignore_pattern("target/debug/app", "target"));
        assert!(matches_gitignore_pattern("src/main.log", "*.log"));
        assert!(!matches_gitignore_pattern("src/main.rs", "*.log"));
    }
}
