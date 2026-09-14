use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use anyhow::Result;

use super::types::*;
use super::context_manager::ContextManager;
use super::text_processor::ProcessedText;

/// Intent engine for classifying and routing intents
pub struct IntentEngine {
    context_manager: Arc<RwLock<ContextManager>>,
    intent_classifiers: HashMap<IntentCategory, IntentClassifier>,
    stats: RwLock<IntentEngineStats>,
}

#[derive(Debug, Default)]
struct IntentEngineStats {
    classified_count: u64,
    total_classification_time_ms: u64,
}

/// Intent classifier for specific categories
struct IntentClassifier {
    category: IntentCategory,
    patterns: Vec<IntentPattern>,
    confidence_threshold: f64,
}

/// Intent pattern
struct IntentPattern {
    pattern: regex::Regex,
    intent_name: String,
    confidence_boost: f64,
}

impl IntentEngine {
    /// Create a new intent engine
    pub async fn new(context_manager: Arc<RwLock<ContextManager>>) -> Result<Self> {
        let mut intent_classifiers = HashMap::new();
        
        // Code generation classifier
        intent_classifiers.insert(
            IntentCategory::CodeGeneration,
            IntentClassifier {
                category: IntentCategory::CodeGeneration,
                patterns: vec![
                    IntentPattern {
                        pattern: regex::Regex::new(r"(?i)(create|generate|build|write|make|implement|develop)\s+(a\s+)?(new\s+)?(\w+\s+)?(function|class|component|module|service|api|endpoint|method|interface|type|struct|enum)").unwrap(),
                        intent_name: "code_generation".to_string(),
                        confidence_boost: 0.3,
                    },
                    IntentPattern {
                        pattern: regex::Regex::new(r"(?i)(add|insert|include)\s+(a\s+)?(new\s+)?(feature|functionality|capability)").unwrap(),
                        intent_name: "feature_addition".to_string(),
                        confidence_boost: 0.2,
                    },
                ],
                confidence_threshold: 0.5,
            },
        );
        
        // Code review classifier
        intent_classifiers.insert(
            IntentCategory::CodeReview,
            IntentClassifier {
                category: IntentCategory::CodeReview,
                patterns: vec![
                    IntentPattern {
                        pattern: regex::Regex::new(r"(?i)(review|analyze|check|inspect|audit|evaluate)\s+(the\s+)?(code|implementation|solution|approach)").unwrap(),
                        intent_name: "code_review".to_string(),
                        confidence_boost: 0.3,
                    },
                    IntentPattern {
                        pattern: regex::Regex::new(r"(?i)(find|identify|spot)\s+(any\s+)?(issues|problems|bugs|errors)").unwrap(),
                        intent_name: "issue_detection".to_string(),
                        confidence_boost: 0.2,
                    },
                ],
                confidence_threshold: 0.5,
            },
        );
        
        // Testing classifier
        intent_classifiers.insert(
            IntentCategory::Testing,
            IntentClassifier {
                category: IntentCategory::Testing,
                patterns: vec![
                    IntentPattern {
                        pattern: regex::Regex::new(r"(?i)(test|write\s+tests|create\s+tests|add\s+tests|unit\s+tests|integration\s+tests|e2e\s+tests)").unwrap(),
                        intent_name: "test_generation".to_string(),
                        confidence_boost: 0.3,
                    },
                    IntentPattern {
                        pattern: regex::Regex::new(r"(?i)(run|execute|trigger)\s+(the\s+)?(tests|test\s+suite|test\s+cases)").unwrap(),
                        intent_name: "test_execution".to_string(),
                        confidence_boost: 0.2,
                    },
                ],
                confidence_threshold: 0.5,
            },
        );
        
        // Debugging classifier
        intent_classifiers.insert(
            IntentCategory::Debugging,
            IntentClassifier {
                category: IntentCategory::Debugging,
                patterns: vec![
                    IntentPattern {
                        pattern: regex::Regex::new(r"(?i)(debug|fix|resolve|troubleshoot|diagnose|investigate)\s+(the\s+)?(error|bug|issue|problem|crash|failure)").unwrap(),
                        intent_name: "debugging".to_string(),
                        confidence_boost: 0.3,
                    },
                    IntentPattern {
                        pattern: regex::Regex::new(r"(?i)(why|what|how)\s+(is|does|did)\s+(this|it)\s+(not|fail|crash|break|error)").unwrap(),
                        intent_name: "error_investigation".to_string(),
                        confidence_boost: 0.2,
                    },
                ],
                confidence_threshold: 0.5,
            },
        );
        
        // Documentation classifier
        intent_classifiers.insert(
            IntentCategory::Documentation,
            IntentClassifier {
                category: IntentCategory::Documentation,
                patterns: vec![
                    IntentPattern {
                        pattern: regex::Regex::new(r"(?i)(document|write\s+docs|create\s+documentation|add\s+comments|explain|describe)").unwrap(),
                        intent_name: "documentation".to_string(),
                        confidence_boost: 0.3,
                    },
                    IntentPattern {
                        pattern: regex::Regex::new(r"(?i)(what|how|why)\s+(does|is|should)\s+(this|it|the)\s+(code|function|method|class)").unwrap(),
                        intent_name: "code_explanation".to_string(),
                        confidence_boost: 0.2,
                    },
                ],
                confidence_threshold: 0.5,
            },
        );

        Ok(Self {
            context_manager,
            intent_classifiers,
            stats: RwLock::new(IntentEngineStats::default()),
        })
    }

    /// Classify intent from processed text
    pub async fn classify_intent(&self, processed_text: ProcessedText) -> Result<Intent> {
        let start = std::time::Instant::now();
        
        let mut best_intent = Intent {
            name: "unknown".to_string(),
            category: IntentCategory::Unknown,
            confidence: 0.0,
            parameters: HashMap::new(),
        };
        
        // Try each classifier
        for (_category, classifier) in &self.intent_classifiers {
            let intent = self.classify_with_classifier(&processed_text.content, classifier).await?;
            if intent.confidence > best_intent.confidence {
                best_intent = intent;
            }
        }
        
        // If no specific intent found, use conversation
        if best_intent.confidence < 0.1 {
            best_intent = Intent {
                name: "conversation".to_string(),
                category: IntentCategory::Conversation,
                confidence: 0.8,
                parameters: HashMap::new(),
            };
        }
        
        // Update stats
        let duration = start.elapsed().as_millis() as u64;
        {
            let mut stats = self.stats.write().await;
            stats.classified_count += 1;
            stats.total_classification_time_ms += duration;
        }
        
        Ok(best_intent)
    }

    /// Classify intent from vision input
    pub async fn classify_vision_intent(&self, vision: ProcessedVision) -> Result<Intent> {
        let start = std::time::Instant::now();
        
        let mut best_intent = Intent {
            name: "unknown".to_string(),
            category: IntentCategory::Unknown,
            confidence: 0.0,
            parameters: HashMap::new(),
        };
        
        // Analyze vision content for intent
        let description = &vision.description;
        
        // Check for UI-related intents
        if description.contains("button") || description.contains("form") || description.contains("input") {
            best_intent = Intent {
                name: "ui_interaction".to_string(),
                category: IntentCategory::UI,
                confidence: 0.7,
                parameters: HashMap::new(),
            };
        }
        
        // Check for code-related intents
        if description.contains("code") || description.contains("editor") || description.contains("function") {
            best_intent = Intent {
                name: "code_analysis".to_string(),
                category: IntentCategory::CodeReview,
                confidence: 0.7,
                parameters: HashMap::new(),
            };
        }
        
        // Check for error-related intents
        if description.contains("error") || description.contains("bug") || description.contains("crash") {
            best_intent = Intent {
                name: "error_detection".to_string(),
                category: IntentCategory::Debugging,
                confidence: 0.7,
                parameters: HashMap::new(),
            };
        }
        
        // Default to conversation if no specific intent
        if best_intent.confidence < 0.3 {
            best_intent = Intent {
                name: "visual_analysis".to_string(),
                category: IntentCategory::Conversation,
                confidence: 0.6,
                parameters: HashMap::new(),
            };
        }
        
        // Update stats
        let duration = start.elapsed().as_millis() as u64;
        {
            let mut stats = self.stats.write().await;
            stats.classified_count += 1;
            stats.total_classification_time_ms += duration;
        }
        
        Ok(best_intent)
    }

    /// Classify intent from file input
    pub async fn classify_file_intent(&self, file: ProcessedFile) -> Result<Intent> {
        let start = std::time::Instant::now();
        
        let mut best_intent = Intent {
            name: "unknown".to_string(),
            category: IntentCategory::Unknown,
            confidence: 0.0,
            parameters: HashMap::new(),
        };
        
        // Analyze file content for intent
        match file.file_type {
            FileType::Code => {
                best_intent = Intent {
                    name: "code_analysis".to_string(),
                    category: IntentCategory::CodeReview,
                    confidence: 0.8,
                    parameters: HashMap::new(),
                };
            }
            FileType::Document => {
                best_intent = Intent {
                    name: "document_analysis".to_string(),
                    category: IntentCategory::Documentation,
                    confidence: 0.8,
                    parameters: HashMap::new(),
                };
            }
            FileType::Configuration => {
                best_intent = Intent {
                    name: "configuration_analysis".to_string(),
                    category: IntentCategory::Unknown,
                    confidence: 0.7,
                    parameters: HashMap::new(),
                };
            }
            _ => {
                best_intent = Intent {
                    name: "file_analysis".to_string(),
                    category: IntentCategory::Unknown,
                    confidence: 0.6,
                    parameters: HashMap::new(),
                };
            }
        }
        
        // Update stats
        let duration = start.elapsed().as_millis() as u64;
        {
            let mut stats = self.stats.write().await;
            stats.classified_count += 1;
            stats.total_classification_time_ms += duration;
        }
        
        Ok(best_intent)
    }

    /// Classify with specific classifier
    async fn classify_with_classifier(&self, text: &str, classifier: &IntentClassifier) -> Result<Intent> {
        let mut best_intent = Intent {
            name: "unknown".to_string(),
            category: classifier.category.clone(),
            confidence: 0.0,
            parameters: HashMap::new(),
        };
        
        for pattern in &classifier.patterns {
            if pattern.pattern.is_match(text) {
                let confidence = self.calculate_pattern_confidence(text, &pattern.pattern).await?;
                let adjusted_confidence = (confidence + pattern.confidence_boost).min(1.0);
                
                if adjusted_confidence > best_intent.confidence {
                    best_intent = Intent {
                        name: pattern.intent_name.clone(),
                        category: classifier.category.clone(),
                        confidence: adjusted_confidence,
                        parameters: self.extract_parameters(text, &pattern.pattern).await?,
                    };
                }
            }
        }
        
        Ok(best_intent)
    }

    /// Calculate pattern confidence
    async fn calculate_pattern_confidence(&self, text: &str, pattern: &regex::Regex) -> Result<f64> {
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

    /// Extract parameters from text
    async fn extract_parameters(&self, text: &str, pattern: &regex::Regex) -> Result<HashMap<String, String>> {
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

    /// Get classification statistics
    pub async fn get_stats(&self) -> u64 {
        self.stats.read().await.classified_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_intent_classification() {
        let context_manager = Arc::new(RwLock::new(ContextManager::new()));
        let engine = IntentEngine::new(context_manager).await.unwrap();
        
        // Debug: check if the pattern matches
        let text = "Create a new React component for user authentication";
        let pattern = regex::Regex::new(r"(?i)(create|generate|build|write|make|implement|develop)\s+(a\s+)?(new\s+)?(function|class|component|module|service|api|endpoint|method|interface|type|struct|enum)").unwrap();
        println!("Pattern matches: {}", pattern.is_match(text));
        
        let processed_text = ProcessedText {
            content: text.to_string(),
            intent: Intent {
                name: "unknown".to_string(),
                category: IntentCategory::Unknown,
                confidence: 0.0,
                parameters: HashMap::new(),
            },
            entities: Vec::new(),
            confidence: 0.8,
            processing_time_ms: 10,
        };
        
        let result = engine.classify_intent(processed_text).await.unwrap();
        // Debug: print the result
        println!("Intent result: {:?}", result);
        assert_eq!(result.category, IntentCategory::CodeGeneration);
        assert!(result.confidence > 0.0);
    }

    #[tokio::test]
    async fn test_vision_intent_classification() {
        let context_manager = Arc::new(RwLock::new(ContextManager::new()));
        let engine = IntentEngine::new(context_manager).await.unwrap();
        
        let vision = ProcessedVision {
            description: "A screenshot showing a button and form input".to_string(),
            objects: Vec::new(),
            text: None,
            ui_elements: None,
            entities: Vec::new(),
            confidence: 0.8,
            analysis: VisionAnalysis {
                scene_type: SceneType::WebPage,
                color_palette: Vec::new(),
                complexity: 0.5,
                readability: 0.8,
                accessibility: AccessibilityScore {
                    color_contrast: 0.9,
                    font_size: 0.9,
                    spacing: 0.9,
                    navigation: 0.9,
                    overall: 0.9,
                },
            },
        };
        
        let result = engine.classify_vision_intent(vision).await.unwrap();
        assert_eq!(result.category, IntentCategory::UI);
    }
}