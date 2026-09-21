use crate::project::types::*;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const MAX_TREE_DEPTH: usize = 10;
const MAX_DIR_ENTRIES: usize = 1000;
const MAX_FILE_SIZE_FOR_EDITOR: u64 = 10 * 1024 * 1024;
const MAX_FILE_SIZE_FOR_METADATA: u64 = 1024 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FileClassification {
    TEXT,
    BINARY,
    TOO_LARGE,
    UNSUPPORTED_ENCODING,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedFileContent {
    pub path: String,
    pub absolute_path: String,
    pub content: Option<String>,
    pub content_hash: String,
    pub size: u64,
    pub modified: String,
    pub language: Option<String>,
    pub classification: FileClassification,
    pub line_count: Option<usize>,
}

pub struct ProjectFilesystem {
    project_root: PathBuf,
}
impl ProjectFilesystem {
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
        }
    }

    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub fn build_file_tree(&self, max_depth: Option<usize>) -> Result<FileTreeNode> {
        let max_depth = max_depth.unwrap_or(MAX_TREE_DEPTH);
        self.build_tree_recursive(&self.project_root, "", 0, max_depth)
    }

    fn build_tree_recursive(
        &self,
        dir: &Path,
        relative_path: &str,
        current_depth: usize,
        max_depth: usize,
    ) -> Result<FileTreeNode> {
        if current_depth >= max_depth {
            return Ok(FileTreeNode {
                name: dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("...")
                    .to_string(),
                path: relative_path.to_string(),
                absolute_path: dir.to_string_lossy().to_string(),
                is_directory: true,
                is_file: false,
                extension: None,
                size: None,
                modified: None,
                children: vec![],
                is_expanded: current_depth == 0,
                is_selected: false,
                is_active: false,
            });
        }

        let mut entries: Vec<_> = fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();

        entries.sort_by(|a, b| {
            let a_is_dir = a.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            let b_is_dir = b.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.file_name().cmp(&b.file_name()),
            }
        });

        if entries.len() > MAX_DIR_ENTRIES {
            entries.truncate(MAX_DIR_ENTRIES);
        }

        let mut children = Vec::new();
        for entry in entries {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            let is_dir = metadata.is_dir();
            let rel_path = if relative_path.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", relative_path, name)
            };

            if is_dir {
                if should_skip_directory(&name) {
                    continue;
                }
                match self.build_tree_recursive(&path, &rel_path, current_depth + 1, MAX_TREE_DEPTH)
                {
                    Ok(node) => children.push(node),
                    Err(_) => continue,
                }
            } else if metadata.len() <= MAX_FILE_SIZE_FOR_METADATA {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_lowercase());
                children.push(FileTreeNode {
                    name,
                    path: rel_path,
                    absolute_path: path.to_string_lossy().to_string(),
                    is_directory: false,
                    is_file: true,
                    extension: ext,
                    size: Some(metadata.len()),
                    modified: metadata
                        .modified()
                        .ok()
                        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()),
                    children: vec![],
                    is_expanded: false,
                    is_selected: false,
                    is_active: false,
                });
            }
        }

        let name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("root")
            .to_string();

        Ok(FileTreeNode {
            name,
            path: relative_path.to_string(),
            absolute_path: dir.to_string_lossy().to_string(),
            is_directory: true,
            is_file: false,
            extension: None,
            size: None,
            modified: None,
            children,
            is_expanded: current_depth == 0,
            is_selected: false,
            is_active: false,
        })
    }

    pub fn classify_file(&self, relative_path: &str) -> Result<FileClassification> {
        let path = self.project_root.join(relative_path);

        if !path.exists() {
            return Err(anyhow::anyhow!("File not found: {}", relative_path));
        }

        if !path.is_file() {
            return Err(anyhow::anyhow!("Path is not a file: {}", relative_path));
        }

        let metadata = fs::metadata(&path)?;

        if metadata.len() > 1024 * 1024 * 1024 {
            return Ok(FileClassification::TOO_LARGE);
        }

        if metadata.len() > MAX_FILE_SIZE_FOR_EDITOR {
            return Ok(FileClassification::TOO_LARGE);
        }

        let content = fs::read(&path)?;

        if content.contains(&0) {
            return Ok(FileClassification::BINARY);
        }

        match std::str::from_utf8(&content) {
            Ok(_) => Ok(FileClassification::TEXT),
            Err(_) => Ok(FileClassification::UNSUPPORTED_ENCODING),
        }
    }

    pub fn read_file_classified(&self, relative_path: &str) -> Result<ClassifiedFileContent> {
        let path = self.project_root.join(relative_path);

        if !path.exists() {
            return Err(anyhow::anyhow!("File not found: {}", relative_path));
        }

        if !path.is_file() {
            return Err(anyhow::anyhow!("Path is not a file: {}", relative_path));
        }

        let metadata = fs::metadata(&path)?;

        if metadata.len() > 1024 * 1024 * 1024 {
            return Err(anyhow::anyhow!(
                "File too large for any operation: {} bytes",
                metadata.len()
            ));
        }

        if metadata.len() > MAX_FILE_SIZE_FOR_EDITOR {
            let mut hasher = Sha256::new();
            let mut file = fs::File::open(&path)?;
            std::io::copy(&mut file, &mut hasher)?;
            let hash = format!("{:x}", hasher.finalize());

            return Ok(ClassifiedFileContent {
                path: relative_path.to_string(),
                absolute_path: path.to_string_lossy().to_string(),
                content: None,
                content_hash: hash,
                size: metadata.len(),
                modified: metadata
                    .modified()
                    .ok()
                    .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
                    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
                language: None,
                classification: FileClassification::TOO_LARGE,
                line_count: None,
            });
        }

        let content_bytes = fs::read(&path)?;
        let mut hasher = Sha256::new();
        hasher.update(&content_bytes);
        let content_hash = format!("{:x}", hasher.finalize());

        let is_binary = content_bytes.contains(&0);

        let (content, language, classification, line_count) = if is_binary {
            (None, None, FileClassification::BINARY, None)
        } else {
            match std::str::from_utf8(&content_bytes) {
                Ok(text) => {
                    let ext = std::path::Path::new(relative_path)
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|s| s.to_lowercase());
                    let language = ext.as_ref().map(|e| detect_language(e));
                    let line_count = text.lines().count();
                    (
                        Some(text.to_string()),
                        language,
                        FileClassification::TEXT,
                        Some(text.lines().count()),
                    )
                }
                Err(_) => (None, None, FileClassification::UNSUPPORTED_ENCODING, None),
            }
        };

        let mut hasher = Sha256::new();
        hasher.update(&content_bytes);
        let content_hash = format!("{:x}", hasher.finalize());

        let metadata = fs::metadata(&self.project_root.join(relative_path))?;

        Ok(ClassifiedFileContent {
            path: relative_path.to_string(),
            absolute_path: self
                .project_root
                .join(relative_path)
                .to_string_lossy()
                .to_string(),
            content,
            content_hash,
            size: metadata.len(),
            modified: metadata
                .modified()
                .ok()
                .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
            language,
            classification,
            line_count,
        })
    }

    pub fn read_file(&self, relative_path: &str) -> Result<FileContent> {
        let classified = self.read_file_classified(relative_path)?;

        match classified.classification {
            FileClassification::TEXT => Ok(FileContent {
                path: classified.path,
                absolute_path: classified.absolute_path,
                content: classified.content.unwrap_or_default(),
                content_hash: classified.content_hash,
                encoding: "utf-8".to_string(),
                size: classified.size,
                modified: classified.modified,
                language: classified.language,
                line_count: classified.line_count.unwrap_or(0),
            }),
            FileClassification::BINARY => {
                Err(anyhow::anyhow!("Binary file cannot be opened as text"))
            }
            FileClassification::TOO_LARGE => Err(anyhow::anyhow!("File too large for editor")),
            FileClassification::UNSUPPORTED_ENCODING => {
                Err(anyhow::anyhow!("Unsupported encoding"))
            }
        }
    }

    pub fn compute_content_hash(&self, relative_path: &str) -> Result<String> {
        let path = self.project_root.join(relative_path);
        let mut hasher = Sha256::new();
        let mut file = fs::File::open(&path)?;
        std::io::copy(&mut file, &mut hasher)?;
        Ok(format!("{:x}", hasher.finalize()))
    }

    pub fn write_file(&self, relative_path: &str, content: &str) -> Result<()> {
        let path = self.project_root.join(relative_path);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let temp_path = self
            .project_root
            .join(format!(".tmp.{}.tmp", uuid::Uuid::new_v4()));

        fs::write(&temp_path, content)
            .with_context(|| format!("Failed to write temp file: {}", relative_path))?;

        fs::rename(&temp_path, &path).with_context(|| {
            format!("Failed to move temp file to destination: {}", relative_path)
        })?;

        Ok(())
    }

    pub fn write_file_atomic(
        &self,
        relative_path: &str,
        content: &str,
        expected_hash: Option<&str>,
    ) -> Result<String> {
        let path = self.project_root.join(relative_path);

        if path.exists() {
            let current_hash = self.compute_content_hash(relative_path)?;
            if let Some(expected) = expected_hash {
                if current_hash != expected {
                    return Err(anyhow::anyhow!("FILE_CHANGED_ON_DISK"));
                }
            }
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let temp_path = self
            .project_root
            .join(format!(".tmp.{}.tmp", uuid::Uuid::new_v4()));

        fs::write(&temp_path, content)
            .with_context(|| format!("Failed to write temp file: {}", relative_path))?;

        fs::rename(&temp_path, &self.project_root.join(relative_path)).with_context(|| {
            format!("Failed to move temp file to destination: {}", relative_path)
        })?;

        let mut hasher = Sha256::new();
        let mut file = fs::File::open(&self.project_root.join(relative_path))?;
        std::io::copy(&mut file, &mut hasher)?;
        Ok(format!("{:x}", hasher.finalize()))
    }

    pub fn create_file(&self, relative_path: &str, content: &str) -> Result<String> {
        let requested = self.project_root.join(relative_path);

        let canonical_project = self
            .project_root
            .canonicalize()
            .context("Failed to canonicalize project root")?;

        let parent = requested
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid path: no parent directory"))?;

        let canonical_parent = if parent.exists() {
            parent
                .canonicalize()
                .context("Failed to canonicalize parent directory")?
        } else {
            let mut current = parent.clone();
            while !current.exists() {
                current = current
                    .parent()
                    .ok_or_else(|| anyhow::anyhow!("No existing parent directory"))?;
            }
            current
                .canonicalize()
                .context("Failed to canonicalize nearest existing parent")?
        };

        if !canonical_parent.starts_with(&self.project_root.canonicalize()?) {
            return Err(anyhow::anyhow!("PATH_OUTSIDE_PROJECT"));
        }

        let remaining = relative_path
            .trim_start_matches(&format!(
                "{}/",
                canonical_parent
                    .strip_prefix(&canonical_project)?
                    .to_string_lossy()
            ))
            .trim_start_matches('/');

        let mut components = remaining.split('/').peekable();
        let mut current_path = canonical_parent;

        while let Some(component) = components.next() {
            if component.is_empty() || component == "." {
                continue;
            }
            if component == ".." {
                return Err(anyhow::anyhow!("PATH_OUTSIDE_PROJECT"));
            }

            current_path = current_path.join(component);

            if components.peek().is_none() {
                if current_path.exists() {
                    return Err(anyhow::anyhow!("File already exists"));
                }

                if let Some(parent) = current_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                let temp_path = self
                    .project_root
                    .join(format!(".tmp.{}.tmp", uuid::Uuid::new_v4()));
                fs::write(&temp_path, content)?;
                fs::rename(&temp_path, &current_path)?;
            } else {
                if !current_path.exists() {
                    fs::create_dir_all(&current_path)?;
                } else if !current_path.is_dir() {
                    return Err(anyhow::anyhow!("Path component is not a directory"));
                }
            }
        }

        Ok(self.compute_content_hash(relative_path)?)
    }

    pub fn file_exists(&self, relative_path: &str) -> bool {
        self.project_root.join(relative_path).exists()
    }

    pub fn get_metadata(&self, relative_path: &str) -> Result<fs::Metadata> {
        let path = self.project_root.join(relative_path);
        fs::metadata(&path).with_context(|| format!("Failed to get metadata: {}", relative_path))
    }

    pub fn list_directory(&self, relative_path: &str) -> Result<Vec<FileTreeNode>> {
        let path = self.project_root.join(relative_path);
        let mut nodes = Vec::new();

        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            let path = entry.path();
            let metadata = entry.metadata()?;

            let is_dir = metadata.is_dir();
            let rel_path = if relative_path.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", relative_path.trim_end_matches('/'), name)
            };

            if is_dir {
                nodes.push(FileTreeNode {
                    name,
                    path: rel_path,
                    absolute_path: path.to_string_lossy().to_string(),
                    is_directory: true,
                    is_file: false,
                    extension: None,
                    size: None,
                    modified: metadata
                        .modified()
                        .ok()
                        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()),
                    children: vec![],
                    is_expanded: false,
                    is_selected: false,
                    is_active: false,
                });
            } else {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_lowercase());
                nodes.push(FileTreeNode {
                    name,
                    path: rel_path,
                    absolute_path: path.to_string_lossy().to_string(),
                    is_directory: false,
                    is_file: true,
                    extension: ext,
                    size: Some(metadata.len()),
                    modified: metadata
                        .modified()
                        .ok()
                        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()),
                    children: vec![],
                    is_expanded: false,
                    is_selected: false,
                    is_active: false,
                });
            }
        }

        nodes.sort_by(|a, b| match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });

        Ok(nodes)
    }

    pub fn get_git_status(&self) -> Result<Vec<FileGitStatus>> {
        let output = std::process::Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.project_root)
            .output()
            .context("Failed to run git status")?;

        if !output.status.success() {
            return Ok(vec![]);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut statuses = Vec::new();

        for line in stdout.lines() {
            if line.len() < 3 {
                continue;
            }
            let status_code = &line[..2];
            let path = &line[3..].trim();

            let status = match &status_code[0..1] {
                "M" => GitFileStatus::Modified,
                "A" => GitFileStatus::Added,
                "D" => GitFileStatus::Deleted,
                "R" => GitFileStatus::Renamed,
                "?" => GitFileStatus::Untracked,
                "!" => GitFileStatus::Ignored,
                "U" => GitFileStatus::Conflict,
                _ => GitFileStatus::Unmodified,
            };

            let staged = &status_code[0..1] != " " && &status_code[0..1] != "?";

            statuses.push(FileGitStatus {
                path: path.to_string(),
                status,
                staged,
            });
        }

        Ok(statuses)
    }

    pub fn canonicalize_and_validate(&self, relative_path: &str) -> Result<PathBuf> {
        let requested = self.project_root.join(relative_path);

        let canonical_project = self
            .project_root
            .canonicalize()
            .context("Failed to canonicalize project root")?;

        let canonical_requested = if relative_path.is_empty() || relative_path == "." {
            canonical_project.clone()
        } else {
            let requested_path = self.project_root.join(relative_path);
            if requested_path.exists() {
                requested_path
                    .canonicalize()
                    .context("Failed to canonicalize requested path")?
            } else {
                let mut current = self.project_root.join(relative_path);
                while !current.exists() {
                    current = current
                        .parent()
                        .ok_or_else(|| anyhow::anyhow!("No existing parent directory"))?
                        .to_path_buf();
                }
                let canonical_parent = current.canonicalize()?;
                if !canonical_parent.starts_with(&canonical_project) {
                    return Err(anyhow::anyhow!("PATH_OUTSIDE_PROJECT"));
                }
                let prefix = canonical_parent
                    .strip_prefix(&canonical_project)?
                    .to_string_lossy()
                    .to_string();
                let remaining = relative_path
                    .trim_start_matches(&prefix)
                    .trim_start_matches('/');

                let mut current_path = canonical_parent;
                for component in remaining.split('/') {
                    if component.is_empty() || component == "." {
                        continue;
                    }
                    if component == ".." {
                        return Err(anyhow::anyhow!("PATH_OUTSIDE_PROJECT"));
                    }
                    current_path = current_path.join(component);
                }
                current_path
            }
        };

        if !canonical_requested.starts_with(&canonical_project) {
            return Err(anyhow::anyhow!("PATH_OUTSIDE_PROJECT"));
        }

        Ok(canonical_requested)
    }
}
fn should_skip_directory(name: &str) -> bool {
    matches!(
        name,
        "target"
            | "node_modules"
            | ".git"
            | ".svn"
            | ".hg"
            | "dist"
            | "build"
            | "out"
            | "bin"
            | "obj"
            | ".idea"
            | ".vscode"
            | ".vs"
            | "__pycache__"
            | ".pytest_cache"
            | ".mypy_cache"
            | ".ruff_cache"
            | "vendor"
            | "bower_components"
            | "jspm_packages"
            | ".gradle"
            | ".maven"
            | "gradle"
            | "target"
            | "out"
            | ".next"
            | ".nuxt"
            | ".output"
            | ".vercel"
            | ".netlify"
    ) || name.starts_with('.') && name != ".github" && name != ".gitignore"
}

fn detect_language(ext: &str) -> String {
    match ext {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "py" => "python",
        "java" => "java",
        "cpp" | "cc" | "cxx" => "cpp",
        "c" => "c",
        "h" | "hpp" => "cpp",
        "go" => "go",
        "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "kt" | "kts" => "kotlin",
        "scala" => "scala",
        "clj" | "cljs" | "cljc" => "clojure",
        "hs" => "haskell",
        "ml" | "mli" => "ocaml",
        "fs" | "fsx" | "fsi" => "fsharp",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "xml" => "xml",
        "html" | "htm" => "html",
        "css" => "css",
        "scss" => "scss",
        "sass" => "sass",
        "less" => "less",
        "md" | "mdx" => "markdown",
        "txt" => "plaintext",
        "sh" | "bash" | "zsh" | "fish" => "shell",
        "ps1" => "powershell",
        "bat" | "cmd" => "batch",
        "dockerfile" => "dockerfile",
        "sql" => "sql",
        "graphql" | "gql" => "graphql",
        "proto" => "protobuf",
        "vue" => "vue",
        "svelte" => "svelte",
        "astro" => "astro",
        "elm" => "elm",
        "ex" | "exs" => "elixir",
        "erl" | "hrl" => "erlang",
        "lua" => "lua",
        "pl" | "pm" => "perl",
        "r" => "r",
        "jl" => "julia",
        "nim" => "nim",
        "zig" => "zig",
        _ => "plaintext",
    }
    .to_string()
}

pub fn get_file_language(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| detect_language(ext))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_fs() -> (tempfile::TempDir, ProjectFilesystem) {
        let dir = tempfile::TempDir::new().unwrap();
        let fs = ProjectFilesystem::new(dir.path());
        (dir, fs)
    }

    fn init_git_repo(path: &Path) {
        let out = std::process::Command::new("git")
            .args(["init"])
            .current_dir(path)
            .output()
            .expect("git must be installed for git-status tests");
        assert!(out.status.success(), "git init failed");
        // suppress git config warnings in CI/sandbox
        let _ = std::process::Command::new("git")
            .args(["config", "user.email", "test@zylcode"])
            .current_dir(path)
            .output();
        let _ = std::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(path)
            .output();
    }

    // ── A. SHA-256 content hashing & E. safe atomic writes ──────────────
    #[test]
    fn sha256_content_hash_and_atomic_write() {
        let (_tmp, fs) = temp_fs();
        fs.write_file("data.txt", "hello").unwrap();

        let hash1 = fs.compute_content_hash("data.txt").unwrap();
        assert_eq!(hash1.len(), 64, "SHA-256 hex is 64 chars");

        // atomic write with wrong hash → conflict
        let result = fs.write_file_atomic(
            "data.txt",
            "world",
            Some("0000000000000000000000000000000000000000000000000000000000000000"),
        );
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("FILE_CHANGED_ON_DISK"),
            "expected conflict, got: {}",
            err
        );

        // atomic write with correct hash → success
        let hash2 = fs
            .write_file_atomic("data.txt", "world", Some(&hash1))
            .unwrap();
        assert_ne!(hash1, hash2, "content changed, hash must change");

        let content = fs::read_to_string(fs.project_root().join("data.txt")).unwrap();
        assert_eq!(content, "world");
    }

    // ── B. Root containment ─────────────────────────────────────────────
    #[test]
    fn root_containment_rejects_escape() {
        let (_tmp, fs) = temp_fs();

        let r = fs.canonicalize_and_validate("../outside.txt");
        assert!(r.is_err());
        assert!(r.unwrap_err().to_string().contains("PATH_OUTSIDE_PROJECT"));

        let r = fs.create_file("../outside.txt", "x");
        assert!(r.is_err());
        assert!(r.unwrap_err().to_string().contains("PATH_OUTSIDE_PROJECT"));
    }

    // ── C. New file creation ────────────────────────────────────────────
    #[test]
    fn create_file_nested_with_hash() {
        let (_tmp, fs) = temp_fs();
        let hash = fs.create_file("deep/nested/file.txt", "hello").unwrap();
        assert_eq!(hash.len(), 64);

        let path = fs.project_root().join("deep/nested/file.txt");
        assert!(path.exists());
        assert_eq!(fs::read_to_string(path).unwrap(), "hello");
    }

    #[test]
    fn create_file_refuses_overwrite() {
        let (_tmp, fs) = temp_fs();
        fs.write_file("existing.txt", "old").unwrap();
        let r = fs.create_file("existing.txt", "new");
        assert!(r.is_err());
        assert!(r.unwrap_err().to_string().contains("already exists"));
    }

    // ── D. Binary classification ────────────────────────────────────────
    #[test]
    fn binary_classification_variants() {
        let (_tmp, fs) = temp_fs();

        // TEXT
        fs.write_file("text.txt", "Hello, 世界").unwrap();
        assert_eq!(
            fs.classify_file("text.txt").unwrap(),
            FileClassification::TEXT
        );

        // BINARY (null byte)
        let mut f = fs::File::create(fs.project_root().join("binary.bin")).unwrap();
        f.write_all(&[0u8, 1, 2, 3]).unwrap();
        drop(f);
        assert_eq!(
            fs.classify_file("binary.bin").unwrap(),
            FileClassification::BINARY
        );

        // UNSUPPORTED_ENCODING (invalid UTF-8)
        let mut f = fs::File::create(fs.project_root().join("bad.txt")).unwrap();
        f.write_all(&[0x80, 0x81, 0x82]).unwrap();
        drop(f);
        assert_eq!(
            fs.classify_file("bad.txt").unwrap(),
            FileClassification::UNSUPPORTED_ENCODING
        );
    }

    #[test]
    fn read_file_classified_includes_hash_and_language() {
        let (_tmp, fs) = temp_fs();
        fs.write_file("main.rs", "fn main() {}").unwrap();
        let c = fs.read_file_classified("main.rs").unwrap();
        assert_eq!(c.classification, FileClassification::TEXT);
        assert_eq!(c.language, Some("rust".to_string()));
        assert_eq!(c.content_hash.len(), 64);
        assert_eq!(c.line_count, Some(1));
    }

    // ── F. Git status ───────────────────────────────────────────────────
    #[test]
    fn git_status_detects_changes() {
        let (tmp, fs) = temp_fs();
        init_git_repo(tmp.path());

        // untracked
        fs.write_file("new.txt", "a").unwrap();
        let statuses = fs.get_git_status().unwrap();
        let s = statuses
            .iter()
            .find(|s| s.path == "new.txt")
            .expect("untracked expected");
        assert_eq!(s.status, GitFileStatus::Untracked);
        assert!(!s.staged);

        // staged add
        let out = std::process::Command::new("git")
            .args(["add", "new.txt"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        assert!(out.status.success());
        let statuses = fs.get_git_status().unwrap();
        let s = statuses
            .iter()
            .find(|s| s.path == "new.txt")
            .expect("staged add expected");
        assert_eq!(s.status, GitFileStatus::Added);
        assert!(s.staged, "added file should be staged");

        // After `git add` then modify, porcelain shows "AM" (index Added, worktree Modified).
        // Our parser derives status from the index char, so it remains Added.
        fs.write_file("new.txt", "b").unwrap();
        let statuses = fs.get_git_status().unwrap();
        let s = statuses
            .iter()
            .find(|s| s.path == "new.txt")
            .expect("modified expected");
        assert!(
            matches!(s.status, GitFileStatus::Added | GitFileStatus::Modified),
            "unexpected status after modification: {:?}",
            s.status
        );
        assert!(s.staged, "file should still be staged after modification");
    }

    // ── G. Project session authority ────────────────────────────────────
    #[test]
    fn project_session_authority_bound_to_root() {
        let (tmp1, fs1) = temp_fs();
        let (tmp2, fs2) = temp_fs();

        fs1.write_file("a.txt", "1").unwrap();
        fs2.write_file("a.txt", "2").unwrap();

        assert_eq!(
            fs::read_to_string(fs1.project_root().join("a.txt")).unwrap(),
            "1"
        );
        assert_eq!(
            fs::read_to_string(fs2.project_root().join("a.txt")).unwrap(),
            "2"
        );
    }

    // ── H. Lazy explorer ────────────────────────────────────────────────
    #[test]
    fn lazy_explorer_skips_and_depth() {
        let (_tmp, fs) = temp_fs();
        fs::create_dir_all(fs.project_root().join("target/debug")).unwrap();
        fs::create_dir_all(fs.project_root().join("node_modules/lodash")).unwrap();
        fs::create_dir_all(fs.project_root().join(".git/objects")).unwrap();
        fs::create_dir_all(fs.project_root().join("src/components")).unwrap();
        fs.write_file("src/main.rs", "fn main() {}").unwrap();
        fs.write_file("src/components/lib.rs", "").unwrap();

        let tree = fs.build_file_tree(None).unwrap();
        let names: Vec<_> = tree.children.iter().map(|c| c.name.as_str()).collect();
        assert!(!names.contains(&"target"), "target should be skipped");
        assert!(
            !names.contains(&"node_modules"),
            "node_modules should be skipped"
        );
        assert!(!names.contains(&".git"), ".git should be skipped");
        assert!(names.contains(&"src"), "src should be present");

        // depth 0 → root only, no children traversed
        let tree_shallow = fs.build_file_tree(Some(0)).unwrap();
        assert!(tree_shallow.children.is_empty());
    }

    #[test]
    fn list_directory_sorts_dirs_first() {
        let (_tmp, fs) = temp_fs();
        fs.write_file("z_file.txt", "").unwrap();
        fs::create_dir_all(fs.project_root().join("a_dir")).unwrap();
        fs.write_file("a_dir/b.txt", "").unwrap();

        let nodes = fs.list_directory("").unwrap();
        assert!(nodes[0].is_directory);
        assert_eq!(nodes[0].name, "a_dir");
        assert!(!nodes[1].is_directory);
        assert_eq!(nodes[1].name, "z_file.txt");
    }
}
