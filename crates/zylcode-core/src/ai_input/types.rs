use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Input types supported by the AI Input System
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InputType {
    Text,
    Voice,
    Vision,
    File,
}

/// Input context for processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputContext {
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub project_id: Option<String>,
    pub language: String,
    pub timezone: String,
    pub preferences: HashMap<String, String>,
    pub history: Vec<ProcessedInput>,
}

impl Default for InputContext {
    fn default() -> Self {
        Self {
            user_id: None,
            session_id: None,
            project_id: None,
            language: "en".to_string(),
            timezone: "UTC".to_string(),
            preferences: HashMap::new(),
            history: Vec::new(),
        }
    }
}

/// Processed input result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedInput {
    pub input_type: InputType,
    pub content: String,
    pub intent: Intent,
    pub entities: Vec<Entity>,
    pub confidence: f64,
    pub context: InputContext,
    pub timestamp: DateTime<Utc>,
}

/// Intent classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub name: String,
    pub category: IntentCategory,
    pub confidence: f64,
    pub parameters: HashMap<String, String>,
}

/// Intent categories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum IntentCategory {
    CodeGeneration,
    CodeReview,
    CodeRefactoring,
    Testing,
    Documentation,
    Debugging,
    Deployment,
    Database,
    API,
    UI,
    FileOperation,
    SearchQuery,
    Conversation,
    Unknown,
}

/// Entity extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub name: String,
    pub entity_type: EntityType,
    pub value: String,
    pub confidence: f64,
    pub start_pos: usize,
    pub end_pos: usize,
}

/// Entity types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EntityType {
    Code,
    FilePath,
    URL,
    Email,
    PhoneNumber,
    Date,
    Time,
    Number,
    Boolean,
    Language,
    Framework,
    Library,
    Command,
    Parameter,
    Variable,
    Function,
    Class,
    Interface,
    Unknown,
}

impl fmt::Display for EntityType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EntityType::Code => write!(f, "Code"),
            EntityType::FilePath => write!(f, "FilePath"),
            EntityType::URL => write!(f, "URL"),
            EntityType::Email => write!(f, "Email"),
            EntityType::PhoneNumber => write!(f, "PhoneNumber"),
            EntityType::Date => write!(f, "Date"),
            EntityType::Time => write!(f, "Time"),
            EntityType::Number => write!(f, "Number"),
            EntityType::Boolean => write!(f, "Boolean"),
            EntityType::Language => write!(f, "Language"),
            EntityType::Framework => write!(f, "Framework"),
            EntityType::Library => write!(f, "Library"),
            EntityType::Command => write!(f, "Command"),
            EntityType::Parameter => write!(f, "Parameter"),
            EntityType::Variable => write!(f, "Variable"),
            EntityType::Function => write!(f, "Function"),
            EntityType::Class => write!(f, "Class"),
            EntityType::Interface => write!(f, "Interface"),
            EntityType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Audio buffer for voice processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioBuffer {
    pub data: Vec<u8>,
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub duration_ms: u64,
}

/// Processed voice result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedVoice {
    pub text: String,
    pub confidence: f64,
    pub language: String,
    pub duration_ms: u64,
    pub words: Vec<Word>,
    pub voice_analysis: Option<VoiceAnalysis>,
}

/// Word with timing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Word {
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub confidence: f64,
}

/// Voice analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceAnalysis {
    pub tone: Tone,
    pub emotion: Emotion,
    pub speaking_rate: f64,
    pub volume: f64,
    pub pitch: f64,
}

/// Tone analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Tone {
    Neutral,
    Happy,
    Sad,
    Angry,
    Surprised,
    Fearful,
    Disgusted,
}

/// Emotion analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Emotion {
    Neutral,
    Positive,
    Negative,
    Excited,
    Calm,
    Stressed,
}

/// Image buffer for vision processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageBuffer {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
    pub channels: u8,
}

/// Image formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImageFormat {
    PNG,
    JPEG,
    GIF,
    BMP,
    WebP,
    TIFF,
}

/// Processed vision result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedVision {
    pub description: String,
    pub objects: Vec<DetectedObject>,
    pub text: Option<String>,
    pub ui_elements: Option<Vec<UiElement>>,
    pub entities: Vec<Entity>,
    pub confidence: f64,
    pub analysis: VisionAnalysis,
}

/// Detected object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedObject {
    pub name: String,
    pub confidence: f64,
    pub bounding_box: BoundingBox,
    pub category: ObjectCategory,
}

/// Bounding box
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Object categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectCategory {
    Person,
    Animal,
    Vehicle,
    Furniture,
    Electronics,
    Food,
    Building,
    Nature,
    Text,
    UI,
    Code,
    Unknown,
}

/// UI element detection
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

/// Vision analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionAnalysis {
    pub scene_type: SceneType,
    pub color_palette: Vec<String>,
    pub complexity: f64,
    pub readability: f64,
    pub accessibility: AccessibilityScore,
}

/// Scene types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SceneType {
    CodeEditor,
    WebPage,
    MobileApp,
    DesktopApp,
    Document,
    Image,
    Video,
    Diagram,
    Chart,
    Table,
    Form,
    Navigation,
    Dashboard,
    Settings,
    Unknown,
}

/// Accessibility score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityScore {
    pub color_contrast: f64,
    pub font_size: f64,
    pub spacing: f64,
    pub navigation: f64,
    pub overall: f64,
}

/// File for processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub name: String,
    pub path: String,
    pub content: Vec<u8>,
    pub file_type: FileType,
    pub size: u64,
    pub metadata: HashMap<String, String>,
}

/// File types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileType {
    Text,
    Code,
    Document,
    Image,
    Audio,
    Video,
    Archive,
    Database,
    Configuration,
    Unknown,
}

/// Processed file result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedFile {
    pub summary: String,
    pub file_type: FileType,
    pub language: Option<String>,
    pub line_count: Option<usize>,
    pub word_count: Option<usize>,
    pub entities: Vec<Entity>,
    pub confidence: f64,
    pub analysis: FileAnalysis,
}

/// File analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysis {
    pub complexity: f64,
    pub maintainability: f64,
    pub test_coverage: Option<f64>,
    pub dependencies: Vec<String>,
    pub issues: Vec<Issue>,
}

/// Issue detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub severity: IssueSeverity,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub suggestion: Option<String>,
}

/// Issue severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

/// AI Input System statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIInputStats {
    pub text_processed: u64,
    pub voice_processed: u64,
    pub vision_processed: u64,
    pub files_processed: u64,
    pub intents_classified: u64,
}

/// Voice command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceCommand {
    pub trigger: String,
    pub action: String,
    pub parameters: HashMap<String, String>,
    pub description: String,
}

/// Input recording
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputRecording {
    pub id: String,
    pub events: Vec<InputEvent>,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
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

/// Mouse buttons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

/// Keyboard keys
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

/// Window actions
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
