use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;
use zylcode_mcp::ToolRegistry;
use zylcode_mcp::real_tools::{get_real_tool, ToolContext, ToolResult as RealToolResult};

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
}

impl AgentLoop {
    pub fn new(task_description: &str, working_dir: PathBuf, config: Option<AgentConfig>, tool_registry: Arc<ToolRegistry>) -> Self {
        let config = config.unwrap_or_default();
        let session_id = Uuid::new_v4().to_string();
        let task_id = Uuid::new_v4().to_string();
        
        let session = AgentSession {
            id: session_id,
            task_id: task_id.clone(),
            state: AgentState::Created,
            messages: vec![
                AgentMessage {
                    id: Uuid::new_v4().to_string(),
                    role: MessageRole::User,
                    content: task_description.to_string(),
                    timestamp: chrono::Utc::now(),
                    tool_calls: None,
                    tool_results: None,
                }
            ],
            context: SessionContext {
                working_directory: working_dir,
                files_read: Vec::new(),
                files_modified: Vec::new(),
                commands_executed: Vec::new(),
                test_results: Vec::new(),
                plan: None,
                verification_status: None,
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
        }
    }
    
    /// Execute the agent loop until completion or failure
    pub async fn run(&mut self) -> Result<AgentState> {
        while self.session.state != AgentState::Completed 
            && self.session.state != AgentState::Failed 
            && self.session.iterations < self.session.max_iterations 
        {
            self.step().await?;
            
            // Check timeout
            if self.start_time.elapsed() > self.config.timeout {
                self.session.state = AgentState::Failed;
                self.add_system_message("Task timed out".to_string());
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
                if self.config.require_approval {
                    self.transition_to(AgentState::AwaitingApproval).await?;
                } else {
                    self.transition_to(AgentState::Executing).await?;
                }
            }
            AgentState::AwaitingApproval => {
                // In a real implementation, this would wait for user input
                // For now, we auto-approve
                self.transition_to(AgentState::Executing).await?;
            }
            AgentState::Executing => {
                self.execute_plan().await?;
                self.transition_to(AgentState::Verifying).await?;
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
        // Read relevant files based on task description
        let task_desc = self.session.messages[0].content.clone();
        
        // For now, just read the current directory
        // In a real implementation, this would analyze the task and read relevant files
        self.add_system_message(format!("Analyzing task: {}", task_desc));
        
        // TODO: Implement real context gathering
        // - List files in working directory
        // - Read relevant source files
        // - Parse AST if needed
        
        Ok(())
    }
    
    async fn generate_plan(&mut self) -> Result<()> {
        // Call LLM to generate a plan
        let task_desc = self.session.messages[0].content.clone();
        
        // For now, generate a simple plan
        // In a real implementation, this would call the LLM
        let plan = format!(
            "Plan for: {}\n\n1. Analyze the codebase\n2. Identify changes needed\n3. Implement changes\n4. Test changes\n5. Verify results",
            task_desc
        );
        
        self.session.context.plan = Some(plan.clone());
        self.add_assistant_message(format!("Here's my plan:\n{}", plan));
        
        Ok(())
    }
    
    async fn execute_plan(&mut self) -> Result<()> {
        // Execute the plan using tools
        let plan = self.session.context.plan.clone().unwrap_or_default();
        
        // For now, implement a simple execution:
        // 1. Read a file
        // 2. Modify it
        // 3. Run a command
        
        self.add_system_message("Executing plan...".to_string());
        println!("DEBUG: execute_plan called");
        
        // Example: Read Cargo.toml
        let read_tool = self.tool_registry.get("fs.read").await;
        println!("DEBUG: fs.read tool found: {}", read_tool.is_some());
        
        if let Some(tool) = read_tool {
            let params = serde_json::json!({
                "action": "read",
                "path": "Cargo.toml"
            });
            
            match tool.call(params).await {
                Ok(result) => {
                    self.add_tool_result("fs.read", result.clone(), true, 0);
                    
                    // Check if we got content
                    if let Some(content) = result.get("content") {
                        self.session.context.files_read.push(PathBuf::from("Cargo.toml"));
                        self.add_system_message(format!("Read Cargo.toml ({} bytes)", content.as_str().map(|s| s.len()).unwrap_or(0)));
                    }
                }
                Err(e) => {
                    self.add_tool_result("fs.read", serde_json::json!({"error": e.to_string()}), false, 0);
                }
            }
        }
        
        // Example: Run a command
        let shell_tool = self.tool_registry.get("shell.execute").await;
        println!("DEBUG: shell.execute tool found: {}", shell_tool.is_some());
        
        if let Some(tool) = shell_tool {
            let params = serde_json::json!({
                "command": "echo",
                "args": ["Agent execution complete"]
            });
            
            match tool.call(params).await {
                Ok(result) => {
                    println!("DEBUG: shell.execute result: {:?}", result);
                    self.add_tool_result("shell.execute", result.clone(), true, 0);
                    
                    if let Some(stdout) = result.get("stdout") {
                        let cmd = ExecutedCommand {
                            command: "echo".to_string(),
                            args: vec!["Agent execution complete".to_string()],
                            exit_code: result.get("exit_code").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                            stdout: stdout.as_str().unwrap_or("").to_string(),
                            stderr: result.get("stderr").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            duration_ms: 0,
                        };
                        self.session.context.commands_executed.push(cmd);
                        println!("DEBUG: Command added to executed list");
                    } else {
                        println!("DEBUG: No stdout in result");
                    }
                }
                Err(e) => {
                    println!("DEBUG: shell.execute error: {}", e);
                    self.add_tool_result("shell.execute", serde_json::json!({"error": e.to_string()}), false, 0);
                }
            }
        }
        
        Ok(())
    }
    
    async fn verify_results(&mut self) -> Result<()> {
        // Verify the results of execution
        // For now, assume success
        // In a real implementation, this would:
        // 1. Run tests
        // 2. Check compilation
        // 3. Verify against requirements
        
        self.session.context.verification_status = Some(true);
        self.add_system_message("Verification passed".to_string());
        
        Ok(())
    }
    
    async fn repair_errors(&mut self) -> Result<()> {
        // Attempt to repair errors found in verification
        // For now, just log
        // In a real implementation, this would:
        // 1. Analyze the error
        // 2. Generate a fix
        // 3. Apply the fix
        
        self.add_system_message("Attempting to repair errors...".to_string());
        
        Ok(())
    }
    
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
    
    fn add_tool_result(&mut self, tool_name: &str, result: serde_json::Value, success: bool, duration_ms: u64) {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_agent_loop_creation() {
        let registry = Arc::new(ToolRegistry::new());
        let agent = AgentLoop::new(
            "Fix the bug in main.rs",
            PathBuf::from("."),
            None,
            registry,
        );
        
        assert_eq!(agent.session.state, AgentState::Created);
        assert_eq!(agent.session.messages.len(), 1);
        assert_eq!(agent.session.messages[0].role, MessageRole::User);
    }
    
    #[tokio::test]
    async fn test_agent_loop_step() {
        let registry = Arc::new(ToolRegistry::new());
        let mut agent = AgentLoop::new(
            "Fix the bug in main.rs",
            PathBuf::from("."),
            None,
            registry,
        );
        
        // First step should transition to Analyzing
        agent.step().await.unwrap();
        assert_eq!(agent.session.state, AgentState::Analyzing);
    }
}