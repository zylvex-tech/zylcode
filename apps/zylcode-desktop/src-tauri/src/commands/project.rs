use crate::EngineState;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, State};
use zylcode_core::project::{ProjectFilesystem, Project, ProjectResult, CreateProjectRequest, OpenProjectRequest, FileTreeNode, FileContent, FileGitStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = \"snake_case\")]
pub enum ProjectError {
    ProjectNotOpen,
    PathOutsideProject,
    FileNotFound,
    FileChangedOnDisk,
    FileTooLarge,
    BinaryFile,
    UnsupportedEncoding,
    PermissionDenied,
    GitUnavailable,
    NotGitRepository,
    IoError { message: String },
    ProjectAlreadyOpen,
    InvalidProjectPath,
    NotAFile,
    NotADirectory,
}

impl From<ProjectError> for String {
    fn from(err: ProjectError) -> String {
        serde_json::to_string(&err).unwrap_or_else(|_| format!(\"{:?}\", err))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenProjectResponse {
    pub success: bool,
    pub project: Option<Project>,
    pub root_listing: Vec<FileTreeNode>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadFileResponse {
    pub success: bool,
    pub file: Option<FileContent>,
    pub version: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteFileRequest {
    pub relative_path: String,
    pub content: String,
    pub expected_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteFileResponse {
    pub success: bool,
    pub new_version: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListDirectoryResponse {
    pub success: bool,
    pub children: Option<Vec<FileTreeNode>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatusResponse {
    pub success: bool,
    pub statuses: Option<Vec<FileGitStatus>>,
    pub is_git_repo: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseProjectResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProjectSession {
    pub project: Project,
    filesystem: ProjectFilesystem,
    file_versions: std::collections::HashMap<String, String>,
}

impl ProjectSession {
    pub fn new(project: Project) -> Result<Self> {
        let filesystem = ProjectFilesystem::new(&project.root_path)?;
        Ok(Self {
            project,
            filesystem,
            file_versions: std::collections::HashMap::new(),
        }
    }

    fn compute_version(&self, path: &Path) -> Result<String> {
        let metadata = std::fs::metadata(path)?;
        let mtime = metadata.modified()
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos().to_string())
            .unwrap_or_else(|_| \"0\".to_string());
        let size = metadata.len();
        Ok(format!(\"{}:{}\", mtime, size))
    }

    fn canonicalize_and_validate(&self, relative_path: &str) -> Result<PathBuf> {
        let requested = self.filesystem.project_root().join(relative_path);
        
        let canonical_project = self.filesystem.project_root().canonicalize()
            .context(\"Failed to canonicalize project root\")?;
        let canonical_requested = requested.canonicalize()
            .context(\"Failed to canonicalize requested path\")?;

        if !canonical_requested.starts_with(&canonical_project) {
            return Err(anyhow::anyhow!(\"PathOutsideProject\"));
        }

        Ok(canonical_requested)
    }
}

#[tauri::command]
pub async fn create_project(
    request: CreateProjectRequest,
    app: AppHandle,
    state: State<'_, EngineState>,
) -> Result<OpenProjectResponse, String> {
    let engine_state = state.inner();
    
    let path = PathBuf::from(&request.root_path);
    if !path.exists() {
        return Ok(OpenProjectResponse {
            success: false,
            project: None,
            root_listing: vec![],
            error: Some(ProjectError::InvalidProjectPath.to_string()),
        });
    }

    if !path.is_dir() {
        return Ok(OpenProjectResponse {
            success: false,
            project: None,
            root_listing: vec![],
            error: Some(ProjectError::NotADirectory.to_string()),
        });
    }

    let canonical_path = path.canonicalize()
        .map_err(|e| ProjectError::IoError { message: e.to_string() })?;

    let project_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let repo_info = get_repository_info(&path).await;

    let project = Project {
        id: project_id.clone(),
        name: request.name,
        root_path: canonical_path.to_string_lossy().to_string(),
        repository: repo_info,
        open_files: vec![],
        recent_files: vec![],
        ui_state: Default::default(),
        created_at: now.clone(),
        last_accessed: now,
    };

    let session = ProjectSession::new(project.clone())
        .map_err(|e| ProjectError::IoError { message: e.to_string() })?;

    let root_listing = session.filesystem.list_directory(\"\")
        .unwrap_or_default();

    engine_state.current_project.write().await = Some(session);

    Ok(OpenProjectResponse {
        success: true,
        project: Some(project),
        root_listing,
        error: None,
    })
}

#[tauri::command]
pub async fn open_project(
    request: OpenProjectRequest,
    app: AppHandle,
    state: State<'_, EngineState>,
) -> Result<OpenProjectResponse, String> {
    let engine_state = state.inner();
    
    let path = PathBuf::from(&request.path);
    if !path.exists() {
        return Ok(OpenProjectResponse {
            success: false,
            project: None,
            root_listing: vec![],
            error: Some(ProjectError::InvalidProjectPath.to_string()),
        });
    }

    let canonical_path = path.canonicalize()
        .map_err(|e| ProjectError::IoError { message: e.to_string() })?;

    let project_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let repo_info = get_repository_info(&canonical_path).await;

    let project = Project {
        id: project_id,
        name: canonical_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(\"Unknown\")
            .to_string(),
        root_path: canonical_path.to_string_lossy().to_string(),
        repository: repo_info,
        open_files: vec![],
        recent_files: vec![],
        ui_state: Default::default(),
        created_at: now.clone(),
        last_accessed: now,
    };

    let session = ProjectSession::new(project.clone())
        .map_err(|e| ProjectError::IoError { message: e.to_string() })?;

    let root_listing = session.filesystem.list_directory(\"\")
        .unwrap_or_default();

    engine_state.current_project.write().await = Some(session);

    Ok(OpenProjectResponse {
        success: true,
        project: Some(project),
        root_listing,
        error: None,
    })
}

#[tauri::command]
pub async fn list_projects(
    state: State<'_, EngineState>,
) -> Result<Vec<Project>, String> {
    let engine_state = state.inner();
    let current = engine_state.current_project.read().await;
    if let Some(session) = current.as_ref() {
        Ok(vec![session.project.clone()])
    } else {
        Ok(vec![])
    }
}

#[tauri::command]
pub async fn read_file(
    relative_path: String,
    state: State<'_, EngineState>,
) -> Result<ReadFileResponse, String> {
    let engine_state = state.inner();
    let current = engine_state.current_project.read().await;
    let session = match current.as_ref() {
        Some(s) => s,
        None => return Ok(ReadFileResponse {
            success: false,
            file: None,
            version: None,
            error: Some(ProjectError::ProjectNotOpen.to_string()),
        }),
    };

    let canonical_path = session.canonicalize_and_validate(&relative_path)
        .map_err(|_| ProjectError::PathOutsideProject.to_string())?;

    if !canonical_path.is_file() {
        return Ok(ReadFileResponse {
            success: false,
            file: None,
            version: None,
            error: Some(ProjectError::NotAFile.to_string()),
        });
    }

    let metadata = std::fs::metadata(&canonical_path)
        .map_err(|e| ProjectError::IoError { message: e.to_string() })?;

    if metadata.len() > 10_000_000 {
        return Ok(ReadFileResponse {
            success: false,
            file: None,
            version: None,
            error: Some(ProjectError::FileTooLarge.to_string()),
        });
    }

    let content = std::fs::read_to_string(&canonical_path)
        .map_err(|e| ProjectError::IoError { message: e.to_string() })?;

    let ext = canonical_path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase());
    let language = ext.as_ref().map(|e| detect_language(e));
    let line_count = content.lines().count();

    let version = session.compute_version(&canonical_path)
        .unwrap_or_else(|_| \"0\".to_string());

    let ext = canonical_path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase());
    let language = ext.as_ref().map(|e| detect_language(e));

    Ok(ReadFileResponse {
        success: true,
        file: Some(zylcode_core::project::types::FileContent {
            path: relative_path,
            absolute_path: canonical_path.to_string_lossy().to_string(),
            content,
            encoding: \"utf-8\".to_string(),
            language,
            size: metadata.len(),
            modified: chrono::Utc::now().to_rfc3339(),
            line_count: content.lines().count(),
        }),
        version: Some(version),
        error: None,
    })
}

#[tauri::command]
pub async fn write_file(
    request: WriteFileRequest,
    state: State<'_, EngineState>,
) -> Result<WriteFileResponse, String> {
    let engine_state = state.inner();
    let mut current = engine_state.current_project.write().await;
    let session = match current.as_mut() {
        Some(s) => s,
        None => return Ok(WriteFileResponse {
            success: false,
            new_version: None,
            error: Some(ProjectError::ProjectNotOpen.to_string()),
        }),
    };

    let canonical_path = session.canonicalize_and_validate(&request.relative_path)
        .map_err(|_| ProjectError::PathOutsideProject.to_string())?;

    if !canonical_path.is_file() {
        return Ok(WriteFileResponse {
            success: false,
            new_version: None,
            error: Some(ProjectError::NotAFile.to_string()),
        });
    }

    if let Some(expected) = &request.expected_version {
        let current_version = session.compute_version(&PathBuf::from(&request.relative_path))
            .unwrap_or_default();
        if current_version != *expected {
            return Ok(WriteFileResponse {
                success: false,
                new_version: None,
                error: Some(ProjectError::FileChangedOnDisk.to_string()),
            });
        }
    }

    let temp_path = session.filesystem.project_root().join(format!(\".tmp.{}.tmp\", uuid::Uuid::new_v4()));
    
    std::fs::write(&temp_path, &request.content)
        .map_err(|e| ProjectError::IoError { message: e.to_string() })?;

    let target_path = session.filesystem.project_root().join(&request.relative_path);
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ProjectError::IoError { message: e.to_string() })?;
    }

    std::fs::rename(&temp_path, &target_path)
        .map_err(|e| ProjectError::IoError { message: e.to_string() })?;

    let new_version = session.compute_version(&PathBuf::from(&request.relative_path))
        .unwrap_or_else(|_| \"0\".to_string());

    session.file_versions.insert(request.relative_path.clone(), new_version.clone());

    Ok(WriteFileResponse {
        success: true,
        new_version: Some(new_version),
        error: None,
    })
}

#[tauri::command]
pub async fn list_directory(
    relative_path: String,
    state: State<'_, EngineState>,
) -> Result<ListDirectoryResponse, String> {
    let engine_state = state.inner();
    let current = engine_state.current_project.read().await;
    let session = match current.as_ref() {
        Some(s) => s,
        None => return Ok(ListDirectoryResponse {
            success: false,
            children: None,
            error: Some(ProjectError::ProjectNotOpen.to_string()),
        }),
    };

    let canonical_path = session.canonicalize_and_validate(&relative_path)
        .map_err(|_| ProjectError::PathOutsideProject.to_string())?;

    if !canonical_path.is_dir() {
        return Ok(ListDirectoryResponse {
            success: false,
            children: None,
            error: Some(ProjectError::NotADirectory.to_string()),
        });
    }

    let children = session.filesystem.list_directory(&relative_path)
        .map_err(|e| ProjectError::IoError { message: e.to_string() })?;

    Ok(ListDirectoryResponse {
        success: true,
        children: Some(children),
        error: None,
    })
}

#[tauri::command]
pub async fn get_git_status(
    state: State<'_, EngineState>,
) -> Result<GitStatusResponse, String> {
    let engine_state = state.inner();
    let current = engine_state.current_project.read().await;
    let session = match current.as_ref() {
        Some(s) => s,
        None => return Ok(GitStatusResponse {
            success: false,
            statuses: None,
            is_git_repo: false,
            error: Some(ProjectError::ProjectNotOpen.to_string()),
        }),
    };

    match session.filesystem.get_git_status() {
        Ok(statuses) => Ok(GitStatusResponse {
            success: true,
            statuses: Some(statuses),
            is_git_repo: true,
            error: None,
        }),
        Err(_) => Ok(GitStatusResponse {
            success: true,
            statuses: Some(vec![]),
            is_git_repo: false,
            error: None,
        }),
    }
}

#[tauri::command]
pub async fn close_project(
    state: State<'_, EngineState>,
) -> Result<CloseProjectResponse, String> {
    let engine_state = state.inner();
    let mut current = engine_state.current_project.write().await;
    
    if current.is_none() {
        return Ok(CloseProjectResponse {
            success: false,
            error: Some(ProjectError::ProjectNotOpen.to_string()),
        });
    }

    *current = None;

    Ok(CloseProjectResponse {
        success: true,
        error: None,
    })
}

async fn get_repository_info(path: &Path) -> Option<zylcode_core::project::types::RepositoryInfo> {
    let output = std::process::Command::new(\"git\")
        .args([\"rev-parse\", \"--abbrev-ref\", \"HEAD\"])
        .current_dir(path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let has_changes = std::process::Command::new(\"git\")
        .args([\"status\", \"--porcelain\"])
        .current_dir(path)
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);

    let last_commit = std::process::Command::new(\"git\")
        .args([\"rev-parse\", \"--short\", \"HEAD\"])
        .current_dir(path)
        .output()
        .ok()
        .and_then(|o| if o.status.success() { Some(String::from_utf8_lossy(&o.stdout).trim().to_string()) } else { None });

    let remote = std::process::Command::new(\"git\")
        .args([\"config\", \"--get\", \"remote.origin.url\"])
        .current_dir(path)
        .output()
        .ok()
        .and_then(|o| if o.status.success() { Some(String::from_utf8_lossy(&o.stdout).trim().to_string()) } else { None });

    Some(zylcode_core::project::types::RepositoryInfo {
        remote_url: remote,
        current_branch: branch,
        has_uncommitted_changes: has_changes,
        last_commit_sha: last_commit,
    })
}

fn detect_language(ext: &str) -> String {
    match ext {
        \"rs\" => \"rust\",
        \"ts\" | \"tsx\" => \"typescript\",
        \"js\" | \"jsx\" | \"mjs\" | \"cjs\" => \"javascript\",
        \"py\" => \"python\",
        \"java\" => \"java\",
        \"cpp\" | \"cc\" | \"cxx\" => \"cpp\",
        \"c\" => \"c\",
        \"h\" | \"hpp\" => \"cpp\",
        \"go\" => \"go\",
        \"rb\" => \"ruby\",
        \"php\" => \"php\",
        \"swift\" => \"swift\",
        \"kt\" | \"kts\" => \"kotlin\",
        \"scala\" => \"scala\",
        \"clj\" | \"cljs\" | \"cljc\" => \"clojure\",
        \"hs\" => \"haskell\",
        \"ml\" | \"mli\" => \"ocaml\",
        \"fs\" | \"fsx\" | \"fsi\" => \"fsharp\",
        \"json\" => \"json\",
        \"yaml\" | \"yml\" => \"yaml\",
        \"toml\" => \"toml\",
        \"xml\" => \"xml\",
        \"html\" | \"htm\" => \"html\",
        \"css\" => \"css\",
        \"scss\" => \"scss\",
        \"sass\" => \"sass\",
        \"less\" => \"less\",
        \"md\" | \"mdx\" => \"markdown\",
        \"txt\" => \"plaintext\",
        \"sh\" | \"bash\" | \"zsh\" | \"fish\" => \"shell\",
        \"ps1\" => \"powershell\",
        \"bat\" | \"cmd\" => \"batch\",
        \"dockerfile\" => \"dockerfile\",
        \"sql\" => \"sql\",
        \"graphql\" | \"gql\" => \"graphql\",
        \"proto\" => \"protobuf\",
        \"vue\" => \"vue\",
        \"svelte\" => \"svelte\",
        \"astro\" => \"astro\",
        \"elm\" => \"elm\",
        \"ex\" | \"exs\" => \"elixir\",
        \"erl\" | \"hrl\" => \"erlang\",
        \"lua\" => \"lua\",
        \"pl\" | \"pm\" => \"perl\",
        \"r\" => \"r\",
        \"jl\" => \"julia\",
        \"nim\" => \"nim\",
        \"zig\" => \"zig\",
        _ => \"plaintext\",
    }.to_string()
}
