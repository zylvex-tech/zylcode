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

// ---------------------------------------------------------------------------
// Provider-bound request (G3)
//
// The model never sees `session.messages`: every request is assembled
// locally from the task, the plan and this loop's gathered intelligence, and
// then handed to `ModelClient::call` as two strings. Repository-intelligence
// citations therefore had to be carried on the *session* — not merely stored
// as a message — or they stop at the boundary exactly where the previous
// commissioning found them stopping.
// ---------------------------------------------------------------------------

/// System prompt sent with every provider-bound request.
const BASE_SYSTEM_PROMPT: &str = "You are an expert software engineering agent. You must respond with valid JSON matching the AgentDecision protocol.";

/// Upper bound on citation lines carried by one request.
///
/// `ContextBuilder` gathers with the same limit, so this is the belt to the
/// gatherer's braces: even a future caller that gathers more cannot push an
/// unbounded block through this boundary.
pub const REPO_CONTEXT_MAX_CITATIONS: usize = 12;

/// Byte budget for the rendered citation block, applied to **whole lines**.
///
/// A citation is never cut in half — `file:line` integrity is the whole point
/// of the block — so overflowing lines are dropped and the drop is stated
/// rather than silent.
pub const REPO_CONTEXT_MAX_CHARS: usize = 4_096;

/// The two strings handed to the provider. This *is* the dispatch boundary:
/// [`AgentLoop::call_model`] passes them to [`ModelClient::call`] unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderRequest {
    pub prompt: String,
    pub system: String,
}

/// Render gathered citations as the block prepended to every request.
///
/// Returns `None` when there is nothing to say — never an empty block, which
/// would read to a model as "no relationships exist". Truncation is
/// deterministic (first N whole lines within the byte budget) and always
/// announced in the output, so a reviewer can see exactly what was dropped.
pub fn repository_context_block(citations: &[String]) -> Option<String> {
    if citations.is_empty() {
        return None;
    }

    let mut kept: Vec<&str> = Vec::new();
    let mut used = 0usize;
    let mut dropped = 0usize;

    for (index, line) in citations.iter().enumerate() {
        if index >= REPO_CONTEXT_MAX_CITATIONS {
            dropped += citations.len() - index;
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // +1 for the newline that will join it.
        if used + line.len() + 1 > REPO_CONTEXT_MAX_CHARS {
            if kept.is_empty() {
                // Never drop *everything*: one whole citation is always
                // cheaper than an unbounded claim.
                kept.push(line);
                dropped += citations.len() - index - 1;
                break;
            }
            dropped += citations.len() - index;
            break;
        }
        used += line.len() + 1;
        kept.push(line);
    }

    if kept.is_empty() {
        return None;
    }

    // The header has to describe what is actually below it: calling a single
    // "no evidence exists" marker "Deterministic evidence resolved from the
    // parsed repository index" would be the same class of overstatement the
    // wave forbids everywhere else.
    let carries_evidence = kept.iter().any(|line| line.starts_with("DETERMINISTIC"));
    let mut block = String::from(if carries_evidence {
        "REPOSITORY INTELLIGENCE CONTEXT\n\n\
         Deterministic evidence resolved from the parsed repository index:\n"
    } else {
        "REPOSITORY INTELLIGENCE CONTEXT\n\n\
         No graph evidence is available for this task:\n"
    });
    for line in &kept {
        block.push_str(line);
        block.push('\n');
    }
    if dropped > 0 {
        block.push_str(&format!(
            "… {dropped} further citation line{} omitted by the repository-context \
             budget ({} lines / {} bytes) — their absence is a budget limit, not a \
             finding.\n",
            if dropped == 1 { "" } else { "s" },
            REPO_CONTEXT_MAX_CITATIONS,
            REPO_CONTEXT_MAX_CHARS,
        ));
    }
    block.push_str(
        "\nInstructions:\n\
         - Treat the lines above as repository evidence for this task.\n\
         - Cite the `file:line` locations above when you make a repository-specific claim.\n\
         - Do not claim a file, symbol or relationship that is not covered above.\n\
         - If the lines above say no graph evidence exists, do not invent one.\n\
         - Anything not covered above is your own interpretation — label it as such, and \
         never present an inference as a graph fact.",
    );
    Some(block)
}

/// Prepend the repository block to `prompt`, when there is one.
fn with_repository_context(prompt: &str, citations: &[String]) -> String {
    match repository_context_block(citations) {
        Some(block) => format!("{block}\n\n{prompt}"),
        None => prompt.to_string(),
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
    /// Deterministic repository-graph evidence gathered for this session's
    /// goal, in the engine's own words (`agent_citation_block` via
    /// `ContextBuilder::build`).
    ///
    /// Held here and not only inside `session.messages`, because the
    /// messages are never serialized into a provider request — the loop
    /// builds each request from the task, the plan and this field (G3).
    /// `#[serde(default)]` keeps checkpoints written before this field
    /// existed readable: they resume with *no* citations rather than with a
    /// fabricated empty block.
    #[serde(default)]
    pub navigation_citations: Vec<String>,
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
                navigation_citations: Vec::new(),
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
                // Ask the model what to do next. FIX D2: its answer is honoured
                // as given and is never rewritten into an invented tool call.
                //  - ToolCall: execute it (path below, unchanged).
                //  - Complete / Verify: the model says the work is done or asks
                //    for verification. Claiming success is still verification's
                //    call, so the run moves to Verifying rather than trusting
                //    the declaration.
                //  - Fail: the model declares failure; record it and stop.
                //  - anything else, including a provider error: the run ends
                //    honestly as Failed with the reason recorded in the
                //    session - the same treatment this loop already gives the
                //    other cannot-continue conditions (timeout, step limit,
                //    tool not found). No branch below can execute a tool the
                //    model did not request or report a success nobody verified.
                let tool_call = match self.get_next_execution_decision().await {
                    Ok(AgentDecision::ToolCall {
                        tool_id,
                        arguments,
                        reason,
                        expected_result,
                    }) => ToolCallRequest {
                        tool_id,
                        arguments,
                        reason,
                        expected_result,
                    },
                    Ok(AgentDecision::Complete {
                        summary,
                        evidence,
                        remaining_limitations: _,
                    }) => {
                        self.add_system_message(format!(
                            "Model declared completion during execution: {summary} \
                             (evidence: {evidence:?})"
                        ));
                        self.transition_to(AgentState::Verifying).await?;
                        return Ok(());
                    }
                    Ok(AgentDecision::Verify { checks }) => {
                        self.add_system_message(format!(
                            "Model requested verification during execution ({} checks)",
                            checks.len()
                        ));
                        self.transition_to(AgentState::Verifying).await?;
                        return Ok(());
                    }
                    Ok(AgentDecision::Fail {
                        reason,
                        error,
                        repair_suggestions: _,
                    }) => {
                        self.add_system_message(format!(
                            "Model declared failure during execution: {reason} - {error}"
                        ));
                        self.transition_to(AgentState::Failed).await?;
                        return Ok(());
                    }
                    Ok(other) => {
                        self.add_system_message(format!(
                            "Tool selection failed: model returned {other:?} instead of a \
                             ToolCall; refusing to fabricate a tool call"
                        ));
                        self.transition_to(AgentState::Failed).await?;
                        return Ok(());
                    }
                    Err(e) => {
                        self.add_system_message(format!("Tool selection failed: {e}"));
                        self.transition_to(AgentState::Failed).await?;
                        return Ok(());
                    }
                };

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

        // Keep the citations on the session, not only inside `messages`.
        // `messages` never reaches a provider request (G3): this field is
        // what `provider_request` reads when it builds the strings the model
        // actually receives. Replacing rather than appending keeps a re-gather
        // from stacking stale evidence for an older goal.
        self.session.context.navigation_citations = context.navigation_citations.clone();

        // Add context to session messages as system message
        let mut context_str = format!(
            "Workspace: {}\nFiles: {:?}\nRelevant: {:?}\nGit: {:?}",
            context.workspace_root,
            context.file_tree.len(),
            context.relevant_files,
            context.git_status
        );

        // Repository-graph evidence, kept separate from the ranked file list
        // because the two carry different weight: the list is retrieval, these
        // lines are resolved facts the model is expected to quote.
        if !context.navigation_citations.is_empty() {
            context_str.push_str(&format!(
                "\n\nRepository graph evidence (deterministic, resolved from the parsed \
repository index):\n{}\n\
Cite these locations when you make a structural claim about this code. Anything not \
covered by a line above is your own interpretation — label it as such, and never \
present an inference as a graph fact.",
                context.navigation_citations.join("\n")
            ));
        }

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
                            let tool_result = self.call_tool(tool, arguments.clone()).await;

                            match tool_result {
                                Ok(result) => {
                                    let ok = Self::tool_reported_success(&result);

                                    // Add tool result to session
                                    self.add_tool_result(&tool_id, result.clone(), ok, 0);

                                    // Update session context. Only a read that
                                    // actually succeeded counts as read.
                                    if ok && tool_id.starts_with("fs.") {
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
                                        success: ok,
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
                            let tool_result = self.call_tool(tool, arguments.clone()).await;

                            match tool_result {
                                Ok(result) => {
                                    let ok = Self::tool_reported_success(&result);
                                    self.add_tool_result(&tool_id, result.clone(), ok, 0);
                                    self.add_system_message(if ok {
                                        "Repair tool executed successfully".to_string()
                                    } else {
                                        format!("Repair tool reported a failure: {result}")
                                    });
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

    /// Did the tool report that it succeeded?
    ///
    /// The payload carries an explicit `success` out of
    /// `zylcode_mcp::real_tools::dispatch`, through `DynamicTool::call`. Reading
    /// it is the difference between recording what happened and recording that
    /// *something* was attempted: these call sites previously passed `true`
    /// unconditionally, so a refused or failed `fs.read` was stored in the
    /// session as a **successful** tool result — a fabricated result, which
    /// `docs/governance/TOOL_CATALOGUE_TRUTH_TABLE.md` §1.1 forbids.
    ///
    /// With the field absent the honest default is `false`: a failure reported
    /// as a failure costs a retry, whereas success reported for work nobody
    /// confirmed is a lie.
    fn tool_reported_success(result: &serde_json::Value) -> bool {
        result
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    /// Run a registered tool under this loop's actor **and** workspace bindings.
    ///
    /// The workspace binding is what makes `fs.*` containment apply to *this
    /// loop's* working directory rather than to whatever directory the process
    /// happened to be started in — see `zylcode_mcp::workspace`. Without it,
    /// `zylcode --workspace D:\other build` launched from anywhere else would
    /// contain `fs.read` against the caller's shell directory.
    async fn call_tool(
        &self,
        tool: Arc<dyn zylcode_mcp::Tool>,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let root = self.session.context.working_directory.clone();
        zylcode_mcp::with_actor(self.actor_id(), async move {
            zylcode_mcp::with_workspace_root(root, tool.call(params)).await
        })
        .await
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

    /// Ask the model for its next execution decision.
    ///
    /// FIX D2: the decision is returned exactly as the model gave it, or the
    /// provider error is propagated. This used to fall back to an invented
    /// `fs.read Cargo.toml` call whenever the answer was not a ToolCall,
    /// which wrote tool intent the model never expressed into the ledger and
    /// then executed it as real work. Deciding what a non-ToolCall answer
    /// means is the caller's job (see the `AgentState::Executing` arm of
    /// `step`).
    async fn get_next_execution_decision(&mut self) -> Result<AgentDecision> {
        let prompt = format!(
            "You are executing a plan. The current plan is: {:?}\n\n\
             Based on the plan and current state, select the next tool to execute.\n\
             Return a JSON object with action 'ToolCall' and the tool details.",
            self.session.context.plan
        );

        match self.call_model(&prompt).await {
            Ok(decision) => Ok(decision),
            // FIX D2: a provider error is an error, not an excuse to invent a
            // tool call. Propagate it unchanged.
            Err(e) => Err(e.context("model call failed while requesting the next tool call")),
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
            let tool_result = self.call_tool(tool, tool_call.arguments.clone()).await;

            match tool_result {
                Ok(result) => {
                    let ok = Self::tool_reported_success(&result);

                    // Log to ledger: Executed.
                    //
                    // `Executed` records that the executor *ran* — which it did,
                    // whether or not the work inside it succeeded. The verdict
                    // travels in the payload (`success`) and in the `Recorded`
                    // observation below, so the chain stays truthful without
                    // redefining what `Failed` means (it is the tool-error
                    // path, not a completed-with-a-false-verdict path).
                    self.ledger
                        .update_state(
                            entry_id,
                            ExecutionState::Executed,
                            Some(result.clone()),
                            None,
                        )
                        .await?;

                    // Add tool result to session
                    self.add_tool_result(&tool_call.tool_id, result.clone(), ok, 0);

                    // Update session context. A read that was refused — outside
                    // the workspace, over the byte bound, or simply missing —
                    // was not read, so it must not be recorded as if it were.
                    if ok && tool_call.tool_id.starts_with("fs.") {
                        if let Some(path) = tool_call.arguments.get("path").and_then(|v| v.as_str())
                        {
                            self.session.context.files_read.push(PathBuf::from(path));
                        }
                    }

                    // Add observation
                    let observation = ToolObservation {
                        tool_id: tool_call.tool_id.clone(),
                        success: ok,
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
    /// Build the exact request handed to the provider for `prompt`.
    ///
    /// This is the dispatch boundary: [`Self::call_model`] passes these two
    /// strings to [`ModelClient::call`] untouched, so repository intelligence
    /// only reaches the model if it is present here. Everything the loop
    /// stores anywhere else — messages, checkpoints, the index on disk — is
    /// invisible to the model.
    fn provider_request(&self, prompt: &str) -> ProviderRequest {
        ProviderRequest {
            prompt: with_repository_context(prompt, &self.session.context.navigation_citations),
            system: BASE_SYSTEM_PROMPT.to_string(),
        }
    }

    async fn call_model(&self, prompt: &str) -> Result<AgentDecision> {
        // The single place a request is assembled for the provider.
        let request = self.provider_request(prompt);

        let response = self
            .model_client
            .call(&request.prompt, &request.system)
            .await?;

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
            // FIX D2: unparseable model output is never rewritten into a
            // Plan/ToolCall/Complete that the model did not write. It becomes
            // an honest Think carrying the raw text, so the caller sees the
            // response as reasoning and can decide what to do with it. The
            // parse error itself is logged (without the response body, which
            // the Think already records in the session).
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "model response was not valid AgentDecision JSON; \
                     surfacing it as an honest Think decision"
                );
                Ok(AgentDecision::Think {
                    thought: response.clone(),
                })
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

    // -----------------------------------------------------------------------
    // G3 — repository intelligence at the provider boundary
    //
    // The failure these replace: citations were computed, written into
    // `session.messages`, and then never serialized into any request the model
    // received — every model-facing prompt was assembled from the task and
    // plan alone. Every test below therefore observes the *boundary*: the
    // strings handed to `ModelClient::call`. A test one layer earlier (the
    // context builder, the session transcript) would have passed the whole
    // time while the model stayed blind.
    // -----------------------------------------------------------------------

    /// A fixture the navigation index can actually resolve: `helper` is
    /// declared in `src/util.rs` and called from `src/lib.rs`.
    fn intel_fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("temp dir");
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).expect("src dir");
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .expect("cargo.toml");
        std::fs::write(
            src.join("lib.rs"),
            "pub mod util;\n\npub fn entry() -> u32 {\n    util::helper()\n}\n",
        )
        .expect("lib.rs");
        std::fs::write(src.join("util.rs"), "pub fn helper() -> u32 {\n    1\n}\n")
            .expect("util.rs");
        dir
    }

    /// Records the exact strings handed to the provider — this *is* the
    /// dispatch boundary under test.
    struct RecordingModelClient {
        responses: Vec<String>,
        index: std::sync::atomic::AtomicUsize,
        seen: std::sync::Mutex<Vec<ProviderRequest>>,
    }

    impl RecordingModelClient {
        fn new(responses: Vec<String>) -> Arc<Self> {
            Arc::new(Self {
                responses,
                index: std::sync::atomic::AtomicUsize::new(0),
                seen: std::sync::Mutex::new(Vec::new()),
            })
        }

        fn seen(&self) -> Vec<ProviderRequest> {
            self.seen.lock().expect("recorder poisoned").clone()
        }
    }

    #[async_trait::async_trait]
    impl ModelClient for RecordingModelClient {
        async fn call(&self, prompt: &str, system: &str) -> Result<String> {
            self.seen
                .lock()
                .expect("recorder poisoned")
                .push(ProviderRequest {
                    prompt: prompt.to_string(),
                    system: system.to_string(),
                });
            let idx = self.index.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(self.responses.get(idx).cloned().unwrap_or_else(|| {
                r#"{"action": "Complete", "payload": {"summary": "Task completed", "evidence": [], "remaining_limitations": []}}"#
                    .to_string()
            }))
        }
    }

    fn loop_over(dir: &std::path::Path, goal: &str) -> (AgentLoop, Arc<RecordingModelClient>) {
        let client = RecordingModelClient::new(Vec::new());
        let agent = AgentLoop::new(
            goal,
            dir.to_path_buf(),
            None,
            Arc::new(ToolRegistry::new()),
            Arc::clone(&client) as Arc<dyn ModelClient>,
            Arc::new(MemoryLedgerStore::new()),
        );
        (agent, client)
    }

    #[tokio::test]
    async fn gathered_citations_are_deterministic_and_stored_on_the_session() {
        let dir = intel_fixture();
        let (mut agent, _client) = loop_over(dir.path(), "find where helper is used");

        agent.gather_context().await.expect("context gathers");

        let citations = &agent.session.context.navigation_citations;
        assert!(!citations.is_empty(), "citations must be produced");
        assert!(
            citations
                .iter()
                .all(|line| line.starts_with("DETERMINISTIC")),
            "only deterministic evidence is offered to the model: {citations:?}"
        );
        assert!(
            citations.iter().any(|line| line.contains(".rs:")),
            "expected a file:line location, got {citations:?}"
        );
        // H, part one: the evidence still reaches the session transcript too.
        assert!(
            agent
                .session
                .messages
                .iter()
                .any(|m| m.content.contains("Repository graph evidence")),
            "the session transcript must still carry the evidence block"
        );
    }

    #[tokio::test]
    async fn the_provider_bound_request_contains_the_deterministic_citations() {
        let dir = intel_fixture();
        let (mut agent, client) = loop_over(dir.path(), "find where helper is used");
        agent.gather_context().await.expect("context gathers");

        let request = agent.provider_request("Answer the task.");
        assert!(
            request
                .prompt
                .starts_with("REPOSITORY INTELLIGENCE CONTEXT"),
            "context must precede the task: {}",
            request.prompt
        );
        assert!(
            request.prompt.ends_with("Answer the task."),
            "the task prompt must remain intact: {}",
            request.prompt
        );
        assert!(request.prompt.contains(".rs:"), "{}", request.prompt);
        assert_eq!(request.system, BASE_SYSTEM_PROMPT);

        // Through the real dispatch: `call_model` hands these two strings to
        // `ModelClient::call` with nothing inserted or dropped in between.
        agent
            .call_model("Answer the task.")
            .await
            .expect("model answers");

        let seen = client.seen();
        assert_eq!(seen.len(), 1);
        assert_eq!(
            seen[0].prompt, request.prompt,
            "what was built is what was sent"
        );
        assert_eq!(seen[0].system, request.system);
        assert!(
            seen[0].prompt.contains("DETERMINISTIC ·"),
            "{}",
            seen[0].prompt
        );
        assert!(seen[0].prompt.contains(".rs:"), "{}", seen[0].prompt);
    }

    #[tokio::test]
    async fn every_request_of_a_real_run_carries_the_repository_context() {
        let dir = intel_fixture();
        let client = RecordingModelClient::new(vec![
            r#"{"action": "Plan", "payload": {"steps": [{"id": "1", "description": "Read the source", "expected_files": ["src/lib.rs"], "expected_tools": ["fs.read"], "risk": "Read", "verification": "file read"}]}}"#.to_string(),
            r#"{"action": "ToolCall", "payload": {"tool_id": "fs.read", "arguments": {"action": "read", "path": "Cargo.toml"}, "reason": "inspect the manifest", "expected_result": "file content"}}"#.to_string(),
            r#"{"action": "Complete", "payload": {"summary": "done", "evidence": ["read"], "remaining_limitations": []}}"#.to_string(),
        ]);

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

        let mut agent = AgentLoop::new(
            "find where helper is used",
            dir.path().to_path_buf(),
            None,
            registry,
            Arc::clone(&client) as Arc<dyn ModelClient>,
            Arc::new(MemoryLedgerStore::new()),
        );
        agent.run().await.expect("run completes");

        let seen = client.seen();
        assert!(
            seen.len() >= 3,
            "expected several model turns, got {}",
            seen.len()
        );
        for (i, request) in seen.iter().enumerate() {
            assert!(
                request
                    .prompt
                    .starts_with("REPOSITORY INTELLIGENCE CONTEXT"),
                "request {i} is blind to the repository index: {}",
                request.prompt
            );
            assert!(
                request.prompt.contains("DETERMINISTIC ·"),
                "request {i} carries no deterministic citation: {}",
                request.prompt
            );
            assert_eq!(request.system, BASE_SYSTEM_PROMPT, "request {i}");
        }
    }

    #[tokio::test]
    async fn a_turn_that_names_no_symbol_receives_no_repository_evidence() {
        let dir = intel_fixture();
        let (mut agent, _client) = loop_over(dir.path(), "write a haiku about the ocean");

        agent.gather_context().await.expect("context gathers");

        let request = agent.provider_request("Answer the task.");
        assert!(
            !request.prompt.contains("DETERMINISTIC"),
            "an unrelated turn must not be handed repository evidence: {}",
            request.prompt
        );
        assert!(
            !request.prompt.contains(".rs:"),
            "an unrelated turn must not be handed file locations: {}",
            request.prompt
        );
        assert!(
            request.prompt.contains("No graph evidence is available"),
            "the absence must be stated, not left blank: {}",
            request.prompt
        );
    }

    #[tokio::test]
    async fn an_unbuildable_index_states_itself_in_the_request_instead_of_leaving_a_hole() {
        let missing = std::env::temp_dir().join(format!("zylcode-g3-{}", Uuid::new_v4()));
        let (mut agent, _client) = loop_over(&missing, "find where helper is used");

        agent.gather_context().await.expect("context gathers");

        let citations = &agent.session.context.navigation_citations;
        assert_eq!(citations.len(), 1, "{citations:?}");
        assert!(
            citations[0].contains("repository graph unavailable"),
            "the engine's own words must survive: {citations:?}"
        );

        let request = agent.provider_request("Answer the task.");
        assert!(
            request.prompt.contains("repository graph unavailable"),
            "{}",
            request.prompt
        );
        assert!(
            !request.prompt.contains("DETERMINISTIC"),
            "an unavailable index must never be rendered as an empty citation list: {}",
            request.prompt
        );
        assert!(
            !missing.exists(),
            "gathering must never create a workspace that was not there (G6)"
        );
    }

    #[tokio::test]
    async fn a_request_built_before_any_intelligence_is_gathered_carries_nothing() {
        let dir = intel_fixture();
        let (agent, _client) = loop_over(dir.path(), "find where helper is used");

        let request = agent.provider_request("Answer the task.");
        assert_eq!(request.prompt, "Answer the task.");
        assert!(!request.prompt.contains("REPOSITORY INTELLIGENCE CONTEXT"));
    }

    #[test]
    fn the_repository_context_budget_is_deterministic_and_announced() {
        let citations: Vec<String> = (0..50)
            .map(|i| format!("DETERMINISTIC · sym{i} @ src/f{i}.rs:{i} — declared — excerpt {i}"))
            .collect();

        let first = repository_context_block(&citations).expect("block renders");
        let second = repository_context_block(&citations).expect("block renders");
        assert_eq!(first, second, "rendering must be deterministic");

        for line in citations.iter().take(REPO_CONTEXT_MAX_CITATIONS) {
            assert!(first.contains(line.as_str()), "kept line missing: {line}");
        }
        for line in citations.iter().skip(REPO_CONTEXT_MAX_CITATIONS) {
            assert!(
                !first.contains(line.as_str()),
                "dropped line leaked: {line}"
            );
        }
        assert!(
            first.contains("omitted by the repository-context budget"),
            "truncation must be stated: {first}"
        );
        assert!(
            first.len() <= REPO_CONTEXT_MAX_CHARS + 1_024,
            "{}",
            first.len()
        );

        // Byte cap: lines are dropped whole, never cut through a `file:line`.
        let long: Vec<String> = (0..4)
            .map(|i| {
                format!(
                    "DETERMINISTIC · s{i} @ src/f{i}.rs:{i} — declared — {}",
                    "x".repeat(REPO_CONTEXT_MAX_CHARS)
                )
            })
            .collect();
        let block = repository_context_block(&long).expect("block renders");
        assert!(
            block.lines().any(|l| l == long[0]),
            "the first citation must survive whole: {block}"
        );
        assert!(
            !block.contains(long[1].as_str()),
            "the second must be dropped whole"
        );
        assert!(
            block.contains("omitted by the repository-context budget"),
            "truncation must be stated: {block}"
        );
        assert!(
            block.len() <= REPO_CONTEXT_MAX_CHARS + 1_024,
            "{}",
            block.len()
        );
    }

    #[test]
    fn citations_survive_session_serialisation_and_legacy_records_read_as_empty() {
        let citations = vec![
            "DETERMINISTIC · util::helper @ src/util.rs:1 — declared — pub fn helper() -> u32"
                .to_string(),
        ];
        let ctx = SessionContext {
            working_directory: PathBuf::from("."),
            files_read: vec![],
            files_modified: vec![],
            commands_executed: vec![],
            test_results: vec![],
            plan: None,
            verification_status: None,
            pending_tool_call: None,
            navigation_citations: citations.clone(),
        };

        let json = serde_json::to_string(&ctx).expect("serialises");
        assert!(json.contains("src/util.rs:1"), "{json}");

        let back: SessionContext = serde_json::from_str(&json).expect("deserialises");
        assert_eq!(back.navigation_citations, citations);

        // A checkpoint written before the field existed still reads: it resumes
        // with *no* citations, never with a fabricated empty block.
        let mut legacy = serde_json::to_value(&ctx).expect("to value");
        legacy
            .as_object_mut()
            .expect("object")
            .remove("navigation_citations");
        let restored: SessionContext = serde_json::from_value(legacy).expect("legacy reads");
        assert!(restored.navigation_citations.is_empty());
    }
}
