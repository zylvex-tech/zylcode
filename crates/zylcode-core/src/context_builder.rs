use anyhow::Result;
use std::path::PathBuf;
use walkdir::WalkDir;
use crate::agent_protocol::AgentContext;

/// Builds context for the agent model
pub struct ContextBuilder {
    workspace_root: PathBuf,
    max_files: usize,
    max_file_size: usize,
}

impl ContextBuilder {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            max_files: 100,
            max_file_size: 1024 * 100, // 100KB
        }
    }
    
    /// Build context for the agent
    pub async fn build(&self, user_goal: &str, _recent_files: &[PathBuf]) -> Result<AgentContext> {
        let file_tree = self.get_file_tree()?;
        // P1.2 integration: rank relevant files with the real Repository
        // Intelligence pipeline (the same one the benchmark and the
        // `repo-context` CLI surface use), so the AgentLoop's gathered
        // context reflects proven retrieval quality. Falls back to the
        // legacy keyword heuristic when indexing is unavailable or yields
        // nothing, keeping the agent functional outside a workable repo.
        let relevant_files = match self.find_relevant_files_intelligent(user_goal) {
            Ok(files) if !files.is_empty() => files,
            _ => self.find_relevant_files(user_goal, &file_tree).await?,
        };
        let git_status = self.get_git_status().await.ok();
        
        Ok(AgentContext {
            workspace_root: self.workspace_root.to_string_lossy().to_string(),
            file_tree,
            relevant_files,
            user_goal: user_goal.to_string(),
            recent_tool_results: Vec::new(),
            current_plan: None,
            current_state: "Created".to_string(),
            git_status,
            errors: Vec::new(),
        })
    }
    
    /// Get file tree of workspace
    fn get_file_tree(&self) -> Result<Vec<String>> {
        let mut files = Vec::new();
        
        for entry in WalkDir::new(&self.workspace_root)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            let relative = path.strip_prefix(&self.workspace_root).unwrap_or(path);
            
            // Skip hidden files and directories
            if relative.to_string_lossy().starts_with('.') {
                continue;
            }
            
            // Skip node_modules, target, etc.
            let path_str = relative.to_string_lossy();
            if path_str.contains("node_modules") || path_str.contains("target") || path_str.contains(".git") {
                continue;
            }
            
            if path.is_file() {
                files.push(relative.to_string_lossy().to_string());
            }
            
            if files.len() >= self.max_files {
                break;
            }
        }
        
        Ok(files)
    }
    
    /// Find relevant files based on user goal
    async fn find_relevant_files(&self, user_goal: &str, file_tree: &[String]) -> Result<Vec<String>> {
        let goal_lower = user_goal.to_lowercase();
        let mut relevant = Vec::new();
        
        // Simple keyword matching
        for file in file_tree {
            let file_lower = file.to_lowercase();
            
            // Check if file matches keywords in goal. All four arms of the
            // previous `else if` chain had the identical body, so the chain is
            // exactly equivalent to a single disjunction — and a disjunction
            // cannot push the same file more than once.
            let matches_goal = (goal_lower.contains("test") && file_lower.contains("test"))
                || (goal_lower.contains("readme") && file_lower.contains("readme"))
                || (goal_lower.contains("config") && file_lower.contains("config"))
                || (goal_lower.contains("src") && file_lower.contains("src"));
            if matches_goal {
                relevant.push(file.clone());
            }
        }
        
        // If no specific matches, include main files
        if relevant.is_empty() {
            for file in file_tree {
                if file.ends_with(".rs") || file.ends_with(".ts") || file.ends_with(".js") || file.ends_with(".py") {
                    relevant.push(file.clone());
                    if relevant.len() >= 10 {
                        break;
                    }
                }
            }
        }
        
        Ok(relevant)
    }
    
    /// Get git status
    async fn get_git_status(&self) -> Result<String> {
        let output = tokio::process::Command::new("git")
            .arg("status")
            .arg("--short")
            .current_dir(&self.workspace_root)
            .output()
            .await?;
        
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
    
    /// Read file content
    pub async fn read_file(&self, path: &str) -> Result<String> {
        let full_path = self.workspace_root.join(path);
        let content = tokio::fs::read_to_string(&full_path).await?;
        
        if content.len() > self.max_file_size {
            Ok(content[..self.max_file_size].to_string() + "\n... [truncated]")
        } else {
            Ok(content)
        }
    }

    /// Rank files for the task with the real Repository Intelligence
    /// pipeline (P1.2). Returns file ids (repo-relative paths) ordered by
    /// retrieval relevance, capped at the builder's file budget.
    fn find_relevant_files_intelligent(&self, task: &str) -> Result<Vec<String>> {
        let query = crate::intelligence::query::build_repo_query(&self.workspace_root)?;
        let results = query.relevant_context(task);
        Ok(results
            .iter()
            .filter(|r| r.resource_type == "file")
            .take(10)
            .map(|r| r.resource.clone())
            .collect())
    }
}

#[cfg(test)]
mod intelligence_integration_tests {
    use super::*;

    /// P1.2 integration proof: ContextBuilder::build must consume the real
    /// Repository Intelligence implementation, not duplicate search logic.
    /// The proof: with the workspace rooted at the ZylCode repository
    /// itself, the ranked files must include the crash-recovery test files
    /// for a crash-recovery task — a result the legacy keyword heuristic
    /// (test/readme/config/src substring matching) cannot produce for this
    /// phrasing, and that the benchmark proves comes from the intelligence
    /// pipeline.
    #[tokio::test]
    async fn context_builder_uses_intelligence_ranking() {
        // Locate the repository root relative to this crate.
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .canonicalize()
            .expect("repo root");
        let builder = ContextBuilder::new(root.clone());

        let ctx = builder
            .build(
                "Which files would likely need inspection to modify crash recovery?",
                &[],
            )
            .await
            .expect("context build");

        assert!(
            ctx.relevant_files
                .iter()
                .any(|f| f.contains("crash_recovery")),
            "intelligence-ranked relevant_files must contain crash_recovery; got {:?}",
            ctx.relevant_files
        );
        assert!(
            ctx.relevant_files.iter().any(|f| f.contains("agent.rs")),
            "co-change evidence must surface agent.rs; got {:?}",
            ctx.relevant_files
        );
    }
}