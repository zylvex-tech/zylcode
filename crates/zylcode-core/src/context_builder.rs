use crate::agent_protocol::AgentContext;
use anyhow::Result;
use std::path::PathBuf;
use walkdir::WalkDir;

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

        // Deterministic repository-graph evidence for the goal: declarations
        // and references resolved through `zylcode-nav`, never inferred. An
        // index that cannot be built produces an explicit "unavailable" line
        // rather than an empty list — an empty list reads to a model as "no
        // relationships exist", which is exactly the claim this wave forbids.
        let navigation_citations =
            match crate::intelligence::nav_api::agent_citation_block(&self.workspace_root, user_goal, 12)
            {
                Ok(lines) => lines,
                Err(e) => vec![format!(
                    "(repository graph unavailable: {e} — treat every structural claim as interpretation)"
                )],
            };

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
            navigation_citations,
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
            if path_str.contains("node_modules")
                || path_str.contains("target")
                || path_str.contains(".git")
            {
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
    async fn find_relevant_files(
        &self,
        user_goal: &str,
        file_tree: &[String],
    ) -> Result<Vec<String>> {
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
                if file.ends_with(".rs")
                    || file.ends_with(".ts")
                    || file.ends_with(".js")
                    || file.ends_with(".py")
                {
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
    ///
    /// Served through the persisted index: the first call indexes, later
    /// calls with an unchanged tree are content-hash cache hits — the
    /// agent loop no longer pays a full re-index per gathered context.
    fn find_relevant_files_intelligent(&self, task: &str) -> Result<Vec<String>> {
        let query =
            crate::intelligence::persisted::PersistedIndex::new(&self.workspace_root).build()?;
        let results = query.relevant_context(task);
        // The ranked stream interleaves file, symbol, package and
        // entry-point resources. Capping the mixed stream before filtering
        // starved the file list whenever symbol results outranked files
        // (the co-change-backed evidence for `src/ledger.rs` sits below
        // several symbol hits). Filter to files first, then cap.
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
        // The co-change half: a file whose path shares no word with the task
        // must still surface, because it changes alongside the crash-recovery
        // tests in git history. This used to assert `agent.rs`; the module
        // split moved it to `agent/mod.rs`, which has no commit history of
        // its own yet, so the strongest co-change-backed file that still
        // lives at its historical path stands in. `repo-context` gives its
        // reason verbatim: "changes together with 'crates/zylcode-core/tests/
        // crash_recovery.rs' in git history".
        assert!(
            ctx.relevant_files
                .iter()
                .any(|f| f.contains("crates/zylcode-core/src/ledger.rs")),
            "co-change evidence must surface src/ledger.rs; got {:?}",
            ctx.relevant_files
        );
    }

    /// The AI-integration half of the wave: an agent's gathered context must
    /// carry repository-graph evidence it can quote, and every line in it must
    /// be deterministic — an `HEURISTIC` citation would let a model present a
    /// guess as a graph fact.
    #[tokio::test]
    async fn agent_context_carries_deterministic_graph_citations() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path();
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .expect("manifest");
        std::fs::create_dir_all(root.join("src")).expect("mkdir");
        std::fs::write(
            root.join("src/lib.rs"),
            "pub mod helper;\n\npub fn entry() {\n    helper::run();\n}\n",
        )
        .expect("lib");
        std::fs::write(
            root.join("src/helper.rs"),
            "pub fn run() -> u32 {\n    1\n}\n",
        )
        .expect("helper");

        let builder = ContextBuilder::new(root.to_path_buf());
        let ctx = builder
            .build("which files call run from entry", &[])
            .await
            .expect("context build");

        assert!(
            !ctx.navigation_citations.is_empty(),
            "the goal names a real symbol, so the context must carry graph evidence: {:?}",
            ctx.navigation_citations
        );
        assert!(
            ctx.navigation_citations
                .iter()
                .all(|line| line.starts_with("DETERMINISTIC")),
            "an agent may only ever quote deterministic evidence: {:?}",
            ctx.navigation_citations
        );
        assert!(
            ctx.navigation_citations
                .iter()
                .any(|line| line.contains("run")),
            "the cited symbol's own name must be quotable: {:?}",
            ctx.navigation_citations
        );
        assert!(
            ctx.navigation_citations
                .iter()
                .any(|line| line.contains("src/helper.rs:")),
            "every citation must be a location: {:?}",
            ctx.navigation_citations
        );
    }

    /// A workspace the index cannot read must say so in the context rather
    /// than passing an empty list — an empty list reads to a model as "no
    /// relationships exist".
    ///
    /// The root is a *file*, not a missing path: the Repository Intelligence
    /// warm-start writes `<root>/.zylcode/…`, and `create_dir_all` on that
    /// path silently brings a non-existent workspace root into existence
    /// before the navigation index ever sees it. A file cannot be turned into
    /// a directory, so `RepoIndex::build` refuses it for real.
    #[tokio::test]
    async fn an_unindexable_workspace_says_the_graph_is_unavailable() {
        let dir = tempfile::tempdir().expect("temp dir");
        let not_a_directory = dir.path().join("workspace.txt");
        std::fs::write(&not_a_directory, "not a workspace\n").expect("file root");
        let builder = ContextBuilder::new(not_a_directory);
        let ctx = builder
            .build("do anything", &[])
            .await
            .expect("context build must still succeed");

        assert_eq!(
            ctx.navigation_citations.len(),
            1,
            "{:?}",
            ctx.navigation_citations
        );
        assert!(
            ctx.navigation_citations[0].contains("repository graph unavailable"),
            "{}",
            ctx.navigation_citations[0]
        );
        assert!(
            ctx.navigation_citations[0].contains("interpretation"),
            "the line must tell the model to label structural claims as interpretation: {}",
            ctx.navigation_citations[0]
        );
    }

    /// G6, end to end: the warm-start index sits *before* the citation block
    /// on this path, so a missing workspace root used to be created by
    /// `create_dir_all(<root>/.zylcode)` and the nav guard then passed against
    /// the empty directory that write had just produced. Both halves must now
    /// fail closed: the context still builds (the agent must not die), but the
    /// citations say the graph is unavailable and the root stays missing.
    #[tokio::test]
    async fn a_missing_workspace_root_is_never_created_by_a_warm_start() {
        let dir = tempfile::tempdir().expect("temp dir");
        let missing = dir.path().join("no-such-workspace");
        assert!(!missing.exists());

        let builder = ContextBuilder::new(missing.clone());
        let ctx = builder
            .build("anything", &[])
            .await
            .expect("context build must still succeed");

        assert_eq!(
            ctx.navigation_citations.len(),
            1,
            "{:?}",
            ctx.navigation_citations
        );
        assert!(
            ctx.navigation_citations[0].contains("repository graph unavailable"),
            "{}",
            ctx.navigation_citations[0]
        );
        assert!(
            !missing.exists(),
            "indexing must not bring a missing workspace root into existence"
        );
    }
}
