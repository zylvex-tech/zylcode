pub mod text_processor;
pub mod voice_processor;
pub mod vision_processor;
pub mod file_processor;
pub mod intent_engine;
pub mod context_manager;
pub mod types;

use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

use self::text_processor::TextProcessor;
use self::voice_processor::VoiceProcessor;
use self::vision_processor::VisionProcessor;
use self::file_processor::FileProcessor;
use self::intent_engine::IntentEngine;
use self::context_manager::ContextManager;
pub use self::types::*;

/// AI Input System - Multi-modal input processing
pub struct AIInputSystem {
    text_processor: Arc<TextProcessor>,
    voice_processor: Arc<VoiceProcessor>,
    vision_processor: Arc<VisionProcessor>,
    file_processor: Arc<FileProcessor>,
    intent_engine: Arc<IntentEngine>,
    context_manager: Arc<RwLock<ContextManager>>,
}

impl AIInputSystem {
    /// Create a new AI Input System
    pub async fn new() -> Result<Self> {
        let context_manager = Arc::new(RwLock::new(ContextManager::new()));
        let text_processor = Arc::new(TextProcessor::new(context_manager.clone()).await?);
        let voice_processor = Arc::new(VoiceProcessor::new(text_processor.clone()).await?);
        let vision_processor = Arc::new(VisionProcessor::new().await?);
        let file_processor = Arc::new(FileProcessor::new().await?);
        let intent_engine = Arc::new(IntentEngine::new(context_manager.clone()).await?);

        Ok(Self {
            text_processor,
            voice_processor,
            vision_processor,
            file_processor,
            intent_engine,
            context_manager,
        })
    }

    /// Process text input
    pub async fn process_text(&self, text: &str, context: InputContext) -> Result<ProcessedInput> {
        let processed_text = self.text_processor.process(text, &context).await?;
        let intent = self.intent_engine.classify_intent(processed_text.clone()).await?;
        
        Ok(ProcessedInput {
            input_type: InputType::Text,
            content: processed_text.content.clone(),
            intent: intent.clone(),
            entities: processed_text.entities.clone(),
            confidence: processed_text.confidence,
            context: context.clone(),
            timestamp: chrono::Utc::now(),
        })
    }

    /// Process voice input
    pub async fn process_voice(&self, audio: AudioBuffer, context: InputContext) -> Result<ProcessedInput> {
        let processed_voice = self.voice_processor.process_audio(audio).await?;
        let processed_text = self.text_processor.process(&processed_voice.text, &context).await?;
        let intent = self.intent_engine.classify_intent(processed_text.clone()).await?;
        
        Ok(ProcessedInput {
            input_type: InputType::Voice,
            content: processed_text.content.clone(),
            intent: intent.clone(),
            entities: processed_text.entities.clone(),
            confidence: processed_voice.confidence,
            context: context.clone(),
            timestamp: chrono::Utc::now(),
        })
    }

    /// Process vision input
    pub async fn process_vision(&self, image: ImageBuffer, context: InputContext) -> Result<ProcessedInput> {
        let processed_vision = self.vision_processor.analyze_image(image).await?;
        let intent = self.intent_engine.classify_vision_intent(processed_vision.clone()).await?;
        
        Ok(ProcessedInput {
            input_type: InputType::Vision,
            content: processed_vision.description.clone(),
            intent: intent.clone(),
            entities: processed_vision.entities.clone(),
            confidence: processed_vision.confidence,
            context: context.clone(),
            timestamp: chrono::Utc::now(),
        })
    }

    /// Process file input
    pub async fn process_file(&self, file: File, context: InputContext) -> Result<ProcessedInput> {
        let processed_file = self.file_processor.process_file(file).await?;
        let intent = self.intent_engine.classify_file_intent(processed_file.clone()).await?;
        
        Ok(ProcessedInput {
            input_type: InputType::File,
            content: processed_file.summary.clone(),
            intent: intent.clone(),
            entities: processed_file.entities.clone(),
            confidence: processed_file.confidence,
            context: context.clone(),
            timestamp: chrono::Utc::now(),
        })
    }

    /// Get current context
    pub async fn get_context(&self) -> InputContext {
        self.context_manager.read().await.get_current_context()
    }

    /// Update context
    pub async fn update_context(&self, context: InputContext) -> Result<()> {
        self.context_manager.write().await.update_context(context);
        Ok(())
    }

    /// Get supported input types
    pub fn supported_input_types(&self) -> Vec<InputType> {
        vec![
            InputType::Text,
            InputType::Voice,
            InputType::Vision,
            InputType::File,
        ]
    }

    /// Get system statistics
    pub async fn get_stats(&self) -> AIInputStats {
        AIInputStats {
            text_processed: self.text_processor.get_stats().await,
            voice_processed: self.voice_processor.get_stats().await,
            vision_processed: self.vision_processor.get_stats().await,
            files_processed: self.file_processor.get_stats().await,
            intents_classified: self.intent_engine.get_stats().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ai_input_system_creation() {
        let system = AIInputSystem::new().await.unwrap();
        let input_types = system.supported_input_types();
        assert!(input_types.contains(&InputType::Text));
        assert!(input_types.contains(&InputType::Voice));
        assert!(input_types.contains(&InputType::Vision));
        assert!(input_types.contains(&InputType::File));
    }

    #[tokio::test]
    async fn test_text_processing() {
        let system = AIInputSystem::new().await.unwrap();
        let context = InputContext::default();
        
        let result = system.process_text("Create a new React component", context).await.unwrap();
        assert_eq!(result.input_type, InputType::Text);
        assert!(result.confidence > 0.0);
    }
}