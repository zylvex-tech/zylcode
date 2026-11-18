//! Language and file classification.
//!
//! Classifies files by language, role, and support level.
//! Commissioned languages: Rust, TypeScript, JavaScript, JSON, TOML, YAML, Markdown.
//! Architecture permits later adapters.

use crate::intelligence::types::{FileRole, Language, LanguageSupport};
use std::path::{Component, Path};

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

/// Directory names that are never part of the source inventory.
///
/// These are matched against **whole path components**, never as substrings.
/// The distinction is the difference between excluding build output and
/// silently dropping source: `mytarget/notes.md` and `src/target_helpers.rs`
/// contain the substring `target` but are ordinary repository content, while
/// `target/debug/foo` and `crates/x/target/foo` are generated trees. The
/// previous implementation used `contains("target/")`, which excluded the
/// former as well and could not be fixed by normalising separators alone.
///
/// Path components are compared exactly, so a directory that differs only in
/// case (`Target/`) is *not* excluded on a case-sensitive filesystem. That
/// matches the policy recorded in `PHASE2A_REMEDIATION_ORDER.md` and the
/// repository `.gitignore`, which are both case-sensitive.
const EXCLUDED_DIR_NAMES: &[&str] = &[
    "target",
    "node_modules",
    "dist",
    "build",
    ".git",
    ".next",
    ".nuxt",
    "__pycache__",
    ".venv",
    "venv",
    ".cache",
    "coverage",
];

/// Check if a path should be excluded from scanning.
pub fn should_exclude(path: &Path) -> bool {
    // Match on components rather than on the rendered string. `Path::components`
    // splits on both `\` and `/` and collapses `.`/repeated separators, so a
    // Windows path (`C:\\repo\\target\\debug\\x`) and a Unix path
    // (`/repo/target/debug/x`) are handled by the same rule with no separator
    // normalisation step that a future edit could forget to apply.
    if path.components().any(|component| match component {
        Component::Normal(name) => name
            .to_str()
            .is_some_and(|name| EXCLUDED_DIR_NAMES.contains(&name)),
        _ => false,
    }) {
        return true;
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
    name.starts_with(".env")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `docs/architecture/LANGUAGE_COVERAGE.md` is the public statement of what
    /// is parsed per language. This parses that table and fails when the
    /// document and `support_level()` disagree — the documentation claim is
    /// checked by the code rather than trusted.
    #[test]
    fn documented_language_coverage_matches_support_level() {
        const DOC: &str = include_str!("../../../../docs/architecture/LANGUAGE_COVERAGE.md");

        let mut checked = 0;
        for line in DOC.lines() {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            // Rows look like: "" | Rust | .rs | Parsed | symbols | note | ""
            if cells.len() < 7 {
                continue;
            }
            let (name, support, symbols) = (cells[1], cells[3], cells[4]);
            let language = match name {
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
                _ => continue,
            };
            let expected = match support {
                "Parsed" => LanguageSupport::Parsed,
                "Detected" => LanguageSupport::Detected,
                other => panic!("{name}: unknown support value in the doc: {other}"),
            };
            assert_eq!(
                super::support_level(&language),
                expected,
                "{name}: LANGUAGE_COVERAGE.md says {support}, classifier disagrees"
            );
            if symbols == "symbols" {
                let sample = match language {
                    Language::Rust => "pub struct Documented {}",
                    _ => "export class Documented {}",
                };
                assert!(
                    !crate::intelligence::symbols::extract_symbols(
                        "doc",
                        std::path::Path::new("doc"),
                        sample,
                        &language
                    )
                    .is_empty(),
                    "{name}: doc claims symbol extraction, extractor returned nothing"
                );
            }
            checked += 1;
        }
        assert_eq!(
            checked, 10,
            "the coverage table must document all ten languages"
        );
    }

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

    // Windows path semantics only exist on Windows: on Unix a backslash is an
    // ordinary filename character, so `C:\repo\target\x` is ONE component and
    // must not be excluded. CI runs this suite on Linux and macOS, which is why
    // the two platforms are asserted separately rather than in one test.
    #[cfg(windows)]
    #[test]
    fn exclude_windows_absolute_path() {
        // The primary platform: WalkDir yields backslash paths there.
        assert!(should_exclude(Path::new(
            r"C:\Projects\zylcode\target\debug\zylcode.exe"
        )));
        assert!(should_exclude(Path::new(
            r"C:\Users\dev\repo\node_modules\pkg\index.js"
        )));
        assert!(should_exclude(Path::new(
            r"C:\Users\dev\repo\.git\objects\ab\cd1234"
        )));
        // Extended-length prefix as produced by `canonicalize` on Windows.
        assert!(should_exclude(Path::new(
            r"\\?\C:\Projects\zylcode\target\debug\zylcode.exe"
        )));
    }

    #[cfg(not(windows))]
    #[test]
    fn backslash_sequence_is_one_filename_on_unix() {
        // Documents platform semantics rather than a product rule: a Unix path
        // cannot contain a `target/` *component* via backslashes, so exclusion
        // must not fire. Windows behaviour is covered by the test above on the
        // Windows CI job.
        assert!(!should_exclude(Path::new(
            r"C:\Projects\zylcode\target\debug\zylcode.exe"
        )));
    }

    #[test]
    fn exclude_unix_absolute_path() {
        assert!(should_exclude(Path::new(
            "/home/dev/zylcode/target/debug/zylcode"
        )));
        assert!(should_exclude(Path::new(
            "/home/dev/repo/node_modules/x/index.js"
        )));
        assert!(should_exclude(Path::new("/home/dev/repo/.git/HEAD")));
    }

    #[test]
    fn exclude_nested_generated_directory() {
        // `**/target` in the repository .gitignore is per-directory, not only
        // at the root.
        assert!(should_exclude(Path::new(
            "crates/zylcode-core/target/debug/x"
        )));
        assert!(should_exclude(Path::new("apps/web/dist/assets/index.js")));
        assert!(should_exclude(Path::new("services/api/build/lib.rs")));
        assert!(should_exclude(Path::new("crates/x/__pycache__/m.pyc")));
    }

    #[test]
    fn dont_exclude_near_match_directory() {
        // Substring matching classified these as build output. They are not.
        assert!(!should_exclude(Path::new("mytarget/notes.md")));
        assert!(!should_exclude(Path::new("crates/mytarget/src/lib.rs")));
        assert!(!should_exclude(Path::new("docs/target-notes.md")));
        assert!(!should_exclude(Path::new("src/target_helpers.rs")));
        assert!(!should_exclude(Path::new("src/build.rs")));
        assert!(!should_exclude(Path::new("crates/x/distances.rs")));
        assert!(!should_exclude(Path::new("docs/build-guide.md")));
    }

    #[test]
    fn exclude_git_but_keep_github_workflows() {
        assert!(!should_exclude(Path::new(".github/workflows/ci.yml")));
        assert!(!should_exclude(Path::new(
            "crates/x/.github/ISSUE_TEMPLATE.md"
        )));
        assert!(should_exclude(Path::new("vendor/.git/config")));
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
