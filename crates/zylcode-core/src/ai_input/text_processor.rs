use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;

use super::types::*;
use super::context_manager::ContextManager;

/// Text processor for natural language processing
pub struct TextProcessor {
    context_manager: Arc<RwLock<ContextManager>>,
    intent_patterns: HashMap<String, Regex>,
    entity_patterns: HashMap<EntityType, Regex>,
    stats: RwLock<TextProcessorStats>,
}

#[derive(Debug, Default)]
struct TextProcessorStats {
    processed_count: u64,
    total_processing_time_ms: u64,
}

impl TextProcessor {
    /// Create a new text processor
    pub async fn new(context_manager: Arc<RwLock<ContextManager>>) -> Result<Self> {
        let mut intent_patterns = HashMap::new();
        let mut entity_patterns = HashMap::new();

        // Intent patterns
        intent_patterns.insert(
            "code_generation".to_string(),
            Regex::new(r"(?i)(create|generate|build|write|make|implement|develop)\s+(a\s+)?(new\s+)?(\w+\s+)?(function|class|component|module|service|api|endpoint|method|interface|type|struct|enum)").unwrap(),
        );
        intent_patterns.insert(
            "code_review".to_string(),
            Regex::new(r"(?i)(review|analyze|check|inspect|audit|evaluate)\s+(the\s+)?(code|implementation|solution|approach)").unwrap(),
        );
        intent_patterns.insert(
            "code_refactoring".to_string(),
            Regex::new(r"(?i)(refactor|improve|optimize|clean|simplify|restructure|reorganize)\s+(the\s+)?(code|implementation|function|method|class)").unwrap(),
        );
        intent_patterns.insert(
            "testing".to_string(),
            Regex::new(r"(?i)(test|write\s+tests|create\s+tests|add\s+tests|unit\s+tests|integration\s+tests|e2e\s+tests)").unwrap(),
        );
        intent_patterns.insert(
            "documentation".to_string(),
            Regex::new(r"(?i)(document|write\s+docs|create\s+documentation|add\s+comments|explain|describe)").unwrap(),
        );
        intent_patterns.insert(
            "debugging".to_string(),
            Regex::new(r"(?i)(debug|fix|resolve|troubleshoot|diagnose|investigate)\s+(the\s+)?(error|bug|issue|problem|crash|failure)").unwrap(),
        );
        intent_patterns.insert(
            "deployment".to_string(),
            Regex::new(r"(?i)(deploy|release|publish|push|ship|launch)\s+(to\s+)?(production|staging|dev|server|cloud|aws|gcp|azure)").unwrap(),
        );
        intent_patterns.insert(
            "database".to_string(),
            Regex::new(r"(?i)(query|database|sql|table|schema|migration|index|optimize)\s+(the\s+)?(database|query|table|record)").unwrap(),
        );
        intent_patterns.insert(
            "api".to_string(),
            Regex::new(r"(?i)(api|endpoint|rest|graphql|grpc|webhook)\s+(for|to|that|which)").unwrap(),
        );
        intent_patterns.insert(
            "ui".to_string(),
            Regex::new(r"(?i)(ui|interface|design|layout|button|form|modal|page|screen)\s+(for|to|that|which)").unwrap(),
        );

        // Entity patterns
        entity_patterns.insert(
            EntityType::Code,
            Regex::new(r"```[\s\S]*?```|`[^`]+`").unwrap(),
        );
        entity_patterns.insert(
            EntityType::FilePath,
            Regex::new(r"(?:[\w\-\.]+/)+[\w\-\.]+|[\w\-\.]+\.(?:js|ts|jsx|tsx|py|rs|go|java|cpp|c|h|html|css|json|yaml|yml|toml|md|txt)").unwrap(),
        );
        entity_patterns.insert(
            EntityType::URL,
            Regex::new(r"https?://[^\s]+").unwrap(),
        );
        entity_patterns.insert(
            EntityType::Email,
            Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap(),
        );
        entity_patterns.insert(
            EntityType::Language,
            Regex::new(r"(?i)(javascript|typescript|python|rust|go|java|c\+\+|ruby|php|swift|kotlin|scala|clojure|haskell|elixir|erlang|sql|html|css|bash|powershell)").unwrap(),
        );
        entity_patterns.insert(
            EntityType::Framework,
            Regex::new(r"(?i)(react|vue|angular|svelte|next\.js|nuxt|gatsby|express|fastify|django|flask|fastapi|spring|rails|laravel|actix|axum|rocket)").unwrap(),
        );
        entity_patterns.insert(
            EntityType::Library,
            Regex::new(r"(?i)(lodash|moment|axios|fetch|redux|mobx|zustand|recoil|tailwind|bootstrap|material-ui|antd|styled-components|framer-motion|three\.js|d3|chart\.js)").unwrap(),
        );

        Ok(Self {
            context_manager,
            intent_patterns,
            entity_patterns,
            stats: RwLock::new(TextProcessorStats::default()),
        })
    }

    /// Process text input
    pub async fn process(&self, text: &str, context: &InputContext) -> Result<ProcessedText> {
        let start = std::time::Instant::now();
        
        // Extract intent
        let intent = self.extract_intent(text).await?;
        
        // Extract entities
        let entities = self.extract_entities(text).await?;
        
        // Calculate confidence
        let confidence = self.calculate_confidence(text, &intent, &entities).await?;
        
        // Update stats
        let duration = start.elapsed().as_millis() as u64;
        {
            let mut stats = self.stats.write().await;
            stats.processed_count += 1;
            stats.total_processing_time_ms += duration;
        }

        Ok(ProcessedText {
            content: text.to_string(),
            intent,
            entities,
            confidence,
            processing_time_ms: duration,
        })
    }

    /// Extract intent from text
    async fn extract_intent(&self, text: &str) -> Result<Intent> {
        let mut best_intent = Intent {
            name: "unknown".to_string(),
            category: IntentCategory::Unknown,
            confidence: 0.0,
            parameters: HashMap::new(),
        };

        for (intent_name, pattern) in &self.intent_patterns {
            if pattern.is_match(text) {
                let confidence = self.calculate_intent_confidence(text, pattern).await?;
                if confidence > best_intent.confidence {
                    best_intent = Intent {
                        name: intent_name.clone(),
                        category: self.map_intent_category(intent_name),
                        confidence,
                        parameters: self.extract_intent_parameters(text, pattern).await?,
                    };
                }
            }
        }

        // If no specific intent found, classify as conversation
        if best_intent.confidence < 0.3 {
            best_intent = Intent {
                name: "conversation".to_string(),
                category: IntentCategory::Conversation,
                confidence: 0.8,
                parameters: HashMap::new(),
            };
        }

        Ok(best_intent)
    }

    /// Extract entities from text
    async fn extract_entities(&self, text: &str) -> Result<Vec<Entity>> {
        let mut entities = Vec::new();

        for (entity_type, pattern) in &self.entity_patterns {
            for mat in pattern.find_iter(text) {
                entities.push(Entity {
                    name: entity_type.to_string(),
                    entity_type: entity_type.clone(),
                    value: mat.as_str().to_string(),
                    confidence: 0.9,
                    start_pos: mat.start(),
                    end_pos: mat.end(),
                });
            }
        }

        Ok(entities)
    }

    /// Calculate confidence for processed text
    async fn calculate_confidence(&self, text: &str, intent: &Intent, entities: &[Entity]) -> Result<f64> {
        let mut confidence = 0.5; // Base confidence

        // Increase confidence based on text length
        if text.len() > 10 {
            confidence += 0.1;
        }
        if text.len() > 50 {
            confidence += 0.1;
        }

        // Increase confidence based on intent confidence
        confidence += intent.confidence * 0.3;

        // Increase confidence based on entity count
        confidence += (entities.len() as f64 * 0.05).min(0.2);

        Ok(confidence.min(1.0))
    }

    /// Calculate intent confidence
    async fn calculate_intent_confidence(&self, text: &str, pattern: &Regex) -> Result<f64> {
        let matches: Vec<_> = pattern.find_iter(text).collect();
        if matches.is_empty() {
            return Ok(0.0);
        }

        let match_count = matches.len() as f64;
        let text_length = text.len() as f64;
        let match_length: usize = matches.iter().map(|m| m.len()).sum();
        let match_ratio = match_length as f64 / text_length;

        Ok((match_count * 0.3 + match_ratio * 0.7).min(1.0))
    }

    /// Extract parameters from intent
    async fn extract_intent_parameters(&self, text: &str, pattern: &Regex) -> Result<HashMap<String, String>> {
        let mut parameters = HashMap::new();
        
        if let Some(captures) = pattern.captures(text) {
            for (i, capture) in captures.iter().enumerate() {
                if let Some(matched) = capture {
                    parameters.insert(format!("param_{}", i), matched.as_str().to_string());
                }
            }
        }

        Ok(parameters)
    }

    /// Map intent name to category
    fn map_intent_category(&self, intent_name: &str) -> IntentCategory {
        match intent_name {
            "code_generation" => IntentCategory::CodeGeneration,
            "code_review" => IntentCategory::CodeReview,
            "code_refactoring" => IntentCategory::CodeRefactoring,
            "testing" => IntentCategory::Testing,
            "documentation" => IntentCategory::Documentation,
            "debugging" => IntentCategory::Debugging,
            "deployment" => IntentCategory::Deployment,
            "database" => IntentCategory::Database,
            "api" => IntentCategory::API,
            "ui" => IntentCategory::UI,
            _ => IntentCategory::Unknown,
        }
    }

    /// Get processing statistics
    pub async fn get_stats(&self) -> u64 {
        self.stats.read().await.processed_count
    }
}

/// Processed text result
#[derive(Debug, Clone)]
pub struct ProcessedText {
    pub content: String,
    pub intent: Intent,
    pub entities: Vec<Entity>,
    pub confidence: f64,
    pub processing_time_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_text_processing() {
        let context_manager = Arc::new(RwLock::new(ContextManager::new()));
        let processor = TextProcessor::new(context_manager).await.unwrap();
        let context = InputContext::default();
        
        let result = processor.process("Create a new React component for user authentication", &context).await.unwrap();
        assert_eq!(result.intent.category, IntentCategory::CodeGeneration);
        assert!(result.confidence > 0.0);
    }

    #[tokio::test]
    async fn test_entity_extraction() {
        let context_manager = Arc::new(RwLock::new(ContextManager::new()));
        let processor = TextProcessor::new(context_manager).await.unwrap();
        let context = InputContext::default();
        
        let result = processor.process("Fix the bug in src/main.ts using TypeScript and React", &context).await.unwrap();
        assert!(!result.entities.is_empty());
    }
}