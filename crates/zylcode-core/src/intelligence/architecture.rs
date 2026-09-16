//! Architectural fingerprint generation.
//!
//! Detects frameworks, languages, tools, and architectural patterns
//! based on evidence in the repository.

use crate::intelligence::types::Package;
use crate::intelligence::types::{ArchitecturalFact, ArchitecturalFingerprint, Provenance};
use anyhow::Result;
use std::path::Path;

/// Generate an architectural fingerprint for a repository.
pub fn generate_fingerprint(root: &Path, packages: &[Package]) -> Result<ArchitecturalFingerprint> {
    let mut facts = Vec::new();

    // Detect Rust workspace
    if root.join("Cargo.toml").exists() {
        let content = std::fs::read_to_string(root.join("Cargo.toml")).unwrap_or_default();
        if content.contains("[workspace]") {
            facts.push(ArchitecturalFact {
                fact: "workspace".to_string(),
                value: "Rust workspace".to_string(),
                evidence: vec!["Cargo.toml: [workspace] section".to_string()],
                provenance: Provenance::Parsed,
            });
        }
    }

    // Detect Tauri
    if root
        .join("apps/zylcode-desktop/src-tauri/tauri.conf.json")
        .exists()
    {
        let conf =
            std::fs::read_to_string(root.join("apps/zylcode-desktop/src-tauri/tauri.conf.json"))
                .unwrap_or_default();
        let version = if conf.contains("\"v2\"") || conf.contains("schema.tauri.app/config/2") {
            "v2"
        } else {
            "v1"
        };
        facts.push(ArchitecturalFact {
            fact: "framework".to_string(),
            value: format!("Tauri {}", version),
            evidence: vec!["apps/zylcode-desktop/src-tauri/tauri.conf.json".to_string()],
            provenance: Provenance::Parsed,
        });
    }

    // Detect React frontend
    let apps_dir = root.join("apps");
    if apps_dir.exists() {
        for entry in std::fs::read_dir(&apps_dir).unwrap() {
            let entry = entry.unwrap();
            let pkg_json = entry.path().join("package.json");
            if pkg_json.exists() {
                let content = std::fs::read_to_string(&pkg_json).unwrap_or_default();
                if content.contains("\"react\"") {
                    facts.push(ArchitecturalFact {
                        fact: "frontend".to_string(),
                        value: "React".to_string(),
                        evidence: vec![format!(
                            "{}/package.json: react dependency",
                            entry.file_name().to_string_lossy()
                        )],
                        provenance: Provenance::Parsed,
                    });
                }
                if content.contains("\"typescript\"") {
                    facts.push(ArchitecturalFact {
                        fact: "language".to_string(),
                        value: "TypeScript".to_string(),
                        evidence: vec![format!(
                            "{}/package.json: typescript dependency",
                            entry.file_name().to_string_lossy()
                        )],
                        provenance: Provenance::Parsed,
                    });
                }
            }
        }
    }

    // Detect pnpm workspace
    if root.join("pnpm-workspace.yaml").exists() {
        facts.push(ArchitecturalFact {
            fact: "package_manager".to_string(),
            value: "pnpm".to_string(),
            evidence: vec!["pnpm-workspace.yaml exists".to_string()],
            provenance: Provenance::Observed,
        });
    }

    // Detect SQLite usage
    for pkg in packages {
        if pkg.dependencies.iter().any(|d| {
            matches!(&d.to, crate::intelligence::types::DependencyTarget::External { name, .. } if name == "rusqlite")
        }) {
            facts.push(ArchitecturalFact {
                fact: "database".to_string(),
                value: "SQLite".to_string(),
                evidence: vec![
                    format!("{}: rusqlite dependency", pkg.name),
                ],
                provenance: Provenance::Parsed,
            });
            break;
        }
    }

    // Detect Tokio async runtime
    for pkg in packages {
        if pkg.dependencies.iter().any(|d| {
            matches!(&d.to, crate::intelligence::types::DependencyTarget::External { name, .. } if name == "tokio")
        }) {
            facts.push(ArchitecturalFact {
                fact: "async_runtime".to_string(),
                value: "Tokio".to_string(),
                evidence: vec![
                    format!("{}: tokio dependency", pkg.name),
                ],
                provenance: Provenance::Parsed,
            });
            break;
        }
    }

    // Detect MCP subsystem
    if packages.iter().any(|p| p.name.contains("mcp")) {
        facts.push(ArchitecturalFact {
            fact: "protocol".to_string(),
            value: "MCP (Model Context Protocol)".to_string(),
            evidence: vec!["zylcode-mcp crate exists".to_string()],
            provenance: Provenance::Observed,
        });
    }

    // Detect agent/core boundaries
    let has_core = packages.iter().any(|p| p.name.contains("core"));
    let has_mcp = packages.iter().any(|p| p.name.contains("mcp"));
    let has_desktop = packages.iter().any(|p| p.name.contains("desktop"));

    if has_core && has_mcp && has_desktop {
        facts.push(ArchitecturalFact {
            fact: "architecture".to_string(),
            value: "layered: desktop → core → mcp".to_string(),
            evidence: vec![
                "zylcode-desktop crate exists".to_string(),
                "zylcode-core crate exists".to_string(),
                "zylcode-mcp crate exists".to_string(),
            ],
            provenance: Provenance::Derived,
        });
    }

    // Detect Vite
    if root.join("apps/zylcode-desktop/vite.config.ts").exists() {
        facts.push(ArchitecturalFact {
            fact: "bundler".to_string(),
            value: "Vite".to_string(),
            evidence: vec!["apps/zylcode-desktop/vite.config.ts exists".to_string()],
            provenance: Provenance::Observed,
        });
    }

    // Detect Tailwind CSS
    let tailwind_config = root.join("apps/zylcode-desktop/tailwind.config.js");
    let tailwind_config_ts = root.join("apps/zylcode-desktop/tailwind.config.ts");
    if tailwind_config.exists() || tailwind_config_ts.exists() {
        facts.push(ArchitecturalFact {
            fact: "css_framework".to_string(),
            value: "Tailwind CSS".to_string(),
            evidence: vec!["tailwind.config.{js,ts} exists".to_string()],
            provenance: Provenance::Observed,
        });
    }

    Ok(ArchitecturalFingerprint { facts })
}

/// Get a specific architectural fact by key.
pub fn get_fact<'a>(
    fingerprint: &'a ArchitecturalFingerprint,
    key: &str,
) -> Option<&'a ArchitecturalFact> {
    fingerprint.facts.iter().find(|f| f.fact == key)
}

/// Get all facts of a specific type.
pub fn get_facts_by_type<'a>(
    fingerprint: &'a ArchitecturalFingerprint,
    fact_type: &str,
) -> Vec<&'a ArchitecturalFact> {
    fingerprint
        .facts
        .iter()
        .filter(|f| f.fact == fact_type)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn detect_rust_workspace() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/test\"]\n",
        )
        .unwrap();

        let fingerprint = generate_fingerprint(root, &[]).unwrap();
        let ws = get_fact(&fingerprint, "workspace");
        assert!(ws.is_some());
        assert_eq!(ws.unwrap().value, "Rust workspace");
    }

    #[test]
    fn detect_tokio() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        let pkg = Package {
            id: "test".to_string(),
            name: "test".to_string(),
            version: "0.1.0".to_string(),
            root: root.to_path_buf(),
            language: crate::intelligence::types::Language::Rust,
            manifest: "Cargo.toml".to_string(),
            files: Vec::new(),
            dependencies: vec![crate::intelligence::types::Dependency {
                from: "test".to_string(),
                to: crate::intelligence::types::DependencyTarget::External {
                    name: "tokio".to_string(),
                    registry: "crates.io".to_string(),
                },
                kind: crate::intelligence::types::DependencyKind::Normal,
                version_constraint: "1.38".to_string(),
                features: Vec::new(),
                optional: false,
            }],
            dev_dependencies: Vec::new(),
            build_commands: Vec::new(),
            test_commands: Vec::new(),
            entry_points: Vec::new(),
        };

        let fingerprint = generate_fingerprint(root, &[pkg]).unwrap();
        let runtime = get_fact(&fingerprint, "async_runtime");
        assert!(runtime.is_some());
        assert_eq!(runtime.unwrap().value, "Tokio");
    }
}
