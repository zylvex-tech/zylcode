//! Language and file classification.
//!
//! Classifies files by language, role, and support level.
//! Commissioned languages: Rust, TypeScript, JavaScript, JSON, TOML, YAML, Markdown.
//! Architecture permits later adapters.

use crate::intelligence::types::{FileRole, Language, LanguageSupport};
use std::path::Path;

/// Classify a file's language from its extension.
pub fn classify_language(path: &Path) -> Language {
    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some(e) => e.to_ascii_lowercase(),
        None => {
            // Check for extensionless known files
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            return match name {
                "Makefile" | "makefile" | "GNUmakefile" => Language::Shell,
                "Dockerfile" | "Containerfile" => Language::Shell,
                _ => Language::Unknown(String::new()),
            };
        }
    };

    match ext.as_str() {
        "rs" => Language::Rust,
        "ts" | "mts" | "cts" => Language::TypeScript,
        "tsx" => Language::TypeScript,
        "js" | "mjs" | "cjs" => Language::JavaScript,
        "jsx" => Language::JavaScript,
        "json" | "jsonc" => Language::Json,
        "toml" => Language::Toml,
        "yaml" | "yml" => Language::Yaml,
        "md" | "mdx" => Language::Markdown,
        "html" | "htm" => Language::Html,
        "css" | "scss" | "sass" | "less" => Language::Css,
        "sh" | "bash" | "zsh" | "fish" | "ps1" | "bat" | "cmd" => Language::Shell,
        _ => Language::Unknown(ext),
    }
}

/// Classify a file's role in the repository.
pub fn classify_role(path: &Path, _content: Option<&str>, _package_name: Option<&str>) -> FileRole {
    // Normalise separators so the `/segment/` substring checks below also match
    // on Windows, where paths arrive with backslashes.
    let path_str = path.to_string_lossy().replace('\\', "/").to_lowercase();
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    // Lock files
    if name == "cargo.lock"
        || name == "package-lock.json"
        || name == "yarn.lock"
        || name == "pnpm-lock.yaml"
    {
        return FileRole::LockFile;
    }

    // CI configuration
    if path_str.contains(".github/workflows")
        || path_str.contains(".gitlab-ci")
        || name == "ci.yml"
        || name == "ci.yaml"
        || name == "jenkinsfile"
    {
        return FileRole::Ci;
    }

    // Build scripts
    if name == "build.rs" || name == "build.sh" || name == "makefile" || name == "justfile" {
        return FileRole::BuildScript;
    }

    // Configuration files
    if is_config_file(&name, &path_str) {
        return FileRole::Config;
    }

    // Documentation
    if name.ends_with(".md") || name.ends_with(".mdx") || name.ends_with(".txt") {
        return FileRole::Documentation;
    }
    if name == "readme" || name == "changelog" || name == "license" || name == "contributing" {
        return FileRole::Documentation;
    }

    // Test files
    if is_test_file(path, &path_str, &name) {
        return FileRole::Test;
    }

    // Entry points — detected from manifest evidence, not just filename
    if name == "main.rs" || name == "lib.rs" || name == "index.ts" || name == "index.tsx" {
        return FileRole::EntryPoint;
    }

    // Generated files
    if path_str.contains("/generated/")
        || path_str.contains("/dist/")
        || path_str.contains("/build/")
        || name.ends_with(".d.ts")
        || name.ends_with(".min.js")
        || name.ends_with(".min.css")
    {
        return FileRole::Generated;
    }

    // Vendor / dependency caches
    if path_str.contains("/vendor/")
        || path_str.contains("/node_modules/")
        || path_str.contains("/target/")
    {
        return FileRole::Vendor;
    }

    // Source files
    let lang = classify_language(path);
    match lang {
        Language::Rust
        | Language::TypeScript
        | Language::JavaScript
        | Language::Html
        | Language::Css => FileRole::Source,
        Language::Shell => {
            if path_str.contains("/scripts/") || name.ends_with(".sh") {
                FileRole::Script
            } else {
                FileRole::Source
            }
        }
        _ => FileRole::Other(name),
    }
}

/// Return the semantic support level for a language.
pub fn support_level(lang: &Language) -> LanguageSupport {
    match lang {
        Language::Rust => LanguageSupport::Parsed,
        Language::TypeScript | Language::JavaScript => LanguageSupport::Parsed,
        Language::Json | Language::Toml | Language::Yaml => LanguageSupport::Parsed,
        Language::Markdown => LanguageSupport::Detected,
        Language::Html | Language::Css => LanguageSupport::Detected,
        Language::Shell => LanguageSupport::Detected,
        Language::Unknown(_) => LanguageSupport::Detected,
    }
}

/// Check if a file is a test file.
fn is_test_file(_path: &Path, path_str: &str, name: &str) -> bool {
    // Rust tests directory
    if path_str.contains("/tests/") && name.ends_with(".rs") {
        return true;
    }

    // Rust inline test files
    if name.ends_with("_test.rs") || name.ends_with("_tests.rs") {
        return true;
    }

    // JavaScript/TypeScript test files
    if name.ends_with(".test.ts")
        || name.ends_with(".test.tsx")
        || name.ends_with(".test.js")
        || name.ends_with(".test.jsx")
        || name.ends_with(".spec.ts")
        || name.ends_with(".spec.tsx")
        || name.ends_with(".spec.js")
        || name.ends_with(".spec.jsx")
    {
        return true;
    }

    // __tests__ directory
    if path_str.contains("/__tests__/") {
        return true;
    }

    false
}

/// Check if a file is a configuration file.
fn is_config_file(name: &str, path_str: &str) -> bool {
    let config_names = [
        "cargo.toml",
        "package.json",
        "tsconfig.json",
        "pnpm-workspace.yaml",
        ".gitignore",
        ".eslintrc",
        ".prettierrc",
        "rustfmt.toml",
        "rust-toolchain.toml",
        "clippy.toml",
        "tauri.conf.json",
        "vite.config.ts",
        "tailwind.config.js",
        "tailwind.config.ts",
        "postcss.config.js",
        "jest.config.js",
        "vitest.config.ts",
    ];

    if config_names.contains(&name) {
        return true;
    }

    // Config directories
    if path_str.contains("/.github/")
        || path_str.contains("/.vscode/")
        || path_str.contains("/.config/")
    {
        return true;
    }

    false
}

/// Check if a path should be excluded from scanning.
pub fn should_exclude(path: &Path) -> bool {
    // Normalise separators before matching: on Windows, WalkDir yields
    // backslash paths, and every exclusion substring below uses forward
    // slashes. Without this, `target/`, `node_modules/` and `.git/` are never
    // excluded on Windows and the scanner descends into build output — which
    // is what made the Phase 2A indexing benchmark fail on re-execution.
    let path_str = path.to_string_lossy().replace('\\', "/");

    // Standard exclusions
    let excluded = [
        "/target/",
        "/node_modules/",
        "/dist/",
        "/build/",
        "/.git/",
        "/.next/",
        "/.nuxt/",
        "/__pycache__/",
        "/.venv/",
        "/venv/",
        "/.cache/",
        "/coverage/",
        "target/",
        "node_modules/",
        ".git/",
    ];

    for exc in &excluded {
        if path_str.contains(exc) {
            return true;
        }
    }

    // Hidden files at the root level (but allow .github, .vscode etc.)
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if name.starts_with('.') && !name.starts_with(".github") && !name.starts_with(".vscode") {
        // Check if it's a root-level hidden file
        if path.parent().and_then(|p| p.file_name()).is_none() {
            return true;
        }
    }

    false
}

/// Check if a file is a secret file that should NOT be indexed.
pub fn is_secret_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let secret_patterns = [
        ".env",
        ".env.local",
        ".env.production",
        ".env.development",
        "id_rsa",
        "id_ed25519",
        "id_ecdsa",
        "*.pem",
        "*.key",
        "*.p12",
        "*.pfx",
        "*.keystore",
        "credentials.json",
        "secrets.json",
        "token.json",
        ".npmrc",
        ".cargo/credentials",
    ];

    for pattern in &secret_patterns {
        if let Some(suffix) = pattern.strip_prefix('*') {
            if name.ends_with(suffix) {
                return true;
            }
        } else if name == *pattern {
            return true;
        }
    }

    // .env files at any depth
    if name.starts_with(".env") {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_rust_source() {
        let lang = classify_language(Path::new("src/agent.rs"));
        assert_eq!(lang, Language::Rust);
    }

    #[test]
    fn classify_typescript() {
        let lang = classify_language(Path::new("src/App.tsx"));
        assert_eq!(lang, Language::TypeScript);
    }

    #[test]
    fn classify_json() {
        let lang = classify_language(Path::new("package.json"));
        assert_eq!(lang, Language::Json);
    }

    #[test]
    fn classify_toml() {
        let lang = classify_language(Path::new("Cargo.toml"));
        assert_eq!(lang, Language::Toml);
    }

    #[test]
    fn classify_yaml() {
        let lang = classify_language(Path::new("pnpm-workspace.yaml"));
        assert_eq!(lang, Language::Yaml);
    }

    #[test]
    fn classify_markdown() {
        let lang = classify_language(Path::new("README.md"));
        assert_eq!(lang, Language::Markdown);
    }

    #[test]
    fn classify_unknown_extension() {
        let lang = classify_language(Path::new("file.xyz"));
        assert_eq!(lang, Language::Unknown("xyz".to_string()));
    }

    #[test]
    fn classify_makefile() {
        let lang = classify_language(Path::new("Makefile"));
        assert_eq!(lang, Language::Shell);
    }

    #[test]
    fn role_test_file() {
        let role = classify_role(
            Path::new("crates/zylcode-core/tests/crash_recovery.rs"),
            None,
            None,
        );
        assert_eq!(role, FileRole::Test);
    }

    #[test]
    fn role_test_ts() {
        let role = classify_role(Path::new("src/App.test.tsx"), None, None);
        assert_eq!(role, FileRole::Test);
    }

    #[test]
    fn role_config() {
        let role = classify_role(Path::new("Cargo.toml"), None, None);
        assert_eq!(role, FileRole::Config);
    }

    #[test]
    fn role_documentation() {
        let role = classify_role(Path::new("README.md"), None, None);
        assert_eq!(role, FileRole::Documentation);
    }

    #[test]
    fn role_lockfile() {
        let role = classify_role(Path::new("Cargo.lock"), None, None);
        assert_eq!(role, FileRole::LockFile);
    }

    #[test]
    fn role_ci() {
        let role = classify_role(Path::new(".github/workflows/ci.yml"), None, None);
        assert_eq!(role, FileRole::Ci);
    }

    #[test]
    fn role_entry_point() {
        let role = classify_role(Path::new("src/main.rs"), None, None);
        assert_eq!(role, FileRole::EntryPoint);
    }

    #[test]
    fn role_source() {
        let role = classify_role(Path::new("src/agent.rs"), None, None);
        assert_eq!(role, FileRole::Source);
    }

    #[test]
    fn role_build_script() {
        let role = classify_role(Path::new("build.rs"), None, None);
        assert_eq!(role, FileRole::BuildScript);
    }

    #[test]
    fn exclude_target() {
        assert!(should_exclude(Path::new("target/debug/build/foo")));
    }

    #[test]
    fn exclude_node_modules() {
        assert!(should_exclude(Path::new("node_modules/foo/index.js")));
    }

    #[test]
    fn exclude_git() {
        assert!(should_exclude(Path::new(".git/objects/abc")));
    }

    #[test]
    fn dont_exclude_src() {
        assert!(!should_exclude(Path::new("src/main.rs")));
    }

    #[test]
    fn secret_env_file() {
        assert!(is_secret_file(Path::new(".env")));
        assert!(is_secret_file(Path::new(".env.local")));
        assert!(is_secret_file(Path::new(".env.production")));
    }

    #[test]
    fn secret_key_file() {
        assert!(is_secret_file(Path::new("server.key")));
        assert!(is_secret_file(Path::new("cert.pem")));
    }

    #[test]
    fn not_secret_source() {
        assert!(!is_secret_file(Path::new("src/main.rs")));
        assert!(!is_secret_file(Path::new("Cargo.toml")));
    }

    #[test]
    fn support_level_rust() {
        assert_eq!(support_level(&Language::Rust), LanguageSupport::Parsed);
    }

    #[test]
    fn support_level_markdown() {
        assert_eq!(
            support_level(&Language::Markdown),
            LanguageSupport::Detected
        );
    }
}
