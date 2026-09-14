use serde_json::json;
use crate::plugin_marketplace::{PluginDefinition, PluginPricing, PluginExecution, PluginPermission, PluginMarketplaceInfo};

/// Generate a list of 25 pre-shipped plugins
pub fn get_preshipped_plugins() -> Vec<PluginDefinition> {
    vec![
        // 1. AI Model Provider Plugin
        create_ai_model_provider_plugin(),
        // 2. File Explorer Plugin
        create_file_explorer_plugin(),
        // 3. Git Integration Plugin
        create_git_integration_plugin(),
        // 4. Database Manager Plugin
        create_database_manager_plugin(),
        // 5. Theme Studio Plugin
        create_theme_studio_plugin(),
        // 6. Code Formatter Plugin
        create_code_formatter_plugin(),
        // 7. Terminal Emulator Plugin
        create_terminal_emulator_plugin(),
        // 8. API Tester Plugin
        create_api_tester_plugin(),
        // 9. Docker Manager Plugin
        create_docker_manager_plugin(),
        // 10. Cloud Deployer Plugin
        create_cloud_deployer_plugin(),
        // 11. Performance Monitor Plugin
        create_performance_monitor_plugin(),
        // 12. Security Auditor Plugin
        create_security_auditor_plugin(),
        // 13. Database Designer Plugin
        create_database_designer_plugin(),
        // 14. GraphQL Playground Plugin
        create_graphql_playground_plugin(),
        // 15. WebSocket Tester Plugin
        create_websocket_tester_plugin(),
        // 16. Markdown Editor Plugin
        create_markdown_editor_plugin(),
        // 17. Image Optimizer Plugin
        create_image_optimizer_plugin(),
        // 18. CSV/JSON Viewer Plugin
        create_csv_json_viewer_plugin(),
        // 19. Regex Tester Plugin
        create_regex_tester_plugin(),
        // 20. Color Picker Plugin
        create_color_picker_plugin(),
        // 21. Lorem Ipsum Generator Plugin
        create_lorem_ipsum_generator_plugin(),
        // 22. UUID Generator Plugin
        create_uuid_generator_plugin(),
        // 23. Hash Generator Plugin
        create_hash_generator_plugin(),
        // 24. Base64 Encoder/Decoder Plugin
        create_base64_plugin(),
        // 25. JSON Formatter Plugin
        create_json_formatter_plugin(),
    ]
}

fn create_ai_model_provider_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "ai-model-provider".to_string(),
        name: "AI Model Provider".to_string(),
        version: "1.0.0".to_string(),
        description: "Multi-model AI provider with OpenAI, Anthropic, and DeepSeek support".to_string(),
        author: "ZylCode Team".to_string(),
        category: "ai-models".to_string(),
        tags: vec!["ai".to_string(), "models".to_string(), "openai".to_string(), "deepseek".to_string()],
        pricing: PluginPricing {
            pricing_type: "freemium".to_string(),
            price: 0.0,
            currency: "USD".to_string(),
            period: None,
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "default_model": { "type": "string", "default": "gpt-4", "description": "Default AI model" },
                "api_keys": { "type": "object", "description": "API keys for different providers" }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: true,
            timeout_ms: 30000,
        },
        permissions: vec![
            PluginPermission { resource: "ai-models".to_string(), actions: vec!["access".to_string(), "execute".to_string()] },
            PluginPermission { resource: "network".to_string(), actions: vec!["request".to_string()] },
        ],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/ai-model-provider".to_string(),
            downloads: 15000,
            rating: 4.8,
            reviews: 245,
        },
    }
}

fn create_file_explorer_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "file-explorer".to_string(),
        name: "File Explorer".to_string(),
        version: "1.2.0".to_string(),
        description: "Advanced file explorer with preview and search capabilities".to_string(),
        author: "ZylCode Team".to_string(),
        category: "productivity".to_string(),
        tags: vec!["files".to_string(), "explorer".to_string(), "preview".to_string()],
        pricing: PluginPricing {
            pricing_type: "free".to_string(),
            price: 0.0,
            currency: "USD".to_string(),
            period: None,
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "show_hidden": { "type": "boolean", "default": false, "description": "Show hidden files" },
                "preview_enabled": { "type": "boolean", "default": true, "description": "Enable file preview" }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: false,
            timeout_ms: 10000,
        },
        permissions: vec![
            PluginPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string()] },
        ],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/file-explorer".to_string(),
            downloads: 25000,
            rating: 4.6,
            reviews: 189,
        },
    }
}

fn create_git_integration_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "git-integration".to_string(),
        name: "Git Integration".to_string(),
        version: "2.0.0".to_string(),
        description: "Advanced Git integration with visual diff and branch management".to_string(),
        author: "ZylCode Team".to_string(),
        category: "development".to_string(),
        tags: vec!["git".to_string(), "version-control".to_string(), "diff".to_string()],
        pricing: PluginPricing {
            pricing_type: "free".to_string(),
            price: 0.0,
            currency: "USD".to_string(),
            period: None,
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "auto_fetch": { "type": "boolean", "default": true, "description": "Automatically fetch remote changes" },
                "visual_diff": { "type": "boolean", "default": true, "description": "Enable visual diff viewer" }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: true,
            timeout_ms: 30000,
        },
        permissions: vec![
            PluginPermission { resource: "git".to_string(), actions: vec!["read".to_string(), "write".to_string(), "execute".to_string()] },
            PluginPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string()] },
        ],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/git-integration".to_string(),
            downloads: 30000,
            rating: 4.9,
            reviews: 312,
        },
    }
}

fn create_database_manager_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "database-manager".to_string(),
        name: "Database Manager".to_string(),
        version: "1.5.0".to_string(),
        description: "Database management with query editor and visualization".to_string(),
        author: "ZylCode Team".to_string(),
        category: "database".to_string(),
        tags: vec!["database".to_string(), "sql".to_string(), "query".to_string()],
        pricing: PluginPricing {
            pricing_type: "freemium".to_string(),
            price: 9.99,
            currency: "USD".to_string(),
            period: Some("monthly".to_string()),
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "connections": { "type": "array", "description": "Database connections" },
                "query_history": { "type": "boolean", "default": true, "description": "Save query history" }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: false,
            timeout_ms: 60000,
        },
        permissions: vec![
            PluginPermission { resource: "database".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
            PluginPermission { resource: "network".to_string(), actions: vec!["request".to_string()] },
        ],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/database-manager".to_string(),
            downloads: 12000,
            rating: 4.7,
            reviews: 156,
        },
    }
}

fn create_theme_studio_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "theme-studio".to_string(),
        name: "Theme Studio".to_string(),
        version: "1.0.0".to_string(),
        description: "Create and customize themes with visual editor".to_string(),
        author: "ZylCode Team".to_string(),
        category: "creative".to_string(),
        tags: vec!["themes".to_string(), "design".to_string(), "customization".to_string()],
        pricing: PluginPricing {
            pricing_type: "free".to_string(),
            price: 0.0,
            currency: "USD".to_string(),
            period: None,
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "live_preview": { "type": "boolean", "default": true, "description": "Enable live preview" },
                "export_formats": { "type": "array", "items": {"type": "string"}, "default": ["css", "json"], "description": "Export formats" }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: false,
            timeout_ms: 15000,
        },
        permissions: vec![
            PluginPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
            PluginPermission { resource: "ui".to_string(), actions: vec!["render".to_string()] },
        ],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/theme-studio".to_string(),
            downloads: 8000,
            rating: 4.5,
            reviews: 98,
        },
    }
}

fn create_code_formatter_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "code-formatter".to_string(),
        name: "Code Formatter".to_string(),
        version: "1.0.0".to_string(),
        description: "Format code with Prettier, ESLint, and more".to_string(),
        author: "ZylCode Team".to_string(),
        category: "development".to_string(),
        tags: vec!["formatter".to_string(), "prettier".to_string(), "eslint".to_string()],
        pricing: PluginPricing {
            pricing_type: "free".to_string(),
            price: 0.0,
            currency: "USD".to_string(),
            period: None,
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "default_formatter": { "type": "string", "enum": ["prettier", "eslint", "black", "rustfmt"], "default": "prettier" },
                "format_on_save": { "type": "boolean", "default": true }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: false,
            timeout_ms: 5000,
        },
        permissions: vec![
            PluginPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
        ],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/code-formatter".to_string(),
            downloads: 20000,
            rating: 4.7,
            reviews: 178,
        },
    }
}

fn create_terminal_emulator_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "terminal-emulator".to_string(),
        name: "Terminal Emulator".to_string(),
        version: "1.0.0".to_string(),
        description: "Integrated terminal with multiple tabs and shells".to_string(),
        author: "ZylCode Team".to_string(),
        category: "development".to_string(),
        tags: vec!["terminal".to_string(), "shell".to_string(), "console".to_string()],
        pricing: PluginPricing {
            pricing_type: "free".to_string(),
            price: 0.0,
            currency: "USD".to_string(),
            period: None,
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "default_shell": { "type": "string", "enum": ["bash", "zsh", "powershell", "cmd"], "default": "bash" },
                "max_tabs": { "type": "number", "default": 10 }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: false,
            timeout_ms: 5000,
        },
        permissions: vec![
            PluginPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
            PluginPermission { resource: "process".to_string(), actions: vec!["execute".to_string()] },
        ],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/terminal-emulator".to_string(),
            downloads: 18000,
            rating: 4.6,
            reviews: 167,
        },
    }
}

fn create_api_tester_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "api-tester".to_string(),
        name: "API Tester".to_string(),
        version: "1.0.0".to_string(),
        description: "Test APIs with Postman-like interface".to_string(),
        author: "ZylCode Team".to_string(),
        category: "development".to_string(),
        tags: vec!["api".to_string(), "testing".to_string(), "http".to_string()],
        pricing: PluginPricing {
            pricing_type: "free".to_string(),
            price: 0.0,
            currency: "USD".to_string(),
            period: None,
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "save_history": { "type": "boolean", "default": true },
                "default_headers": { "type": "object" }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: false,
            timeout_ms: 30000,
        },
        permissions: vec![
            PluginPermission { resource: "network".to_string(), actions: vec!["request".to_string()] },
            PluginPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
        ],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/api-tester".to_string(),
            downloads: 22000,
            rating: 4.8,
            reviews: 198,
        },
    }
}

fn create_docker_manager_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "docker-manager".to_string(),
        name: "Docker Manager".to_string(),
        version: "1.0.0".to_string(),
        description: "Manage Docker containers, images, and compose".to_string(),
        author: "ZylCode Team".to_string(),
        category: "devops".to_string(),
        tags: vec!["docker".to_string(), "containers".to_string(), "devops".to_string()],
        pricing: PluginPricing {
            pricing_type: "free".to_string(),
            price: 0.0,
            currency: "USD".to_string(),
            period: None,
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "docker_host": { "type": "string", "default": "unix:///var/run/docker.sock" },
                "auto_refresh": { "type": "boolean", "default": true }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: false,
            timeout_ms: 15000,
        },
        permissions: vec![
            PluginPermission { resource: "docker".to_string(), actions: vec!["read".to_string(), "write".to_string(), "execute".to_string()] },
        ],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/docker-manager".to_string(),
            downloads: 15000,
            rating: 4.7,
            reviews: 134,
        },
    }
}

fn create_cloud_deployer_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "cloud-deployer".to_string(),
        name: "Cloud Deployer".to_string(),
        version: "1.0.0".to_string(),
        description: "Deploy to AWS, GCP, Azure, Vercel, Netlify".to_string(),
        author: "ZylCode Team".to_string(),
        category: "cloud".to_string(),
        tags: vec!["cloud".to_string(), "deploy".to_string(), "aws".to_string(), "gcp".to_string(), "azure".to_string()],
        pricing: PluginPricing {
            pricing_type: "freemium".to_string(),
            price: 4.99,
            currency: "USD".to_string(),
            period: Some("monthly".to_string()),
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "default_provider": { "type": "string", "enum": ["aws", "gcp", "azure", "vercel", "netlify"], "default": "vercel" },
                "auto_deploy": { "type": "boolean", "default": false }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: false,
            timeout_ms: 60000,
        },
        permissions: vec![
            PluginPermission { resource: "cloud".to_string(), actions: vec!["deploy".to_string(), "configure".to_string()] },
            PluginPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string()] },
        ],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/cloud-deployer".to_string(),
            downloads: 10000,
            rating: 4.6,
            reviews: 112,
        },
    }
}

fn create_performance_monitor_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "performance-monitor".to_string(),
        name: "Performance Monitor".to_string(),
        version: "1.0.0".to_string(),
        description: "Monitor CPU, memory, and network usage".to_string(),
        author: "ZylCode Team".to_string(),
        category: "devops".to_string(),
        tags: vec!["performance".to_string(), "monitoring".to_string(), "metrics".to_string()],
        pricing: PluginPricing {
            pricing_type: "free".to_string(),
            price: 0.0,
            currency: "USD".to_string(),
            period: None,
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "refresh_interval": { "type": "number", "default": 1000 },
                "alert_threshold": { "type": "number", "default": 80 }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: true,
            timeout_ms: 5000,
        },
        permissions: vec![
            PluginPermission { resource: "system".to_string(), actions: vec!["read".to_string()] },
        ],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/performance-monitor".to_string(),
            downloads: 14000,
            rating: 4.5,
            reviews: 89,
        },
    }
}

fn create_security_auditor_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "security-auditor".to_string(),
        name: "Security Auditor".to_string(),
        version: "1.0.0".to_string(),
        description: "Audit code for security vulnerabilities".to_string(),
        author: "ZylCode Team".to_string(),
        category: "security".to_string(),
        tags: vec!["security".to_string(), "audit".to_string(), "vulnerabilities".to_string()],
        pricing: PluginPricing {
            pricing_type: "freemium".to_string(),
            price: 9.99,
            currency: "USD".to_string(),
            period: Some("monthly".to_string()),
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "scan_depth": { "type": "string", "enum": ["basic", "deep", "comprehensive"], "default": "deep" },
                "auto_fix": { "type": "boolean", "default": false }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: false,
            timeout_ms: 120000,
        },
        permissions: vec![
            PluginPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string()] },
            PluginPermission { resource: "security".to_string(), actions: vec!["scan".to_string()] },
        ],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/security-auditor".to_string(),
            downloads: 9000,
            rating: 4.8,
            reviews: 145,
        },
    }
}

fn create_database_designer_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "database-designer".to_string(),
        name: "Database Designer".to_string(),
        version: "1.0.0".to_string(),
        description: "Design database schemas with visual editor".to_string(),
        author: "ZylCode Team".to_string(),
        category: "database".to_string(),
        tags: vec!["database".to_string(), "schema".to_string(), "design".to_string()],
        pricing: PluginPricing {
            pricing_type: "free".to_string(),
            price: 0.0,
            currency: "USD".to_string(),
            period: None,
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "db_type": { "type": "string", "enum": ["postgresql", "mysql", "sqlite", "mongodb"], "default": "postgresql" },
                "auto_generate_migrations": { "type": "boolean", "default": true }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: false,
            timeout_ms: 30000,
        },
        permissions: vec![
            PluginPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
            PluginPermission { resource: "database".to_string(), actions: vec!["read".to_string()] },
        ],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/database-designer".to_string(),
            downloads: 11000,
            rating: 4.6,
            reviews: 123,
        },
    }
}

fn create_graphql_playground_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "graphql-playground".to_string(),
        name: "GraphQL Playground".to_string(),
        version: "1.0.0".to_string(),
        description: "Interactive GraphQL IDE".to_string(),
        author: "ZylCode Team".to_string(),
        category: "development".to_string(),
        tags: vec!["graphql".to_string(), "api".to_string(), "playground".to_string()],
        pricing: PluginPricing {
            pricing_type: "free".to_string(),
            price: 0.0,
            currency: "USD".to_string(),
            period: None,
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "endpoint": { "type": "string", "default": "/graphql" },
                "auto_introspect": { "type": "boolean", "default": true }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: false,
            timeout_ms: 10000,
        },
        permissions: vec![
            PluginPermission { resource: "network".to_string(), actions: vec!["request".to_string()] },
        ],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/graphql-playground".to_string(),
            downloads: 13000,
            rating: 4.7,
            reviews: 156,
        },
    }
}

fn create_websocket_tester_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "websocket-tester".to_string(),
        name: "WebSocket Tester".to_string(),
        version: "1.0.0".to_string(),
        description: "Test WebSocket connections and messages".to_string(),
        author: "ZylCode Team".to_string(),
        category: "development".to_string(),
        tags: vec!["websocket".to_string(), "testing".to_string(), "real-time".to_string()],
        pricing: PluginPricing {
            pricing_type: "free".to_string(),
            price: 0.0,
            currency: "USD".to_string(),
            period: None,
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "auto_reconnect": { "type": "boolean", "default": true },
                "message_history": { "type": "boolean", "default": true }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: false,
            timeout_ms: 10000,
        },
        permissions: vec![
            PluginPermission { resource: "network".to_string(), actions: vec!["request".to_string(), "listen".to_string()] },
        ],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/websocket-tester".to_string(),
            downloads: 8000,
            rating: 4.5,
            reviews: 89,
        },
    }
}

fn create_markdown_editor_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "markdown-editor".to_string(),
        name: "Markdown Editor".to_string(),
        version: "1.0.0".to_string(),
        description: "Rich markdown editor with preview".to_string(),
        author: "ZylCode Team".to_string(),
        category: "productivity".to_string(),
        tags: vec!["markdown".to_string(), "editor".to_string(), "preview".to_string()],
        pricing: PluginPricing {
            pricing_type: "free".to_string(),
            price: 0.0,
            currency: "USD".to_string(),
            period: None,
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "live_preview": { "type": "boolean", "default": true },
                "export_formats": { "type": "array", "items": {"type": "string"}, "default": ["html", "pdf"] }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: false,
            timeout_ms: 5000,
        },
        permissions: vec![
            PluginPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
        ],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/markdown-editor".to_string(),
            downloads: 16000,
            rating: 4.6,
            reviews: 145,
        },
    }
}

fn create_image_optimizer_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "image-optimizer".to_string(),
        name: "Image Optimizer".to_string(),
        version: "1.0.0".to_string(),
        description: "Optimize images for web performance".to_string(),
        author: "ZylCode Team".to_string(),
        category: "productivity".to_string(),
        tags: vec!["image".to_string(), "optimization".to_string(), "compression".to_string()],
        pricing: PluginPricing {
            pricing_type: "free".to_string(),
            price: 0.0,
            currency: "USD".to_string(),
            period: None,
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "quality": { "type": "number", "default": 80 },
                "formats": { "type": "array", "items": {"type": "string"}, "default": ["webp", "avif", "png", "jpg"] }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: false,
            timeout_ms: 30000,
        },
        permissions: vec![
            PluginPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
        ],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/image-optimizer".to_string(),
            downloads: 9000,
            rating: 4.5,
            reviews: 78,
        },
    }
}

fn create_csv_json_viewer_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "csv-json-viewer".to_string(),
        name: "CSV/JSON Viewer".to_string(),
        version: "1.0.0".to_string(),
        description: "View and edit CSV and JSON files".to_string(),
        author: "ZylCode Team".to_string(),
        category: "productivity".to_string(),
        tags: vec!["csv".to_string(), "json".to_string(), "viewer".to_string()],
        pricing: PluginPricing {
            pricing_type: "free".to_string(),
            price: 0.0,
            currency: "USD".to_string(),
            period: None,
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "auto_detect_format": { "type": "boolean", "default": true },
                "max_rows": { "type": "number", "default": 10000 }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: false,
            timeout_ms: 10000,
        },
        permissions: vec![
            PluginPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
        ],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/csv-json-viewer".to_string(),
            downloads: 12000,
            rating: 4.4,
            reviews: 98,
        },
    }
}

fn create_regex_tester_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "regex-tester".to_string(),
        name: "Regex Tester".to_string(),
        version: "1.0.0".to_string(),
        description: "Test and debug regular expressions".to_string(),
        author: "ZylCode Team".to_string(),
        category: "development".to_string(),
        tags: vec!["regex".to_string(), "testing".to_string(), "debugging".to_string()],
        pricing: PluginPricing {
            pricing_type: "free".to_string(),
            price: 0.0,
            currency: "USD".to_string(),
            period: None,
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "flavor": { "type": "string", "enum": ["javascript", "python", "pcre", "posix"], "default": "javascript" },
                "show_matches": { "type": "boolean", "default": true }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: false,
            timeout_ms: 5000,
        },
        permissions: vec![],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/regex-tester".to_string(),
            downloads: 7000,
            rating: 4.3,
            reviews: 67,
        },
    }
}

fn create_color_picker_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "color-picker".to_string(),
        name: "Color Picker".to_string(),
        version: "1.0.0".to_string(),
        description: "Pick and convert colors".to_string(),
        author: "ZylCode Team".to_string(),
        category: "creative".to_string(),
        tags: vec!["color".to_string(), "picker".to_string(), "design".to_string()],
        pricing: PluginPricing {
            pricing_type: "free".to_string(),
            price: 0.0,
            currency: "USD".to_string(),
            period: None,
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "output_format": { "type": "string", "enum": ["hex", "rgb", "hsl", "oklch"], "default": "hex" },
                "color_palette": { "type": "boolean", "default": true }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: false,
            timeout_ms: 5000,
        },
        permissions: vec![
            PluginPermission { resource: "ui".to_string(), actions: vec!["render".to_string()] },
        ],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/color-picker".to_string(),
            downloads: 6000,
            rating: 4.4,
            reviews: 56,
        },
    }
}

fn create_lorem_ipsum_generator_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "lorem-ipsum-generator".to_string(),
        name: "Lorem Ipsum Generator".to_string(),
        version: "1.0.0".to_string(),
        description: "Generate placeholder text".to_string(),
        author: "ZylCode Team".to_string(),
        category: "productivity".to_string(),
        tags: vec!["lorem".to_string(), "placeholder".to_string(), "text".to_string()],
        pricing: PluginPricing {
            pricing_type: "free".to_string(),
            price: 0.0,
            currency: "USD".to_string(),
            period: None,
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "paragraphs": { "type": "number", "default": 3 },
                "language": { "type": "string", "enum": ["latin", "english", "spanish"], "default": "latin" }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: false,
            timeout_ms: 1000,
        },
        permissions: vec![],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/lorem-ipsum-generator".to_string(),
            downloads: 5000,
            rating: 4.2,
            reviews: 45,
        },
    }
}

fn create_uuid_generator_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "uuid-generator".to_string(),
        name: "UUID Generator".to_string(),
        version: "1.0.0".to_string(),
        description: "Generate UUIDs and GUIDs".to_string(),
        author: "ZylCode Team".to_string(),
        category: "development".to_string(),
        tags: vec!["uuid".to_string(), "guid".to_string(), "generator".to_string()],
        pricing: PluginPricing {
            pricing_type: "free".to_string(),
            price: 0.0,
            currency: "USD".to_string(),
            period: None,
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "version": { "type": "string", "enum": ["v1", "v4", "v5", "v7"], "default": "v4" },
                "count": { "type": "number", "default": 1 }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: false,
            timeout_ms: 1000,
        },
        permissions: vec![],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/uuid-generator".to_string(),
            downloads: 8000,
            rating: 4.5,
            reviews: 78,
        },
    }
}

fn create_hash_generator_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "hash-generator".to_string(),
        name: "Hash Generator".to_string(),
        version: "1.0.0".to_string(),
        description: "Generate MD5, SHA1, SHA256 hashes".to_string(),
        author: "ZylCode Team".to_string(),
        category: "security".to_string(),
        tags: vec!["hash".to_string(), "md5".to_string(), "sha".to_string()],
        pricing: PluginPricing {
            pricing_type: "free".to_string(),
            price: 0.0,
            currency: "USD".to_string(),
            period: None,
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "algorithm": { "type": "string", "enum": ["md5", "sha1", "sha256", "sha512"], "default": "sha256" },
                "encoding": { "type": "string", "enum": ["hex", "base64"], "default": "hex" }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: false,
            timeout_ms: 1000,
        },
        permissions: vec![],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/hash-generator".to_string(),
            downloads: 7000,
            rating: 4.4,
            reviews: 67,
        },
    }
}

fn create_base64_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "base64-encoder".to_string(),
        name: "Base64 Encoder/Decoder".to_string(),
        version: "1.0.0".to_string(),
        description: "Encode and decode Base64".to_string(),
        author: "ZylCode Team".to_string(),
        category: "development".to_string(),
        tags: vec!["base64".to_string(), "encoder".to_string(), "decoder".to_string()],
        pricing: PluginPricing {
            pricing_type: "free".to_string(),
            price: 0.0,
            currency: "USD".to_string(),
            period: None,
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "auto_detect": { "type": "boolean", "default": true },
                "url_safe": { "type": "boolean", "default": false }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: false,
            timeout_ms: 1000,
        },
        permissions: vec![],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/base64-encoder".to_string(),
            downloads: 9000,
            rating: 4.5,
            reviews: 89,
        },
    }
}

fn create_json_formatter_plugin() -> PluginDefinition {
    PluginDefinition {
        id: "json-formatter".to_string(),
        name: "JSON Formatter".to_string(),
        version: "1.0.0".to_string(),
        description: "Format, validate, and visualize JSON".to_string(),
        author: "ZylCode Team".to_string(),
        category: "development".to_string(),
        tags: vec!["json".to_string(), "formatter".to_string(), "validator".to_string()],
        pricing: PluginPricing {
            pricing_type: "free".to_string(),
            price: 0.0,
            currency: "USD".to_string(),
            period: None,
        },
        dependencies: vec![],
        config_schema: json!({
            "type": "object",
            "properties": {
                "indent": { "type": "number", "default": 2 },
                "sort_keys": { "type": "boolean", "default": false }
            }
        }),
        execution: PluginExecution {
            runtime: "node".to_string(),
            entry_point: "src/index.js".to_string(),
            background: false,
            timeout_ms: 1000,
        },
        permissions: vec![
            PluginPermission { resource: "clipboard".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
        ],
        marketplace: PluginMarketplaceInfo {
            screenshots: vec![],
            documentation: "docs/README.md".to_string(),
            changelog: "CHANGELOG.md".to_string(),
            support_url: "https://support.zylcode.com".to_string(),
            source_code: "https://github.com/zylcode/json-formatter".to_string(),
            downloads: 11000,
            rating: 4.6,
            reviews: 112,
        },
    }
}
