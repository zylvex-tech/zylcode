use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Screen capture options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureOptions {
    pub include_cursor: bool,
    pub include_windows: bool,
    pub format: ImageFormat,
    pub quality: u8,
}

impl Default for CaptureOptions {
    fn default() -> Self {
        Self {
            include_cursor: true,
            include_windows: true,
            format: ImageFormat::PNG,
            quality: 90,
        }
    }
}

/// Image formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImageFormat {
    PNG,
    JPEG,
    BMP,
    TIFF,
}

/// Screen image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenImage {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
    pub timestamp: DateTime<Utc>,
    pub cursor_position: Option<CursorPosition>,
}

/// Cursor position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub x: i32,
    pub y: i32,
}

/// Region for capture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Window ID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowId {
    pub id: u64,
    pub title: Option<String>,
    pub process_id: Option<u32>,
}

/// Screen analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenAnalysis {
    pub elements: Vec<UiElement>,
    pub text_regions: Vec<TextRegion>,
    pub color_palette: Vec<String>,
    pub complexity: f64,
    pub readability: f64,
}

/// UI element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiElement {
    pub element_type: UiElementType,
    pub text: Option<String>,
    pub bounding_box: BoundingBox,
    pub confidence: f64,
    pub properties: HashMap<String, String>,
}

/// UI element types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UiElementType {
    Button,
    Input,
    Text,
    Link,
    Image,
    Menu,
    Dropdown,
    Checkbox,
    RadioButton,
    Slider,
    Toggle,
    Tab,
    Table,
    Form,
    Modal,
    Tooltip,
    Icon,
    Navigation,
    Header,
    Footer,
    Sidebar,
    Card,
    List,
    Grid,
    Unknown,
}

/// Bounding box
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Text region
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextRegion {
    pub text: String,
    pub bounding_box: BoundingBox,
    pub confidence: f64,
    pub language: Option<String>,
}

/// Mouse button
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

/// Keyboard key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Key {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    Escape,
    Tab,
    CapsLock,
    Shift,
    Ctrl,
    Alt,
    Super,
    Space,
    Enter,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
    PrintScreen,
    ScrollLock,
    Pause,
    NumLock,
    Clear,
    Menu,
}

/// Window action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WindowAction {
    Focus,
    Minimize,
    Maximize,
    Restore,
    Close,
    Move { x: i32, y: i32 },
    Resize { width: u32, height: u32 },
}

/// Workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
    pub variables: HashMap<String, String>,
    pub triggers: Vec<WorkflowTrigger>,
}

/// Workflow step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub name: String,
    pub step_type: StepType,
    pub parameters: HashMap<String, String>,
    pub conditions: Vec<StepCondition>,
    pub on_failure: FailureAction,
}

/// Step types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepType {
    CaptureScreen,
    AnalyzeScreen,
    ClickElement,
    TypeText,
    PressKey,
    WaitForElement,
    WaitForTimeout,
    Conditional,
    Loop,
    Custom,
}

/// Step condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepCondition {
    pub condition_type: ConditionType,
    pub value: String,
    pub operator: ComparisonOperator,
}

/// Condition types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionType {
    ElementExists,
    ElementVisible,
    TextContains,
    ImageMatches,
    VariableEquals,
    TimeElapsed,
}

/// Comparison operators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOperator {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

/// Failure actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FailureAction {
    Continue,
    Retry,
    Stop,
    Skip,
    Custom,
}

/// Workflow trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTrigger {
    pub trigger_type: TriggerType,
    pub parameters: HashMap<String, String>,
}

/// Trigger types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerType {
    Manual,
    Scheduled,
    Event,
    FileChange,
    Webhook,
}

/// Workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub definition: WorkflowDefinition,
    pub status: WorkflowStatus,
    pub current_step: Option<String>,
    pub variables: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Workflow status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowStatus {
    Created,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// Workflow result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    pub workflow_id: String,
    pub status: WorkflowStatus,
    pub steps_executed: usize,
    pub duration_ms: u64,
    pub output: HashMap<String, String>,
    pub errors: Vec<WorkflowError>,
}

/// Workflow error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowError {
    pub step_id: String,
    pub error_type: ErrorType,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

/// Error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorType {
    ElementNotFound,
    Timeout,
    InvalidInput,
    PermissionDenied,
    NetworkError,
    Unknown,
}

/// Schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub schedule_type: ScheduleType,
    pub interval: Option<u64>,
    pub cron_expression: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

/// Schedule types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScheduleType {
    Once,
    Interval,
    Cron,
    Daily,
    Weekly,
    Monthly,
}

/// Recording ID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingId {
    pub id: String,
    pub started_at: DateTime<Utc>,
}

/// Input recording
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputRecording {
    pub id: String,
    pub events: Vec<InputEvent>,
    pub duration_ms: u64,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
}

/// Input event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputEvent {
    pub event_type: InputEventType,
    pub timestamp: DateTime<Utc>,
    pub data: InputEventData,
}

/// Input event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputEventType {
    MouseMove,
    MouseClick,
    MouseDoubleClick,
    MouseDrag,
    MouseScroll,
    KeyPress,
    KeyRelease,
    KeyType,
    ClipboardCopy,
    ClipboardPaste,
    WindowFocus,
    WindowResize,
}

/// Input event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InputEventData {
    MouseMove { x: i32, y: i32 },
    MouseClick { x: i32, y: i32, button: MouseButton },
    KeyPress { key: Key, modifiers: Vec<Key> },
    KeyType { text: String },
    Clipboard { content: String },
    Window { id: String, action: WindowAction },
}

/// Computer Use System statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputerUseStats {
    pub screens_captured: u64,
    pub gui_actions: u64,
    pub vision_analyses: u64,
    pub workflows_executed: u64,
    pub recordings: u64,
}
