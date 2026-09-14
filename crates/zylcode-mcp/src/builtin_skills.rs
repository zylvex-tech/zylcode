use serde_json::json;
use crate::skills_system::SkillDefinition;
use crate::skills_system::{SkillDependency, SkillExecution, SkillPermission, MarketplaceInfo};

/// Generate a list of 25 pre-built skills
pub fn get_builtin_skills() -> Vec<SkillDefinition> {
    vec![
        // 1. Code Review Skill
        create_code_review_skill(),
        // 2. Documentation Generator Skill
        create_doc_generator_skill(),
        // 3. Test Generator Skill
        create_test_generator_skill(),
        // 4. Data Analysis Skill
        create_data_analysis_skill(),
        // 5. Security Scanner Skill
        create_security_scanner_skill(),
        // 6. Code Refactoring Skill
        create_code_refactoring_skill(),
        // 7. Performance Optimizer Skill
        create_performance_optimizer_skill(),
        // 8. API Designer Skill
        create_api_designer_skill(),
        // 9. Database Optimizer Skill
        create_database_optimizer_skill(),
        // 10. Cloud Architect Skill
        create_cloud_architect_skill(),
        // 11. CI/CD Pipeline Builder Skill
        create_cicd_builder_skill(),
        // 12. Container Orchestrator Skill
        create_container_orchestrator_skill(),
        // 13. Frontend Builder Skill
        create_frontend_builder_skill(),
        // 14. Backend Builder Skill
        create_backend_builder_skill(),
        // 15. Mobile App Builder Skill
        create_mobile_builder_skill(),
        // 16. AI Model Trainer Skill
        create_ai_model_trainer_skill(),
        // 17. Data Pipeline Builder Skill
        create_data_pipeline_skill(),
        // 18. Microservice Architect Skill
        create_microservice_architect_skill(),
        // 19. GraphQL Generator Skill
        create_graphql_generator_skill(),
        // 20. WebSocket Handler Skill
        create_websocket_handler_skill(),
        // 21. Auth System Builder Skill
        create_auth_system_skill(),
        // 22. Logging & Monitoring Skill
        create_logging_monitoring_skill(),
        // 23. File Converter Skill
        create_file_converter_skill(),
        // 24. Code Translator Skill
        create_code_translator_skill(),
        // 25. Technical Writer Skill
        create_technical_writer_skill(),
    ]
}

fn create_code_review_skill() -> SkillDefinition {
    SkillDefinition {
        id: "code-review".to_string(),
        name: "Code Review".to_string(),
        version: "1.0.0".to_string(),
        description: "Automated code review with AI analysis".to_string(),
        author: "ZylCode Team".to_string(),
        category: "development".to_string(),
        tags: vec!["code-review".to_string(), "ai".to_string(), "quality".to_string()],
        dependencies: vec![
            SkillDependency { name: "git-tools".to_string(), version: ">=1.0.0".to_string(), optional: false },
            SkillDependency { name: "ai-models".to_string(), version: ">=2.0.0".to_string(), optional: false },
        ],
        config_schema: json!({
            "type": "object",
            "properties": {
                "model": { "type": "string", "default": "gpt-4", "description": "AI model to use" },
                "severity_levels": { "type": "array", "items": {"type": "string"}, "default": ["error", "warning", "info"] },
                "auto_fix": { "type": "boolean", "default": false }
            }
        }),
        execution: SkillExecution { runtime: "node".to_string(), entry_point: "src/index.js".to_string(), timeout_ms: 300000, memory_limit_mb: 512 },
        permissions: vec![
            SkillPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string()] },
            SkillPermission { resource: "git".to_string(), actions: vec!["read".to_string(), "status".to_string()] },
            SkillPermission { resource: "ai-models".to_string(), actions: vec!["execute".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 0.0, category: "Development Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}

fn create_doc_generator_skill() -> SkillDefinition {
    SkillDefinition {
        id: "doc-generator".to_string(),
        name: "Documentation Generator".to_string(),
        version: "1.0.0".to_string(),
        description: "Generate documentation from code".to_string(),
        author: "ZylCode Team".to_string(),
        category: "development".to_string(),
        tags: vec!["documentation".to_string(), "ai".to_string(), "generation".to_string()],
        dependencies: vec![SkillDependency { name: "ai-models".to_string(), version: ">=2.0.0".to_string(), optional: false }],
        config_schema: json!({
            "type": "object",
            "properties": {
                "format": { "type": "string", "enum": ["markdown", "html", "pdf"], "default": "markdown" },
                "include_examples": { "type": "boolean", "default": true }
            }
        }),
        execution: SkillExecution { runtime: "node".to_string(), entry_point: "src/index.js".to_string(), timeout_ms: 180000, memory_limit_mb: 256 },
        permissions: vec![
            SkillPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
            SkillPermission { resource: "ai-models".to_string(), actions: vec!["execute".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 0.0, category: "Development Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}

fn create_test_generator_skill() -> SkillDefinition {
    SkillDefinition {
        id: "test-generator".to_string(),
        name: "Test Generator".to_string(),
        version: "1.0.0".to_string(),
        description: "Generate unit tests from code".to_string(),
        author: "ZylCode Team".to_string(),
        category: "development".to_string(),
        tags: vec!["testing".to_string(), "ai".to_string(), "generation".to_string()],
        dependencies: vec![SkillDependency { name: "ai-models".to_string(), version: ">=2.0.0".to_string(), optional: false }],
        config_schema: json!({
            "type": "object",
            "properties": {
                "framework": { "type": "string", "enum": ["jest", "vitest", "mocha", "pytest"], "default": "jest" },
                "coverage_target": { "type": "number", "default": 80 }
            }
        }),
        execution: SkillExecution { runtime: "node".to_string(), entry_point: "src/index.js".to_string(), timeout_ms: 240000, memory_limit_mb: 256 },
        permissions: vec![
            SkillPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
            SkillPermission { resource: "ai-models".to_string(), actions: vec!["execute".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 0.0, category: "Development Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}

fn create_data_analysis_skill() -> SkillDefinition {
    SkillDefinition {
        id: "data-analysis".to_string(),
        name: "Data Analysis".to_string(),
        version: "1.0.0".to_string(),
        description: "Analyze data and generate insights".to_string(),
        author: "ZylCode Team".to_string(),
        category: "ai-ml".to_string(),
        tags: vec!["data".to_string(), "analysis".to_string(), "ai".to_string()],
        dependencies: vec![
            SkillDependency { name: "ai-models".to_string(), version: ">=2.0.0".to_string(), optional: false },
            SkillDependency { name: "data-processing".to_string(), version: ">=1.0.0".to_string(), optional: false },
        ],
        config_schema: json!({
            "type": "object",
            "properties": {
                "analysis_type": { "type": "string", "enum": ["descriptive", "diagnostic", "predictive", "prescriptive"], "default": "descriptive" },
                "output_format": { "type": "string", "enum": ["report", "chart", "table"], "default": "report" }
            }
        }),
        execution: SkillExecution { runtime: "python".to_string(), entry_point: "src/main.py".to_string(), timeout_ms: 600000, memory_limit_mb: 1024 },
        permissions: vec![
            SkillPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string()] },
            SkillPermission { resource: "ai-models".to_string(), actions: vec!["execute".to_string()] },
            SkillPermission { resource: "database".to_string(), actions: vec!["read".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 9.99, category: "AI/ML Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}

fn create_security_scanner_skill() -> SkillDefinition {
    SkillDefinition {
        id: "security-scanner".to_string(),
        name: "Security Scanner".to_string(),
        version: "1.0.0".to_string(),
        description: "Scan code for security vulnerabilities".to_string(),
        author: "ZylCode Team".to_string(),
        category: "security".to_string(),
        tags: vec!["security".to_string(), "scanning".to_string(), "vulnerabilities".to_string()],
        dependencies: vec![SkillDependency { name: "security-tools".to_string(), version: ">=1.0.0".to_string(), optional: false }],
        config_schema: json!({
            "type": "object",
            "properties": {
                "scan_type": { "type": "string", "enum": ["sast", "dast", "sca", "full"], "default": "full" },
                "severity_threshold": { "type": "string", "enum": ["low", "medium", "high", "critical"], "default": "medium" }
            }
        }),
        execution: SkillExecution { runtime: "node".to_string(), entry_point: "src/index.js".to_string(), timeout_ms: 300000, memory_limit_mb: 512 },
        permissions: vec![
            SkillPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string()] },
            SkillPermission { resource: "security".to_string(), actions: vec!["scan".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 19.99, category: "Security Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}

fn create_code_refactoring_skill() -> SkillDefinition {
    SkillDefinition {
        id: "code-refactoring".to_string(),
        name: "Code Refactoring".to_string(),
        version: "1.0.0".to_string(),
        description: "Automated code refactoring with AI suggestions".to_string(),
        author: "ZylCode Team".to_string(),
        category: "development".to_string(),
        tags: vec!["refactoring".to_string(), "ai".to_string(), "optimization".to_string()],
        dependencies: vec![SkillDependency { name: "ai-models".to_string(), version: ">=2.0.0".to_string(), optional: false }],
        config_schema: json!({
            "type": "object",
            "properties": {
                "refactor_type": { "type": "string", "enum": ["extract-method", "rename", "move", "inline", "simplify"], "default": "extract-method" },
                "preserve_behavior": { "type": "boolean", "default": true }
            }
        }),
        execution: SkillExecution { runtime: "node".to_string(), entry_point: "src/index.js".to_string(), timeout_ms: 200000, memory_limit_mb: 256 },
        permissions: vec![
            SkillPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
            SkillPermission { resource: "ai-models".to_string(), actions: vec!["execute".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 4.99, category: "Development Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}

fn create_performance_optimizer_skill() -> SkillDefinition {
    SkillDefinition {
        id: "performance-optimizer".to_string(),
        name: "Performance Optimizer".to_string(),
        version: "1.0.0".to_string(),
        description: "Optimize code for better performance".to_string(),
        author: "ZylCode Team".to_string(),
        category: "optimization".to_string(),
        tags: vec!["performance".to_string(), "optimization".to_string(), "profiling".to_string()],
        dependencies: vec![SkillDependency { name: "ai-models".to_string(), version: ">=2.0.0".to_string(), optional: false }],
        config_schema: json!({
            "type": "object",
            "properties": {
                "target_metric": { "type": "string", "enum": ["latency", "throughput", "memory", "cpu"], "default": "latency" },
                "threshold": { "type": "number", "default": 100 }
            }
        }),
        execution: SkillExecution { runtime: "node".to_string(), entry_point: "src/index.js".to_string(), timeout_ms: 300000, memory_limit_mb: 512 },
        permissions: vec![
            SkillPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
            SkillPermission { resource: "ai-models".to_string(), actions: vec!["execute".to_string()] },
            SkillPermission { resource: "monitoring".to_string(), actions: vec!["read".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 9.99, category: "Optimization Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}

fn create_api_designer_skill() -> SkillDefinition {
    SkillDefinition {
        id: "api-designer".to_string(),
        name: "API Designer".to_string(),
        version: "1.0.0".to_string(),
        description: "Design and generate RESTful APIs".to_string(),
        author: "ZylCode Team".to_string(),
        category: "development".to_string(),
        tags: vec!["api".to_string(), "rest".to_string(), "design".to_string()],
        dependencies: vec![SkillDependency { name: "ai-models".to_string(), version: ">=2.0.0".to_string(), optional: false }],
        config_schema: json!({
            "type": "object",
            "properties": {
                "api_style": { "type": "string", "enum": ["rest", "graphql", "grpc"], "default": "rest" },
                "auth_type": { "type": "string", "enum": ["jwt", "oauth2", "api-key", "basic"], "default": "jwt" }
            }
        }),
        execution: SkillExecution { runtime: "node".to_string(), entry_point: "src/index.js".to_string(), timeout_ms: 180000, memory_limit_mb: 256 },
        permissions: vec![
            SkillPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
            SkillPermission { resource: "ai-models".to_string(), actions: vec!["execute".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 4.99, category: "Development Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}

fn create_database_optimizer_skill() -> SkillDefinition {
    SkillDefinition {
        id: "database-optimizer".to_string(),
        name: "Database Optimizer".to_string(),
        version: "1.0.0".to_string(),
        description: "Optimize database queries and schema".to_string(),
        author: "ZylCode Team".to_string(),
        category: "database".to_string(),
        tags: vec!["database".to_string(), "sql".to_string(), "optimization".to_string()],
        dependencies: vec![SkillDependency { name: "database-tools".to_string(), version: ">=1.0.0".to_string(), optional: false }],
        config_schema: json!({
            "type": "object",
            "properties": {
                "db_type": { "type": "string", "enum": ["postgresql", "mysql", "sqlite", "mongodb"], "default": "postgresql" },
                "optimization_level": { "type": "string", "enum": ["basic", "advanced", "expert"], "default": "advanced" }
            }
        }),
        execution: SkillExecution { runtime: "node".to_string(), entry_point: "src/index.js".to_string(), timeout_ms: 240000, memory_limit_mb: 256 },
        permissions: vec![
            SkillPermission { resource: "database".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
            SkillPermission { resource: "ai-models".to_string(), actions: vec!["execute".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 14.99, category: "Database Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}

fn create_cloud_architect_skill() -> SkillDefinition {
    SkillDefinition {
        id: "cloud-architect".to_string(),
        name: "Cloud Architect".to_string(),
        version: "1.0.0".to_string(),
        description: "Design and implement cloud architecture".to_string(),
        author: "ZylCode Team".to_string(),
        category: "cloud".to_string(),
        tags: vec!["cloud".to_string(), "architecture".to_string(), "aws".to_string(), "gcp".to_string(), "azure".to_string()],
        dependencies: vec![SkillDependency { name: "cloud-tools".to_string(), version: ">=1.0.0".to_string(), optional: false }],
        config_schema: json!({
            "type": "object",
            "properties": {
                "cloud_provider": { "type": "string", "enum": ["aws", "gcp", "azure"], "default": "aws" },
                "architecture_pattern": { "type": "string", "enum": ["microservices", "serverless", "monolith", "event-driven"], "default": "microservices" }
            }
        }),
        execution: SkillExecution { runtime: "node".to_string(), entry_point: "src/index.js".to_string(), timeout_ms: 300000, memory_limit_mb: 512 },
        permissions: vec![
            SkillPermission { resource: "cloud".to_string(), actions: vec!["read".to_string(), "write".to_string(), "execute".to_string()] },
            SkillPermission { resource: "ai-models".to_string(), actions: vec!["execute".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 19.99, category: "Cloud Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}

fn create_cicd_builder_skill() -> SkillDefinition {
    SkillDefinition {
        id: "cicd-builder".to_string(),
        name: "CI/CD Pipeline Builder".to_string(),
        version: "1.0.0".to_string(),
        description: "Build and configure CI/CD pipelines".to_string(),
        author: "ZylCode Team".to_string(),
        category: "devops".to_string(),
        tags: vec!["cicd".to_string(), "pipeline".to_string(), "automation".to_string()],
        dependencies: vec![SkillDependency { name: "devops-tools".to_string(), version: ">=1.0.0".to_string(), optional: false }],
        config_schema: json!({
            "type": "object",
            "properties": {
                "platform": { "type": "string", "enum": ["github-actions", "gitlab-ci", "jenkins", "circleci"], "default": "github-actions" },
                "stages": { "type": "array", "items": {"type": "string"}, "default": ["build", "test", "deploy"] }
            }
        }),
        execution: SkillExecution { runtime: "node".to_string(), entry_point: "src/index.js".to_string(), timeout_ms: 180000, memory_limit_mb: 256 },
        permissions: vec![
            SkillPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
            SkillPermission { resource: "git".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 9.99, category: "DevOps Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}

fn create_container_orchestrator_skill() -> SkillDefinition {
    SkillDefinition {
        id: "container-orchestrator".to_string(),
        name: "Container Orchestrator".to_string(),
        version: "1.0.0".to_string(),
        description: "Manage Docker and Kubernetes deployments".to_string(),
        author: "ZylCode Team".to_string(),
        category: "devops".to_string(),
        tags: vec!["docker".to_string(), "kubernetes".to_string(), "containers".to_string()],
        dependencies: vec![SkillDependency { name: "devops-tools".to_string(), version: ">=1.0.0".to_string(), optional: false }],
        config_schema: json!({
            "type": "object",
            "properties": {
                "orchestrator": { "type": "string", "enum": ["docker-compose", "kubernetes", "swarm"], "default": "kubernetes" },
                "replicas": { "type": "number", "default": 3 }
            }
        }),
        execution: SkillExecution { runtime: "node".to_string(), entry_point: "src/index.js".to_string(), timeout_ms: 240000, memory_limit_mb: 256 },
        permissions: vec![
            SkillPermission { resource: "containers".to_string(), actions: vec!["read".to_string(), "write".to_string(), "execute".to_string()] },
            SkillPermission { resource: "cloud".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 14.99, category: "DevOps Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}

fn create_frontend_builder_skill() -> SkillDefinition {
    SkillDefinition {
        id: "frontend-builder".to_string(),
        name: "Frontend Builder".to_string(),
        version: "1.0.0".to_string(),
        description: "Build and optimize frontend applications".to_string(),
        author: "ZylCode Team".to_string(),
        category: "development".to_string(),
        tags: vec!["frontend".to_string(), "react".to_string(), "vue".to_string(), "angular".to_string()],
        dependencies: vec![SkillDependency { name: "ai-models".to_string(), version: ">=2.0.0".to_string(), optional: false }],
        config_schema: json!({
            "type": "object",
            "properties": {
                "framework": { "type": "string", "enum": ["react", "vue", "angular", "svelte"], "default": "react" },
                "styling": { "type": "string", "enum": ["tailwind", "css-modules", "styled-components", "emotion"], "default": "tailwind" }
            }
        }),
        execution: SkillExecution { runtime: "node".to_string(), entry_point: "src/index.js".to_string(), timeout_ms: 300000, memory_limit_mb: 512 },
        permissions: vec![
            SkillPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
            SkillPermission { resource: "ai-models".to_string(), actions: vec!["execute".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 4.99, category: "Development Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}

fn create_backend_builder_skill() -> SkillDefinition {
    SkillDefinition {
        id: "backend-builder".to_string(),
        name: "Backend Builder".to_string(),
        version: "1.0.0".to_string(),
        description: "Build and optimize backend services".to_string(),
        author: "ZylCode Team".to_string(),
        category: "development".to_string(),
        tags: vec!["backend".to_string(), "node".to_string(), "python".to_string(), "rust".to_string()],
        dependencies: vec![SkillDependency { name: "ai-models".to_string(), version: ">=2.0.0".to_string(), optional: false }],
        config_schema: json!({
            "type": "object",
            "properties": {
                "language": { "type": "string", "enum": ["node", "python", "rust", "go", "java"], "default": "node" },
                "framework": { "type": "string", "enum": ["express", "fastify", "django", "flask", "actix", "axum"], "default": "express" }
            }
        }),
        execution: SkillExecution { runtime: "node".to_string(), entry_point: "src/index.js".to_string(), timeout_ms: 300000, memory_limit_mb: 512 },
        permissions: vec![
            SkillPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
            SkillPermission { resource: "ai-models".to_string(), actions: vec!["execute".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 4.99, category: "Development Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}

fn create_mobile_builder_skill() -> SkillDefinition {
    SkillDefinition {
        id: "mobile-builder".to_string(),
        name: "Mobile App Builder".to_string(),
        version: "1.0.0".to_string(),
        description: "Build cross-platform mobile applications".to_string(),
        author: "ZylCode Team".to_string(),
        category: "development".to_string(),
        tags: vec!["mobile".to_string(), "react-native".to_string(), "flutter".to_string(), "ios".to_string(), "android".to_string()],
        dependencies: vec![SkillDependency { name: "ai-models".to_string(), version: ">=2.0.0".to_string(), optional: false }],
        config_schema: json!({
            "type": "object",
            "properties": {
                "platform": { "type": "string", "enum": ["react-native", "flutter", "ios", "android"], "default": "react-native" },
                "target_os": { "type": "array", "items": {"type": "string"}, "default": ["ios", "android"] }
            }
        }),
        execution: SkillExecution { runtime: "node".to_string(), entry_point: "src/index.js".to_string(), timeout_ms: 600000, memory_limit_mb: 1024 },
        permissions: vec![
            SkillPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
            SkillPermission { resource: "ai-models".to_string(), actions: vec!["execute".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 14.99, category: "Development Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}

fn create_ai_model_trainer_skill() -> SkillDefinition {
    SkillDefinition {
        id: "ai-model-trainer".to_string(),
        name: "AI Model Trainer".to_string(),
        version: "1.0.0".to_string(),
        description: "Train and fine-tune AI models".to_string(),
        author: "ZylCode Team".to_string(),
        category: "ai-ml".to_string(),
        tags: vec!["ai".to_string(), "ml".to_string(), "training".to_string(), "fine-tuning".to_string()],
        dependencies: vec![
            SkillDependency { name: "ai-models".to_string(), version: ">=2.0.0".to_string(), optional: false },
            SkillDependency { name: "ml-frameworks".to_string(), version: ">=1.0.0".to_string(), optional: false },
        ],
        config_schema: json!({
            "type": "object",
            "properties": {
                "model_type": { "type": "string", "enum": ["classification", "regression", "nlp", "vision"], "default": "nlp" },
                "training_method": { "type": "string", "enum": ["full", "fine-tune", "lora", "qlora"], "default": "lora" }
            }
        }),
        execution: SkillExecution { runtime: "python".to_string(), entry_point: "src/main.py".to_string(), timeout_ms: 3600000, memory_limit_mb: 4096 },
        permissions: vec![
            SkillPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
            SkillPermission { resource: "gpu".to_string(), actions: vec!["execute".to_string()] },
            SkillPermission { resource: "ai-models".to_string(), actions: vec!["execute".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 29.99, category: "AI/ML Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}

fn create_data_pipeline_skill() -> SkillDefinition {
    SkillDefinition {
        id: "data-pipeline".to_string(),
        name: "Data Pipeline Builder".to_string(),
        version: "1.0.0".to_string(),
        description: "Build ETL and data processing pipelines".to_string(),
        author: "ZylCode Team".to_string(),
        category: "data".to_string(),
        tags: vec!["data".to_string(), "pipeline".to_string(), "etl".to_string(), "streaming".to_string()],
        dependencies: vec![SkillDependency { name: "data-tools".to_string(), version: ">=1.0.0".to_string(), optional: false }],
        config_schema: json!({
            "type": "object",
            "properties": {
                "pipeline_type": { "type": "string", "enum": ["batch", "streaming", "hybrid"], "default": "batch" },
                "data_sources": { "type": "array", "items": {"type": "string"}, "default": ["database", "api", "file"] }
            }
        }),
        execution: SkillExecution { runtime: "python".to_string(), entry_point: "src/main.py".to_string(), timeout_ms: 600000, memory_limit_mb: 1024 },
        permissions: vec![
            SkillPermission { resource: "database".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
            SkillPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 19.99, category: "Data Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}

fn create_microservice_architect_skill() -> SkillDefinition {
    SkillDefinition {
        id: "microservice-architect".to_string(),
        name: "Microservice Architect".to_string(),
        version: "1.0.0".to_string(),
        description: "Design and implement microservice architecture".to_string(),
        author: "ZylCode Team".to_string(),
        category: "architecture".to_string(),
        tags: vec!["microservices".to_string(), "architecture".to_string(), "distributed".to_string()],
        dependencies: vec![SkillDependency { name: "ai-models".to_string(), version: ">=2.0.0".to_string(), optional: false }],
        config_schema: json!({
            "type": "object",
            "properties": {
                "communication": { "type": "string", "enum": ["rest", "grpc", "message-queue", "event-driven"], "default": "rest" },
                "service_mesh": { "type": "boolean", "default": false }
            }
        }),
        execution: SkillExecution { runtime: "node".to_string(), entry_point: "src/index.js".to_string(), timeout_ms: 300000, memory_limit_mb: 512 },
        permissions: vec![
            SkillPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
            SkillPermission { resource: "ai-models".to_string(), actions: vec!["execute".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 24.99, category: "Architecture Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}

fn create_graphql_generator_skill() -> SkillDefinition {
    SkillDefinition {
        id: "graphql-generator".to_string(),
        name: "GraphQL Generator".to_string(),
        version: "1.0.0".to_string(),
        description: "Generate GraphQL schemas and resolvers".to_string(),
        author: "ZylCode Team".to_string(),
        category: "development".to_string(),
        tags: vec!["graphql".to_string(), "api".to_string(), "schema".to_string()],
        dependencies: vec![SkillDependency { name: "ai-models".to_string(), version: ">=2.0.0".to_string(), optional: false }],
        config_schema: json!({
            "type": "object",
            "properties": {
                "framework": { "type": "string", "enum": ["apollo", "relay", "typegraphql", "nexus"], "default": "apollo" },
                "database_integration": { "type": "boolean", "default": true }
            }
        }),
        execution: SkillExecution { runtime: "node".to_string(), entry_point: "src/index.js".to_string(), timeout_ms: 180000, memory_limit_mb: 256 },
        permissions: vec![
            SkillPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
            SkillPermission { resource: "ai-models".to_string(), actions: vec!["execute".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 9.99, category: "Development Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}

fn create_websocket_handler_skill() -> SkillDefinition {
    SkillDefinition {
        id: "websocket-handler".to_string(),
        name: "WebSocket Handler".to_string(),
        version: "1.0.0".to_string(),
        description: "Build real-time WebSocket applications".to_string(),
        author: "ZylCode Team".to_string(),
        category: "development".to_string(),
        tags: vec!["websocket".to_string(), "real-time".to_string(), "socket.io".to_string()],
        dependencies: vec![SkillDependency { name: "ai-models".to_string(), version: ">=2.0.0".to_string(), optional: false }],
        config_schema: json!({
            "type": "object",
            "properties": {
                "library": { "type": "string", "enum": ["ws", "socket.io", "native"], "default": "socket.io" },
                "scaling": { "type": "string", "enum": ["single", "redis", "cluster"], "default": "single" }
            }
        }),
        execution: SkillExecution { runtime: "node".to_string(), entry_point: "src/index.js".to_string(), timeout_ms: 180000, memory_limit_mb: 256 },
        permissions: vec![
            SkillPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
            SkillPermission { resource: "network".to_string(), actions: vec!["listen".to_string(), "connect".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 9.99, category: "Development Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}

fn create_auth_system_skill() -> SkillDefinition {
    SkillDefinition {
        id: "auth-system".to_string(),
        name: "Auth System Builder".to_string(),
        version: "1.0.0".to_string(),
        description: "Build authentication and authorization systems".to_string(),
        author: "ZylCode Team".to_string(),
        category: "security".to_string(),
        tags: vec!["auth".to_string(), "jwt".to_string(), "oauth".to_string(), "security".to_string()],
        dependencies: vec![SkillDependency { name: "security-tools".to_string(), version: ">=1.0.0".to_string(), optional: false }],
        config_schema: json!({
            "type": "object",
            "properties": {
                "auth_type": { "type": "string", "enum": ["jwt", "oauth2", "session", "magic-link"], "default": "jwt" },
                "providers": { "type": "array", "items": {"type": "string"}, "default": ["email", "google", "github"] }
            }
        }),
        execution: SkillExecution { runtime: "node".to_string(), entry_point: "src/index.js".to_string(), timeout_ms: 240000, memory_limit_mb: 256 },
        permissions: vec![
            SkillPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
            SkillPermission { resource: "security".to_string(), actions: vec!["configure".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 14.99, category: "Security Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}

fn create_logging_monitoring_skill() -> SkillDefinition {
    SkillDefinition {
        id: "logging-monitoring".to_string(),
        name: "Logging & Monitoring".to_string(),
        version: "1.0.0".to_string(),
        description: "Set up logging, metrics, and monitoring".to_string(),
        author: "ZylCode Team".to_string(),
        category: "devops".to_string(),
        tags: vec!["logging".to_string(), "monitoring".to_string(), "observability".to_string(), "metrics".to_string()],
        dependencies: vec![SkillDependency { name: "devops-tools".to_string(), version: ">=1.0.0".to_string(), optional: false }],
        config_schema: json!({
            "type": "object",
            "properties": {
                "stack": { "type": "string", "enum": ["elk", "prometheus-grafana", "datadog", "newrelic"], "default": "prometheus-grafana" },
                "log_level": { "type": "string", "enum": ["debug", "info", "warn", "error"], "default": "info" }
            }
        }),
        execution: SkillExecution { runtime: "node".to_string(), entry_point: "src/index.js".to_string(), timeout_ms: 180000, memory_limit_mb: 256 },
        permissions: vec![
            SkillPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
            SkillPermission { resource: "monitoring".to_string(), actions: vec!["configure".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 9.99, category: "DevOps Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}

fn create_file_converter_skill() -> SkillDefinition {
    SkillDefinition {
        id: "file-converter".to_string(),
        name: "File Converter".to_string(),
        version: "1.0.0".to_string(),
        description: "Convert files between different formats".to_string(),
        author: "ZylCode Team".to_string(),
        category: "productivity".to_string(),
        tags: vec!["converter".to_string(), "file".to_string(), "format".to_string()],
        dependencies: vec![SkillDependency { name: "file-tools".to_string(), version: ">=1.0.0".to_string(), optional: false }],
        config_schema: json!({
            "type": "object",
            "properties": {
                "source_format": { "type": "string", "enum": ["json", "csv", "xml", "yaml", "markdown", "pdf"], "default": "json" },
                "target_format": { "type": "string", "enum": ["json", "csv", "xml", "yaml", "markdown", "pdf"], "default": "csv" }
            }
        }),
        execution: SkillExecution { runtime: "node".to_string(), entry_point: "src/index.js".to_string(), timeout_ms: 60000, memory_limit_mb: 256 },
        permissions: vec![
            SkillPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 0.0, category: "Productivity Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}

fn create_code_translator_skill() -> SkillDefinition {
    SkillDefinition {
        id: "code-translator".to_string(),
        name: "Code Translator".to_string(),
        version: "1.0.0".to_string(),
        description: "Translate code between programming languages".to_string(),
        author: "ZylCode Team".to_string(),
        category: "development".to_string(),
        tags: vec!["translator".to_string(), "conversion".to_string(), "multi-language".to_string()],
        dependencies: vec![SkillDependency { name: "ai-models".to_string(), version: ">=2.0.0".to_string(), optional: false }],
        config_schema: json!({
            "type": "object",
            "properties": {
                "source_language": { "type": "string", "enum": ["javascript", "python", "rust", "go", "java", "c++", "typescript"], "default": "javascript" },
                "target_language": { "type": "string", "enum": ["javascript", "python", "rust", "go", "java", "c++", "typescript"], "default": "python" }
            }
        }),
        execution: SkillExecution { runtime: "node".to_string(), entry_point: "src/index.js".to_string(), timeout_ms: 300000, memory_limit_mb: 512 },
        permissions: vec![
            SkillPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
            SkillPermission { resource: "ai-models".to_string(), actions: vec!["execute".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 9.99, category: "Development Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}

fn create_technical_writer_skill() -> SkillDefinition {
    SkillDefinition {
        id: "technical-writer".to_string(),
        name: "Technical Writer".to_string(),
        version: "1.0.0".to_string(),
        description: "Generate technical documentation and guides".to_string(),
        author: "ZylCode Team".to_string(),
        category: "documentation".to_string(),
        tags: vec!["technical-writing".to_string(), "documentation".to_string(), "guides".to_string()],
        dependencies: vec![SkillDependency { name: "ai-models".to_string(), version: ">=2.0.0".to_string(), optional: false }],
        config_schema: json!({
            "type": "object",
            "properties": {
                "doc_type": { "type": "string", "enum": ["api-doc", "user-guide", "tutorial", "architecture-doc"], "default": "api-doc" },
                "audience": { "type": "string", "enum": ["developer", "end-user", "admin", "architect"], "default": "developer" }
            }
        }),
        execution: SkillExecution { runtime: "node".to_string(), entry_point: "src/index.js".to_string(), timeout_ms: 240000, memory_limit_mb: 256 },
        permissions: vec![
            SkillPermission { resource: "filesystem".to_string(), actions: vec!["read".to_string(), "write".to_string()] },
            SkillPermission { resource: "ai-models".to_string(), actions: vec!["execute".to_string()] },
        ],
        marketplace: MarketplaceInfo { price: 4.99, category: "Documentation Tools".to_string(), screenshots: vec![], documentation: "docs/README.md".to_string() },
    }
}
