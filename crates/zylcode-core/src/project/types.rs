use serde::{Deserialize, Serialize};

/// A ZylCode project represents a persistent software development workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Unique project identifier
    pub id: String,
    /// Human-readable project name
    pub name: String,
    /// Absolute path to the project root directory
    pub root_path: String,
    /// Repository information if the project is a git repository
    #[serde(default)]
    pub repository: Option<RepositoryInfo>,
    /// Currently open files (relative paths from project root)
    #[serde(default)]
    pub open_files: Vec<String>,
    /// Recently accessed files (relative paths from project root)
    #[serde(default)]
    pub recent_files: Vec<String>,
    /// UI state (panel sizes, theme, etc.)
    #[serde(default)]
    pub ui_state: UIState,
    /// Created timestamp (ISO 8601)
    pub created_at: String,
    /// Last accessed timestamp (ISO 8601)
    pub last_accessed: String,
}

/// Repository information for git-tracked projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryInfo {
    /// Remote URL (if available)
    pub remote_url: Option<String>,
    /// Current branch
    pub current_branch: String,
    /// Whether there are uncommitted changes
    pub has_uncommitted_changes: bool,
    /// Last commit SHA (short)
    pub last_commit_sha: Option<String>,
}

/// Persistent UI state for the project
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UIState {
    /// Active surface/view
    #[serde(default)]
    pub active_surface: String,
    /// Activity rail selection
    #[serde(default = "default_activity")]
    pub active_activity: String,
    /// Context sidebar state
    #[serde(default)]
    pub sidebar_width: Option<u32>,
    /// Agent dock state
    #[serde(default)]
    pub dock_module: Option<String>,
    /// Bottom panel state
    #[serde(default)]
    pub bottom_panel_open: bool,
    #[serde(default)]
    pub bottom_panel_tab: Option<String>,
    /// Agent dock visibility
    #[serde(default)]
    pub dock_visible: bool,
    /// Context sidebar visibility
    #[serde(default)]
    pub sidebar_visible: bool,
}

fn default_activity() -> String {
    "explorer".to_string()
}

/// Project metadata for listing (without full UI state)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub last_accessed: String,
    pub repository: Option<RepositoryInfo>,
}

/// Request to create a new project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub root_path: String,
}

/// Request to open an existing project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenProjectRequest {
    pub path: String,
}

/// Result of a project operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectResult {
    pub success: bool,
    pub project: Option<Project>,
    pub error: Option<String>,
}

/// File tree node for Explorer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTreeNode {
    pub name: String,
    pub path: String, // Relative to project root
    pub absolute_path: String,
    pub is_directory: bool,
    pub is_file: bool,
    pub extension: Option<String>,
    pub size: Option<u64>,
    pub modified: Option<String>, // ISO 8601
    #[serde(default)]
    pub children: Vec<FileTreeNode>,
    #[serde(default)]
    pub is_expanded: bool,
    #[serde(default)]
    pub is_selected: bool,
    #[serde(default)]
    pub is_active: bool,
}

/// File content for opening in editor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContent {
    pub path: String, // Relative to project root
    pub absolute_path: String,
    pub content: String,
    pub content_hash: String,
    pub encoding: String,
    pub language: Option<String>,
    pub size: u64,
    pub modified: String,
    pub line_count: usize,
}

/// Git status for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileGitStatus {
    pub path: String,
    pub status: GitFileStatus,
    pub staged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GitFileStatus {
    Unmodified,
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    Ignored,
    Conflict,
}

/// Project filesystem operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemOperation {
    pub operation: FilesystemOpType,
    pub source: String,
    pub destination: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FilesystemOpType {
    CreateFile,
    CreateDirectory,
    Delete,
    Rename,
    Move,
    Copy,
}
