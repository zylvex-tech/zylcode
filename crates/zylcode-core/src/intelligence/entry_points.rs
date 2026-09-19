//! Entry-point discovery.
//!
//! Discovers application/runtime boundaries using manifest evidence
//! (not just filename heuristics).

use crate::intelligence::types::{EntryPoint, EntryPointKind, Package};
use anyhow::Result;
use std::path::{Path, PathBuf};

/// Resolve a package's directory relative to the repository root.
///
/// Uses the package manifest (always repo-relative) so this works
/// regardless of how `pkg.root` was populated.
fn package_dir(root: &Path, pkg: &Package) -> PathBuf {
    root.join(&pkg.manifest)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| root.to_path_buf())
}

/// Convert an absolute path under `root` to a repo-relative forward-slash
/// string. Leaves already-relative paths untouched. Entry-point paths are
/// stored repo-relative so model output does not embed the checkout
/// location (determinism requirement).
fn to_repo_relative(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Discover all entry points in a repository.
pub fn discover_entry_points(root: &Path, packages: &[Package]) -> Result<Vec<EntryPoint>> {
    let mut entry_points = Vec::new();

    for pkg in packages {
        let pkg_dir = package_dir(root, pkg);
        // Check for Rust binary targets from Cargo.toml
        let manifest_path = root.join(&pkg.manifest);
        if manifest_path.exists() {
            let content = std::fs::read_to_string(&manifest_path).unwrap_or_default();

            // Binary targets
            if let Ok(doc) = toml::from_str::<toml::Value>(&content) {
                // Explicit [[bin]] targets
                if let Some(bins) = doc.get("bin").and_then(|b| b.as_array()) {
                    for bin in bins {
                        if let (Some(name), Some(path)) = (
                            bin.get("name").and_then(|n| n.as_str()),
                            bin.get("path").and_then(|p| p.as_str()),
                        ) {
                            entry_points.push(EntryPoint {
                                path: PathBuf::from(to_repo_relative(&pkg_dir.join(path), root)),
                                kind: EntryPointKind::Binary(name.to_string()),
                                package: Some(pkg.id.clone()),
                                evidence: vec![
                                    format!("{}: [[bin]] section", pkg.manifest),
                                    format!("name = \"{}\"", name),
                                    format!("path = \"{}\"", path),
                                ],
                            });
                        }
                    }
                }

                // Library target
                if let Some(lib) = doc.get("lib") {
                    let lib_path = lib
                        .get("path")
                        .and_then(|p| p.as_str())
                        .unwrap_or("src/lib.rs");
                    entry_points.push(EntryPoint {
                        path: PathBuf::from(to_repo_relative(&pkg_dir.join(lib_path), root)),
                        kind: EntryPointKind::Lib,
                        package: Some(pkg.id.clone()),
                        evidence: vec![
                            format!("{}: [lib] section", pkg.manifest),
                            format!("path = \"{}\"", lib_path),
                        ],
                    });
                }

                // Default main.rs (if no explicit [[bin]] and src/main.rs exists)
                if doc.get("bin").is_none() {
                    let main_path = pkg_dir.join("src/main.rs");
                    if main_path.exists() {
                        entry_points.push(EntryPoint {
                            path: PathBuf::from(to_repo_relative(&main_path, root)),
                            kind: EntryPointKind::Main,
                            package: Some(pkg.id.clone()),
                            evidence: vec![
                                format!("{}: no [[bin]] section", pkg.manifest),
                                "src/main.rs exists".to_string(),
                            ],
                        });
                    }
                }

                // Build script
                let build_rs = pkg_dir.join("build.rs");
                if build_rs.exists() {
                    entry_points.push(EntryPoint {
                        path: PathBuf::from(to_repo_relative(&build_rs, root)),
                        kind: EntryPointKind::BuildScript,
                        package: Some(pkg.id.clone()),
                        evidence: vec![format!("{}: build.rs exists", pkg.manifest)],
                    });
                }

                // Test targets
                let tests_dir = pkg_dir.join("tests");
                if tests_dir.exists() && tests_dir.is_dir() {
                    for entry in std::fs::read_dir(&tests_dir)? {
                        let entry = entry?;
                        let path = entry.path();
                        if path.extension().map(|e| e.to_str()) == Some(Some("rs")) {
                            entry_points.push(EntryPoint {
                                path: PathBuf::from(to_repo_relative(&path, root)),
                                kind: EntryPointKind::Test,
                                package: Some(pkg.id.clone()),
                                evidence: vec![format!("tests/ directory in {}", pkg.name)],
                            });
                        }
                    }
                }
            }
        }

        // Check for Tauri app
        let tauri_conf = pkg_dir.join("tauri.conf.json");
        if tauri_conf.exists() {
            // pkg_dir is the package containing tauri.conf.json (the src-tauri
            // directory for a cargo package), so the Rust main is directly
            // under it — not under a second src-tauri level.
            entry_points.push(EntryPoint {
                path: PathBuf::from(to_repo_relative(&pkg_dir.join("src/main.rs"), root)),
                kind: EntryPointKind::TauriApp,
                package: Some(pkg.id.clone()),
                evidence: vec!["tauri.conf.json exists".to_string()],
            });
        }

        // Check for package.json scripts
        let pkg_json = pkg_dir.join("package.json");
        if pkg_json.exists() {
            if let Ok(content) = std::fs::read_to_string(&pkg_json) {
                if let Ok(doc) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(scripts) = doc.get("scripts").and_then(|s| s.as_object()) {
                        // dev/build scripts indicate an application
                        if scripts.contains_key("dev") || scripts.contains_key("build") {
                            // Check for common frontend entry points
                            for entry_name in &[
                                "index.html",
                                "src/index.tsx",
                                "src/index.ts",
                                "src/main.tsx",
                                "src/main.ts",
                                "src/App.tsx",
                                "src/App.ts",
                            ] {
                                let entry_path = pkg_dir.join(entry_name);
                                if entry_path.exists() {
                                    entry_points.push(EntryPoint {
                                        path: PathBuf::from(to_repo_relative(&entry_path, root)),
                                        kind: EntryPointKind::ReactBootstrap,
                                        package: Some(pkg.id.clone()),
                                        evidence: vec![
                                            format!("package.json in {}", pkg.name),
                                            format!("{} exists", entry_name),
                                        ],
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(entry_points)
}

/// Get build commands for a package.
pub fn build_commands(pkg: &Package) -> Vec<String> {
    pkg.build_commands.clone()
}

/// Get test commands for a package.
pub fn test_commands(pkg: &Package) -> Vec<String> {
    pkg.test_commands.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn discover_rust_binary_entry_point() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(
            root.join("Cargo.toml"),
            r#"[package]
name = "test-app"
version = "0.1.0"
"#,
        )
        .unwrap();

        let pkg = Package {
            id: "test-app".to_string(),
            name: "test-app".to_string(),
            version: "0.1.0".to_string(),
            root: root.to_path_buf(),
            language: crate::intelligence::types::Language::Rust,
            manifest: "Cargo.toml".to_string(),
            files: Vec::new(),
            dependencies: Vec::new(),
            dev_dependencies: Vec::new(),
            build_commands: vec!["cargo build".to_string()],
            test_commands: vec!["cargo test".to_string()],
            entry_points: Vec::new(),
        };

        let entry_points = discover_entry_points(root, &[pkg]).unwrap();
        let main_entries: Vec<_> = entry_points
            .iter()
            .filter(|ep| ep.kind == EntryPointKind::Main)
            .collect();
        assert_eq!(main_entries.len(), 1);
    }

    #[test]
    fn discover_library_entry_point() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/lib.rs"), "pub mod agent;").unwrap();
        fs::write(
            root.join("Cargo.toml"),
            r#"
[package]
name = "test-lib"
version = "0.1.0"

[lib]
path = "src/lib.rs"
"#,
        )
        .unwrap();

        let pkg = Package {
            id: "test-lib".to_string(),
            name: "test-lib".to_string(),
            version: "0.1.0".to_string(),
            root: root.to_path_buf(),
            language: crate::intelligence::types::Language::Rust,
            manifest: "Cargo.toml".to_string(),
            files: Vec::new(),
            dependencies: Vec::new(),
            dev_dependencies: Vec::new(),
            build_commands: vec!["cargo build".to_string()],
            test_commands: vec!["cargo test".to_string()],
            entry_points: Vec::new(),
        };

        let entry_points = discover_entry_points(root, &[pkg]).unwrap();
        let lib_entries: Vec<_> = entry_points
            .iter()
            .filter(|ep| ep.kind == EntryPointKind::Lib)
            .collect();
        assert_eq!(lib_entries.len(), 1);
    }

    #[test]
    fn discover_build_script() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        fs::write(root.join("build.rs"), "fn main() {}").unwrap();
        fs::write(
            root.join("Cargo.toml"),
            r#"[package]
name = "test-build"
version = "0.1.0"
"#,
        )
        .unwrap();

        let pkg = Package {
            id: "test-build".to_string(),
            name: "test-build".to_string(),
            version: "0.1.0".to_string(),
            root: root.to_path_buf(),
            language: crate::intelligence::types::Language::Rust,
            manifest: "Cargo.toml".to_string(),
            files: Vec::new(),
            dependencies: Vec::new(),
            dev_dependencies: Vec::new(),
            build_commands: vec!["cargo build".to_string()],
            test_commands: vec!["cargo test".to_string()],
            entry_points: Vec::new(),
        };

        let entry_points = discover_entry_points(root, &[pkg]).unwrap();
        let build_entries: Vec<_> = entry_points
            .iter()
            .filter(|ep| ep.kind == EntryPointKind::BuildScript)
            .collect();
        assert_eq!(build_entries.len(), 1);
    }
}
