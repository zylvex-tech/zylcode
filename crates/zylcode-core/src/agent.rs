use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;
use zylcode_mcp::ToolRegistry;

#[cfg(test)]
use zylcode_mcp::{DynamicTool, McpToolConfig, McpTransport};

use crate::agent_protocol::*;
use crate::context_builder::ContextBuilder;
use crate::ledger::{ExecutionState, LedgerEntry, LedgerStore, SessionCheckpoint};
use crate::router::TokenRouter;
use zylcode_mcp::real_tools::RiskLevel;

/// Trait for model clients
#[async_trait]
pub trait ModelClient: Send + Sync {
    async fn call(&self, prompt: &str, system: &str) -> Result<String>;
}

/// Real model client using TokenRouter
pub struct RealModelClient {
    router: Arc<TokenRouter>,
}

impl RealModelClient {
    pub fn new(router: Arc<TokenRouter>) -> Self {
        Self { router }
    }
}

#[async_trait]
impl ModelClient for RealModelClient {
    async fn call(&self, prompt: &str, system: &str) -> Result<String> {
        self.router.dispatch_prompt(prompt, system).await
    }
}

/// Test model client for deterministic testing
pub struct TestModelClient {
    responses: Vec<String>,
    index: std::sync::atomic::AtomicUsize,
}

impl TestModelClient {
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            responses,
            index: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl ModelClient for TestModelClient {
    async fn call(&self, _prompt: &str, _system: &str) -> Result<String> {
        let idx = self.index.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if idx < self.responses.len() {
            Ok(self.responses[idx].clone())
        } else {
            // Return a default response
            Ok(r#"{"action": "Complete", "payload": {"summary": "Task completed", "evidence": [], "remaining_limitations": []}}"#.to_string())
        }
    }
}

/// Agent execution state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentState {
    /// Initial state
    Created,
    /// Gathering context (files, repo structure, etc.)
    Analyzing,
    /// LLM generating a plan
    Planning,
    /// Waiting for user confirmation
    AwaitingApproval,
    /// Executing tools
    Executing,
    /// Checking results (tests, compilation, etc.)
    Verifying,
    /// Fixing errors found in verification
    Repairing,
    /// Task finished successfully
    Completed,
    /// Task failed
    Failed,
}

/// A single task in the agent session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: String,
    pub description: String,
    pub state: AgentState,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub tools_used: Vec<String>,
    pub context_files: Vec<PathBuf>,
}

/// Conversation message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_results: Option<Vec<ToolResult>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub call_id: String,
    pub output: serde_json::Value,
    pub success: bool,
    pub duration_ms: u64,
}

/// Agent session representing a complete task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: String,
    pub task_id: String,
    pub state: AgentState,
    pub messages: Vec<AgentMessage>,
    pub context: SessionContext,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub total_tokens: u32,
    pub total_cost: f64,
    pub iterations: u32,
    pub max_iterations: u32,
}

/// Context for the session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    pub working_directory: PathBuf,
    pub files_read: Vec<PathBuf>,
    pub files_modified: Vec<PathBuf>,
    pub commands_executed: Vec<ExecutedCommand>,
    pub test_results: Vec<TestResult>,
    pub plan: Option<String>,
    pub verification_status: Option<bool>,
    pub pending_tool_call: Option<ToolCallRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutedCommand {
    pub command: String,
    pub args: Vec<String>,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_name: String,
    pub passed: bool,
    pub output: String,
    pub duration_ms: u64,
}

/// Agent loop configuration
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub max_iterations: u32,
    pub require_approval: bool,
    pub auto_repair: bool,
    pub max_repair_attempts: u32,
    pub timeout: Duration,
    pub model: String,
    pub temperature: f32,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            require_approval: false,
            auto_repair: true,
            max_repair_attempts: 3,
            timeout: Duration::from_secs(300),
            model: "claude-3-sonnet-20240229".to_string(),
            temperature: 0.7,
        }
    }
}

/// The main agent loop engine
pub struct AgentLoop {
    config: AgentConfig,
    session: AgentSession,
    start_time: Instant,
    tool_registry: Arc<ToolRegistry>,
    model_client: Arc<dyn ModelClient>,
    context_builder: ContextBuilder,
    ledger: Arc<dyn LedgerStore>,
    /// Actor identity recorded on every tool invocation this loop makes.
    /// Defaults to `agent:{session_id}`; see [`AgentLoop::with_actor`].
    actor: Option<String>,
}

/// Tool call request from the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub tool_id: String,
    pub arguments: serde_json::Value,
    pub reason: String,
    pub expected_result: Option<String>,
}

/// Result of a recovery operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryResult {
    pub checkpoint_found: bool,
    pub agent_state: String,
    pub reconciliations: Vec<ReconciliationResult>,
}

/// Result of reconciling an ambiguous execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationResult {
    pub entry_id: Uuid,
    pub action_id: String,
    pub status: ReconciliationStatus,
    pub details: String,
}

/// Status of a reconciliation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReconciliationStatus {
    /// Side effect confirmed as executed
    ReconciledAsExecuted,
    /// Side effect confirmed as not executed
    Cancelled,
    /// Cannot determine — requires manual intervention
    RequiresReconciliation,
    /// Entry marked as Recorded
    Recorded,
}

impl AgentLoop {
    pub fn new(
        task_description: &str,
        working_dir: PathBuf,
        config: Option<AgentConfig>,
        tool_registry: Arc<ToolRegistry>,
        model_client: Arc<dyn ModelClient>,
        ledger: Arc<dyn LedgerStore>,
    ) -> Self {
        let config = config.unwrap_or_default();
        let session_id = Uuid::new_v4().to_string();
        let task_id = Uuid::new_v4().to_string();

        let session = AgentSession {
            id: session_id,
            task_id: task_id.clone(),
            state: AgentState::Created,
            messages: vec![AgentMessage {
                id: Uuid::new_v4().to_string(),
                role: MessageRole::User,
                content: task_description.to_string(),
                timestamp: chrono::Utc::now(),
                tool_calls: None,
                tool_results: None,
            }],
            context: SessionContext {
                working_directory: working_dir.clone(),
                files_read: Vec::new(),
                files_modified: Vec::new(),
                commands_executed: Vec::new(),
                test_results: Vec::new(),
                plan: None,
                verification_status: None,
                pending_tool_call: None,
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            total_tokens: 0,
            total_cost: 0.0,
            iterations: 0,
            max_iterations: config.max_iterations,
        };

        Self {
            config,
            session,
            start_time: Instant::now(),
            tool_registry,
            model_client,
            context_builder: ContextBuilder::new(working_dir),
            ledger,
            actor: None,
        }
    }

    /// Bind the actor identity recorded on every tool invocation this loop
    /// makes.
    ///
    /// The identity is bound task-locally around each dispatch (see
    /// `zylcode_mcp::with_actor`), so it reaches the evidence ledger even
    /// though registry tools receive a `ToolContext` with `actor: None`.
    /// Unset, the loop records `agent:{session_id}` — the run is identified
    /// even when the caller did not name it.
    pub fn with_actor(mut self, actor: impl Into<String>) -> Self {
        self.actor = Some(actor.into());
        self
    }

    /// The actor identity recorded for this loop's invocations.
    fn actor_id(&self) -> String {
        self.actor
            .clone()
            .unwrap_or_else(|| format!("agent:{}", self.session.id))
    }

    /// Save a checkpoint of the current session state to the ledger.
    pub async fn save_checkpoint(&self) -> Result<()> {
        // Get the last entry ID from the ledger
        let session_id = Uuid::parse_str(&self.session.id)?;
        let entries = self.ledger.get_entries(session_id).await?;
        let last_entry_id = entries.last().map(|e| e.id).unwrap_or(Uuid::nil());

        let checkpoint = SessionCheckpoint {
            session_id,
            last_entry_id,
            agent_state: format!("{:?}", self.session.state),
            context: serde_json::to_value(&self.session.context)?,
            timestamp: chrono::Utc::now(),
        };
        self.ledger.save_checkpoint(checkpoint).await?;
        Ok(())
    }

    /// Get the session ID.
    pub fn session_id(&self) -> &str {
        &self.session.id
    }

    /// Recover session state from a checkpoint in the ledger.
    pub async fn recover_from_checkpoint(&mut self) -> Result<RecoveryResult> {
        let session_id = Uuid::parse_str(&self.session.id)?;

        if let Some(checkpoint) = self.ledger.load_checkpoint(session_id).await? {
            // Restore agent state
            self.session.state = match checkpoint.agent_state.as_str() {
                "Created" => AgentState::Created,
                "Analyzing" => AgentState::Analyzing,
                "Planning" => AgentState::Planning,
                "AwaitingApproval" => AgentState::AwaitingApproval,
                "Executing" => AgentState::Executing,
                "Verifying" => AgentState::Verifying,
                "Repairing" => AgentState::Repairing,
                "Completed" => AgentState::Completed,
                "Failed" => AgentState::Failed,
                _ => AgentState::Created,
            };

            // Restore context
            if let Ok(context) = serde_json::from_value::<SessionContext>(checkpoint.context) {
                self.session.context = context;
            }

            // Scan ledger for incomplete executions and reconcile
            let entries = self.ledger.get_entries(session_id).await?;
            let mut reconciliation_results = Vec::new();

            for entry in &entries {
                match entry.state {
                    ExecutionState::Started => {
                        // Tool started but we don't know if it finished
                        // This is an ambiguous side effect — we must reconcile
                        let reconciliation = self.reconcile_ambiguous_execution(entry).await?;
                        reconciliation_results.push(reconciliation);
                    }
                    ExecutionState::Executed => {
                        // Tool finished but evidence wasn't recorded
                        // We can safely mark it as Recorded
                        self.ledger
                            .update_state(
                                entry.id,
                                ExecutionState::Recorded,
                                entry.payload.clone(),
                                None,
                            )
                            .await?;

                        reconciliation_results.push(ReconciliationResult {
                            entry_id: entry.id,
                            action_id: entry.action_id.clone(),
                            status: ReconciliationStatus::Recorded,
                            details: "Executed entry marked as Recorded".to_string(),
                        });
                    }
                    _ => {}
                }
            }

            Ok(RecoveryResult {
                checkpoint_found: true,
                agent_state: format!("{:?}", self.session.state),
                reconciliations: reconciliation_results,
            })
        } else {
            Ok(RecoveryResult {
                checkpoint_found: false,
                agent_state: "None".to_string(),
                reconciliations: Vec::new(),
            })
        }
    }

    /// Reconcile an ambiguous execution (Started but not Executed).
    /// This is critical for preventing duplicate side effects.
    async fn reconcile_ambiguous_execution(
        &self,
        entry: &LedgerEntry,
    ) -> Result<ReconciliationResult> {
        // Check the action type and inspect external state
        match entry.action_id.as_str() {
            id if id.starts_with("fs.write") || id.starts_with("fs.create") => {
                // File write — check if file exists and has expected content
                if let Some(path) = entry.arguments.get("path").and_then(|v| v.as_str()) {
                    let path_buf = std::path::PathBuf::from(path);
                    if path_buf.exists() {
                        // File exists — side effect likely occurred
                        // Mark as Executed but with warning
                        self.ledger
                            .update_state(
                                entry.id,
                                ExecutionState::Executed,
                                Some(serde_json::json!({
                                    "reconciliation": "File exists after crash",
                                    "path": path
                                })),
                                None,
                            )
                            .await?;

                        return Ok(ReconciliationResult {
                            entry_id: entry.id,
                            action_id: entry.action_id.clone(),
                            status: ReconciliationStatus::ReconciledAsExecuted,
                            details: format!("File {} exists after crash", path),
                        });
                    } else {
                        // File doesn't exist — side effect likely didn't occur
                        self.ledger
                            .update_state(
                                entry.id,
                                ExecutionState::Cancelled,
                                None,
                                Some("File does not exist after crash".to_string()),
                            )
                            .await?;

                        return Ok(ReconciliationResult {
                            entry_id: entry.id,
                            action_id: entry.action_id.clone(),
                            status: ReconciliationStatus::Cancelled,
                            details: format!("File {} does not exist after crash", path),
                        });
                    }
                }
            }
            id if id.starts_with("git.") => {
                // Git operation — check git status/log
                if id == "git.commit" {
                    // Check if commit exists in git log
                    // For now, mark as RequiresReconciliation
                    self.ledger
                        .update_state(
                            entry.id,
                            ExecutionState::Cancelled,
                            None,
                            Some("Git commit requires manual reconciliation".to_string()),
                        )
                        .await?;

                    return Ok(ReconciliationResult {
                        entry_id: entry.id,
                        action_id: entry.action_id.clone(),
                        status: ReconciliationStatus::RequiresReconciliation,
                        details: "Git commit cannot be automatically reconciled".to_string(),
                    });
                }
            }
            id if id.starts_with("shell.") => {
                // Shell command — cannot easily reconcile
                self.ledger
                    .update_state(
                        entry.id,
                        ExecutionState::Cancelled,
                        None,
                        Some("Shell command requires manual reconciliation".to_string()),
                    )
                    .await?;

                return Ok(ReconciliationResult {
                    entry_id: entry.id,
                    action_id: entry.action_id.clone(),
                    status: ReconciliationStatus::RequiresReconciliation,
                    details: "Shell command cannot be automatically reconciled".to_string(),
                });
            }
            _ => {
                // Unknown action — cannot reconcile
                self.ledger
                    .update_state(
                        entry.id,
                        ExecutionState::Cancelled,
                        None,
                        Some("Unknown action requires manual reconciliation".to_string()),
                    )
                    .await?;

                return Ok(ReconciliationResult {
                    entry_id: entry.id,
                    action_id: entry.action_id.clone(),
                    status: ReconciliationStatus::RequiresReconciliation,
                    details: "Unknown action cannot be automatically reconciled".to_string(),
                });
            }
        }

        // Default: mark as cancelled
        self.ledger
            .update_state(
                entry.id,
                ExecutionState::Cancelled,
                None,
                Some("Ambiguous execution cancelled".to_string()),
            )
            .await?;

        Ok(ReconciliationResult {
            entry_id: entry.id,
            action_id: entry.action_id.clone(),
            status: ReconciliationStatus::Cancelled,
            details: "Ambiguous execution cancelled".to_string(),
        })
    }

    /// Execute the agent loop until completion or failure
    pub async fn run(&mut self) -> Result<AgentState> {
        while self.session.state != AgentState::Completed
            && self.session.state != AgentState::Failed
            && self.session.state != AgentState::AwaitingApproval
            && self.session.iterations < self.session.max_iterations
        {
            self.step().await?;

            // Check timeout
            if self.start_time.elapsed() > self.config.timeout {
                self.session.state = AgentState::Failed;
                self.add_system_message("Task timed out".to_string());
                break;
            }

            // Check step limit
            if self.session.iterations >= self.config.max_iterations {
                self.session.state = AgentState::Failed;
                self.add_system_message("Step limit reached".to_string());
                break;
            }
        }

        Ok(self.session.state.clone())
    }

    /// Execute a single step of the agent loop
    pub async fn step(&mut self) -> Result<()> {
        self.session.iterations += 1;
        self.session.updated_at = chrono::Utc::now();

        match self.session.state {
            AgentState::Created => {
                self.transition_to(AgentState::Analyzing).await?;
            }
            AgentState::Analyzing => {
                self.gather_context().await?;
                self.transition_to(AgentState::Planning).await?;
            }
            AgentState::Planning => {
                self.generate_plan().await?;
                // Check if approval is required for the plan
                if self.config.require_approval {
                    self.transition_to(AgentState::AwaitingApproval).await?;
                } else {
                    self.transition_to(AgentState::Executing).await?;
                }
            }
            AgentState::AwaitingApproval => {
                // In a real implementation, this would wait for user input
                // For now, we stay in AwaitingApproval state
                // The agent loop will stop here until approval is granted
            }
            AgentState::Executing => {
                // Get the next tool call from the model
                let tool_call = self.get_next_tool_call().await?;

                // Check if approval is required for this tool
                if self.requires_approval(&tool_call) {
                    // Add approval request to session
                    self.add_system_message(format!(
                        "Approval required for: {} ({})",
                        tool_call.tool_id, tool_call.reason
                    ));
                    self.transition_to(AgentState::AwaitingApproval).await?;
                    // Store the tool call for later execution
                    self.session.context.pending_tool_call = Some(tool_call);
                } else {
                    // Auto-approve and execute the tool directly
                    // Log to ledger: Approved (auto-approved)
                    let prev_hash = self.compute_prev_hash().await?;
                    let entry_id = Uuid::new_v4();
                    let entry = LedgerEntry {
                        id: entry_id,
                        session_id: Uuid::parse_str(&self.session.id).unwrap(),
                        action_id: tool_call.tool_id.clone(),
                        arguments: tool_call.arguments.clone(),
                        state: ExecutionState::Approved,
                        prev_hash,
                        timestamp: chrono::Utc::now(),
                        payload: Some(serde_json::json!({
                            "auto_approved": true,
                            "actor": self.actor_id(),
                        })),
                        error: None,
                    };
                    self.ledger.append(entry).await?;

                    self.execute_tool_call(tool_call).await?;
                    self.transition_to(AgentState::Verifying).await?;
                }
            }
            AgentState::Verifying => {
                self.verify_results().await?;
                if self.session.context.verification_status == Some(false) {
                    if self.config.auto_repair {
                        self.transition_to(AgentState::Repairing).await?;
                    } else {
                        self.transition_to(AgentState::Failed).await?;
                    }
                } else {
                    self.transition_to(AgentState::Completed).await?;
                }
            }
            AgentState::Repairing => {
                self.repair_errors().await?;
                self.transition_to(AgentState::Executing).await?;
            }
            AgentState::Completed | AgentState::Failed => {
                // Terminal states, no action needed
            }
        }

        Ok(())
    }

    async fn transition_to(&mut self, new_state: AgentState) -> Result<()> {
        self.session.state = new_state;
        Ok(())
    }

    async fn gather_context(&mut self) -> Result<()> {
        // Gather context using ContextBuilder
        let task_desc = self.session.messages[0].content.clone();

        let context = self
            .context_builder
            .build(&task_desc, &self.session.context.files_read)
            .await?;

        // Add context to session messages as system message
        let context_str = format!(
            "Workspace: {}\nFiles: {:?}\nRelevant: {:?}\nGit: {:?}",
            context.workspace_root,
            context.file_tree.len(),
            context.relevant_files,
            context.git_status
        );

        self.add_system_message(format!("Context gathered:\n{}", context_str));

        // Read relevant files
        for file in &context.relevant_files {
            if let Ok(content) = self.context_builder.read_file(file).await {
                self.add_system_message(format!("Content of {}:\n{}", file, content));
                self.session.context.files_read.push(PathBuf::from(file));
            }
        }

        Ok(())
    }

    async fn generate_plan(&mut self) -> Result<()> {
        // Call model to generate a plan
        let task_desc = self.session.messages[0].content.clone();

        // Construct prompt for planning
        let prompt = format!(
            "You are an expert software engineer. Your task is to: {}\n\n\
             Based on the context provided, create a detailed plan to accomplish this task.\n\
             Return a JSON object with action 'Plan' and a list of steps.\n\n\
             Example:\n\
             {{\n\
               \"action\": \"Plan\",\n\
               \"payload\": {{\n\
                 \"steps\": [\n\
                   {{\n\
                     \"id\": \"1\",\n\
                     \"description\": \"Read the source file\",\n\
                     \"expected_files\": [\"src/main.rs\"],\n\
                     \"expected_tools\": [\"fs.read\"],\n\
                     \"risk\": \"Read\",\n\
                     \"verification\": \"File content loaded\"\n\
                   }}\n\
                 ]\n\
               }}\n\
             }}",
            task_desc
        );

        // Call model
        match self.call_model(&prompt).await {
            Ok(decision) => {
                match decision {
                    AgentDecision::Plan { steps } => {
                        self.session.context.plan = Some(serde_json::to_string(&steps)?);
                        self.add_assistant_message(format!(
                            "Plan generated with {} steps",
                            steps.len()
                        ));
                    }
                    _ => {
                        // If model returns something else, create a default plan
                        self.session.context.plan =
                            Some("Default plan: Analyze, Implement, Test".to_string());
                        self.add_assistant_message("Generated default plan".to_string());
                    }
                }
            }
            Err(e) => {
                // Fallback to default plan if model call fails
                self.session.context.plan =
                    Some("Default plan: Analyze, Implement, Test".to_string());
                self.add_system_message(format!("Model call failed, using default plan: {}", e));
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    async fn execute_plan(&mut self) -> Result<()> {
        // Execute the plan using model-driven tool selection
        self.add_system_message("Executing plan...".to_string());

        // Get tool schemas for the model
        let tool_schemas = self.tool_registry.list_schemas().await;

        // Construct prompt for execution
        let prompt = format!(
            "You are executing a plan. The current plan is: {:?}\n\n\
             Available tools:\n{}\n\n\
             Based on the plan and current state, select the next tool to execute.\n\
             Return a JSON object with action 'ToolCall' and the tool details.\n\n\
             Example:\n\
             {{\n\
               \"action\": \"ToolCall\",\n\
               \"payload\": {{\n\
                 \"tool_id\": \"fs.read\",\n\
                 \"arguments\": {{\"action\": \"read\", \"path\": \"src/main.rs\"}},\n\
                 \"reason\": \"Need to read the source file to understand the code\",\n\
                 \"expected_result\": \"File content\"\n\
               }}\n\
             }}",
            self.session.context.plan,
            tool_schemas
                .iter()
                .map(|s| format!("- {}: {}", s.id, s.description))
                .collect::<Vec<_>>()
                .join("\n")
        );

        // Call model to get tool call
        match self.call_model(&prompt).await {
            Ok(decision) => {
                match decision {
                    AgentDecision::ToolCall {
                        tool_id,
                        arguments,
                        reason,
                        expected_result: _,
                    } => {
                        self.add_system_message(format!(
                            "Model selected tool: {} - {}",
                            tool_id, reason
                        ));

                        // Execute the tool with actor attribution (see
                        // `execute_tool_call`).
                        let tool = self.tool_registry.get(&tool_id).await;
                        if let Some(tool) = tool {
                            let tool_result = zylcode_mcp::with_actor(self.actor_id(), async {
                                tool.call(arguments.clone()).await
                            })
                            .await;

                            match tool_result {
                                Ok(result) => {
                                    // Add tool result to session
                                    self.add_tool_result(&tool_id, result.clone(), true, 0);

                                    // Update session context
                                    if tool_id.starts_with("fs.") {
                                        if let Some(path) =
                                            arguments.get("path").and_then(|v| v.as_str())
                                        {
                                            self.session
                                                .context
                                                .files_read
                                                .push(PathBuf::from(path));
                                        }
                                    }

                                    // Add observation
                                    let observation = ToolObservation {
                                        tool_id: tool_id.clone(),
                                        success: true,
                                        stdout: result
                                            .get("stdout")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string()),
                                        stderr: result
                                            .get("stderr")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string()),
                                        exit_code: result
                                            .get("exit_code")
                                            .and_then(|v| v.as_i64())
                                            .map(|i| i as i32),
                                        changed_files: Vec::new(),
                                        diagnostics: Vec::new(),
                                        evidence_id: Uuid::new_v4().to_string(),
                                    };

                                    self.add_system_message(format!(
                                        "Tool execution result: {:?}",
                                        observation
                                    ));
                                }
                                Err(e) => {
                                    self.add_tool_result(
                                        &tool_id,
                                        serde_json::json!({"error": e.to_string()}),
                                        false,
                                        0,
                                    );
                                    self.add_system_message(format!(
                                        "Tool execution failed: {}",
                                        e
                                    ));
                                }
                            }
                        } else {
                            self.add_system_message(format!("Tool not found: {}", tool_id));
                            // Tool not found — cannot continue
                            self.session.state = AgentState::Failed;
                            return Ok(());
                        }
                    }
                    AgentDecision::Complete {
                        summary,
                        evidence: _,
                        remaining_limitations: _,
                    } => {
                        self.add_system_message(format!("Model declared completion: {}", summary));
                        self.session.state = AgentState::Completed;
                    }
                    _ => {
                        self.add_system_message("Model returned unexpected decision".to_string());
                    }
                }
            }
            Err(e) => {
                self.add_system_message(format!("Model call failed: {}", e));
            }
        }

        Ok(())
    }

    /// Execute the plan using model-driven tool selection
    async fn verify_results(&mut self) -> Result<()> {
        // Compute hash of previous entry for chain integrity
        let prev_hash = self.compute_prev_hash().await?;

        // Log to ledger: Verification started
        let entry_id = Uuid::new_v4();
        let entry = LedgerEntry {
            id: entry_id,
            session_id: Uuid::parse_str(&self.session.id).unwrap(),
            action_id: "verification".to_string(),
            arguments: serde_json::json!({}),
            state: ExecutionState::Started,
            prev_hash,
            timestamp: chrono::Utc::now(),
            payload: None,
            error: None,
        };
        self.ledger.append(entry).await?;

        // Call model to verify results
        let prompt = format!(
            "You are verifying the results of your work.\n\n\
             Task: {}\n\
             Files modified: {:?}\n\
             Commands executed: {:?}\n\n\
             Determine if the task is complete and all tests pass.\n\
             Return a JSON object with action 'Verify' or 'Complete' or 'Fail'.",
            self.session.messages[0].content,
            self.session.context.files_modified,
            self.session.context.commands_executed
        );

        match self.call_model(&prompt).await {
            Ok(decision) => {
                match decision {
                    AgentDecision::Verify { checks } => {
                        // Run verification checks
                        let mut check_results = Vec::new();

                        if checks.is_empty() {
                            // No checks provided — cannot verify
                            self.add_system_message(
                                "Verification failed: no checks provided".to_string(),
                            );
                            self.session.context.verification_status = Some(false);

                            // Log to ledger: Failed
                            self.ledger
                                .update_state(
                                    entry_id,
                                    ExecutionState::Failed,
                                    None,
                                    Some("No verification checks provided".to_string()),
                                )
                                .await?;
                        } else {
                            // For now, we require the model to provide checks
                            // In a real implementation, we would run actual checks
                            // But we cannot assume they pass — we need evidence
                            for check in &checks {
                                self.add_system_message(format!(
                                    "Verification check: {} - Requires evidence",
                                    check
                                ));
                                check_results.push(check.clone());
                            }

                            // Since we cannot actually run checks yet,
                            // we mark as UNVERIFIED, not Verified
                            self.session.context.verification_status = Some(false);

                            // Log to ledger: Failed (cannot verify without running checks)
                            self.ledger
                                .update_state(
                                    entry_id,
                                    ExecutionState::Failed,
                                    Some(serde_json::json!({
                                        "checks": check_results,
                                        "reason": "Checks provided but not executed"
                                    })),
                                    Some("Verification checks not executed".to_string()),
                                )
                                .await?;
                        }
                    }
                    AgentDecision::Complete {
                        summary,
                        evidence,
                        remaining_limitations: _,
                    } => {
                        // Debug: print evidence
                        tracing::info!(evidence = ?evidence, "Verification evidence received");

                        // Verify that evidence supports completion
                        if self.verify_completion_evidence(&evidence) {
                            self.add_system_message(format!("Verification passed: {}", summary));
                            self.session.context.verification_status = Some(true);

                            // Log to ledger: Verified
                            self.ledger.update_state(
                                entry_id,
                                ExecutionState::Verified,
                                Some(serde_json::json!({"summary": summary, "evidence": evidence})),
                                None,
                            ).await?;
                        } else {
                            self.add_system_message(
                                "Completion rejected: insufficient evidence".to_string(),
                            );
                            self.session.context.verification_status = Some(false);

                            // Log to ledger: Failed
                            self.ledger
                                .update_state(
                                    entry_id,
                                    ExecutionState::Failed,
                                    None,
                                    Some("Insufficient evidence".to_string()),
                                )
                                .await?;
                        }
                    }
                    AgentDecision::Fail {
                        reason,
                        error,
                        repair_suggestions: _,
                    } => {
                        self.add_system_message(format!(
                            "Verification failed: {} - {}",
                            reason, error
                        ));
                        self.session.context.verification_status = Some(false);

                        // Log to ledger: Failed
                        self.ledger
                            .update_state(
                                entry_id,
                                ExecutionState::Failed,
                                None,
                                Some(format!("{}: {}", reason, error)),
                            )
                            .await?;
                    }
                    _ => {
                        self.add_system_message(
                            "Model returned unexpected verification decision".to_string(),
                        );
                        // Unexpected decision = cannot verify = FAILED
                        self.session.context.verification_status = Some(false);

                        // Log to ledger: Failed (unexpected decision)
                        self.ledger
                            .update_state(
                                entry_id,
                                ExecutionState::Failed,
                                None,
                                Some("Unexpected verification decision from model".to_string()),
                            )
                            .await?;
                    }
                }
            }
            Err(e) => {
                self.add_system_message(format!("Verification model call failed: {}", e));
                // Model call failure = cannot verify = FAILED
                self.session.context.verification_status = Some(false);

                // Log to ledger: Failed (model call failed)
                self.ledger
                    .update_state(
                        entry_id,
                        ExecutionState::Failed,
                        None,
                        Some(format!("Verification model call failed: {}", e)),
                    )
                    .await?;
            }
        }

        Ok(())
    }

    /// Verify that completion evidence is sufficient
    fn verify_completion_evidence(&self, evidence: &[String]) -> bool {
        // Check if we have evidence of actual work
        // For now, require at least one tool execution
        let has_tool_execution = self
            .session
            .messages
            .iter()
            .any(|m| m.role == MessageRole::Tool);

        // Check if we have file modifications or command executions
        let _has_changes = !self.session.context.files_modified.is_empty()
            || !self.session.context.commands_executed.is_empty();

        // Check if we have evidence in the evidence list
        let has_evidence = !evidence.is_empty();

        // Require at least tool execution OR evidence
        has_tool_execution || has_evidence
    }

    async fn repair_errors(&mut self) -> Result<()> {
        // Call model to repair errors
        let prompt = format!(
            "You encountered an error during verification.\n\n\
             Task: {}\n\
             Error context: {:?}\n\n\
             Diagnose the issue and suggest repairs.\n\
             Return a JSON object with action 'ToolCall' to fix the issue.",
            self.session.messages[0].content,
            self.session.context.commands_executed.last()
        );

        match self.call_model(&prompt).await {
            Ok(decision) => {
                match decision {
                    AgentDecision::ToolCall {
                        tool_id,
                        arguments,
                        reason,
                        expected_result: _,
                    } => {
                        self.add_system_message(format!("Repair: {} - {}", tool_id, reason));

                        // Execute repair tool with actor attribution (see
                        // `execute_tool_call`).
                        let tool = self.tool_registry.get(&tool_id).await;
                        if let Some(tool) = tool {
                            let tool_result = zylcode_mcp::with_actor(self.actor_id(), async {
                                tool.call(arguments.clone()).await
                            })
                            .await;

                            match tool_result {
                                Ok(result) => {
                                    self.add_tool_result(&tool_id, result.clone(), true, 0);
                                    self.add_system_message(
                                        "Repair tool executed successfully".to_string(),
                                    );
                                }
                                Err(e) => {
                                    self.add_tool_result(
                                        &tool_id,
                                        serde_json::json!({"error": e.to_string()}),
                                        false,
                                        0,
                                    );
                                    self.add_system_message(format!("Repair tool failed: {}", e));
                                }
                            }
                        }
                    }
                    _ => {
                        self.add_system_message(
                            "Model returned unexpected repair decision".to_string(),
                        );
                    }
                }
            }
            Err(e) => {
                self.add_system_message(format!("Repair model call failed: {}", e));
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    fn add_user_message(&mut self, content: String) {
        self.session.messages.push(AgentMessage {
            id: Uuid::new_v4().to_string(),
            role: MessageRole::User,
            content,
            timestamp: chrono::Utc::now(),
            tool_calls: None,
            tool_results: None,
        });
    }

    fn add_assistant_message(&mut self, content: String) {
        self.session.messages.push(AgentMessage {
            id: Uuid::new_v4().to_string(),
            role: MessageRole::Assistant,
            content,
            timestamp: chrono::Utc::now(),
            tool_calls: None,
            tool_results: None,
        });
    }

    fn add_system_message(&mut self, content: String) {
        self.session.messages.push(AgentMessage {
            id: Uuid::new_v4().to_string(),
            role: MessageRole::System,
            content,
            timestamp: chrono::Utc::now(),
            tool_calls: None,
            tool_results: None,
        });
    }

    fn add_tool_result(
        &mut self,
        tool_name: &str,
        result: serde_json::Value,
        success: bool,
        duration_ms: u64,
    ) {
        let call_id = Uuid::new_v4().to_string();

        // Add tool call message
        self.session.messages.push(AgentMessage {
            id: Uuid::new_v4().to_string(),
            role: MessageRole::Assistant,
            content: String::new(),
            timestamp: chrono::Utc::now(),
            tool_calls: Some(vec![ToolCall {
                id: call_id.clone(),
                name: tool_name.to_string(),
                arguments: serde_json::json!({}),
            }]),
            tool_results: None,
        });

        // Add tool result message
        self.session.messages.push(AgentMessage {
            id: Uuid::new_v4().to_string(),
            role: MessageRole::Tool,
            content: serde_json::to_string_pretty(&result).unwrap_or_default(),
            timestamp: chrono::Utc::now(),
            tool_calls: None,
            tool_results: Some(vec![ToolResult {
                call_id,
                output: result,
                success,
                duration_ms,
            }]),
        });
    }

    /// Get a reference to the current session
    pub fn session(&self) -> &AgentSession {
        &self.session
    }

    /// Check if a tool call requires approval
    fn requires_approval(&self, tool_call: &ToolCallRequest) -> bool {
        // Check if the tool requires approval based on risk level
        let risk = match tool_call.tool_id.as_str() {
            id if id.starts_with("fs.read") => RiskLevel::Read,
            id if id.starts_with("fs.") => RiskLevel::Write,
            id if id.starts_with("shell.") => {
                // Check for destructive commands
                if let Some(command) = tool_call.arguments.get("command").and_then(|v| v.as_str()) {
                    if command == "rm" || command == "rmdir" || command == "del" {
                        return true;
                    }
                }
                RiskLevel::Execute
            }
            id if id.starts_with("git.") => RiskLevel::GitWrite,
            id if id.starts_with("search.") => RiskLevel::Read,
            _ => RiskLevel::Execute,
        };

        // For now, require approval for destructive and release operations
        // In a real implementation, this would check against a policy
        matches!(risk, RiskLevel::Destructive | RiskLevel::Release)
    }

    /// Get the next tool call from the model
    async fn get_next_tool_call(&mut self) -> Result<ToolCallRequest> {
        // Call model to get tool call
        let prompt = format!(
            "You are executing a plan. The current plan is: {:?}\n\n\
             Based on the plan and current state, select the next tool to execute.\n\
             Return a JSON object with action 'ToolCall' and the tool details.",
            self.session.context.plan
        );

        match self.call_model(&prompt).await {
            Ok(decision) => {
                match decision {
                    AgentDecision::ToolCall {
                        tool_id,
                        arguments,
                        reason,
                        expected_result,
                    } => Ok(ToolCallRequest {
                        tool_id,
                        arguments,
                        reason,
                        expected_result,
                    }),
                    _ => {
                        // Default to reading Cargo.toml
                        Ok(ToolCallRequest {
                            tool_id: "fs.read".to_string(),
                            arguments: serde_json::json!({"action": "read", "path": "Cargo.toml"}),
                            reason: "Default tool call".to_string(),
                            expected_result: None,
                        })
                    }
                }
            }
            Err(e) => {
                // Default to reading Cargo.toml
                Ok(ToolCallRequest {
                    tool_id: "fs.read".to_string(),
                    arguments: serde_json::json!({"action": "read", "path": "Cargo.toml"}),
                    reason: format!("Fallback due to model error: {}", e),
                    expected_result: None,
                })
            }
        }
    }

    /// Compute SHA-256 hash of the previous ledger entry for chain integrity.
    async fn compute_prev_hash(&self) -> Result<String> {
        use sha2::{Digest, Sha256};

        let session_id = Uuid::parse_str(&self.session.id)?;
        let entries = self.ledger.get_entries(session_id).await?;

        if let Some(last_entry) = entries.last() {
            // Hash the previous entry's chain position together with its
            // action. The caller-chosen `prev_hash` alone proves nothing (a
            // caller can append any string); binding `action_id` into the
            // digest makes this a hash chain over (prev_hash, action) pairs,
            // so a re-ordered, re-targeted, or injected entry no longer
            // hashes consistently with its neighbour's recorded value.
            let content = format!("{}:{}", last_entry.prev_hash, last_entry.action_id);
            let mut hasher = Sha256::new();
            hasher.update(content.as_bytes());
            let hash = hasher.finalize();
            Ok(format!("{:x}", hash))
        } else {
            // Genesis entry: the empty string. The first entry's recorded
            // prev_hash is the literal genesis value an auditor expects.
            Ok(String::new())
        }
    }

    /// Execute a tool call
    async fn execute_tool_call(&mut self, tool_call: ToolCallRequest) -> Result<()> {
        self.add_system_message(format!(
            "Executing tool: {} - {}",
            tool_call.tool_id, tool_call.reason
        ));

        // Compute hash of previous entry for chain integrity
        let prev_hash = self.compute_prev_hash().await?;

        // Log to ledger: Started
        let entry_id = Uuid::new_v4();
        let entry = LedgerEntry {
            id: entry_id,
            session_id: Uuid::parse_str(&self.session.id).unwrap(),
            action_id: tool_call.tool_id.clone(),
            arguments: tool_call.arguments.clone(),
            state: ExecutionState::Started,
            prev_hash,
            timestamp: chrono::Utc::now(),
            payload: None,
            error: None,
        };
        self.ledger.append(entry).await?;

        // Execute the tool. The call is wrapped in the session's actor
        // binding: `real_tools::dispatch` resolves the actor from the
        // task-local scope (registry tools carry `actor: None` by design), so
        // without this every agent-driven invocation would be persisted as
        // unidentified.
        let tool = self.tool_registry.get(&tool_call.tool_id).await;
        if let Some(tool) = tool {
            let tool_result = zylcode_mcp::with_actor(self.actor_id(), async {
                tool.call(tool_call.arguments.clone()).await
            })
            .await;

            match tool_result {
                Ok(result) => {
                    // Log to ledger: Executed
                    self.ledger
                        .update_state(
                            entry_id,
                            ExecutionState::Executed,
                            Some(result.clone()),
                            None,
                        )
                        .await?;

                    // Add tool result to session
                    self.add_tool_result(&tool_call.tool_id, result.clone(), true, 0);

                    // Update session context
                    if tool_call.tool_id.starts_with("fs.") {
                        if let Some(path) = tool_call.arguments.get("path").and_then(|v| v.as_str())
                        {
                            self.session.context.files_read.push(PathBuf::from(path));
                        }
                    }

                    // Add observation
                    let observation = ToolObservation {
                        tool_id: tool_call.tool_id.clone(),
                        success: true,
                        stdout: result
                            .get("stdout")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        stderr: result
                            .get("stderr")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        exit_code: result
                            .get("exit_code")
                            .and_then(|v| v.as_i64())
                            .map(|i| i as i32),
                        changed_files: Vec::new(),
                        diagnostics: Vec::new(),
                        evidence_id: Uuid::new_v4().to_string(),
                    };

                    self.add_system_message(format!("Tool execution result: {:?}", observation));

                    // Log to ledger: Recorded
                    self.ledger
                        .update_state(
                            entry_id,
                            ExecutionState::Recorded,
                            Some(serde_json::to_value(&observation)?),
                            None,
                        )
                        .await?;
                }
                Err(e) => {
                    // Log to ledger: Failed
                    self.ledger
                        .update_state(entry_id, ExecutionState::Failed, None, Some(e.to_string()))
                        .await?;

                    self.add_tool_result(
                        &tool_call.tool_id,
                        serde_json::json!({"error": e.to_string()}),
                        false,
                        0,
                    );
                    self.add_system_message(format!("Tool execution failed: {}", e));
                }
            }
        } else {
            self.add_system_message(format!("Tool not found: {}", tool_call.tool_id));
            // Tool not found — mark as failed
            self.ledger
                .update_state(
                    entry_id,
                    ExecutionState::Failed,
                    None,
                    Some(format!("Tool not found: {}", tool_call.tool_id)),
                )
                .await?;
            return Err(anyhow::anyhow!("Tool not found: {}", tool_call.tool_id));
        }

        Ok(())
    }

    /// Call the model with a prompt and parse the response
    async fn call_model(&self, prompt: &str) -> Result<AgentDecision> {
        // Use the model client to call the model
        let system_prompt = "You are an expert software engineering agent. You must respond with valid JSON matching the AgentDecision protocol.";

        let response = self.model_client.call(prompt, system_prompt).await?;

        // Parse the response as JSON
        // Try to extract JSON from the response
        let json_str = if response.contains("```json") {
            response
                .split("```json")
                .nth(1)
                .unwrap_or(&response)
                .split("```")
                .next()
                .unwrap_or(&response)
        } else if response.contains("```") {
            response
                .split("```")
                .nth(1)
                .unwrap_or(&response)
                .split("```")
                .next()
                .unwrap_or(&response)
        } else {
            &response
        };

        // Try to parse as AgentDecision
        match serde_json::from_str::<AgentDecision>(json_str.trim()) {
            Ok(decision) => Ok(decision),
            Err(e) => {
                // If parsing fails, try to create a default decision
                // This handles cases where the model doesn't return perfect JSON
                tracing::warn!(
                    "Failed to parse model response as AgentDecision: {}. Response: {}",
                    e,
                    response
                );

                // Try to extract action from response
                if response.contains("\"action\": \"Plan\"")
                    || response.contains("\"action\":\"Plan\"")
                {
                    // Try to parse as plan
                    Ok(AgentDecision::Plan { steps: vec![] })
                } else if response.contains("\"action\": \"ToolCall\"")
                    || response.contains("\"action\":\"ToolCall\"")
                {
                    // Try to parse as tool call
                    Ok(AgentDecision::ToolCall {
                        tool_id: "fs.read".to_string(),
                        arguments: serde_json::json!({"action": "read", "path": "Cargo.toml"}),
                        reason: "Fallback tool call".to_string(),
                        expected_result: None,
                    })
                } else if response.contains("\"action\": \"Complete\"")
                    || response.contains("\"action\":\"Complete\"")
                {
                    Ok(AgentDecision::Complete {
                        summary: "Task completed".to_string(),
                        evidence: vec![],
                        remaining_limitations: vec![],
                    })
                } else if response.contains("<zylcode-response>") || response.contains("artifact") {
                    // Old XML format — treat as completion with evidence
                    Ok(AgentDecision::Complete {
                        summary: "Task completed (XML response)".to_string(),
                        evidence: vec!["XML response received".to_string()],
                        remaining_limitations: vec!["Response in old XML format".to_string()],
                    })
                } else {
                    // Default to thinking
                    Ok(AgentDecision::Think {
                        thought: response.clone(),
                    })
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory_ledger::MemoryLedgerStore;

    #[tokio::test]
    async fn test_agent_loop_creation() {
        let registry = Arc::new(ToolRegistry::new());
        let model_client = Arc::new(TestModelClient::new(vec![]));
        let ledger = Arc::new(MemoryLedgerStore::new());
        let agent = AgentLoop::new(
            "Fix the bug in main.rs",
            PathBuf::from("."),
            None,
            registry,
            model_client,
            ledger,
        );

        assert_eq!(agent.session.state, AgentState::Created);
        assert_eq!(agent.session.messages.len(), 1);
        assert_eq!(agent.session.messages[0].role, MessageRole::User);
    }

    #[tokio::test]
    async fn test_agent_loop_step() {
        let registry = Arc::new(ToolRegistry::new());
        let model_client = Arc::new(TestModelClient::new(vec![]));
        let ledger = Arc::new(MemoryLedgerStore::new());
        let mut agent = AgentLoop::new(
            "Fix the bug in main.rs",
            PathBuf::from("."),
            None,
            registry,
            model_client,
            ledger,
        );

        // First step should transition to Analyzing
        agent.step().await.unwrap();
        assert_eq!(agent.session.state, AgentState::Analyzing);
    }

    #[tokio::test]
    async fn test_approval_enforcement() {
        let registry = Arc::new(ToolRegistry::new());

        // Register tools
        let shell_config = McpToolConfig {
            id: "shell.execute".to_string(),
            command: "execute".to_string(),
            transport: McpTransport::Stdio,
            env: Default::default(),
            enabled: true,
            description: Some("Execute shell command".to_string()),
        };
        registry
            .register(Arc::new(DynamicTool::new(shell_config)))
            .await;

        let model_client = Arc::new(TestModelClient::new(vec![
            // Response for planning
            r#"{"action": "Plan", "payload": {"steps": []}}"#.to_string(),
            // Response for tool call (destructive action)
            r#"{"action": "ToolCall", "payload": {"tool_id": "shell.execute", "arguments": {"command": "rm", "args": ["-rf", "/"]}, "reason": "Destructive action", "expected_result": "Error"}}"#.to_string(),
        ]));

        let ledger = Arc::new(MemoryLedgerStore::new());
        let mut agent = AgentLoop::new(
            "Execute destructive command",
            PathBuf::from("."),
            None,
            registry,
            model_client,
            ledger,
        );

        // Run the agent loop
        let final_state = agent.run().await.unwrap();

        // Should be in AwaitingApproval state because destructive action requires approval
        assert_eq!(final_state, AgentState::AwaitingApproval);
    }

    #[tokio::test]
    async fn test_completion_verification() {
        let registry = Arc::new(ToolRegistry::new());

        // Register fs.read tool
        let fs_config = McpToolConfig {
            id: "fs.read".to_string(),
            command: "read".to_string(),
            transport: McpTransport::Stdio,
            env: Default::default(),
            enabled: true,
            description: Some("Read file".to_string()),
        };
        registry
            .register(Arc::new(DynamicTool::new(fs_config)))
            .await;

        let model_client = Arc::new(TestModelClient::new(vec![
            // Response for planning
            r#"{"action": "Plan", "payload": {"steps": []}}"#.to_string(),
            // Response for tool call
            r#"{"action": "ToolCall", "payload": {"tool_id": "fs.read", "arguments": {"action": "read", "path": "Cargo.toml"}, "reason": "Read file", "expected_result": "File content"}}"#.to_string(),
            // Response for verification (complete without evidence)
            r#"{"action": "Complete", "payload": {"summary": "Task completed", "evidence": [], "remaining_limitations": []}}"#.to_string(),
        ]));

        let ledger = Arc::new(MemoryLedgerStore::new());
        let mut agent = AgentLoop::new(
            "Read Cargo.toml",
            PathBuf::from("."),
            None,
            registry,
            model_client,
            ledger,
        );

        // Run the agent loop
        let final_state = agent.run().await.unwrap();

        // Should complete because the tool was executed (providing evidence)
        assert_eq!(final_state, AgentState::Completed);
    }

    #[tokio::test]
    async fn test_step_limit() {
        let registry = Arc::new(ToolRegistry::new());

        // Register fs.read tool
        let fs_config = McpToolConfig {
            id: "fs.read".to_string(),
            command: "read".to_string(),
            transport: McpTransport::Stdio,
            env: Default::default(),
            enabled: true,
            description: Some("Read file".to_string()),
        };
        registry
            .register(Arc::new(DynamicTool::new(fs_config)))
            .await;

        let model_client = Arc::new(TestModelClient::new(vec![
            // Keep returning tool calls to exceed step limit
            r#"{"action": "ToolCall", "payload": {"tool_id": "fs.read", "arguments": {"action": "read", "path": "Cargo.toml"}, "reason": "Read file", "expected_result": "File content"}}"#.to_string(),
            r#"{"action": "ToolCall", "payload": {"tool_id": "fs.read", "arguments": {"action": "read", "path": "Cargo.toml"}, "reason": "Read file", "expected_result": "File content"}}"#.to_string(),
            r#"{"action": "ToolCall", "payload": {"tool_id": "fs.read", "arguments": {"action": "read", "path": "Cargo.toml"}, "reason": "Read file", "expected_result": "File content"}}"#.to_string(),
            r#"{"action": "ToolCall", "payload": {"tool_id": "fs.read", "arguments": {"action": "read", "path": "Cargo.toml"}, "reason": "Read file", "expected_result": "File content"}}"#.to_string(),
            r#"{"action": "ToolCall", "payload": {"tool_id": "fs.read", "arguments": {"action": "read", "path": "Cargo.toml"}, "reason": "Read file", "expected_result": "File content"}}"#.to_string(),
        ]));

        let config = AgentConfig {
            max_iterations: 3, // Set low limit
            ..Default::default()
        };

        let ledger = Arc::new(MemoryLedgerStore::new());
        let mut agent = AgentLoop::new(
            "Read Cargo.toml",
            PathBuf::from("."),
            Some(config),
            registry,
            model_client,
            ledger,
        );

        // Run the agent loop
        let final_state = agent.run().await.unwrap();

        // Should fail due to step limit
        assert_eq!(final_state, AgentState::Failed);
    }

    /// The ledger hash chain must be intact: every entry's recorded
    /// prev_hash is exactly the hash of its predecessor's (prev_hash,
    /// action_id) pair, and the first entry chains from the documented
    /// genesis value. `compute_prev_hash` is private; the loop's public
    /// surface (`run`) is what produces the entries, so the chain is
    /// verified through it — the same way an auditor would.
    #[tokio::test]
    async fn ledger_hash_chain_is_intact_after_a_run() {
        use sha2::{Digest, Sha256};

        let registry = Arc::new(ToolRegistry::new());
        let fs_cfg = McpToolConfig {
            id: "fs.read".into(),
            command: "read".into(),
            transport: McpTransport::Stdio,
            env: Default::default(),
            enabled: true,
            description: None,
        };
        registry.register(Arc::new(DynamicTool::new(fs_cfg))).await;

        let model_client = Arc::new(TestModelClient::new(vec![
            // Plan, then one tool call, then complete.
            r#"{"action": "Plan", "payload": {"steps": [{"id": "1", "description": "Read Cargo.toml", "expected_files": ["Cargo.toml"], "expected_tools": ["fs.read"], "risk": "Read", "verification": "File read successfully"}]}}"#.to_string(),
            r#"{"action": "ToolCall", "payload": {"tool_id": "fs.read", "arguments": {"action": "read", "path": "Cargo.toml"}, "reason": "Need to read Cargo.toml", "expected_result": "File content"}}"#.to_string(),
            r#"{"action": "Complete", "payload": {"summary": "done", "evidence": ["read"], "remaining_limitations": []}}"#.to_string(),
        ]));

        let ledger = Arc::new(MemoryLedgerStore::new());
        let mut agent = AgentLoop::new(
            "Read Cargo.toml",
            PathBuf::from("."),
            None,
            registry,
            model_client,
            Arc::clone(&ledger) as Arc<dyn LedgerStore>,
        );
        agent.run().await.unwrap();

        let session_id = Uuid::parse_str(&agent.session().id).unwrap();
        let entries = ledger.get_entries(session_id).await.unwrap();
        assert!(
            !entries.is_empty(),
            "the run must have produced ledger entries"
        );

        let mut prev_hash = String::new(); // documented genesis value
        for (i, entry) in entries.iter().enumerate() {
            assert_eq!(
                entry.prev_hash, prev_hash,
                "entry {i} ({}) breaks the chain: recorded prev_hash {:?}, expected {:?}",
                entry.action_id, entry.prev_hash, prev_hash
            );
            let content = format!("{}:{}", entry.prev_hash, entry.action_id);
            let mut hasher = Sha256::new();
            hasher.update(content.as_bytes());
            prev_hash = format!("{:x}", hasher.finalize());
        }
        assert_ne!(
            entries[0].prev_hash, "TODO",
            "the literal 'TODO' chain break must never reappear"
        );
    }

    /// Every tool invocation the loop makes must be attributed. The loop
    /// binds its actor around each dispatch, so the task-local resolution in
    /// `real_tools::dispatch` records it — verified here through the real
    /// JSONL sink the registered tool is constructed with.
    #[tokio::test]
    async fn agent_run_attributes_invocations_to_the_actor() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("evidence.jsonl");

        let registry = Arc::new(ToolRegistry::new());
        let fs_cfg = McpToolConfig {
            id: "fs.read".into(),
            command: "read".into(),
            transport: McpTransport::Stdio,
            env: Default::default(),
            enabled: true,
            description: None,
        };
        registry
            .register(Arc::new(DynamicTool::with_runtime(
                fs_cfg,
                zylcode_mcp::ToolRuntime::restrictive().with_evidence_at(&path),
            )))
            .await;

        let model_client = Arc::new(TestModelClient::new(vec![
            r#"{"action": "Plan", "payload": {"steps": [{"id": "1", "description": "Read Cargo.toml", "expected_files": ["Cargo.toml"], "expected_tools": ["fs.read"], "risk": "Read", "verification": "File read successfully"}]}}"#.to_string(),
            r#"{"action": "ToolCall", "payload": {"tool_id": "fs.read", "arguments": {"action": "read", "path": "Cargo.toml"}, "reason": "Need to read Cargo.toml", "expected_result": "File content"}}"#.to_string(),
            r#"{"action": "Complete", "payload": {"summary": "done", "evidence": ["read"], "remaining_limitations": []}}"#.to_string(),
        ]));

        let ledger = Arc::new(MemoryLedgerStore::new());
        let mut agent = AgentLoop::new(
            "Read Cargo.toml",
            PathBuf::from("."),
            None,
            registry,
            model_client,
            Arc::clone(&ledger) as Arc<dyn LedgerStore>,
        )
        .with_actor("agent:test-loop");
        agent.run().await.unwrap();

        let sink = zylcode_mcp::JsonlEvidenceSink::new(&path);
        let records = sink.read_all().unwrap();
        assert!(!records.is_empty(), "tool executions must reach the sink");
        for record in &records {
            assert_eq!(
                record.actor.as_deref(),
                Some("agent:test-loop"),
                "the loop's actor binding must reach every persisted record"
            );
        }
    }
}
