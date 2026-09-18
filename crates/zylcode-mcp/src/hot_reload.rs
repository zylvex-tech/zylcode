use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use notify::{Watcher, RecursiveMode, Event, EventKind};
use std::sync::mpsc::channel;
use std::time::Duration;

use crate::enhanced_bridge::{EnhancedMcpBridge, ToolDefinition};
use crate::registry::ToolRegistry;

/// Hot-reload manager for dynamic tool registration
pub struct HotReloadManager {
    watchers: RwLock<HashMap<String, notify::RecommendedWatcher>>,
    reload_callbacks: RwLock<Vec<ReloadCallback>>,
    config: HotReloadConfig,
    tool_registry: Arc<ToolRegistry>,
}

/// Configuration for hot-reload
#[derive(Debug, Clone)]
pub struct HotReloadConfig {
    pub watch_directories: Vec<PathBuf>,
    pub file_patterns: Vec<String>,
    pub debounce_ms: u64,
    pub auto_reload: bool,
}

impl Default for HotReloadConfig {
    fn default() -> Self {
        Self {
            watch_directories: vec![PathBuf::from("tools")],
            file_patterns: vec!["*.json".to_string(), "*.yaml".to_string(), "*.yml".to_string()],
            debounce_ms: 500,
            auto_reload: true,
        }
    }
}

/// Callback for reload events
pub type ReloadCallback = Arc<dyn Fn(&str) + Send + Sync>;

impl HotReloadManager {
    /// Create a new hot-reload manager
    pub async fn new(
        config: HotReloadConfig,
        tool_registry: Arc<ToolRegistry>,
    ) -> Result<Self> {
        Ok(Self {
            watchers: RwLock::new(HashMap::new()),
            reload_callbacks: RwLock::new(Vec::new()),
            config,
            tool_registry,
        })
    }

    /// Start watching for file changes
    pub async fn start_watching(&self) -> Result<()> {
        let (tx, rx) = channel();
        
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        })?;
        
        for dir in &self.config.watch_directories {
            if dir.exists() {
                watcher.watch(dir, RecursiveMode::Recursive)?;
            }
        }
        
        let callbacks = self.reload_callbacks.read().await.clone();
        let _tool_registry = self.tool_registry.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            loop {
                match rx.recv() {
                    Ok(event) => {
                        if let EventKind::Modify(_) = event.kind {
                            for path in event.paths {
                                if Self::should_reload(&path, &config.file_patterns) {
                                    tracing::info!("File changed: {:?}", path);
                                    for callback in &callbacks {
                                        callback(path.to_str().unwrap_or(""));
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Watch error: {:?}", e);
                    }
                }
            }
        });
        
        let mut watchers = self.watchers.write().await;
        watchers.insert("main".to_string(), watcher);
        
        Ok(())
    }

    /// Register a reload callback
    pub async fn register_callback(&self, callback: ReloadCallback) -> Result<()> {
        let mut callbacks = self.reload_callbacks.write().await;
        callbacks.push(callback);
        Ok(())
    }

    /// Check if file should be reloaded
    fn should_reload(path: &Path, patterns: &[String]) -> bool {
        if let Some(extension) = path.extension() {
            let ext = format!(".{}", extension.to_string_lossy());
            patterns.iter().any(|p| p.ends_with(&ext) || p == &ext)
        } else {
            false
        }
    }

    /// Reload tools from watched directories
    pub async fn reload_tools(&self) -> Result<Vec<ToolDefinition>> {
        let mut tools = Vec::new();
        
        for dir in &self.config.watch_directories {
            if dir.exists() {
                let entries = std::fs::read_dir(dir)?;
                for entry in entries {
                    let entry = entry?;
                    let path = entry.path();
                    
                    if Self::should_reload(&path, &self.config.file_patterns) {
                        match Self::load_tool_from_file(&path).await {
                            Ok(tool) => tools.push(tool),
                            Err(e) => {
                                tracing::error!("Failed to load tool from {:?}: {}", path, e);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(tools)
    }

    /// Load tool definition from file
    async fn load_tool_from_file(path: &Path) -> Result<ToolDefinition> {
        let content = std::fs::read_to_string(path)?;
        let tool: ToolDefinition = serde_json::from_str(&content)?;
        Ok(tool)
    }

    /// Get active watchers count
    pub async fn watcher_count(&self) -> usize {
        self.watchers.read().await.len()
    }
}

/// Enhanced MCP Bridge with hot-reload and analytics
pub struct EnhancedMcpBridgeWithHotReload {
    bridge: EnhancedMcpBridge,
    hot_reload_manager: Arc<HotReloadManager>,
    analytics: Arc<ToolAnalytics>,
}

/// Tool analytics system
pub struct ToolAnalytics {
    usage_stats: RwLock<HashMap<String, UsageStats>>,
    performance_metrics: RwLock<HashMap<String, PerformanceMetrics>>,
    error_tracking: RwLock<HashMap<String, ErrorStats>>,
}

/// Usage statistics for a tool
#[derive(Debug, Clone)]
pub struct UsageStats {
    pub total_calls: u64,
    pub successful_calls: u64,
    pub failed_calls: u64,
    pub average_duration_ms: f64,
    pub last_used: chrono::DateTime<chrono::Utc>,
}

/// Performance metrics for a tool
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub min_duration_ms: u64,
    pub max_duration_ms: u64,
    pub p50_duration_ms: u64,
    pub p95_duration_ms: u64,
    pub p99_duration_ms: u64,
    pub throughput_per_second: f64,
}

/// Error statistics for a tool
#[derive(Debug, Clone)]
pub struct ErrorStats {
    pub total_errors: u64,
    pub error_types: HashMap<String, u64>,
    pub last_error: Option<String>,
    pub last_error_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for ToolAnalytics {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolAnalytics {
    /// Create a new tool analytics system
    pub fn new() -> Self {
        Self {
            usage_stats: RwLock::new(HashMap::new()),
            performance_metrics: RwLock::new(HashMap::new()),
            error_tracking: RwLock::new(HashMap::new()),
        }
    }

    /// Record tool usage
    pub async fn record_usage(&self, tool_id: &str, duration: Duration, success: bool) {
        let mut stats = self.usage_stats.write().await;
        let entry = stats.entry(tool_id.to_string()).or_insert_with(|| UsageStats {
            total_calls: 0,
            successful_calls: 0,
            failed_calls: 0,
            average_duration_ms: 0.0,
            last_used: chrono::Utc::now(),
        });
        
        entry.total_calls += 1;
        if success {
            entry.successful_calls += 1;
        } else {
            entry.failed_calls += 1;
        }
        
        let duration_ms = duration.as_millis() as f64;
        entry.average_duration_ms = (entry.average_duration_ms * (entry.total_calls - 1) as f64 + duration_ms) / entry.total_calls as f64;
        entry.last_used = chrono::Utc::now();
    }

    /// Record tool error
    pub async fn record_error(&self, tool_id: &str, error: &str) {
        let mut errors = self.error_tracking.write().await;
        let entry = errors.entry(tool_id.to_string()).or_insert_with(|| ErrorStats {
            total_errors: 0,
            error_types: HashMap::new(),
            last_error: None,
            last_error_time: None,
        });
        
        entry.total_errors += 1;
        *entry.error_types.entry(error.to_string()).or_insert(0) += 1;
        entry.last_error = Some(error.to_string());
        entry.last_error_time = Some(chrono::Utc::now());
    }

    /// Get usage statistics for a tool
    pub async fn get_usage_stats(&self, tool_id: &str) -> Option<UsageStats> {
        self.usage_stats.read().await.get(tool_id).cloned()
    }

    /// Get performance metrics for a tool
    pub async fn get_performance_metrics(&self, tool_id: &str) -> Option<PerformanceMetrics> {
        self.performance_metrics.read().await.get(tool_id).cloned()
    }

    /// Get error statistics for a tool
    pub async fn get_error_stats(&self, tool_id: &str) -> Option<ErrorStats> {
        self.error_tracking.read().await.get(tool_id).cloned()
    }

    /// Get overall analytics report
    pub async fn get_analytics_report(&self) -> AnalyticsReport {
        let usage_stats = self.usage_stats.read().await.clone();
        let performance_metrics = self.performance_metrics.read().await.clone();
        let error_stats = self.error_tracking.read().await.clone();
        
        AnalyticsReport {
            total_tools: usage_stats.len(),
            total_calls: usage_stats.values().map(|s| s.total_calls).sum(),
            total_successful: usage_stats.values().map(|s| s.successful_calls).sum(),
            total_failed: usage_stats.values().map(|s| s.failed_calls).sum(),
            average_success_rate: {
                let total = usage_stats.values().map(|s| s.total_calls).sum::<u64>() as f64;
                let successful = usage_stats.values().map(|s| s.successful_calls).sum::<u64>() as f64;
                if total > 0.0 { successful / total } else { 0.0 }
            },
            usage_stats,
            performance_metrics,
            error_stats,
        }
    }
}

/// Analytics report
#[derive(Debug, Clone)]
pub struct AnalyticsReport {
    pub total_tools: usize,
    pub total_calls: u64,
    pub total_successful: u64,
    pub total_failed: u64,
    pub average_success_rate: f64,
    pub usage_stats: HashMap<String, UsageStats>,
    pub performance_metrics: HashMap<String, PerformanceMetrics>,
    pub error_stats: HashMap<String, ErrorStats>,
}

impl EnhancedMcpBridgeWithHotReload {
    /// Create a new enhanced MCP bridge with hot-reload
    pub async fn new() -> Result<Self> {
        let bridge = EnhancedMcpBridge::new();
        let tool_registry = Arc::new(ToolRegistry::new());
        let config = HotReloadConfig::default();
        let hot_reload_manager = Arc::new(HotReloadManager::new(config, tool_registry).await?);
        let analytics = Arc::new(ToolAnalytics::new());
        
        Ok(Self {
            bridge,
            hot_reload_manager,
            analytics,
        })
    }

    /// Initialize with 150+ tools
    pub async fn initialize_with_enhanced_tools(&self) -> Result<usize> {
        // Only tools with a real executor are registered. The "additional
        // tools" below are definitions with no executor: they are preserved as
        // catalogue metadata and deliberately NOT counted as registered.
        //
        // The previous implementation logged each of them and then returned
        // `count + 50`, inventing fifty registrations that never happened, in
        // order to satisfy `assert!(count >= 150)`. A fabricated count is the
        // same defect as a fabricated success — it makes an unverifiable claim
        // look like a measurement.
        let registered = self.bridge.initialize_with_builtin_tools().await?;

        for tool in self.get_additional_tools() {
            debug_assert!(
                crate::real_tools::get_real_tool(&tool.id).is_none(),
                "`{}` has a real executor and should be registered, not skipped",
                tool.id
            );
            tracing::debug!(
                tool = %tool.id,
                "definition-only; preserved as catalogue metadata, not registered"
            );
        }

        Ok(registered)
    }

    /// Get additional tools to reach 150+
    ///
    /// NOTE: the doc comment above is the historical claim. These definitions
    /// have no executors, so they are metadata only and do not contribute to
    /// any registered count. See `docs/governance/TOOL_CATALOGUE_TRUTH_TABLE.md`.
    fn get_additional_tools(&self) -> Vec<ToolDefinition> {
        vec![
            // AI/ML Tools
            ToolDefinition {
                id: "ml.model.train".to_string(),
                name: "ML Model Train".to_string(),
                description: "Train machine learning models".to_string(),
                category: "ai-ml".to_string(),
                parameters: serde_json::json!({}),
                required_permissions: vec!["ml.train".to_string()],
            },
            ToolDefinition {
                id: "ml.model.evaluate".to_string(),
                name: "ML Model Evaluate".to_string(),
                description: "Evaluate machine learning models".to_string(),
                category: "ai-ml".to_string(),
                parameters: serde_json::json!({}),
                required_permissions: vec!["ml.evaluate".to_string()],
            },
            ToolDefinition {
                id: "ml.data.preprocess".to_string(),
                name: "ML Data Preprocess".to_string(),
                description: "Preprocess data for machine learning".to_string(),
                category: "ai-ml".to_string(),
                parameters: serde_json::json!({}),
                required_permissions: vec!["ml.data".to_string()],
            },
            ToolDefinition {
                id: "ml.feature.engineer".to_string(),
                name: "ML Feature Engineer".to_string(),
                description: "Engineer features for machine learning".to_string(),
                category: "ai-ml".to_string(),
                parameters: serde_json::json!({}),
                required_permissions: vec!["ml.features".to_string()],
            },
            ToolDefinition {
                id: "ml.model.deploy".to_string(),
                name: "ML Model Deploy".to_string(),
                description: "Deploy machine learning models".to_string(),
                category: "ai-ml".to_string(),
                parameters: serde_json::json!({}),
                required_permissions: vec!["ml.deploy".to_string()],
            },
            
            // Cloud Services
            ToolDefinition {
                id: "cloud.aws.manage".to_string(),
                name: "AWS Management".to_string(),
                description: "Manage AWS services".to_string(),
                category: "cloud".to_string(),
                parameters: serde_json::json!({}),
                required_permissions: vec!["cloud.aws".to_string()],
            },
            ToolDefinition {
                id: "cloud.gcp.manage".to_string(),
                name: "GCP Management".to_string(),
                description: "Manage Google Cloud services".to_string(),
                category: "cloud".to_string(),
                parameters: serde_json::json!({}),
                required_permissions: vec!["cloud.gcp".to_string()],
            },
            ToolDefinition {
                id: "cloud.azure.manage".to_string(),
                name: "Azure Management".to_string(),
                description: "Manage Azure services".to_string(),
                category: "cloud".to_string(),
                parameters: serde_json::json!({}),
                required_permissions: vec!["cloud.azure".to_string()],
            },
            ToolDefinition {
                id: "cloud.serverless.deploy".to_string(),
                name: "Serverless Deploy".to_string(),
                description: "Deploy serverless functions".to_string(),
                category: "cloud".to_string(),
                parameters: serde_json::json!({}),
                required_permissions: vec!["cloud.serverless".to_string()],
            },
            ToolDefinition {
                id: "cloud.kubernetes.manage".to_string(),
                name: "Kubernetes Management".to_string(),
                description: "Manage Kubernetes clusters".to_string(),
                category: "cloud".to_string(),
                parameters: serde_json::json!({}),
                required_permissions: vec!["cloud.kubernetes".to_string()],
            },
            
            // DevOps Tools
            ToolDefinition {
                id: "devops.cicd.manage".to_string(),
                name: "CI/CD Management".to_string(),
                description: "Manage CI/CD pipelines".to_string(),
                category: "devops".to_string(),
                parameters: serde_json::json!({}),
                required_permissions: vec!["devops.cicd".to_string()],
            },
            ToolDefinition {
                id: "devops.monitoring.setup".to_string(),
                name: "Monitoring Setup".to_string(),
                description: "Set up monitoring and alerting".to_string(),
                category: "devops".to_string(),
                parameters: serde_json::json!({}),
                required_permissions: vec!["devops.monitoring".to_string()],
            },
            ToolDefinition {
                id: "devops.logging.setup".to_string(),
                name: "Logging Setup".to_string(),
                description: "Set up logging and analysis".to_string(),
                category: "devops".to_string(),
                parameters: serde_json::json!({}),
                required_permissions: vec!["devops.logging".to_string()],
            },
            ToolDefinition {
                id: "devops.security.scan".to_string(),
                name: "Security Scan".to_string(),
                description: "Scan for security vulnerabilities".to_string(),
                category: "devops".to_string(),
                parameters: serde_json::json!({}),
                required_permissions: vec!["devops.security".to_string()],
            },
            ToolDefinition {
                id: "devops.performance.test".to_string(),
                name: "Performance Test".to_string(),
                description: "Run performance tests".to_string(),
                category: "devops".to_string(),
                parameters: serde_json::json!({}),
                required_permissions: vec!["devops.performance".to_string()],
            },
            
            // Communication Tools
            ToolDefinition {
                id: "comm.slack.send".to_string(),
                name: "Slack Send".to_string(),
                description: "Send messages to Slack".to_string(),
                category: "communication".to_string(),
                parameters: serde_json::json!({}),
                required_permissions: vec!["comm.slack".to_string()],
            },
            ToolDefinition {
                id: "comm.discord.send".to_string(),
                name: "Discord Send".to_string(),
                description: "Send messages to Discord".to_string(),
                category: "communication".to_string(),
                parameters: serde_json::json!({}),
                required_permissions: vec!["comm.discord".to_string()],
            },
            ToolDefinition {
                id: "comm.teams.send".to_string(),
                name: "Teams Send".to_string(),
                description: "Send messages to Microsoft Teams".to_string(),
                category: "communication".to_string(),
                parameters: serde_json::json!({}),
                required_permissions: vec!["comm.teams".to_string()],
            },
            ToolDefinition {
                id: "comm.email.send".to_string(),
                name: "Email Send".to_string(),
                description: "Send email messages".to_string(),
                category: "communication".to_string(),
                parameters: serde_json::json!({}),
                required_permissions: vec!["comm.email".to_string()],
            },
            ToolDefinition {
                id: "comm.notification.send".to_string(),
                name: "Notification Send".to_string(),
                description: "Send notifications".to_string(),
                category: "communication".to_string(),
                parameters: serde_json::json!({}),
                required_permissions: vec!["comm.notification".to_string()],
            },
        ]
    }

    /// Start hot-reload watching
    pub async fn start_hot_reload(&self) -> Result<()> {
        self.hot_reload_manager.start_watching().await
    }

    /// Get analytics report
    pub async fn get_analytics_report(&self) -> AnalyticsReport {
        self.analytics.get_analytics_report().await
    }

    /// Execute tool with analytics
    pub async fn execute_tool_with_analytics(&self, tool_id: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let start = std::time::Instant::now();
        
        // Execute tool
        let result = self.bridge.execute_tool(tool_id, params).await;
        
        let duration = start.elapsed();
        let success = result.is_ok();
        
        // Record analytics
        self.analytics.record_usage(tool_id, duration, success).await;
        
        if let Err(ref e) = result {
            self.analytics.record_error(tool_id, &e.to_string()).await;
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hot_reload_manager() {
        let tool_registry = Arc::new(ToolRegistry::new());
        let config = HotReloadConfig::default();
        let manager = HotReloadManager::new(config, tool_registry).await.unwrap();
        
        assert_eq!(manager.watcher_count().await, 0);
    }

    #[tokio::test]
    async fn test_tool_analytics() {
        let analytics = ToolAnalytics::new();
        
        analytics.record_usage("test_tool", Duration::from_millis(100), true).await;
        analytics.record_usage("test_tool", Duration::from_millis(200), true).await;
        analytics.record_error("test_tool", "test error").await;
        
        let stats = analytics.get_usage_stats("test_tool").await.unwrap();
        assert_eq!(stats.total_calls, 2);
        assert_eq!(stats.successful_calls, 2);
        assert_eq!(stats.failed_calls, 0);
        
        let errors = analytics.get_error_stats("test_tool").await.unwrap();
        assert_eq!(errors.total_errors, 1);
    }

    /// Replaces `assert!(count >= 150)`.
    ///
    /// That assertion was satisfiable only because the implementation
    /// fabricated fifty registrations. The invariant that actually matters is
    /// that the reported count equals what is registered, and that everything
    /// registered has a real executor.
    #[tokio::test]
    async fn enhanced_tools_count_reflects_real_registrations() {
        let bridge = EnhancedMcpBridgeWithHotReload::new().await.unwrap();
        let reported = bridge.initialize_with_enhanced_tools().await.unwrap();

        let actual = bridge.bridge.registered_ids().await;
        assert_eq!(
            reported,
            actual.len(),
            "reported {reported} registrations but {} are actually registered",
            actual.len()
        );
        assert_eq!(bridge.bridge.tool_count().await, reported);

        for id in &actual {
            assert!(
                crate::real_tools::get_real_tool(id).is_some(),
                "`{id}` is registered without a real executor"
            );
        }
    }

    /// The historical `count + 50` fabrication must not return.
    #[tokio::test]
    async fn enhanced_tools_count_is_not_inflated() {
        let bridge = EnhancedMcpBridgeWithHotReload::new().await.unwrap();
        let reported = bridge.initialize_with_enhanced_tools().await.unwrap();

        let additional = bridge.get_additional_tools();
        assert!(!additional.is_empty(), "the historical claim listed additional tools");

        // None of these has an executor, so none may be counted. This is the
        // exact claim the old `count + 50` fabricated.
        for tool in &additional {
            assert!(
                crate::real_tools::get_real_tool(&tool.id).is_none(),
                "`{}` unexpectedly has an executor; it should be registered and counted",
                tool.id
            );
        }
        assert_eq!(
            reported,
            bridge.bridge.registered_ids().await.len(),
            "the count must be the number of real registrations, not a padded figure"
        );
    }
}
