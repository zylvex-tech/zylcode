use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Extension type discriminator
// ---------------------------------------------------------------------------

/// The category of a marketplace extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionType {
    Skill,
    Plugin,
    McpAdapter,
    Theme,
}

impl std::fmt::Display for ExtensionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Skill => write!(f, "skill"),
            Self::Plugin => write!(f, "plugin"),
            Self::McpAdapter => write!(f, "mcp_adapter"),
            Self::Theme => write!(f, "theme"),
        }
    }
}

// ---------------------------------------------------------------------------
// Plugin manifest — describes a publishable / installable extension
// ---------------------------------------------------------------------------

/// Manifest that ships alongside every marketplace extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Unique extension identifier, e.g. `zylcode/prettier-formatter`.
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Semantic version string.
    pub version: String,
    /// Short description shown in search results.
    pub description: String,
    /// Author or publisher identifier.
    pub author: String,
    /// License identifier (SPDX).
    #[serde(default = "default_license")]
    pub license: String,
    /// Homepage or documentation URL.
    #[serde(default)]
    pub homepage: Option<String>,
    /// Repository URL.
    #[serde(default)]
    pub repository: Option<String>,
    /// Extension category.
    pub extension_type: ExtensionType,
    /// Capability keywords for search indexing.
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Minimum compatible ZylCode engine version.
    #[serde(default)]
    pub engine_version: Option<String>,
    /// Runtime entry point (relative path or command).
    #[serde(default)]
    pub entry_point: Option<String>,
    /// Arbitrary typed configuration schema.
    #[serde(default)]
    pub config_schema: Option<serde_json::Value>,
}

fn default_license() -> String {
    "MIT".to_string()
}

// ---------------------------------------------------------------------------
// Skill definition — a discrete capability contributed by an extension
// ---------------------------------------------------------------------------

/// A single skill capability surfaced by an extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    /// Stable skill identifier within its parent extension.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Markdown description of what the skill does.
    pub description: String,
    /// Input JSON schema.
    #[serde(default)]
    pub input_schema: Option<serde_json::Value>,
    /// Output JSON schema.
    #[serde(default)]
    pub output_schema: Option<serde_json::Value>,
    /// Tags for discovery.
    #[serde(default)]
    pub tags: Vec<String>,
}

// ---------------------------------------------------------------------------
// Marketplace extension — top-level registry entry
// ---------------------------------------------------------------------------

/// A fully-resolved marketplace extension combining manifest + skills.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceExtension {
    pub manifest: PluginManifest,
    /// Skills contributed by this extension (may be empty for themes).
    #[serde(default)]
    pub skills: Vec<SkillDefinition>,
    /// Whether this extension is currently installed locally.
    #[serde(default)]
    pub installed: bool,
    /// Local filesystem path when installed.
    #[serde(default)]
    pub install_path: Option<String>,
    /// Download / install statistics.
    #[serde(default)]
    pub download_count: u64,
    /// Average rating (0.0 – 5.0).
    #[serde(default)]
    pub rating: Option<f32>,
}

// ---------------------------------------------------------------------------
// Extension registry — in-memory catalogue
// ---------------------------------------------------------------------------

/// In-memory registry of all known marketplace extensions.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ExtensionRegistry {
    extensions: HashMap<String, MarketplaceExtension>,
}

impl ExtensionRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register or replace an extension.
    pub fn register(&mut self, extension: MarketplaceExtension) {
        self.extensions
            .insert(extension.manifest.id.clone(), extension);
    }

    /// Remove an extension by id. Returns `true` if it existed.
    pub fn unregister(&mut self, id: &str) -> bool {
        self.extensions.remove(id).is_some()
    }

    /// Look up a single extension by id.
    pub fn get(&self, id: &str) -> Option<&MarketplaceExtension> {
        self.extensions.get(id)
    }

    /// Mutable access to a single extension.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut MarketplaceExtension> {
        self.extensions.get_mut(id)
    }

    /// All extensions as a slice.
    pub fn all(&self) -> Vec<&MarketplaceExtension> {
        self.extensions.values().collect()
    }

    /// Extensions filtered by type.
    pub fn by_type(&self, extension_type: &ExtensionType) -> Vec<&MarketplaceExtension> {
        self.extensions
            .values()
            .filter(|e| &e.manifest.extension_type == extension_type)
            .collect()
    }

    /// Case-insensitive substring search across id, name, description, and keywords.
    pub fn search(&self, query: &str) -> Vec<&MarketplaceExtension> {
        let q = query.to_lowercase();
        self.extensions
            .values()
            .filter(|e| {
                e.manifest.id.to_lowercase().contains(&q)
                    || e.manifest.name.to_lowercase().contains(&q)
                    || e.manifest.description.to_lowercase().contains(&q)
                    || e.manifest.keywords.iter().any(|k| k.to_lowercase().contains(&q))
                    || e.skills.iter().any(|s| {
                        s.name.to_lowercase().contains(&q) || s.description.to_lowercase().contains(&q)
                    })
            })
            .collect()
    }

    /// Number of registered extensions.
    pub fn len(&self) -> usize {
        self.extensions.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.extensions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest(id: &str, ext_type: ExtensionType) -> PluginManifest {
        PluginManifest {
            id: id.to_string(),
            name: id.to_string(),
            version: "0.1.0".to_string(),
            description: format!("Test extension {id}"),
            author: "zylcode".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            extension_type: ext_type,
            keywords: vec!["test".to_string()],
            engine_version: None,
            entry_point: None,
            config_schema: None,
        }
    }

    #[test]
    fn registry_search_finds_by_keyword() {
        let mut reg = ExtensionRegistry::new();
        reg.register(MarketplaceExtension {
            manifest: sample_manifest("zylcode/formatter", ExtensionType::Plugin),
            skills: vec![],
            installed: false,
            install_path: None,
            download_count: 0,
            rating: None,
        });
        assert_eq!(reg.search("formatter").len(), 1);
        assert_eq!(reg.search("nonexistent").len(), 0);
    }

    #[test]
    fn registry_by_type_filters_correctly() {
        let mut reg = ExtensionRegistry::new();
        reg.register(MarketplaceExtension {
            manifest: sample_manifest("ext-skill", ExtensionType::Skill),
            skills: vec![],
            installed: false,
            install_path: None,
            download_count: 0,
            rating: None,
        });
        reg.register(MarketplaceExtension {
            manifest: sample_manifest("ext-theme", ExtensionType::Theme),
            skills: vec![],
            installed: false,
            install_path: None,
            download_count: 0,
            rating: None,
        });
        assert_eq!(reg.by_type(&ExtensionType::Skill).len(), 1);
        assert_eq!(reg.by_type(&ExtensionType::Theme).len(), 1);
        assert_eq!(reg.by_type(&ExtensionType::Plugin).len(), 0);
    }
}
