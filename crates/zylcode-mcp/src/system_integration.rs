use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::enhanced_bridge::ToolDefinition;
use crate::enhanced_plugin_marketplace::EnhancedPluginMarketplace;
use crate::enhanced_skills::EnhancedSkillsSystem;
use crate::hot_reload::EnhancedMcpBridgeWithHotReload;

/// Integration configuration
#[derive(Debug, Clone)]
pub struct IntegrationConfig {
    pub enable_skill_tools: bool,
    pub enable_plugin_tools: bool,
    pub enable_composition: bool,
    pub enable_marketplace: bool,
    pub cache_ttl_seconds: u64,
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            enable_skill_tools: true,
            enable_plugin_tools: true,
            enable_composition: true,
            enable_marketplace: true,
            cache_ttl_seconds: 300,
        }
    }
}

/// MCP Bridge + Skills System Integration
pub struct McpSkillsIntegration {
    _mcp_bridge: Arc<EnhancedMcpBridgeWithHotReload>,
    skills_system: Arc<EnhancedSkillsSystem>,
    integration_config: IntegrationConfig,
    skill_tools_cache: RwLock<HashMap<String, ToolDefinition>>,
}

impl McpSkillsIntegration {
    /// Create a new MCP Skills integration
    pub async fn new(
        mcp_bridge: Arc<EnhancedMcpBridgeWithHotReload>,
        skills_system: Arc<EnhancedSkillsSystem>,
        config: IntegrationConfig,
    ) -> Result<Self> {
        Ok(Self {
            _mcp_bridge: mcp_bridge,
            skills_system,
            integration_config: config,
            skill_tools_cache: RwLock::new(HashMap::new()),
        })
    }

    /// Register skills as MCP tools
    pub async fn register_skills_as_tools(&self) -> Result<usize> {
        let mut count = 0;

        if self.integration_config.enable_skill_tools {
            // Get all skills from the skills system
            let skills = self.skills_system.get_all_skills().await?;

            for skill in skills {
                let tool_definition = ToolDefinition {
                    id: format!("skill.{}", skill.id),
                    name: skill.name.clone(),
                    description: skill.description.clone(),
                    category: "skills".to_string(),
                    parameters: skill.config_schema.clone(),
                    required_permissions: vec!["skill.execute".to_string()],
                };

                // Register the tool in the MCP bridge
                // Note: This would require adding a method to register tools dynamically
                // For now, we'll just cache the tool definition
                let mut cache = self.skill_tools_cache.write().await;
                cache.insert(skill.id.clone(), tool_definition);

                count += 1;
            }
        }

        Ok(count)
    }

    /// Execute skill via MCP bridge
    pub async fn execute_skill_via_mcp(&self, skill_id: &str, params: Value) -> Result<Value> {
        // Execute the skill through the skills system
        let result = self.skills_system.execute_skill(skill_id, params).await?;

        Ok(result)
    }

    /// Get all skill tools
    pub async fn get_skill_tools(&self) -> Result<Vec<ToolDefinition>> {
        let cache = self.skill_tools_cache.read().await;
        Ok(cache.values().cloned().collect())
    }

    /// Get skill tool by ID
    pub async fn get_skill_tool(&self, skill_id: &str) -> Option<ToolDefinition> {
        let cache = self.skill_tools_cache.read().await;
        cache.get(skill_id).cloned()
    }
}

/// Skills + Plugin Marketplace Integration
pub struct SkillsPluginIntegration {
    _skills_system: Arc<EnhancedSkillsSystem>,
    plugin_marketplace: Arc<EnhancedPluginMarketplace>,
    _integration_config: IntegrationConfig,
    skill_plugins_cache: RwLock<HashMap<String, Vec<String>>>,
}

impl SkillsPluginIntegration {
    /// Create a new Skills Plugin integration
    pub async fn new(
        skills_system: Arc<EnhancedSkillsSystem>,
        plugin_marketplace: Arc<EnhancedPluginMarketplace>,
        config: IntegrationConfig,
    ) -> Result<Self> {
        Ok(Self {
            _skills_system: skills_system,
            plugin_marketplace,
            _integration_config: config,
            skill_plugins_cache: RwLock::new(HashMap::new()),
        })
    }

    /// Install skill plugin from marketplace
    pub async fn install_skill_plugin(&self, skill_id: &str) -> Result<()> {
        // Find plugins for the skill
        let plugins = self
            .plugin_marketplace
            .search_plugins(&format!("skill:{}", skill_id))
            .await?;

        if let Some(plugin) = plugins.first() {
            // Install the plugin
            self.plugin_marketplace.install_plugin(&plugin.id).await?;

            // Update cache
            let mut cache = self.skill_plugins_cache.write().await;
            cache
                .entry(skill_id.to_string())
                .or_insert_with(Vec::new)
                .push(plugin.id.clone());
        }

        Ok(())
    }

    /// Rate skill plugin
    pub async fn rate_skill_plugin(
        &self,
        skill_id: &str,
        rating: crate::enhanced_skills::Rating,
    ) -> Result<()> {
        // Find plugins for the skill
        let plugins = self
            .plugin_marketplace
            .search_plugins(&format!("skill:{}", skill_id))
            .await?;

        if let Some(plugin) = plugins.first() {
            // Rate the plugin
            self.plugin_marketplace
                .rate_plugin(&plugin.id, rating)
                .await?;
        }

        Ok(())
    }

    /// Get plugins for skill
    pub async fn get_skill_plugins(&self, skill_id: &str) -> Result<Vec<String>> {
        let cache = self.skill_plugins_cache.read().await;
        Ok(cache.get(skill_id).cloned().unwrap_or_default())
    }
}

/// MCP Bridge + Plugin Marketplace Integration
pub struct McpPluginIntegration {
    _mcp_bridge: Arc<EnhancedMcpBridgeWithHotReload>,
    plugin_marketplace: Arc<EnhancedPluginMarketplace>,
    integration_config: IntegrationConfig,
    plugin_tools_cache: RwLock<HashMap<String, ToolDefinition>>,
}

impl McpPluginIntegration {
    /// Create a new MCP Plugin integration
    pub async fn new(
        mcp_bridge: Arc<EnhancedMcpBridgeWithHotReload>,
        plugin_marketplace: Arc<EnhancedPluginMarketplace>,
        config: IntegrationConfig,
    ) -> Result<Self> {
        Ok(Self {
            _mcp_bridge: mcp_bridge,
            plugin_marketplace,
            integration_config: config,
            plugin_tools_cache: RwLock::new(HashMap::new()),
        })
    }

    /// Register plugins as MCP tools
    pub async fn register_plugins_as_tools(&self) -> Result<usize> {
        let mut count = 0;

        if self.integration_config.enable_plugin_tools {
            // Get all plugins from the marketplace
            let plugins = self.plugin_marketplace.get_all_plugins().await?;

            for plugin in plugins {
                let tool_definition = ToolDefinition {
                    id: format!("plugin.{}", plugin.id),
                    name: plugin.name.clone(),
                    description: plugin.description.clone(),
                    category: "plugins".to_string(),
                    parameters: plugin.config_schema.clone(),
                    required_permissions: vec!["plugin.execute".to_string()],
                };

                // Register the tool in the MCP bridge
                let mut cache = self.plugin_tools_cache.write().await;
                cache.insert(plugin.id.clone(), tool_definition);

                count += 1;
            }
        }

        Ok(count)
    }

    /// Execute plugin via MCP bridge
    pub async fn execute_plugin_via_mcp(&self, plugin_id: &str, params: Value) -> Result<Value> {
        // Execute the plugin through the marketplace
        let result = self
            .plugin_marketplace
            .execute_plugin(plugin_id, params)
            .await?;

        Ok(result)
    }

    /// Get all plugin tools
    pub async fn get_plugin_tools(&self) -> Result<Vec<ToolDefinition>> {
        let cache = self.plugin_tools_cache.read().await;
        Ok(cache.values().cloned().collect())
    }

    /// Get plugin tool by ID
    pub async fn get_plugin_tool(&self, plugin_id: &str) -> Option<ToolDefinition> {
        let cache = self.plugin_tools_cache.read().await;
        cache.get(plugin_id).cloned()
    }
}

/// System Integration Manager
pub struct SystemIntegrationManager {
    mcp_skills_integration: Arc<McpSkillsIntegration>,
    skills_plugin_integration: Arc<SkillsPluginIntegration>,
    mcp_plugin_integration: Arc<McpPluginIntegration>,
    config: IntegrationConfig,
}

impl SystemIntegrationManager {
    /// Create a new system integration manager
    pub async fn new(
        mcp_bridge: Arc<EnhancedMcpBridgeWithHotReload>,
        skills_system: Arc<EnhancedSkillsSystem>,
        plugin_marketplace: Arc<EnhancedPluginMarketplace>,
        config: IntegrationConfig,
    ) -> Result<Self> {
        let mcp_skills_integration = Arc::new(
            McpSkillsIntegration::new(mcp_bridge.clone(), skills_system.clone(), config.clone())
                .await?,
        );

        let skills_plugin_integration = Arc::new(
            SkillsPluginIntegration::new(
                skills_system.clone(),
                plugin_marketplace.clone(),
                config.clone(),
            )
            .await?,
        );

        let mcp_plugin_integration = Arc::new(
            McpPluginIntegration::new(
                mcp_bridge.clone(),
                plugin_marketplace.clone(),
                config.clone(),
            )
            .await?,
        );

        Ok(Self {
            mcp_skills_integration,
            skills_plugin_integration,
            mcp_plugin_integration,
            config,
        })
    }

    /// Initialize all integrations
    pub async fn initialize(&self) -> Result<IntegrationStats> {
        let mut stats = IntegrationStats::default();

        // Register skills as MCP tools
        if self.config.enable_skill_tools {
            let skill_tools_count = self
                .mcp_skills_integration
                .register_skills_as_tools()
                .await?;
            stats.skill_tools_registered = skill_tools_count;
        }

        // Register plugins as MCP tools
        if self.config.enable_plugin_tools {
            let plugin_tools_count = self
                .mcp_plugin_integration
                .register_plugins_as_tools()
                .await?;
            stats.plugin_tools_registered = plugin_tools_count;
        }

        stats.initialized = true;
        Ok(stats)
    }

    /// Execute skill via MCP
    pub async fn execute_skill(&self, skill_id: &str, params: Value) -> Result<Value> {
        self.mcp_skills_integration
            .execute_skill_via_mcp(skill_id, params)
            .await
    }

    /// Execute plugin via MCP
    pub async fn execute_plugin(&self, plugin_id: &str, params: Value) -> Result<Value> {
        self.mcp_plugin_integration
            .execute_plugin_via_mcp(plugin_id, params)
            .await
    }

    /// Install skill plugin
    pub async fn install_skill_plugin(&self, skill_id: &str) -> Result<()> {
        self.skills_plugin_integration
            .install_skill_plugin(skill_id)
            .await
    }

    /// Get integration stats
    pub async fn get_stats(&self) -> Result<IntegrationStats> {
        let skill_tools = self.mcp_skills_integration.get_skill_tools().await?;
        let plugin_tools = self.mcp_plugin_integration.get_plugin_tools().await?;

        Ok(IntegrationStats {
            initialized: true,
            skill_tools_registered: skill_tools.len(),
            plugin_tools_registered: plugin_tools.len(),
            total_tools: skill_tools.len() + plugin_tools.len(),
        })
    }
}

/// Integration statistics
#[derive(Debug, Clone, Default)]
pub struct IntegrationStats {
    pub initialized: bool,
    pub skill_tools_registered: usize,
    pub plugin_tools_registered: usize,
    pub total_tools: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_system_integration() {
        // This test would require setting up all the systems
        // For now, we'll just test the configuration
        let config = IntegrationConfig::default();
        assert!(config.enable_skill_tools);
        assert!(config.enable_plugin_tools);
        assert!(config.enable_composition);
        assert!(config.enable_marketplace);
    }
}
