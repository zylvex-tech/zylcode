//! Dependency graph — multiple layers.
//!
//! A. Package dependency graph (crate → crate)
//! B. File/module dependency graph (file → file via imports)
//! C. External dependency graph (crate → external lib)
//!
//! Supports both directions: dependencies_of(X) and dependents_of(X).

use crate::intelligence::types::{DependencyTarget, Package};
use std::collections::{HashMap, HashSet, VecDeque};

/// A dependency graph with bidirectional queries.
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// Adjacency list: package_id → set of package_ids it depends on.
    forward: HashMap<String, HashSet<String>>,
    /// Reverse adjacency: package_id → set of package_ids that depend on it.
    reverse: HashMap<String, HashSet<String>>,
    /// File-level imports: file_id → set of file_ids it imports from.
    file_forward: HashMap<String, HashSet<String>>,
    /// File-level reverse imports.
    file_reverse: HashMap<String, HashSet<String>>,
    /// External dependencies: package_id → set of external dependency names.
    external: HashMap<String, HashSet<String>>,
    /// All known package IDs.
    packages: HashSet<String>,
}

impl DependencyGraph {
    /// Build a dependency graph from packages.
    pub fn from_packages(packages: &[Package]) -> Self {
        let mut graph = Self {
            forward: HashMap::new(),
            reverse: HashMap::new(),
            file_forward: HashMap::new(),
            file_reverse: HashMap::new(),
            external: HashMap::new(),
            packages: HashSet::new(),
        };

        for pkg in packages {
            graph.packages.insert(pkg.id.clone());
            graph.forward.entry(pkg.id.clone()).or_default();
            graph.reverse.entry(pkg.id.clone()).or_default();
            graph.external.entry(pkg.id.clone()).or_default();

            for dep in &pkg.dependencies {
                match &dep.to {
                    DependencyTarget::Internal(target_id) => {
                        graph
                            .forward
                            .entry(pkg.id.clone())
                            .or_default()
                            .insert(target_id.clone());
                        graph
                            .reverse
                            .entry(target_id.clone())
                            .or_default()
                            .insert(pkg.id.clone());
                    }
                    DependencyTarget::External { name, .. } => {
                        graph
                            .external
                            .entry(pkg.id.clone())
                            .or_default()
                            .insert(name.clone());
                    }
                }
            }

            for dep in &pkg.dev_dependencies {
                if let DependencyTarget::External { name, .. } = &dep.to {
                    graph
                        .external
                        .entry(format!("{}:dev", pkg.id))
                        .or_default()
                        .insert(name.clone());
                }
            }
        }

        graph
    }

    /// Add a file-level import edge.
    pub fn add_file_import(&mut self, from_file: &str, to_file: &str) {
        self.file_forward
            .entry(from_file.to_string())
            .or_default()
            .insert(to_file.to_string());
        self.file_reverse
            .entry(to_file.to_string())
            .or_default()
            .insert(from_file.to_string());
    }

    /// Get direct dependencies of a package.
    pub fn dependencies_of(&self, package_id: &str) -> Vec<String> {
        self.forward
            .get(package_id)
            .map(|deps| deps.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get direct dependents of a package (what depends on it).
    pub fn dependents_of(&self, package_id: &str) -> Vec<String> {
        self.reverse
            .get(package_id)
            .map(|deps| deps.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get transitive dependencies of a package (BFS).
    pub fn transitive_dependencies_of(&self, package_id: &str) -> Vec<String> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(package_id.to_string());
        visited.insert(package_id.to_string());

        while let Some(current) = queue.pop_front() {
            if let Some(deps) = self.forward.get(&current) {
                for dep in deps {
                    if visited.insert(dep.clone()) {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }

        visited.into_iter().collect()
    }

    /// Get transitive dependents of a package (reverse BFS).
    pub fn transitive_dependents_of(&self, package_id: &str) -> Vec<String> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(package_id.to_string());
        visited.insert(package_id.to_string());

        while let Some(current) = queue.pop_front() {
            if let Some(deps) = self.reverse.get(&current) {
                for dep in deps {
                    if visited.insert(dep.clone()) {
                        queue.push_back(dep.clone());
                    }
                }
            }
        }

        visited.into_iter().collect()
    }

    /// Get external dependencies of a package.
    pub fn external_dependencies_of(&self, package_id: &str) -> Vec<String> {
        self.external
            .get(package_id)
            .map(|deps| deps.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get files that import from a given file.
    pub fn files_imported_by(&self, file_id: &str) -> Vec<String> {
        self.file_forward
            .get(file_id)
            .map(|deps| deps.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get files that import a given file.
    pub fn files_that_import(&self, file_id: &str) -> Vec<String> {
        self.file_reverse
            .get(file_id)
            .map(|deps| deps.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get all known package IDs.
    pub fn all_packages(&self) -> Vec<String> {
        self.packages.iter().cloned().collect()
    }

    /// Get the number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.forward.values().map(|deps| deps.len()).sum()
    }

    /// Check if a package exists in the graph.
    pub fn has_package(&self, package_id: &str) -> bool {
        self.packages.contains(package_id)
    }

    /// Find packages that use a specific external dependency.
    pub fn packages_using(&self, external_dep: &str) -> Vec<String> {
        let dep_lower = external_dep.to_lowercase();
        self.external
            .iter()
            .filter(|(_, deps)| deps.iter().any(|d| d.to_lowercase().contains(&dep_lower)))
            .map(|(pkg, _)| pkg.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intelligence::types::{Dependency, DependencyKind, Language};
    use std::path::PathBuf;

    fn make_package(name: &str, deps: Vec<&str>) -> Package {
        let dependencies = deps
            .into_iter()
            .map(|d| Dependency {
                from: name.to_string(),
                to: DependencyTarget::Internal(d.to_string()),
                kind: DependencyKind::Normal,
                version_constraint: "0.1.0".to_string(),
                features: Vec::new(),
                optional: false,
            })
            .collect();

        Package {
            id: name.to_string(),
            name: name.to_string(),
            version: "0.1.0".to_string(),
            root: PathBuf::from(format!("/test/{}", name)),
            language: Language::Rust,
            manifest: "Cargo.toml".to_string(),
            files: Vec::new(),
            dependencies,
            dev_dependencies: Vec::new(),
            build_commands: Vec::new(),
            test_commands: Vec::new(),
            entry_points: Vec::new(),
        }
    }

    #[test]
    fn package_dependency_graph() {
        let packages = vec![
            make_package("desktop", vec!["core"]),
            make_package("core", vec!["mcp"]),
            make_package("mcp", vec![]),
        ];

        let graph = DependencyGraph::from_packages(&packages);

        assert_eq!(graph.dependencies_of("desktop"), vec!["core"]);
        assert_eq!(graph.dependencies_of("core"), vec!["mcp"]);
        assert!(graph.dependencies_of("mcp").is_empty());
    }

    #[test]
    fn reverse_dependency_graph() {
        let packages = vec![
            make_package("desktop", vec!["core"]),
            make_package("core", vec!["mcp"]),
            make_package("mcp", vec![]),
        ];

        let graph = DependencyGraph::from_packages(&packages);

        assert_eq!(graph.dependents_of("mcp"), vec!["core"]);
        assert_eq!(graph.dependents_of("core"), vec!["desktop"]);
        assert!(graph.dependents_of("desktop").is_empty());
    }

    #[test]
    fn transitive_dependencies() {
        let packages = vec![
            make_package("desktop", vec!["core"]),
            make_package("core", vec!["mcp"]),
            make_package("mcp", vec![]),
        ];

        let graph = DependencyGraph::from_packages(&packages);

        let mut transitive = graph.transitive_dependencies_of("desktop");
        transitive.sort();
        assert_eq!(transitive, vec!["core", "desktop", "mcp"]);
    }

    #[test]
    fn transitive_dependents() {
        let packages = vec![
            make_package("desktop", vec!["core"]),
            make_package("core", vec!["mcp"]),
            make_package("mcp", vec![]),
        ];

        let graph = DependencyGraph::from_packages(&packages);

        let mut dependents = graph.transitive_dependents_of("mcp");
        dependents.sort();
        assert_eq!(dependents, vec!["core", "desktop", "mcp"]);
    }

    #[test]
    fn packages_using_external_dep() {
        let mut pkg1 = make_package("core", vec![]);
        pkg1.dependencies.push(Dependency {
            from: "core".to_string(),
            to: DependencyTarget::External {
                name: "tokio".to_string(),
                registry: "crates.io".to_string(),
            },
            kind: DependencyKind::Normal,
            version_constraint: "1.38".to_string(),
            features: Vec::new(),
            optional: false,
        });

        let mut pkg2 = make_package("mcp", vec![]);
        pkg2.dependencies.push(Dependency {
            from: "mcp".to_string(),
            to: DependencyTarget::External {
                name: "tokio".to_string(),
                registry: "crates.io".to_string(),
            },
            kind: DependencyKind::Normal,
            version_constraint: "1.38".to_string(),
            features: Vec::new(),
            optional: false,
        });

        let graph = DependencyGraph::from_packages(&[pkg1, pkg2]);

        let mut users = graph.packages_using("tokio");
        users.sort();
        assert_eq!(users, vec!["core", "mcp"]);
    }
}
