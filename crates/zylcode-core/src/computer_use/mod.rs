pub mod screen_capture;
pub mod gui_automation;
pub mod vision_ai;
pub mod workflow_engine;
pub mod input_controller;
pub mod types;

use std::sync::Arc;
use anyhow::Result;

use self::screen_capture::ScreenCapture;
use self::gui_automation::GuiAutomation;
use self::vision_ai::VisionAi;
use self::workflow_engine::WorkflowEngine;
use self::input_controller::InputController;
pub use self::types::*;

/// Computer Use System - Screen capture, GUI automation, and workflow engine
pub struct ComputerUseSystem {
    screen_capture: Arc<ScreenCapture>,
    gui_automation: Arc<GuiAutomation>,
    vision_ai: Arc<VisionAi>,
    workflow_engine: Arc<WorkflowEngine>,
    input_controller: Arc<InputController>,
}

impl ComputerUseSystem {
    /// Create a new Computer Use System
    pub async fn new() -> Result<Self> {
        let screen_capture = Arc::new(ScreenCapture::new().await?);
        let gui_automation = Arc::new(GuiAutomation::new().await?);
        let vision_ai = Arc::new(VisionAi::new().await?);
        let workflow_engine = Arc::new(WorkflowEngine::new().await?);
        let input_controller = Arc::new(InputController::new().await?);

        Ok(Self {
            screen_capture,
            gui_automation,
            vision_ai,
            workflow_engine,
            input_controller,
        })
    }

    /// Capture screen
    pub async fn capture_screen(&self, options: CaptureOptions) -> Result<ScreenImage> {
        self.screen_capture.capture_screen(options).await
    }

    /// Capture region
    pub async fn capture_region(&self, region: Region) -> Result<ScreenImage> {
        self.screen_capture.capture_region(region).await
    }

    /// Capture window
    pub async fn capture_window(&self, window_id: WindowId) -> Result<ScreenImage> {
        self.screen_capture.capture_window(window_id).await
    }

    /// Analyze screen
    pub async fn analyze_screen(&self, image: ScreenImage) -> Result<ScreenAnalysis> {
        self.vision_ai.analyze_screen(image).await
    }

    /// Move mouse
    pub async fn move_mouse(&self, x: i32, y: i32) -> Result<()> {
        self.gui_automation.move_mouse(x, y).await
    }

    /// Click at position
    pub async fn click(&self, x: i32, y: i32, button: MouseButton) -> Result<()> {
        self.gui_automation.click(x, y, button).await
    }

    /// Type text
    pub async fn type_text(&self, text: &str) -> Result<()> {
        self.gui_automation.type_text(text).await
    }

    /// Press hotkey
    pub async fn press_hotkey(&self, keys: Vec<Key>) -> Result<()> {
        self.gui_automation.press_hotkey(keys).await
    }

    /// Manage window
    pub async fn manage_window(&self, action: WindowAction) -> Result<()> {
        self.gui_automation.manage_window(action).await
    }

    /// Copy to clipboard
    pub async fn copy_to_clipboard(&self, content: &str) -> Result<()> {
        self.gui_automation.copy_to_clipboard(content).await
    }

    /// Paste from clipboard
    pub async fn paste_from_clipboard(&self) -> Result<String> {
        self.gui_automation.paste_from_clipboard().await
    }

    /// Detect UI elements
    pub async fn detect_ui_elements(&self, image: ScreenImage) -> Result<Vec<UiElement>> {
        self.vision_ai.detect_elements(image).await
    }

    /// Recognize text
    pub async fn recognize_text(&self, image: ScreenImage) -> Result<String> {
        self.vision_ai.recognize_text(image).await
    }

    /// Create workflow
    pub async fn create_workflow(&self, definition: WorkflowDefinition) -> Result<Workflow> {
        self.workflow_engine.create_workflow(definition).await
    }

    /// Execute workflow
    pub async fn execute_workflow(&self, workflow_id: &str) -> Result<WorkflowResult> {
        self.workflow_engine.execute_workflow(workflow_id).await
    }

    /// Pause workflow
    pub async fn pause_workflow(&self, workflow_id: &str) -> Result<()> {
        self.workflow_engine.pause_workflow(workflow_id).await
    }

    /// Resume workflow
    pub async fn resume_workflow(&self, workflow_id: &str) -> Result<()> {
        self.workflow_engine.resume_workflow(workflow_id).await
    }

    /// Schedule workflow
    pub async fn schedule_workflow(&self, workflow_id: &str, schedule: Schedule) -> Result<()> {
        self.workflow_engine.schedule_workflow(workflow_id, schedule).await
    }

    /// Start input recording
    pub async fn start_recording(&self) -> Result<RecordingId> {
        self.input_controller.start_recording().await
    }

    /// Stop input recording
    pub async fn stop_recording(&self, recording_id: RecordingId) -> Result<InputRecording> {
        self.input_controller.stop_recording(recording_id).await
    }

    /// Play input recording
    pub async fn play_recording(&self, recording: InputRecording) -> Result<()> {
        self.input_controller.play_recording(recording).await
    }

    /// Get system statistics
    pub async fn get_stats(&self) -> ComputerUseStats {
        ComputerUseStats {
            screens_captured: self.screen_capture.get_stats().await,
            gui_actions: self.gui_automation.get_stats().await,
            vision_analyses: self.vision_ai.get_stats().await,
            workflows_executed: self.workflow_engine.get_stats().await,
            recordings: self.input_controller.get_stats().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_computer_use_system_creation() {
        let system = ComputerUseSystem::new().await.unwrap();
        let stats = system.get_stats().await;
        assert_eq!(stats.screens_captured, 0);
    }

    #[tokio::test]
    async fn test_screen_capture() {
        let system = ComputerUseSystem::new().await.unwrap();
        let options = CaptureOptions::default();
        
        // This would fail in a real environment without a display
        // but we're testing the API with simulated success
        let result = system.capture_screen(options).await;
        // In our simulated environment, this should succeed
        assert!(result.is_ok());
    }
}