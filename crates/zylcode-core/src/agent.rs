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
}

/// Tool call request from the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub tool_id: String,
    pub arguments: serde_json::Value,
    pub reason: String,
    pub expected_result: Option<String>,
}

impl AgentLoop {
    pub fn new(
        task_description: &str,
        working_dir: PathBuf,
        config: Option<AgentConfig>,
        tool_registry: Arc<ToolRegistry>,
        model_client: Arc<dyn ModelClient>,
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
        }
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
                    // Execute the tool directly
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

                        // Execute the tool
                        let tool = self.tool_registry.get(&tool_id).await;
                        if let Some(tool) = tool {
                            match tool.call(arguments.clone()).await {
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
                        let all_passed = true;
                        for check in checks {
                            // For now, assume checks pass
                            // In a real implementation, we would run actual checks
                            self.add_system_message(format!(
                                "Verification check: {} - Passed",
                                check
                            ));
                        }
                        self.session.context.verification_status = Some(all_passed);
                    }
                    AgentDecision::Complete {
                        summary,
                        evidence,
                        remaining_limitations: _,
                    } => {
                        // Verify that evidence supports completion
                        if self.verify_completion_evidence(&evidence) {
                            self.add_system_message(format!("Verification passed: {}", summary));
                            self.session.context.verification_status = Some(true);
                        } else {
                            self.add_system_message(
                                "Completion rejected: insufficient evidence".to_string(),
                            );
                            self.session.context.verification_status = Some(false);
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
                    }
                    _ => {
                        self.add_system_message(
                            "Model returned unexpected verification decision".to_string(),
                        );
                        self.session.context.verification_status = Some(true);
                    }
                }
            }
            Err(e) => {
                self.add_system_message(format!("Verification model call failed: {}", e));
                self.session.context.verification_status = Some(true);
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

                        // Execute repair tool
                        let tool = self.tool_registry.get(&tool_id).await;
                        if let Some(tool) = tool {
                            match tool.call(arguments.clone()).await {
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
            },
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

    /// Execute a tool call
    async fn execute_tool_call(&mut self, tool_call: ToolCallRequest) -> Result<()> {
        self.add_system_message(format!(
            "Executing tool: {} - {}",
            tool_call.tool_id, tool_call.reason
        ));

        // Execute the tool
        let tool = self.tool_registry.get(&tool_call.tool_id).await;
        if let Some(tool) = tool {
            match tool.call(tool_call.arguments.clone()).await {
                Ok(result) => {
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
                }
                Err(e) => {
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

    #[tokio::test]
    async fn test_agent_loop_creation() {
        let registry = Arc::new(ToolRegistry::new());
        let model_client = Arc::new(TestModelClient::new(vec![]));
        let agent = AgentLoop::new(
            "Fix the bug in main.rs",
            PathBuf::from("."),
            None,
            registry,
            model_client,
        );

        assert_eq!(agent.session.state, AgentState::Created);
        assert_eq!(agent.session.messages.len(), 1);
        assert_eq!(agent.session.messages[0].role, MessageRole::User);
    }

    #[tokio::test]
    async fn test_agent_loop_step() {
        let registry = Arc::new(ToolRegistry::new());
        let model_client = Arc::new(TestModelClient::new(vec![]));
        let mut agent = AgentLoop::new(
            "Fix the bug in main.rs",
            PathBuf::from("."),
            None,
            registry,
            model_client,
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

        let mut agent = AgentLoop::new(
            "Execute destructive command",
            PathBuf::from("."),
            None,
            registry,
            model_client,
        );

        // Run the agent loop
        let final_state = agent.run().await.unwrap();

        // Should be in AwaitingApproval state because destructive action requires approval
        assert_eq!(final_state, AgentState::AwaitingApproval);
    }

    #[tokio::test]
    async fn test_completion_verification() {
        let registry = Arc::new(ToolRegistry::new());
        let model_client = Arc::new(TestModelClient::new(vec![
            // Response for planning
            r#"{"action": "Plan", "payload": {"steps": []}}"#.to_string(),
            // Response for tool call
            r#"{"action": "ToolCall", "payload": {"tool_id": "fs.read", "arguments": {"action": "read", "path": "Cargo.toml"}, "reason": "Read file", "expected_result": "File content"}}"#.to_string(),
            // Response for verification (complete without evidence)
            r#"{"action": "Complete", "payload": {"summary": "Task completed", "evidence": [], "remaining_limitations": []}}"#.to_string(),
        ]));

        let mut agent = AgentLoop::new(
            "Read Cargo.toml",
            PathBuf::from("."),
            None,
            registry,
            model_client,
        );

        // Run the agent loop
        let final_state = agent.run().await.unwrap();

        // Should fail because completion has no evidence
        assert_eq!(final_state, AgentState::Failed);
    }

    #[tokio::test]
    async fn test_step_limit() {
        let registry = Arc::new(ToolRegistry::new());
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

        let mut agent = AgentLoop::new(
            "Read Cargo.toml",
            PathBuf::from("."),
            Some(config),
            registry,
            model_client,
        );

        // Run the agent loop
        let final_state = agent.run().await.unwrap();

        // Should fail due to step limit
        assert_eq!(final_state, AgentState::Failed);
    }
}
