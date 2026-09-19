use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::text_processor::TextProcessor;
use super::types::*;

/// Voice processor for speech-to-text and voice analysis
pub struct VoiceProcessor {
    /// Injected text processor. Voice processing currently returns its own
    /// simulated transcript and never delegates to it, so the handle is stored
    /// but never read. Kept because `new()` is a public entry point that
    /// already accepts it — class B (unfinished).
    _text_processor: Arc<TextProcessor>,
    command_registry: RwLock<Vec<VoiceCommand>>,
    stats: RwLock<VoiceProcessorStats>,
}

#[derive(Debug, Default)]
struct VoiceProcessorStats {
    processed_count: u64,
    total_processing_time_ms: u64,
    average_confidence: f64,
}

impl VoiceProcessor {
    /// Create a new voice processor
    pub async fn new(text_processor: Arc<TextProcessor>) -> Result<Self> {
        // Register default voice commands. Built as a single literal rather
        // than `Vec::new()` followed by four `push` calls (clippy
        // `vec_init_then_push`); the resulting value is identical.
        let command_registry = vec![
            VoiceCommand {
                trigger: "hey zylcode".to_string(),
                action: "activate".to_string(),
                parameters: HashMap::new(),
                description: "Activate ZylCode assistant".to_string(),
            },
            VoiceCommand {
                trigger: "create a new".to_string(),
                action: "code_generation".to_string(),
                parameters: HashMap::new(),
                description: "Start code generation".to_string(),
            },
            VoiceCommand {
                trigger: "fix the bug".to_string(),
                action: "debugging".to_string(),
                parameters: HashMap::new(),
                description: "Start debugging".to_string(),
            },
            VoiceCommand {
                trigger: "run tests".to_string(),
                action: "testing".to_string(),
                parameters: HashMap::new(),
                description: "Run tests".to_string(),
            },
        ];

        Ok(Self {
            _text_processor: text_processor,
            command_registry: RwLock::new(command_registry),
            stats: RwLock::new(VoiceProcessorStats::default()),
        })
    }

    /// Process audio input
    pub async fn process_audio(&self, audio: AudioBuffer) -> Result<ProcessedVoice> {
        let start = std::time::Instant::now();

        // Simulate speech-to-text processing
        // In a real implementation, this would use a STT engine like Whisper
        let text = self.simulate_stt(&audio).await?;
        let confidence = self.calculate_stt_confidence(&audio).await?;
        let language = self.detect_language(&text).await?;
        let words = self.extract_words(&text).await?;
        let voice_analysis = self.analyze_voice(&audio).await?;

        // Update stats
        let duration = start.elapsed().as_millis() as u64;
        {
            let mut stats = self.stats.write().await;
            stats.processed_count += 1;
            stats.total_processing_time_ms += duration;
            stats.average_confidence =
                (stats.average_confidence * (stats.processed_count - 1) as f64 + confidence)
                    / stats.processed_count as f64;
        }

        Ok(ProcessedVoice {
            text,
            confidence,
            language,
            duration_ms: audio.duration_ms,
            words,
            voice_analysis: Some(voice_analysis),
        })
    }

    /// Register a voice command
    pub async fn register_command(&self, command: VoiceCommand) -> Result<()> {
        let mut registry = self.command_registry.write().await;
        registry.push(command);
        Ok(())
    }

    /// Get registered commands
    pub async fn get_commands(&self) -> Vec<VoiceCommand> {
        self.command_registry.read().await.clone()
    }

    /// Simulate speech-to-text
    async fn simulate_stt(&self, audio: &AudioBuffer) -> Result<String> {
        // Simulate processing time based on audio duration
        let processing_time = (audio.duration_ms / 10).max(10);
        tokio::time::sleep(tokio::time::Duration::from_millis(processing_time)).await;

        // Return simulated text based on audio characteristics
        let text = if audio.duration_ms < 1000 {
            "Hello".to_string()
        } else if audio.duration_ms < 3000 {
            "Create a new React component".to_string()
        } else {
            "Fix the bug in the authentication module and add unit tests".to_string()
        };

        Ok(text)
    }

    /// Calculate STT confidence
    async fn calculate_stt_confidence(&self, audio: &AudioBuffer) -> Result<f64> {
        // Simulate confidence based on audio quality
        let mut confidence: f64 = 0.7; // Base confidence

        // Higher sample rate = higher confidence
        if audio.sample_rate >= 44100 {
            confidence += 0.1;
        }

        // More channels = higher confidence (stereo vs mono)
        if audio.channels >= 2 {
            confidence += 0.05;
        }

        // Higher bit depth = higher confidence
        if audio.bits_per_sample >= 16 {
            confidence += 0.05;
        }

        // Longer audio = higher confidence (up to a point)
        if audio.duration_ms > 2000 {
            confidence += 0.1;
        }

        Ok(confidence.min(1.0))
    }

    /// Detect language from text
    async fn detect_language(&self, text: &str) -> Result<String> {
        // Simple language detection based on common words
        let text_lower = text.to_lowercase();

        if text_lower.contains("the") || text_lower.contains("and") || text_lower.contains("is") {
            Ok("en".to_string())
        } else if text_lower.contains("le")
            || text_lower.contains("la")
            || text_lower.contains("est")
        {
            Ok("fr".to_string())
        } else if text_lower.contains("der")
            || text_lower.contains("die")
            || text_lower.contains("ist")
        {
            Ok("de".to_string())
        } else {
            Ok("en".to_string()) // Default to English
        }
    }

    /// Extract words with timing
    async fn extract_words(&self, text: &str) -> Result<Vec<Word>> {
        let words: Vec<Word> = text
            .split_whitespace()
            .enumerate()
            .map(|(i, word)| Word {
                text: word.to_string(),
                start_ms: (i as u64) * 200,
                end_ms: (i as u64 + 1) * 200,
                confidence: 0.9,
            })
            .collect();

        Ok(words)
    }

    /// Analyze voice characteristics
    async fn analyze_voice(&self, _audio: &AudioBuffer) -> Result<VoiceAnalysis> {
        // Simulate voice analysis.
        //
        // Both branches of the previous `if audio.duration_ms < 2000` checks
        // returned the same value (`Tone::Neutral` / `Emotion::Neutral`), so the
        // condition was dead. Flattened to the value it always produced;
        // introducing a real duration heuristic is a separate change.
        let tone = Tone::Neutral;
        let emotion = Emotion::Neutral;

        let speaking_rate = 150.0; // words per minute
        let volume = 0.7; // 0.0 to 1.0
        let pitch = 150.0; // Hz

        Ok(VoiceAnalysis {
            tone,
            emotion,
            speaking_rate,
            volume,
            pitch,
        })
    }

    /// Get processing statistics
    pub async fn get_stats(&self) -> u64 {
        self.stats.read().await.processed_count
    }
}

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_voice_processing() {
        let context_manager = Arc::new(tokio::sync::RwLock::new(
            super::super::context_manager::ContextManager::new(),
        ));
        let text_processor = Arc::new(TextProcessor::new(context_manager).await.unwrap());
        let voice_processor = VoiceProcessor::new(text_processor).await.unwrap();

        let audio = AudioBuffer {
            data: vec![0; 44100], // 1 second of silence
            sample_rate: 44100,
            channels: 1,
            bits_per_sample: 16,
            duration_ms: 1000,
        };

        let result = voice_processor.process_audio(audio).await.unwrap();
        assert!(!result.text.is_empty());
        assert!(result.confidence > 0.0);
    }

    #[tokio::test]
    async fn test_voice_command_registration() {
        let context_manager = Arc::new(tokio::sync::RwLock::new(
            super::super::context_manager::ContextManager::new(),
        ));
        let text_processor = Arc::new(TextProcessor::new(context_manager).await.unwrap());
        let voice_processor = VoiceProcessor::new(text_processor).await.unwrap();

        let command = VoiceCommand {
            trigger: "test command".to_string(),
            action: "test_action".to_string(),
            parameters: HashMap::new(),
            description: "Test command".to_string(),
        };

        voice_processor.register_command(command).await.unwrap();
        let commands = voice_processor.get_commands().await;
        assert!(commands.iter().any(|c| c.trigger == "test command"));
    }
}
