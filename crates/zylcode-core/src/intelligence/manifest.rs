//! Manifest / Workspace Intelligence.
//!
//! Structured parsing for Cargo.toml, package.json, pnpm-workspace.yaml.
//! Detects workspace members, package names, versions, dependencies, scripts.

use crate::intelligence::types::{
    Dependency, DependencyKind, DependencyTarget, Package, Workspace, WorkspaceType,
};
use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use std::path::Path;

/// Parse all manifests in a repository and return workspaces + packages.
pub fn discover_workspaces(root: &Path) -> Result<Vec<Workspace>> {
    let mut workspaces = Vec::new();

    // Check for Cargo workspace
    let cargo_toml = root.join("Cargo.toml");
    if cargo_toml.exists() {
        if let Ok(ws) = parse_cargo_workspace(root, &cargo_toml) {
            workspaces.push(ws);
        }
    }

    // Check for pnpm workspace
    let pnpm_ws = root.join("pnpm-workspace.yaml");
    if pnpm_ws.exists() {
        if let Ok(ws) = parse_pnpm_workspace(root, &pnpm_ws) {
            workspaces.push(ws);
        }
    }

    // Check for npm/yarn workspaces (package.json with "workspaces" field)
    let pkg_json = root.join("package.json");
    if pkg_json.exists() && !pnpm_ws.exists() {
        if let Ok(ws) = parse_npm_workspace(root, &pkg_json) {
            workspaces.push(ws);
        }
    }

    Ok(workspaces)
}

/// Parse a Cargo workspace from the root Cargo.toml.
fn parse_cargo_workspace(root: &Path, manifest_path: &Path) -> Result<Workspace> {
    let content = std::fs::read_to_string(manifest_path).context("Failed to read Cargo.toml")?;
    let doc: toml::Value = toml::from_str(&content).context("Failed to parse Cargo.toml")?;

    let members = doc
        .get("workspace")
        .and_then(|ws| ws.get("members"))
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // Resolve glob patterns to actual paths
    let mut resolved_members = Vec::new();
    for pattern in &members {
        if pattern.contains('*') {
            // Glob pattern
            for entry in glob::glob(&root.join(pattern).to_string_lossy())
                .unwrap_or_else(|_| glob::glob(pattern).unwrap())
                .flatten()
            {
                if entry.join("Cargo.toml").exists() {
                    resolved_members.push(
                        entry
                            .strip_prefix(root)
                            .unwrap_or(&entry)
                            .to_string_lossy()
                            .to_string(),
                    );
                }
            }
        } else {
            resolved_members.push(pattern.clone());
        }
    }

    let package_ids: Vec<String> = resolved_members
        .iter()
        .filter_map(|m| {
            let manifest = root.join(m).join("Cargo.toml");
            if manifest.exists() {
                let content = std::fs::read_to_string(&manifest).ok()?;
                let doc: toml::Value = toml::from_str(&content).ok()?;
                doc.get("package")
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();

    Ok(Workspace {
        id: format!("cargo:{}", root.to_string_lossy()),
        root: root.to_path_buf(),
        workspace_type: WorkspaceType::Cargo,
        members: package_ids,
        manifest_path: manifest_path
            .strip_prefix(root)
            .unwrap_or(manifest_path)
            .to_string_lossy()
            .to_string(),
    })
}

/// Parse a pnpm workspace.
fn parse_pnpm_workspace(root: &Path, manifest_path: &Path) -> Result<Workspace> {
    let content =
        std::fs::read_to_string(manifest_path).context("Failed to read pnpm-workspace.yaml")?;
    let doc: serde_yaml::Value =
        serde_yaml::from_str(&content).context("Failed to parse pnpm-workspace.yaml")?;

    let packages = doc
        .get("packages")
        .and_then(|p| p.as_sequence())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let mut package_ids = Vec::new();
    for pattern in &packages {
        for entry in glob::glob(&root.join(pattern).to_string_lossy())
            .unwrap_or_else(|_| glob::glob(pattern).unwrap())
            .flatten()
        {
            let pkg_json = entry.join("package.json");
            if pkg_json.exists() {
                if let Ok(content) = std::fs::read_to_string(&pkg_json) {
                    if let Ok(doc) = serde_json::from_str::<JsonValue>(&content) {
                        if let Some(name) = doc.get("name").and_then(|n| n.as_str()) {
                            package_ids.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    Ok(Workspace {
        id: format!("pnpm:{}", root.to_string_lossy()),
        root: root.to_path_buf(),
        workspace_type: WorkspaceType::Pnpm,
        members: package_ids,
        manifest_path: manifest_path
            .strip_prefix(root)
            .unwrap_or(manifest_path)
            .to_string_lossy()
            .to_string(),
    })
}

/// Parse an npm workspace from package.json.
fn parse_npm_workspace(root: &Path, manifest_path: &Path) -> Result<Workspace> {
    let content = std::fs::read_to_string(manifest_path).context("Failed to read package.json")?;
    let doc: JsonValue = serde_json::from_str(&content).context("Failed to parse package.json")?;

    let packages = doc
        .get("workspaces")
        .and_then(|w| {
            // Can be array or { packages: [...] }
            if let Some(arr) = w.as_array() {
                Some(arr.clone())
            } else {
                w.get("packages").and_then(|p| p.as_array()).cloned()
            }
        })
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let mut package_ids = Vec::new();
    for pattern in &packages {
        for entry in glob::glob(&root.join(pattern).to_string_lossy())
            .unwrap_or_else(|_| glob::glob(pattern).unwrap())
            .flatten()
        {
            let pkg_json = entry.join("package.json");
            if pkg_json.exists() {
                if let Ok(content) = std::fs::read_to_string(&pkg_json) {
                    if let Ok(doc) = serde_json::from_str::<JsonValue>(&content) {
                        if let Some(name) = doc.get("name").and_then(|n| n.as_str()) {
                            package_ids.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    Ok(Workspace {
        id: format!("npm:{}", root.to_string_lossy()),
        root: root.to_path_buf(),
        workspace_type: WorkspaceType::Npm,
        members: package_ids,
        manifest_path: manifest_path
            .strip_prefix(root)
            .unwrap_or(manifest_path)
            .to_string_lossy()
            .to_string(),
    })
}

/// Parse a Cargo.toml into a Package.
pub fn parse_cargo_package(root: &Path, manifest_path: &Path) -> Result<Package> {
    let content = std::fs::read_to_string(manifest_path).context("Failed to read Cargo.toml")?;
    let doc: toml::Value = toml::from_str(&content).context("Failed to parse Cargo.toml")?;

    let package_section = doc.get("package");

    let name = package_section
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("unknown")
        .to_string();

    let version = package_section
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0")
        .to_string();

    // Parse dependencies
    let mut dependencies = Vec::new();
    let mut dev_dependencies = Vec::new();

    if let Some(deps) = doc.get("dependencies").and_then(|d| d.as_table()) {
        for (dep_name, dep_spec) in deps {
            let (version_constraint, features, optional) = parse_cargo_dep(dep_spec);
            dependencies.push(Dependency {
                from: name.clone(),
                to: DependencyTarget::External {
                    name: dep_name.clone(),
                    registry: "crates.io".to_string(),
                },
                kind: DependencyKind::Normal,
                version_constraint,
                features,
                optional,
            });
        }
    }

    if let Some(deps) = doc.get("dev-dependencies").and_then(|d| d.as_table()) {
        for (dep_name, dep_spec) in deps {
            let (version_constraint, features, optional) = parse_cargo_dep(dep_spec);
            dev_dependencies.push(Dependency {
                from: name.clone(),
                to: DependencyTarget::External {
                    name: dep_name.clone(),
                    registry: "crates.io".to_string(),
                },
                kind: DependencyKind::Dev,
                version_constraint,
                features,
                optional,
            });
        }
    }

    // Detect build/test commands
    let mut build_commands = vec!["cargo build".to_string()];
    let mut test_commands = vec!["cargo test".to_string()];

    // Check for bench targets
    if doc.get("bench").is_some() {
        test_commands.push("cargo bench".to_string());
    }

    // Check for bin targets
    if let Some(bins) = doc.get("bin").and_then(|b| b.as_array()) {
        for bin in bins {
            if let Some(name) = bin.get("name").and_then(|n| n.as_str()) {
                build_commands.push(format!("cargo build --bin {}", name));
            }
        }
    }

    // Store the package root relative to the repository root, matching the
    // semantics of Package.manifest and FileNode.id. Absolute roots here
    // broke string prefix joins on Windows (an absolute backslash path can
    // never match a relative forward-slash file id) and embedded the
    // checkout location into model output, defeating determinism.
    let pkg_root_abs = manifest_path.parent().unwrap_or(root).to_path_buf();
    let pkg_root = pkg_root_abs
        .strip_prefix(root)
        .unwrap_or(&pkg_root_abs)
        .to_path_buf();

    Ok(Package {
        id: name.clone(),
        name,
        version,
        root: pkg_root,
        language: crate::intelligence::types::Language::Rust,
        manifest: manifest_path
            .strip_prefix(root)
            .unwrap_or(manifest_path)
            .to_string_lossy()
            .to_string(),
        files: Vec::new(), // Filled by scanner
        dependencies,
        dev_dependencies,
        build_commands,
        test_commands,
        entry_points: Vec::new(), // Filled by entry-point discovery
    })
}

/// Parse a package.json into a Package.
pub fn parse_npm_package(root: &Path, manifest_path: &Path) -> Result<Package> {
    let content = std::fs::read_to_string(manifest_path).context("Failed to read package.json")?;
    let doc: JsonValue = serde_json::from_str(&content).context("Failed to parse package.json")?;

    let name = doc
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("unknown")
        .to_string();

    let version = doc
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.0.0")
        .to_string();

    // Parse dependencies
    let mut dependencies = Vec::new();
    let mut dev_dependencies = Vec::new();

    if let Some(deps) = doc.get("dependencies").and_then(|d| d.as_object()) {
        for (dep_name, dep_version) in deps {
            dependencies.push(Dependency {
                from: name.clone(),
                to: DependencyTarget::External {
                    name: dep_name.clone(),
                    registry: "npm".to_string(),
                },
                kind: DependencyKind::Normal,
                version_constraint: dep_version.as_str().unwrap_or("*").to_string(),
                features: Vec::new(),
                optional: false,
            });
        }
    }

    if let Some(deps) = doc.get("devDependencies").and_then(|d| d.as_object()) {
        for (dep_name, dep_version) in deps {
            dev_dependencies.push(Dependency {
                from: name.clone(),
                to: DependencyTarget::External {
                    name: dep_name.clone(),
                    registry: "npm".to_string(),
                },
                kind: DependencyKind::Dev,
                version_constraint: dep_version.as_str().unwrap_or("*").to_string(),
                features: Vec::new(),
                optional: false,
            });
        }
    }

    // Parse scripts
    let mut build_commands = Vec::new();
    let mut test_commands = Vec::new();

    if let Some(scripts) = doc.get("scripts").and_then(|s| s.as_object()) {
        if let Some(build) = scripts.get("build").and_then(|v| v.as_str()) {
            build_commands.push(format!("pnpm {}", build));
        }
        if let Some(test) = scripts.get("test").and_then(|v| v.as_str()) {
            test_commands.push(format!("pnpm {}", test));
        }
        if let Some(lint) = scripts.get("lint").and_then(|v| v.as_str()) {
            test_commands.push(format!("pnpm {}", lint));
        }
    }

    // See parse_cargo_package: package roots are repo-relative.
    let pkg_root_abs = manifest_path.parent().unwrap_or(root).to_path_buf();
    let pkg_root = pkg_root_abs
        .strip_prefix(root)
        .unwrap_or(&pkg_root_abs)
        .to_path_buf();

    Ok(Package {
        id: name.clone(),
        name,
        version,
        root: pkg_root,
        language: crate::intelligence::types::Language::TypeScript,
        manifest: manifest_path
            .strip_prefix(root)
            .unwrap_or(manifest_path)
            .to_string_lossy()
            .to_string(),
        files: Vec::new(),
        dependencies,
        dev_dependencies,
        build_commands,
        test_commands,
        entry_points: Vec::new(),
    })
}

/// Parse a Cargo dependency specification.
fn parse_cargo_dep(spec: &toml::Value) -> (String, Vec<String>, bool) {
    match spec {
        toml::Value::String(v) => (v.clone(), Vec::new(), false),
        toml::Value::Table(t) => {
            let version = t
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("*")
                .to_string();
            let features = t
                .get("features")
                .and_then(|f| f.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let optional = t.get("optional").and_then(|o| o.as_bool()).unwrap_or(false);
            (version, features, optional)
        }
        _ => ("*".to_string(), Vec::new(), false),
    }
}

/// Discover all packages in a repository.
pub fn discover_packages(root: &Path) -> Result<Vec<Package>> {
    let mut packages = Vec::new();

    // Find all Cargo.toml files (excluding target/)
    for entry in walkdir::WalkDir::new(root)
        .max_depth(5)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.') && name != "target" && name != "node_modules"
        })
        .filter_map(|e| e.ok())
    {
        if entry.file_name() == "Cargo.toml" {
            let manifest_path = entry.path();
            // Skip workspace root (it has [workspace] but not [package])
            let content = std::fs::read_to_string(manifest_path).unwrap_or_default();
            if content.contains("[package]") {
                if let Ok(pkg) = parse_cargo_package(root, manifest_path) {
                    packages.push(pkg);
                }
            }
        }
    }

    // Find all package.json files
    for entry in walkdir::WalkDir::new(root)
        .max_depth(5)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.') && name != "target" && name != "node_modules"
        })
        .filter_map(|e| e.ok())
    {
        if entry.file_name() == "package.json" {
            let manifest_path = entry.path();
            if let Ok(pkg) = parse_npm_package(root, manifest_path) {
                // Avoid duplicate manifests (e.g., workspace root parsed twice).
                // Dedup key is the manifest path, NOT the package name: a
                // Tauri app legitimately has a cargo package and an npm package
                // sharing one name, and dropping the npm package hides the
                // frontend entry points.
                let manifest_key = pkg.manifest.replace('\\', "/");
                if !packages
                    .iter()
                    .any(|p| p.manifest.replace('\\', "/") == manifest_key)
                {
                    packages.push(pkg);
                }
            }
        }
    }

    Ok(packages)
}

/// Resolve internal dependencies: for each package, check if a dependency
/// target matches another package in the same repository.
pub fn resolve_internal_deps(packages: &mut [Package]) {
    let package_names: Vec<String> = packages.iter().map(|p| p.name.clone()).collect();

    for pkg in packages.iter_mut() {
        for dep in pkg.dependencies.iter_mut() {
            if let DependencyTarget::External { name, .. } = &dep.to {
                if package_names.contains(name) {
                    dep.to = DependencyTarget::Internal(name.clone());
                }
            }
        }
        for dep in pkg.dev_dependencies.iter_mut() {
            if let DependencyTarget::External { name, .. } = &dep.to {
                if package_names.contains(name) {
                    dep.to = DependencyTarget::Internal(name.clone());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn parse_cargo_workspace_manifest() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("crates/test-crate")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            r#"
[workspace]
members = ["crates/test-crate"]

[workspace.package]
version = "0.2.0"
"#,
        )
        .unwrap();
        fs::write(
            root.join("crates/test-crate/Cargo.toml"),
            r#"
[package]
name = "test-crate"
version = "0.1.0"

[dependencies]
serde = "1.0"
"#,
        )
        .unwrap();

        let ws = parse_cargo_workspace(root, &root.join("Cargo.toml")).unwrap();
        assert_eq!(ws.workspace_type, WorkspaceType::Cargo);
        assert!(ws.members.contains(&"test-crate".to_string()));
    }

    #[test]
    fn parse_cargo_package_deps() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        fs::write(
            root.join("Cargo.toml"),
            r#"
[package]
name = "my-crate"
version = "0.1.0"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.38", features = ["full"], optional = true }

[dev-dependencies]
tempfile = "3"
"#,
        )
        .unwrap();

        let pkg = parse_cargo_package(root, &root.join("Cargo.toml")).unwrap();
        assert_eq!(pkg.name, "my-crate");
        assert_eq!(pkg.dependencies.len(), 2);
        assert_eq!(pkg.dev_dependencies.len(), 1);

        let serde_dep = pkg
            .dependencies
            .iter()
            .find(|d| match &d.to {
                DependencyTarget::External { name, .. } => name == "serde",
                _ => false,
            })
            .unwrap();
        assert_eq!(serde_dep.version_constraint, "1.0");
        assert!(serde_dep.features.contains(&"derive".to_string()));
    }

    #[test]
    fn parse_package_json() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        fs::write(
            root.join("package.json"),
            r#"{
            "name": "my-app",
            "version": "1.0.0",
            "dependencies": {
                "react": "^18.0.0"
            },
            "devDependencies": {
                "typescript": "^5.0.0"
            },
            "scripts": {
                "build": "vite build",
                "test": "vitest"
            }
        }"#,
        )
        .unwrap();

        let pkg = parse_npm_package(root, &root.join("package.json")).unwrap();
        assert_eq!(pkg.name, "my-app");
        assert_eq!(pkg.dependencies.len(), 1);
        assert_eq!(pkg.dev_dependencies.len(), 1);
        assert!(!pkg.build_commands.is_empty());
        assert!(!pkg.test_commands.is_empty());
    }

    #[test]
    fn resolve_internal_dependencies() {
        let mut packages = vec![
            Package {
                id: "core".to_string(),
                name: "core".to_string(),
                version: "0.1.0".to_string(),
                root: PathBuf::from("/test/core"),
                language: crate::intelligence::types::Language::Rust,
                manifest: "Cargo.toml".to_string(),
                files: Vec::new(),
                dependencies: vec![Dependency {
                    from: "core".to_string(),
                    to: DependencyTarget::External {
                        name: "mcp".to_string(),
                        registry: "crates.io".to_string(),
                    },
                    kind: DependencyKind::Normal,
                    version_constraint: "0.1.0".to_string(),
                    features: Vec::new(),
                    optional: false,
                }],
                dev_dependencies: Vec::new(),
                build_commands: Vec::new(),
                test_commands: Vec::new(),
                entry_points: Vec::new(),
            },
            Package {
                id: "mcp".to_string(),
                name: "mcp".to_string(),
                version: "0.1.0".to_string(),
                root: PathBuf::from("/test/mcp"),
                language: crate::intelligence::types::Language::Rust,
                manifest: "Cargo.toml".to_string(),
                files: Vec::new(),
                dependencies: Vec::new(),
                dev_dependencies: Vec::new(),
                build_commands: Vec::new(),
                test_commands: Vec::new(),
                entry_points: Vec::new(),
            },
        ];

        resolve_internal_deps(&mut packages);

        // core's dep on mcp should now be internal
        match &packages[0].dependencies[0].to {
            DependencyTarget::Internal(name) => assert_eq!(name, "mcp"),
            _ => panic!("Expected internal dependency"),
        }
    }
}
