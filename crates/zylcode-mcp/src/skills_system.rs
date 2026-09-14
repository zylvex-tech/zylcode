use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::builtin_skills::get_builtin_skills;

/// Skills System for reusable capabilities
pub struct SkillsSystem {
    skills: RwLock<HashMap<String, Arc<dyn Skill>>>,
    skill_categories: RwLock<HashMap<String, Vec<String>>>,
    execution_history: RwLock<Vec<ExecutionRecord>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub category: String,
    pub tags: Vec<String>,
    pub dependencies: Vec<SkillDependency>,
    pub config_schema: Value,
    pub execution: SkillExecution,
    pub permissions: Vec<SkillPermission>,
    pub marketplace: MarketplaceInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDependency {
    pub name: String,
    pub version: String,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillExecution {
    pub runtime: String,
    pub entry_point: String,
    pub timeout_ms: u64,
    pub memory_limit_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPermission {
    pub resource: String,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarketplaceInfo {
    pub price: f64,
    pub category: String,
    pub screenshots: Vec<String>,
    pub documentation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub skill_id: String,
    pub input: Value,
    pub output: Value,
    pub success: bool,
    pub duration_ms: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
pub trait Skill: Send + Sync + std::fmt::Debug {
    fn id(&self) -> &str;
    fn definition(&self) -> SkillDefinition;
    async fn execute(&self, input: Value, context: ExecutionContext) -> Result<Value>;
    async fn validate_config(&self, config: Value) -> Result<bool>;
    async fn get_schema(&self) -> Value;
}

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub user_id: Option<String>,
    pub project_id: Option<String>,
    pub permissions: Vec<String>,
    pub environment: HashMap<String, String>,
}

impl SkillsSystem {
    pub fn new() -> Self {
        Self {
            skills: RwLock::new(HashMap::new()),
            skill_categories: RwLock::new(HashMap::new()),
            execution_history: RwLock::new(Vec::new()),
        }
    }

    /// Initialize with built-in skills
    pub async fn initialize_with_builtin_skills(&self) -> Result<usize> {
        let builtin_skills = get_builtin_skills();
        let mut total_skills = 0;

        for skill_def in builtin_skills {
            let skill = BuiltinSkill::new(skill_def.clone());
            self.register_skill(Arc::new(skill)).await;
            total_skills += 1;
        }

        tracing::info!("Initialized skills system with {} built-in skills", total_skills);
        Ok(total_skills)
    }

    /// Register a skill
    pub async fn register_skill(&self, skill: Arc<dyn Skill>) {
        let id = skill.id().to_string();
        let category = skill.definition().category.clone();
        
        // Add to skills map
        self.skills.write().await.insert(id.clone(), skill);
        
        // Add to category
        let mut categories = self.skill_categories.write().await;
        categories.entry(category).or_insert_with(Vec::new).push(id);
    }

    /// Execute a skill
    pub async fn execute_skill(
        &self,
        skill_id: &str,
        input: Value,
        context: ExecutionContext,
    ) -> Result<Value> {
        let start = std::time::Instant::now();
        
        // Get skill
        let skill = self.skills.read().await.get(skill_id).cloned()
            .ok_or_else(|| anyhow::anyhow!("Skill not found: {}", skill_id))?;

        // Execute skill
        let result = skill.execute(input.clone(), context).await;
        
        // Record execution
        let duration = start.elapsed().as_millis() as u64;
        let output = match &result {
            Ok(value) => value.clone(),
            Err(e) => serde_json::json!({"error": e.to_string()}),
        };
        let record = ExecutionRecord {
            skill_id: skill_id.to_string(),
            input,
            output,
            success: result.is_ok(),
            duration_ms: duration,
            timestamp: chrono::Utc::now(),
        };
        
        self.execution_history.write().await.push(record);
        
        result
    }

    /// Get skill definition
    pub async fn get_skill_definition(&self, skill_id: &str) -> Option<SkillDefinition> {
        self.skills.read().await.get(skill_id).map(|s| s.definition())
    }

    /// List skills by category
    pub async fn list_skills_by_category(&self, category: &str) -> Vec<String> {
        self.skill_categories.read().await
            .get(category)
            .cloned()
            .unwrap_or_default()
    }

    /// List all categories
    pub async fn list_categories(&self) -> Vec<String> {
        self.skill_categories.read().await.keys().cloned().collect()
    }

    /// Get execution history
    pub async fn get_execution_history(&self, limit: usize) -> Vec<ExecutionRecord> {
        let history = self.execution_history.read().await;
        let start = if history.len() > limit { history.len() - limit } else { 0 };
        history[start..].to_vec()
    }

    /// Get skill count
    pub async fn skill_count(&self) -> usize {
        self.skills.read().await.len()
    }
}

/// Built-in skill implementation
#[derive(Debug, Clone)]
struct BuiltinSkill {
    definition: SkillDefinition,
}

impl BuiltinSkill {
    fn new(definition: SkillDefinition) -> Self {
        Self { definition }
    }
}

#[async_trait]
impl Skill for BuiltinSkill {
    fn id(&self) -> &str {
        &self.definition.id
    }

    fn definition(&self) -> SkillDefinition {
        self.definition.clone()
    }

    async fn execute(&self, input: Value, context: ExecutionContext) -> Result<Value> {
        // Simulate skill execution
        let start = std::time::Instant::now();
        
        // Simulate processing time
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        let duration = start.elapsed().as_millis() as u64;
        
        Ok(serde_json::json!({
            "skill": self.definition.id,
            "input": input,
            "context": {
                "user_id": context.user_id,
                "project_id": context.project_id,
                "permissions": context.permissions
            },
            "result": {
                "success": true,
                "message": format!("Skill {} executed successfully", self.definition.id),
                "duration_ms": duration,
                "output": {
                    "analysis": "Sample analysis output",
                    "suggestions": ["Suggestion 1", "Suggestion 2"],
                    "metrics": {
                        "complexity": 5,
                        "maintainability": 85,
                        "test_coverage": 72
                    }
                }
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    }

    async fn validate_config(&self, config: Value) -> Result<bool> {
        // Simple validation - in real implementation, use JSON Schema validation
        Ok(config.is_object())
    }

    async fn get_schema(&self) -> Value {
        self.definition.config_schema.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_skills_system() {
        let system = SkillsSystem::new();
        let count = system.initialize_with_builtin_skills().await.unwrap();
        
        assert!(count > 0);
        assert_eq!(system.skill_count().await, count);
        
        let categories = system.list_categories().await;
        assert!(categories.contains(&"development".to_string()));
        assert!(categories.contains(&"ai-ml".to_string()));
        assert!(categories.contains(&"security".to_string()));
    }

    #[tokio::test]
    async fn test_skill_execution() {
        let system = SkillsSystem::new();
        system.initialize_with_builtin_skills().await.unwrap();
        
        let input = serde_json::json!({
            "files": ["src/main.rs"],
            "options": {"auto_fix": false}
        });
        
        let context = ExecutionContext {
            user_id: Some("user123".to_string()),
            project_id: Some("project456".to_string()),
            permissions: vec!["filesystem.read".to_string()],
            environment: HashMap::new(),
        };
        
        let result = system.execute_skill("code-review", input, context).await.unwrap();
        assert_eq!(result["skill"], "code-review");
        assert_eq!(result["result"]["success"], true);
    }
}