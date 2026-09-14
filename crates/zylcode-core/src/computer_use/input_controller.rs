use std::sync::RwLock;
use std::collections::HashMap;
use anyhow::Result;
use chrono::Utc;

use super::types::*;

/// Input controller for recording and playing back input events
pub struct InputController {
    recordings: RwLock<HashMap<String, InputRecording>>,
    active_recordings: RwLock<HashMap<String, Vec<InputEvent>>>,
    stats: RwLock<InputControllerStats>,
}

#[derive(Debug, Default)]
struct InputControllerStats {
    recording_count: u64,
    playback_count: u64,
}

impl InputController {
    /// Create a new input controller
    pub async fn new() -> Result<Self> {
        Ok(Self {
            recordings: RwLock::new(HashMap::new()),
            active_recordings: RwLock::new(HashMap::new()),
            stats: RwLock::new(InputControllerStats::default()),
        })
    }

    /// Start recording input events
    pub async fn start_recording(&self) -> Result<RecordingId> {
        let recording_id = uuid::Uuid::new_v4().to_string();
        let started_at = Utc::now();
        
        // Initialize active recording
        let mut active_recordings = self.active_recordings.write().unwrap();
        active_recordings.insert(recording_id.clone(), Vec::new());
        
        Ok(RecordingId {
            id: recording_id,
            started_at,
        })
    }

    /// Record an input event
    pub async fn record_event(&self, recording_id: &str, event: InputEvent) -> Result<()> {
        let mut active_recordings = self.active_recordings.write().unwrap();
        let events = active_recordings.get_mut(recording_id)
            .ok_or_else(|| anyhow::anyhow!("Recording not found: {}", recording_id))?;
        
        events.push(event);
        
        Ok(())
    }

    /// Stop recording
    pub async fn stop_recording(&self, recording_id: RecordingId) -> Result<InputRecording> {
        let mut active_recordings = self.active_recordings.write().unwrap();
        let events = active_recordings.remove(&recording_id.id)
            .ok_or_else(|| anyhow::anyhow!("Recording not found: {}", recording_id.id))?;
        
        let ended_at = Utc::now();
        let duration_ms = (ended_at - recording_id.started_at).num_milliseconds() as u64;
        
        let recording = InputRecording {
            id: recording_id.id.clone(),
            events,
            duration_ms,
            started_at: recording_id.started_at,
            ended_at,
        };
        
        // Store the recording
        let mut recordings = self.recordings.write().unwrap();
        recordings.insert(recording_id.id.clone(), recording.clone());
        
        // Update stats
        {
            let mut stats = self.stats.write().unwrap();
            stats.recording_count += 1;
        }
        
        Ok(recording)
    }

    /// Play back a recording
    pub async fn play_recording(&self, recording: InputRecording) -> Result<()> {
        let start = std::time::Instant::now();
        
        // Simulate playing back events
        for event in &recording.events {
            // Simulate event playback delay
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            
            // In a real implementation, this would execute the event
            match &event.data {
                InputEventData::MouseMove { x, y } => {
                    // Simulate mouse move
                }
                InputEventData::MouseClick { x, y, button } => {
                    // Simulate mouse click
                }
                InputEventData::KeyPress { key, modifiers } => {
                    // Simulate key press
                }
                InputEventData::KeyType { text } => {
                    // Simulate text typing
                }
                InputEventData::Clipboard { content } => {
                    // Simulate clipboard operation
                }
                InputEventData::Window { id, action } => {
                    // Simulate window action
                }
            }
        }
        
        // Update stats
        let duration = start.elapsed().as_millis() as u64;
        {
            let mut stats = self.stats.write().unwrap();
            stats.playback_count += 1;
        }
        
        Ok(())
    }

    /// Get recording by ID
    pub async fn get_recording(&self, recording_id: &str) -> Result<InputRecording> {
        let recordings = self.recordings.read().unwrap();
        recordings.get(recording_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Recording not found: {}", recording_id))
    }

    /// List all recordings
    pub async fn list_recordings(&self) -> Vec<InputRecording> {
        let recordings = self.recordings.read().unwrap();
        recordings.values().cloned().collect()
    }

    /// Delete recording
    pub async fn delete_recording(&self, recording_id: &str) -> Result<()> {
        let mut recordings = self.recordings.write().unwrap();
        recordings.remove(recording_id)
            .ok_or_else(|| anyhow::anyhow!("Recording not found: {}", recording_id))?;
        
        Ok(())
    }

    /// Get controller statistics
    pub async fn get_stats(&self) -> u64 {
        let stats = self.stats.read().unwrap();
        stats.recording_count + stats.playback_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_input_recording() {
        let controller = InputController::new().await.unwrap();
        
        // Start recording
        let recording_id = controller.start_recording().await.unwrap();
        
        // Record some events
        let event1 = InputEvent {
            event_type: InputEventType::MouseMove,
            timestamp: Utc::now(),
            data: InputEventData::MouseMove { x: 100, y: 200 },
        };
        
        let event2 = InputEvent {
            event_type: InputEventType::MouseClick,
            timestamp: Utc::now(),
            data: InputEventData::MouseClick {
                x: 100,
                y: 200,
                button: MouseButton::Left,
            },
        };
        
        controller.record_event(&recording_id.id, event1).await.unwrap();
        controller.record_event(&recording_id.id, event2).await.unwrap();
        
        // Stop recording
        let recording = controller.stop_recording(recording_id).await.unwrap();
        assert_eq!(recording.events.len(), 2);
        
        // Get recording
        let retrieved = controller.get_recording(&recording.id).await.unwrap();
        assert_eq!(retrieved.id, recording.id);
    }

    #[tokio::test]
    async fn test_recording_playback() {
        let controller = InputController::new().await.unwrap();
        
        // Create a recording
        let recording_id = controller.start_recording().await.unwrap();
        
        let event = InputEvent {
            event_type: InputEventType::KeyType,
            timestamp: Utc::now(),
            data: InputEventData::KeyType {
                text: "Hello World".to_string(),
            },
        };
        
        controller.record_event(&recording_id.id, event).await.unwrap();
        let recording = controller.stop_recording(recording_id).await.unwrap();
        
        // Play back the recording
        controller.play_recording(recording).await.unwrap();
        
        let stats = controller.get_stats().await;
        assert!(stats > 0);
    }
}