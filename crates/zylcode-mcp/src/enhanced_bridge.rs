use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::RwLock;

use crate::registry::ToolRegistry;
use crate::tool::{Tool, ToolDescriptor};

/// Enhanced MCP Bridge with 100+ built-in tools
pub struct EnhancedMcpBridge {
    registry: ToolRegistry,
    tool_categories: RwLock<HashMap<String, Vec<String>>>,
    execution_stats: RwLock<ExecutionStats>,
    /// The runtime (policy plus evidence sink) every tool registered by this
    /// bridge inherits.
    ///
    /// Restrictive by default: reads proceed, everything above them requires
    /// approval, and every outcome is recorded. Use
    /// [`EnhancedMcpBridge::with_runtime`] to state a different policy.
    runtime: crate::evidence::ToolRuntime,
}

#[derive(Debug, Default)]
struct ExecutionStats {
    total_calls: u64,
    successful_calls: u64,
    failed_calls: u64,
    total_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCategory {
    pub name: String,
    pub description: String,
    pub tools: Vec<ToolDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub parameters: Value,
    pub required_permissions: Vec<String>,
}

impl Default for EnhancedMcpBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl EnhancedMcpBridge {
    pub fn new() -> Self {
        Self {
            registry: ToolRegistry::new(),
            tool_categories: RwLock::new(HashMap::new()),
            execution_stats: RwLock::new(ExecutionStats::default()),
            runtime: crate::evidence::ToolRuntime::restrictive(),
        }
    }

    /// Build a bridge whose tools share an explicit runtime.
    ///
    /// The caller is stating its policy and where evidence goes. Tests that
    /// need to exercise a mutating executor use
    /// [`crate::evidence::ToolRuntime::permissive_without_evidence`]; nothing
    /// reaches a permissive policy by accident.
    pub fn with_runtime(runtime: crate::evidence::ToolRuntime) -> Self {
        Self {
            runtime,
            ..Self::new()
        }
    }

    pub fn runtime(&self) -> &crate::evidence::ToolRuntime {
        &self.runtime
    }

    /// Initialize with 100+ built-in tools
    /// Populate the executable registry from the **canonical catalogue**.
    ///
    /// The bridge no longer registers its own legacy definitions. It registers
    /// exactly the ids the catalogue marks executable — the set that has a real
    /// executor. Definitions without an executor remain available through
    /// [`Self::all_definitions`] as metadata and are never callable.
    ///
    /// The previous implementation registered all 27 definitions and answered
    /// every call with a fabricated `success: true`.
    ///
    /// Returns the number of tools actually registered.
    pub async fn initialize_with_builtin_tools(&self) -> Result<usize> {
        let catalogue = crate::tool_catalogue::Catalogue::canonical();

        let mut registered = 0usize;
        let mut ids = Vec::new();

        for id in catalogue.executable_ids() {
            let entry = catalogue
                .get(id)
                .expect("an executable id is always present in the catalogue");
            let tool = BuiltinTool::with_runtime(ToolDefinition {
                id: entry.id.clone(),
                name: entry.id.clone(),
                description: entry.description.clone(),
                category: "executable".to_string(),
                parameters: entry.input_schema.clone(),
                required_permissions: Vec::new(),
            }, self.runtime.clone());
            self.registry.register(std::sync::Arc::new(tool)).await;
            ids.push(entry.id.clone());
            registered += 1;
        }

        let mut categories = self.tool_categories.write().await;
        categories.clear();
        categories.insert("executable".to_string(), ids);

        tracing::info!(
            registered,
            definition_only = catalogue.definition_only_ids().len(),
            "initialised MCP bridge from the canonical catalogue"
        );
        Ok(registered)
    }

    /// Get all built-in tool categories
    fn get_builtin_tool_categories(&self) -> Vec<ToolCategory> {
        vec![
            self.get_development_tools(),
            self.get_ai_ml_tools(),
            self.get_database_tools(),
            self.get_cloud_tools(),
            self.get_devops_tools(),
            self.get_communication_tools(),
            self.get_productivity_tools(),
            self.get_security_tools(),
        ]
    }

    /// Every definition this bridge knows about, flattened.
    ///
    /// These are **definitions, not capabilities**: they carry a schema and
    /// nothing else. They have no executor, so they must never be registered as
    /// executable tools. `crate::tool_catalogue::Catalogue` consumes this to
    /// preserve the schemas as `DefinitionOnly` metadata.
    pub fn all_definitions(&self) -> Vec<ToolDefinition> {
        self.get_builtin_tool_categories()
            .into_iter()
            .flat_map(|c| c.tools)
            .collect()
    }

    /// Development Tools (25+)
    fn get_development_tools(&self) -> ToolCategory {
        ToolCategory {
            name: "development".to_string(),
            description: "Development tools for code management, building, and testing".to_string(),
            tools: vec![
                // Git operations
                ToolDefinition {
                    id: "git.commit".to_string(),
                    name: "Git Commit".to_string(),
                    description: "Create a git commit with message and files".to_string(),
                    category: "git".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "message": {"type": "string", "description": "Commit message"},
                            "files": {"type": "array", "items": {"type": "string"}, "description": "Files to commit"},
                            "branch": {"type": "string", "description": "Branch name"}
                        },
                        "required": ["message"]
                    }),
                    required_permissions: vec!["git.write".to_string()],
                },
                ToolDefinition {
                    id: "git.push".to_string(),
                    name: "Git Push".to_string(),
                    description: "Push commits to remote repository".to_string(),
                    category: "git".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "remote": {"type": "string", "description": "Remote name"},
                            "branch": {"type": "string", "description": "Branch name"},
                            "force": {"type": "boolean", "description": "Force push"}
                        }
                    }),
                    required_permissions: vec!["git.write".to_string()],
                },
                ToolDefinition {
                    id: "git.pull".to_string(),
                    name: "Git Pull".to_string(),
                    description: "Pull changes from remote repository".to_string(),
                    category: "git".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "remote": {"type": "string", "description": "Remote name"},
                            "branch": {"type": "string", "description": "Branch name"}
                        }
                    }),
                    required_permissions: vec!["git.read".to_string()],
                },
                // Package managers
                ToolDefinition {
                    id: "npm.install".to_string(),
                    name: "NPM Install".to_string(),
                    description: "Install npm packages".to_string(),
                    category: "package-manager".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "packages": {"type": "array", "items": {"type": "string"}, "description": "Packages to install"},
                            "dev": {"type": "boolean", "description": "Install as dev dependency"},
                            "global": {"type": "boolean", "description": "Install globally"}
                        }
                    }),
                    required_permissions: vec!["filesystem.write".to_string()],
                },
                ToolDefinition {
                    id: "yarn.install".to_string(),
                    name: "Yarn Install".to_string(),
                    description: "Install yarn packages".to_string(),
                    category: "package-manager".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "packages": {"type": "array", "items": {"type": "string"}, "description": "Packages to install"},
                            "dev": {"type": "boolean", "description": "Install as dev dependency"}
                        }
                    }),
                    required_permissions: vec!["filesystem.write".to_string()],
                },
                // Build systems
                ToolDefinition {
                    id: "webpack.build".to_string(),
                    name: "Webpack Build".to_string(),
                    description: "Build project with webpack".to_string(),
                    category: "build".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "config": {"type": "string", "description": "Webpack config path"},
                            "mode": {"type": "string", "enum": ["development", "production"], "description": "Build mode"}
                        }
                    }),
                    required_permissions: vec!["filesystem.write".to_string()],
                },
                ToolDefinition {
                    id: "vite.build".to_string(),
                    name: "Vite Build".to_string(),
                    description: "Build project with Vite".to_string(),
                    category: "build".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "config": {"type": "string", "description": "Vite config path"},
                            "mode": {"type": "string", "enum": ["development", "production"], "description": "Build mode"}
                        }
                    }),
                    required_permissions: vec!["filesystem.write".to_string()],
                },
                // Testing
                ToolDefinition {
                    id: "jest.test".to_string(),
                    name: "Jest Test".to_string(),
                    description: "Run tests with Jest".to_string(),
                    category: "testing".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "pattern": {"type": "string", "description": "Test file pattern"},
                            "coverage": {"type": "boolean", "description": "Generate coverage report"},
                            "watch": {"type": "boolean", "description": "Watch mode"}
                        }
                    }),
                    required_permissions: vec!["filesystem.read".to_string()],
                },
                ToolDefinition {
                    id: "vitest.test".to_string(),
                    name: "Vitest Test".to_string(),
                    description: "Run tests with Vitest".to_string(),
                    category: "testing".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "pattern": {"type": "string", "description": "Test file pattern"},
                            "coverage": {"type": "boolean", "description": "Generate coverage report"}
                        }
                    }),
                    required_permissions: vec!["filesystem.read".to_string()],
                },
                // Linting/Formatting
                ToolDefinition {
                    id: "eslint.lint".to_string(),
                    name: "ESLint".to_string(),
                    description: "Lint code with ESLint".to_string(),
                    category: "linting".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "files": {"type": "array", "items": {"type": "string"}, "description": "Files to lint"},
                            "fix": {"type": "boolean", "description": "Auto-fix issues"}
                        }
                    }),
                    required_permissions: vec!["filesystem.read".to_string()],
                },
                ToolDefinition {
                    id: "prettier.format".to_string(),
                    name: "Prettier".to_string(),
                    description: "Format code with Prettier".to_string(),
                    category: "formatting".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "files": {"type": "array", "items": {"type": "string"}, "description": "Files to format"},
                            "write": {"type": "boolean", "description": "Write changes to files"}
                        }
                    }),
                    required_permissions: vec!["filesystem.write".to_string()],
                },
            ],
        }
    }

    /// AI/ML Tools (15+)
    fn get_ai_ml_tools(&self) -> ToolCategory {
        ToolCategory {
            name: "ai-ml".to_string(),
            description: "AI and Machine Learning tools".to_string(),
            tools: vec![
                ToolDefinition {
                    id: "openai.complete".to_string(),
                    name: "OpenAI Complete".to_string(),
                    description: "Generate text completion with OpenAI".to_string(),
                    category: "ai".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "prompt": {"type": "string", "description": "Input prompt"},
                            "model": {"type": "string", "description": "Model name"},
                            "max_tokens": {"type": "integer", "description": "Maximum tokens"},
                            "temperature": {"type": "number", "description": "Temperature"}
                        },
                        "required": ["prompt"]
                    }),
                    required_permissions: vec!["ai.execute".to_string()],
                },
                ToolDefinition {
                    id: "deepseek.complete".to_string(),
                    name: "DeepSeek Complete".to_string(),
                    description: "Generate text completion with DeepSeek".to_string(),
                    category: "ai".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "prompt": {"type": "string", "description": "Input prompt"},
                            "model": {"type": "string", "description": "Model name"},
                            "max_tokens": {"type": "integer", "description": "Maximum tokens"}
                        },
                        "required": ["prompt"]
                    }),
                    required_permissions: vec!["ai.execute".to_string()],
                },
                ToolDefinition {
                    id: "pinecone.query".to_string(),
                    name: "Pinecone Query".to_string(),
                    description: "Query vectors in Pinecone".to_string(),
                    category: "vector-db".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "vector": {"type": "array", "items": {"type": "number"}, "description": "Query vector"},
                            "top_k": {"type": "integer", "description": "Number of results"},
                            "namespace": {"type": "string", "description": "Namespace"}
                        },
                        "required": ["vector"]
                    }),
                    required_permissions: vec!["database.read".to_string()],
                },
            ],
        }
    }

    /// Database Tools (10+)
    fn get_database_tools(&self) -> ToolCategory {
        ToolCategory {
            name: "database".to_string(),
            description: "Database management and query tools".to_string(),
            tools: vec![
                ToolDefinition {
                    id: "postgres.query".to_string(),
                    name: "PostgreSQL Query".to_string(),
                    description: "Execute SQL query on PostgreSQL".to_string(),
                    category: "sql".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "query": {"type": "string", "description": "SQL query"},
                            "params": {"type": "array", "description": "Query parameters"},
                            "database": {"type": "string", "description": "Database name"}
                        },
                        "required": ["query"]
                    }),
                    required_permissions: vec!["database.read".to_string()],
                },
                ToolDefinition {
                    id: "mysql.query".to_string(),
                    name: "MySQL Query".to_string(),
                    description: "Execute SQL query on MySQL".to_string(),
                    category: "sql".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "query": {"type": "string", "description": "SQL query"},
                            "params": {"type": "array", "description": "Query parameters"},
                            "database": {"type": "string", "description": "Database name"}
                        },
                        "required": ["query"]
                    }),
                    required_permissions: vec!["database.read".to_string()],
                },
                ToolDefinition {
                    id: "mongodb.find".to_string(),
                    name: "MongoDB Find".to_string(),
                    description: "Find documents in MongoDB".to_string(),
                    category: "nosql".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "collection": {"type": "string", "description": "Collection name"},
                            "query": {"type": "object", "description": "Query filter"},
                            "limit": {"type": "integer", "description": "Limit results"}
                        },
                        "required": ["collection"]
                    }),
                    required_permissions: vec!["database.read".to_string()],
                },
            ],
        }
    }

    /// Cloud Services (15+)
    fn get_cloud_tools(&self) -> ToolCategory {
        ToolCategory {
            name: "cloud".to_string(),
            description: "Cloud service management tools".to_string(),
            tools: vec![
                ToolDefinition {
                    id: "aws.s3.upload".to_string(),
                    name: "AWS S3 Upload".to_string(),
                    description: "Upload file to AWS S3".to_string(),
                    category: "aws".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "bucket": {"type": "string", "description": "S3 bucket name"},
                            "key": {"type": "string", "description": "Object key"},
                            "file_path": {"type": "string", "description": "Local file path"}
                        },
                        "required": ["bucket", "key", "file_path"]
                    }),
                    required_permissions: vec!["cloud.write".to_string()],
                },
                ToolDefinition {
                    id: "vercel.deploy".to_string(),
                    name: "Vercel Deploy".to_string(),
                    description: "Deploy project to Vercel".to_string(),
                    category: "vercel".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "project": {"type": "string", "description": "Project name"},
                            "directory": {"type": "string", "description": "Directory to deploy"},
                            "environment": {"type": "string", "enum": ["production", "preview"], "description": "Environment"}
                        },
                        "required": ["project"]
                    }),
                    required_permissions: vec!["cloud.write".to_string()],
                },
            ],
        }
    }

    /// DevOps Tools (10+)
    fn get_devops_tools(&self) -> ToolCategory {
        ToolCategory {
            name: "devops".to_string(),
            description: "DevOps and infrastructure tools".to_string(),
            tools: vec![
                ToolDefinition {
                    id: "docker.build".to_string(),
                    name: "Docker Build".to_string(),
                    description: "Build Docker image".to_string(),
                    category: "docker".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "tag": {"type": "string", "description": "Image tag"},
                            "dockerfile": {"type": "string", "description": "Dockerfile path"},
                            "context": {"type": "string", "description": "Build context"}
                        },
                        "required": ["tag"]
                    }),
                    required_permissions: vec!["system.execute".to_string()],
                },
                ToolDefinition {
                    id: "kubernetes.deploy".to_string(),
                    name: "Kubernetes Deploy".to_string(),
                    description: "Deploy to Kubernetes cluster".to_string(),
                    category: "kubernetes".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "manifest": {"type": "string", "description": "K8s manifest path"},
                            "namespace": {"type": "string", "description": "Namespace"},
                            "context": {"type": "string", "description": "K8s context"}
                        },
                        "required": ["manifest"]
                    }),
                    required_permissions: vec!["cloud.write".to_string()],
                },
            ],
        }
    }

    /// Communication Tools (10+)
    fn get_communication_tools(&self) -> ToolCategory {
        ToolCategory {
            name: "communication".to_string(),
            description: "Communication and notification tools".to_string(),
            tools: vec![
                ToolDefinition {
                    id: "slack.send".to_string(),
                    name: "Slack Send".to_string(),
                    description: "Send message to Slack channel".to_string(),
                    category: "slack".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "channel": {"type": "string", "description": "Channel name"},
                            "message": {"type": "string", "description": "Message text"},
                            "blocks": {"type": "array", "description": "Block kit blocks"}
                        },
                        "required": ["channel", "message"]
                    }),
                    required_permissions: vec!["communication.write".to_string()],
                },
                ToolDefinition {
                    id: "email.send".to_string(),
                    name: "Email Send".to_string(),
                    description: "Send email message".to_string(),
                    category: "email".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "to": {"type": "string", "description": "Recipient email"},
                            "subject": {"type": "string", "description": "Email subject"},
                            "body": {"type": "string", "description": "Email body"},
                            "html": {"type": "boolean", "description": "HTML format"}
                        },
                        "required": ["to", "subject", "body"]
                    }),
                    required_permissions: vec!["communication.write".to_string()],
                },
            ],
        }
    }

    /// Productivity Tools (10+)
    fn get_productivity_tools(&self) -> ToolCategory {
        ToolCategory {
            name: "productivity".to_string(),
            description: "Productivity and project management tools".to_string(),
            tools: vec![
                ToolDefinition {
                    id: "notion.create_page".to_string(),
                    name: "Notion Create Page".to_string(),
                    description: "Create page in Notion".to_string(),
                    category: "notion".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "parent_id": {"type": "string", "description": "Parent page ID"},
                            "title": {"type": "string", "description": "Page title"},
                            "content": {"type": "string", "description": "Page content"}
                        },
                        "required": ["parent_id", "title"]
                    }),
                    required_permissions: vec!["productivity.write".to_string()],
                },
                ToolDefinition {
                    id: "jira.create_issue".to_string(),
                    name: "Jira Create Issue".to_string(),
                    description: "Create issue in Jira".to_string(),
                    category: "jira".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "project": {"type": "string", "description": "Project key"},
                            "summary": {"type": "string", "description": "Issue summary"},
                            "description": {"type": "string", "description": "Issue description"},
                            "issue_type": {"type": "string", "description": "Issue type"}
                        },
                        "required": ["project", "summary"]
                    }),
                    required_permissions: vec!["productivity.write".to_string()],
                },
            ],
        }
    }

    /// Security Tools (5+)
    fn get_security_tools(&self) -> ToolCategory {
        ToolCategory {
            name: "security".to_string(),
            description: "Security scanning and management tools".to_string(),
            tools: vec![
                ToolDefinition {
                    id: "snyk.scan".to_string(),
                    name: "Snyk Scan".to_string(),
                    description: "Scan for vulnerabilities with Snyk".to_string(),
                    category: "scanning".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "description": "Project path"},
                            "severity": {"type": "string", "enum": ["low", "medium", "high", "critical"], "description": "Minimum severity"}
                        },
                        "required": ["path"]
                    }),
                    required_permissions: vec!["security.scan".to_string()],
                },
                ToolDefinition {
                    id: "vault.read_secret".to_string(),
                    name: "Vault Read Secret".to_string(),
                    description: "Read secret from HashiCorp Vault".to_string(),
                    category: "secrets".to_string(),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "description": "Secret path"},
                            "version": {"type": "integer", "description": "Secret version"}
                        },
                        "required": ["path"]
                    }),
                    required_permissions: vec!["secrets.read".to_string()],
                },
            ],
        }
    }

    /// Execute a tool by ID
    pub async fn execute_tool(&self, tool_id: &str, params: Value) -> Result<Value> {
        let start = Instant::now();
        
        // Update stats
        {
            let mut stats = self.execution_stats.write().await;
            stats.total_calls += 1;
        }

        // Get tool from registry
        let tool = self.registry.get(tool_id).await
            .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", tool_id))?;

        // Execute tool
        let result = tool.call(params).await;
        
        // Update stats based on result
        {
            let mut stats = self.execution_stats.write().await;
            let duration = start.elapsed().as_millis() as u64;
            stats.total_duration_ms += duration;
            
            match &result {
                Ok(_) => stats.successful_calls += 1,
                Err(_) => stats.failed_calls += 1,
            }
        }

        result
    }

    /// Get tool categories
    pub async fn get_categories(&self) -> Vec<String> {
        let categories = self.tool_categories.read().await;
        categories.keys().cloned().collect()
    }

    /// Get tools in category
    pub async fn get_tools_in_category(&self, category: &str) -> Option<Vec<String>> {
        let categories = self.tool_categories.read().await;
        categories.get(category).cloned()
    }

    /// Every id currently present in the executable registry.
    ///
    /// This is the authoritative "what is registered" answer. Any count
    /// reported elsewhere must equal `self.registered_ids().await.len()`.
    pub async fn registered_ids(&self) -> Vec<String> {
        let categories = self.tool_categories.read().await;
        let mut ids: Vec<String> = categories.values().flatten().cloned().collect();
        ids.sort();
        ids.dedup();
        ids
    }

    /// Get execution statistics
    pub async fn get_stats(&self) -> (u64, u64, u64, u64) {
        let stats = self.execution_stats.read().await;
        (stats.total_calls, stats.successful_calls, stats.failed_calls, stats.total_duration_ms)
    }

    /// List all available tools
    pub async fn list_tools(&self) -> Vec<ToolDescriptor> {
        self.registry.list().await
    }

    /// Get tool count
    pub async fn tool_count(&self) -> usize {
        self.registry.len().await
    }
}

/// Built-in tool implementation
#[derive(Debug, Clone)]
struct BuiltinTool {
    definition: ToolDefinition,
    /// The policy this tool is subject to and where its evidence goes.
    runtime: crate::evidence::ToolRuntime,
}

impl BuiltinTool {
    fn with_runtime(definition: ToolDefinition, runtime: crate::evidence::ToolRuntime) -> Self {
        Self { definition, runtime }
    }
}

#[async_trait]
impl Tool for BuiltinTool {
    fn id(&self) -> &str {
        &self.definition.id
    }

    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: self.definition.id.clone(),
            transport: "builtin".to_string(),
            command: "internal".to_string(),
            env_keys: Vec::new(),
            description: Some(self.definition.description.clone()),
        }
    }

    async fn call(&self, params: Value) -> Result<Value> {
        // Dispatch to a real executor through the permission gate, or fail
        // closed.
        //
        // This method used to sleep 10 ms and return
        // `{"success": true, "message": "Tool X executed successfully"}`
        // without performing any operation at all. Every one of the 27 bridge
        // tools reported success for work it had not done. That is the single
        // most dangerous pattern in the repository: a caller cannot distinguish
        // a fabricated success from a real one, so the fabrication propagates
        // into whatever the caller does next.
        //
        // See docs/governance/TOOL_CATALOGUE_TRUTH_TABLE.md §1.1.
        let context = crate::real_tools::ToolContext {
            working_directory: std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from(".")),
            environment: std::env::vars().collect(),
            timeout: std::time::Duration::from_secs(30),
            session_id: None,
            actor: None,
            approval_required: false,
        };

        let outcome =
            crate::real_tools::dispatch(&self.definition.id, params, &context, &self.runtime).await;
        let result = outcome.result?;

        Ok(serde_json::json!({
            "tool": self.definition.id,
            "category": self.definition.category,
            "executed": true,
            "success": result.success,
            "output": result.output,
            "bound_operation": result.evidence.bound_operation,
            "risk": result.evidence.risk,
            "approval_decision": result.evidence.approval_decision,
            "invocation_id": result.evidence.invocation_id,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn registered_ids(bridge: &EnhancedMcpBridge) -> std::collections::HashSet<String> {
        let mut set = std::collections::HashSet::new();
        for category in bridge.get_categories().await {
            for id in bridge.get_tools_in_category(&category).await.unwrap_or_default() {
                set.insert(id);
            }
        }
        set
    }

    /// The bridge registers only definitions that have a real executor.
    ///
    /// Replaces `assert!(count > 100)`. Tool quantity is not a correctness
    /// invariant; the invariant that matters is that everything registered can
    /// actually run.
    #[tokio::test]
    async fn bridge_registers_only_executable_tools() {
        let bridge = EnhancedMcpBridge::new();
        let registered = bridge.initialize_with_builtin_tools().await.unwrap();

        assert_eq!(bridge.tool_count().await, registered);

        for id in registered_ids(&bridge).await {
            assert!(
                crate::real_tools::get_real_tool(&id).is_some(),
                "`{id}` was registered as executable but has no real executor"
            );
        }
    }

    /// Definition-only entries are preserved as metadata and never registered.
    #[tokio::test]
    async fn definition_only_entries_are_not_registered() {
        let bridge = EnhancedMcpBridge::new();
        bridge.initialize_with_builtin_tools().await.unwrap();
        let registered = registered_ids(&bridge).await;

        for definition in bridge.all_definitions() {
            let has_executor = crate::real_tools::get_real_tool(&definition.id).is_some();
            assert_eq!(
                registered.contains(&definition.id),
                has_executor,
                "`{}`: registered={} but has_executor={}",
                definition.id,
                registered.contains(&definition.id),
                has_executor
            );
        }
    }

    /// The headline invariant: nothing in this bridge reports success for work
    /// it did not perform.
    ///
    /// The previous implementation answered every one of the 27 definitions
    /// with `{"success": true}` after a 10 ms sleep.
    #[tokio::test]
    async fn bridge_never_fabricates_success() {
        let bridge = EnhancedMcpBridge::new();
        bridge.initialize_with_builtin_tools().await.unwrap();

        // A definition with no executor must fail loudly, not succeed.
        let err = bridge
            .execute_tool("docker.build", serde_json::json!({}))
            .await
            .expect_err("a definition-only tool must not report success");
        let msg = err.to_string();
        assert!(
            msg.contains("not found") || msg.contains("unsupported") || msg.contains("no executor"),
            "expected a typed refusal, got: {msg}"
        );
    }

    /// Unknown ids fail closed.
    #[tokio::test]
    async fn unknown_tool_id_fails_closed() {
        let bridge = EnhancedMcpBridge::new();
        bridge.initialize_with_builtin_tools().await.unwrap();
        assert!(
            bridge
                .execute_tool("no.such.tool", serde_json::json!({}))
                .await
                .is_err()
        );
    }

    /// A cross-operation substitution fails through the bridge too.
    ///
    /// A permissive gate is used deliberately: the gate is checked *before* the
    /// executor, so a restrictive gate would mask the binding violation with a
    /// permission denial. Both refusals are correct, but this test is about the
    /// binding.
    #[tokio::test]
    async fn bridge_git_commit_cannot_push() {
        let bridge = EnhancedMcpBridge::with_runtime(
            crate::evidence::ToolRuntime::permissive_without_evidence(),
        );
        bridge.initialize_with_builtin_tools().await.unwrap();

        let err = bridge
            .execute_tool("git.commit", serde_json::json!({ "subcommand": "push" }))
            .await
            .expect_err("git.commit must not push through the bridge");
        assert!(err.to_string().contains("may not perform"), "{err}");
    }

    /// The gate is consulted before the executor, and a refusal records the
    /// decision without running anything.
    #[tokio::test]
    async fn bridge_gate_refuses_before_the_executor_runs() {
        let bridge = EnhancedMcpBridge::new(); // restrictive by default
        bridge.initialize_with_builtin_tools().await.unwrap();

        // `git.commit` is GitWrite: refused without approval.
        let err = bridge
            .execute_tool("git.commit", serde_json::json!({ "message": "must not run" }))
            .await
            .expect_err("an unapproved GitWrite tool must not run");
        assert!(err.to_string().contains("permission denied"), "{err}");

        // And the refusal is counted as a failure, not a success.
        let (_total, _ok, failed, _dur) = bridge.get_stats().await;
        assert!(failed >= 1, "a refusal must not be counted as a success");
    }

    /// Reads pass the default gate through the bridge.
    #[tokio::test]
    async fn bridge_default_gate_permits_reads() {
        let bridge = EnhancedMcpBridge::new();
        bridge.initialize_with_builtin_tools().await.unwrap();

        let result = bridge
            .execute_tool("fs.read", serde_json::json!({ "path": "Cargo.toml" }))
            .await
            .expect("reads are permitted by the default gate");
        assert_eq!(result["executed"], true);
        assert!(result["approval_decision"]
            .as_str()
            .unwrap_or_default()
            .starts_with("allow: "));
    }

    /// The definitions themselves are still available as metadata.
    #[tokio::test]
    async fn definitions_are_preserved_as_metadata() {
        let bridge = EnhancedMcpBridge::new();
        let definitions = bridge.all_definitions();
        assert!(!definitions.is_empty(), "schemas must be preserved");
        for definition in &definitions {
            assert!(
                definition.parameters.is_object(),
                "`{}` lost its parameter schema",
                definition.id
            );
        }
    }
}
