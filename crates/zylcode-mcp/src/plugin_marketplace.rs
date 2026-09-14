use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::builtin_plugins::get_preshipped_plugins;

/// Plugin Marketplace System
pub struct PluginMarketplace {
    plugins: RwLock<HashMap<String, Arc<dyn Plugin>>>,
    installed_plugins: RwLock<HashMap<String, InstalledPlugin>>,
    plugin_categories: RwLock<HashMap<String, Vec<String>>>,
    marketplace_stats: RwLock<MarketplaceStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDefinition {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub category: String,
    pub tags: Vec<String>,
    pub pricing: PluginPricing,
    pub dependencies: Vec<PluginDependency>,
    pub config_schema: Value,
    pub execution: PluginExecution,
    pub permissions: Vec<PluginPermission>,
    pub marketplace: PluginMarketplaceInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPricing {
    pub pricing_type: String, // "free", "freemium", "paid", "subscription"
    pub price: f64,
    pub currency: String,
    pub period: Option<String>, // "monthly", "yearly"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub name: String,
    pub version: String,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginExecution {
    pub runtime: String,
    pub entry_point: String,
    pub background: bool,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPermission {
    pub resource: String,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginMarketplaceInfo {
    pub screenshots: Vec<String>,
    pub documentation: String,
    pub changelog: String,
    pub support_url: String,
    pub source_code: String,
    pub downloads: u64,
    pub rating: f64,
    pub reviews: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    pub plugin_id: String,
    pub version: String,
    pub installed_at: chrono::DateTime<chrono::Utc>,
    pub enabled: bool,
    pub config: Value,
}

#[derive(Debug, Default)]
struct MarketplaceStats {
    total_plugins: u64,
    total_installations: u64,
    total_revenue: f64,
    active_users: u64,
}

#[async_trait]
pub trait Plugin: Send + Sync + std::fmt::Debug {
    fn id(&self) -> &str;
    fn definition(&self) -> PluginDefinition;
    async fn activate(&self) -> Result<()>;
    async fn deactivate(&self) -> Result<()>;
    async fn execute(&self, command: &str, params: Value) -> Result<Value>;
    async fn get_ui_components(&self) -> Vec<UIComponent>;
    async fn on_config_change(&self, config: Value) -> Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIComponent {
    pub name: String,
    pub component_type: String, // "settings", "dashboard", "widget"
    pub path: String,
    pub props: Value,
}

impl PluginMarketplace {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
            installed_plugins: RwLock::new(HashMap::new()),
            plugin_categories: RwLock::new(HashMap::new()),
            marketplace_stats: RwLock::new(MarketplaceStats::default()),
        }
    }

    /// Initialize with pre-shipped plugins
    pub async fn initialize_with_preshipped_plugins(&self) -> Result<usize> {
        let preshipped_plugins = get_preshipped_plugins();
        let mut total_plugins = 0;

        for plugin_def in preshipped_plugins {
            let plugin = PreShippedPlugin::new(plugin_def.clone());
            self.register_plugin(Arc::new(plugin)).await;
            total_plugins += 1;
        }

        tracing::info!("Initialized plugin marketplace with {} pre-shipped plugins", total_plugins);
        Ok(total_plugins)
    }

    /// Register a plugin
    pub async fn register_plugin(&self, plugin: Arc<dyn Plugin>) {
        let id = plugin.id().to_string();
        let category = plugin.definition().category.clone();
        
        // Add to plugins map
        self.plugins.write().await.insert(id.clone(), plugin);
        
        // Add to category
        let mut categories = self.plugin_categories.write().await;
        categories.entry(category).or_insert_with(Vec::new).push(id);
        
        // Update stats
        let mut stats = self.marketplace_stats.write().await;
        stats.total_plugins += 1;
    }

    /// Install a plugin
    pub async fn install_plugin(&self, plugin_id: &str, config: Value) -> Result<()> {
        let plugin = self.plugins.read().await.get(plugin_id).cloned()
            .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", plugin_id))?;

        // Activate plugin
        plugin.activate().await?;

        // Record installation
        let installed = InstalledPlugin {
            plugin_id: plugin_id.to_string(),
            version: plugin.definition().version.clone(),
            installed_at: chrono::Utc::now(),
            enabled: true,
            config,
        };

        self.installed_plugins.write().await.insert(plugin_id.to_string(), installed);

        // Update stats
        let mut stats = self.marketplace_stats.write().await;
        stats.total_installations += 1;

        tracing::info!("Installed plugin: {}", plugin_id);
        Ok(())
    }

    /// Uninstall a plugin
    pub async fn uninstall_plugin(&self, plugin_id: &str) -> Result<()> {
        let plugin = self.plugins.read().await.get(plugin_id).cloned()
            .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", plugin_id))?;

        // Deactivate plugin
        plugin.deactivate().await?;

        // Remove from installed
        self.installed_plugins.write().await.remove(plugin_id);

        tracing::info!("Uninstalled plugin: {}", plugin_id);
        Ok(())
    }

    /// Execute plugin command
    pub async fn execute_plugin_command(
        &self,
        plugin_id: &str,
        command: &str,
        params: Value,
    ) -> Result<Value> {
        let plugin = self.plugins.read().await.get(plugin_id).cloned()
            .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", plugin_id))?;

        // Check if plugin is installed and enabled
        let installed = self.installed_plugins.read().await;
        let installation = installed.get(plugin_id)
            .ok_or_else(|| anyhow::anyhow!("Plugin not installed: {}", plugin_id))?;

        if !installation.enabled {
            return Err(anyhow::anyhow!("Plugin is disabled: {}", plugin_id));
        }

        // Execute command
        plugin.execute(command, params).await
    }

    /// Get plugin UI components
    pub async fn get_plugin_ui_components(&self, plugin_id: &str) -> Result<Vec<UIComponent>> {
        let plugin = self.plugins.read().await.get(plugin_id).cloned()
            .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", plugin_id))?;

        Ok(plugin.get_ui_components().await)
    }

    /// List plugins by category
    pub async fn list_plugins_by_category(&self, category: &str) -> Vec<String> {
        self.plugin_categories.read().await
            .get(category)
            .cloned()
            .unwrap_or_default()
    }

    /// List all categories
    pub async fn list_categories(&self) -> Vec<String> {
        self.plugin_categories.read().await.keys().cloned().collect()
    }

    /// Get installed plugins
    pub async fn get_installed_plugins(&self) -> Vec<InstalledPlugin> {
        self.installed_plugins.read().await.values().cloned().collect()
    }

    /// Get marketplace stats
    pub async fn get_stats(&self) -> (u64, u64, f64, u64) {
        let stats = self.marketplace_stats.read().await;
        (stats.total_plugins, stats.total_installations, stats.total_revenue, stats.active_users)
    }

    /// Search plugins
    pub async fn search_plugins(&self, query: &str) -> Vec<PluginDefinition> {
        let plugins = self.plugins.read().await;
        let mut results = Vec::new();

        for plugin in plugins.values() {
            let definition = plugin.definition();
            let search_text = format!(
                "{} {} {} {}",
                definition.name,
                definition.description,
                definition.tags.join(" "),
                definition.category
            );

            if search_text.to_lowercase().contains(&query.to_lowercase()) {
                results.push(definition);
            }
        }

        results
    }
}

/// Pre-shipped plugin implementation
#[derive(Debug, Clone)]
struct PreShippedPlugin {
    definition: PluginDefinition,
}

impl PreShippedPlugin {
    fn new(definition: PluginDefinition) -> Self {
        Self { definition }
    }
}

#[async_trait]
impl Plugin for PreShippedPlugin {
    fn id(&self) -> &str {
        &self.definition.id
    }

    fn definition(&self) -> PluginDefinition {
        self.definition.clone()
    }

    async fn activate(&self) -> Result<()> {
        tracing::info!("Activated plugin: {}", self.definition.id);
        Ok(())
    }

    async fn deactivate(&self) -> Result<()> {
        tracing::info!("Deactivated plugin: {}", self.definition.id);
        Ok(())
    }

    async fn execute(&self, command: &str, params: Value) -> Result<Value> {
        // Simulate plugin command execution
        let start = std::time::Instant::now();
        
        // Simulate processing time
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        
        let duration = start.elapsed().as_millis() as u64;
        
        Ok(serde_json::json!({
            "plugin": self.definition.id,
            "command": command,
            "params": params,
            "result": {
                "success": true,
                "message": format!("Command '{}' executed successfully", command),
                "duration_ms": duration,
                "output": {
                    "data": "Sample output data",
                    "metadata": {
                        "version": self.definition.version,
                        "author": self.definition.author
                    }
                }
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }

    async fn get_ui_components(&self) -> Vec<UIComponent> {
        vec![
            UIComponent {
                name: "settings".to_string(),
                component_type: "settings".to_string(),
                path: "ui/settings.html".to_string(),
                props: serde_json::json!({}),
            },
            UIComponent {
                name: "dashboard".to_string(),
                component_type: "dashboard".to_string(),
                path: "ui/dashboard.html".to_string(),
                props: serde_json::json!({}),
            },
        ]
    }

    async fn on_config_change(&self, config: Value) -> Result<()> {
        tracing::info!("Config changed for plugin {}: {:?}", self.definition.id, config);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plugin_marketplace() {
        let marketplace = PluginMarketplace::new();
        let count = marketplace.initialize_with_preshipped_plugins().await.unwrap();
        
        assert!(count > 0);
        
        let categories = marketplace.list_categories().await;
        assert!(categories.contains(&"ai-models".to_string()));
        assert!(categories.contains(&"productivity".to_string()));
        assert!(categories.contains(&"development".to_string()));
    }

    #[tokio::test]
    async fn test_plugin_installation() {
        let marketplace = PluginMarketplace::new();
        marketplace.initialize_with_preshipped_plugins().await.unwrap();
        
        let config = serde_json::json!({
            "default_model": "gpt-4"
        });
        
        marketplace.install_plugin("ai-model-provider", config).await.unwrap();
        
        let installed = marketplace.get_installed_plugins().await;
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].plugin_id, "ai-model-provider");
    }

    #[tokio::test]
    async fn test_plugin_execution() {
        let marketplace = PluginMarketplace::new();
        marketplace.initialize_with_preshipped_plugins().await.unwrap();
        
        let config = serde_json::json!({});
        marketplace.install_plugin("git-integration", config).await.unwrap();
        
        let params = serde_json::json!({
            "message": "test commit",
            "files": ["src/main.rs"]
        });
        
        let result = marketplace.execute_plugin_command(
            "git-integration",
            "commit",
            params,
        ).await.unwrap();
        
        assert_eq!(result["plugin"], "git-integration");
        assert_eq!(result["result"]["success"], true);
    }
}