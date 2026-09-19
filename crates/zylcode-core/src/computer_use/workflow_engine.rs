use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::RwLock;

use super::types::*;

/// Workflow engine for automation workflows
pub struct WorkflowEngine {
    workflows: RwLock<HashMap<String, Workflow>>,
    stats: RwLock<WorkflowEngineStats>,
}

#[derive(Debug, Default)]
struct WorkflowEngineStats {
    executed_count: u64,
    total_execution_time_ms: u64,
}

impl WorkflowEngine {
    /// Create a new workflow engine
    pub async fn new() -> Result<Self> {
        Ok(Self {
            workflows: RwLock::new(HashMap::new()),
            stats: RwLock::new(WorkflowEngineStats::default()),
        })
    }

    /// Create a new workflow
    pub async fn create_workflow(&self, definition: WorkflowDefinition) -> Result<Workflow> {
        let workflow = Workflow {
            id: definition.id.clone(),
            definition,
            status: WorkflowStatus::Created,
            current_step: None,
            variables: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let mut workflows = self.workflows.write().unwrap();
        workflows.insert(workflow.id.clone(), workflow.clone());

        Ok(workflow)
    }

    /// Execute workflow
    pub async fn execute_workflow(&self, workflow_id: &str) -> Result<WorkflowResult> {
        let start = std::time::Instant::now();

        // Take the workflow under a short-lived write guard. The guard must not
        // be held across the `sleep` below: a `std::sync::RwLockWriteGuard` is
        // not `Send`, and holding one across an await point can deadlock the
        // runtime. Mark it Running, take a copy, and release the lock.
        let mut workflow = {
            let mut workflows = self.workflows.write().unwrap();
            let workflow = workflows
                .get_mut(workflow_id)
                .ok_or_else(|| anyhow::anyhow!("Workflow not found: {}", workflow_id))?;
            workflow.status = WorkflowStatus::Running;
            workflow.updated_at = Utc::now();
            workflow.clone()
        };

        // Simulate workflow execution
        let steps_executed = workflow.definition.steps.len();
        let mut output = HashMap::new();
        let errors = Vec::new();

        // Execute each step
        for step in &workflow.definition.steps {
            // Simulate step execution
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // Update current step
            workflow.current_step = Some(step.id.clone());

            // Simulate step result
            output.insert(format!("step_{}_result", step.id), "success".to_string());
        }

        // Update workflow status and persist the result back under a fresh,
        // non-awaited guard.
        workflow.status = WorkflowStatus::Completed;
        workflow.updated_at = Utc::now();
        {
            let mut workflows = self.workflows.write().unwrap();
            workflows.insert(workflow.id.clone(), workflow);
        }

        // Update stats
        let duration = start.elapsed().as_millis() as u64;
        {
            let mut stats = self.stats.write().unwrap();
            stats.executed_count += 1;
            stats.total_execution_time_ms += duration;
        }

        Ok(WorkflowResult {
            workflow_id: workflow_id.to_string(),
            status: WorkflowStatus::Completed,
            steps_executed,
            duration_ms: duration,
            output,
            errors,
        })
    }

    /// Pause workflow
    pub async fn pause_workflow(&self, workflow_id: &str) -> Result<()> {
        let mut workflows = self.workflows.write().unwrap();
        let workflow = workflows
            .get_mut(workflow_id)
            .ok_or_else(|| anyhow::anyhow!("Workflow not found: {}", workflow_id))?;

        workflow.status = WorkflowStatus::Paused;
        workflow.updated_at = Utc::now();

        Ok(())
    }

    /// Resume workflow
    pub async fn resume_workflow(&self, workflow_id: &str) -> Result<()> {
        let mut workflows = self.workflows.write().unwrap();
        let workflow = workflows
            .get_mut(workflow_id)
            .ok_or_else(|| anyhow::anyhow!("Workflow not found: {}", workflow_id))?;

        workflow.status = WorkflowStatus::Running;
        workflow.updated_at = Utc::now();

        Ok(())
    }

    /// Schedule workflow
    pub async fn schedule_workflow(&self, workflow_id: &str, _schedule: Schedule) -> Result<()> {
        // In a real implementation, this would schedule the workflow
        // For now, we just validate the workflow exists
        let workflows = self.workflows.read().unwrap();
        workflows
            .get(workflow_id)
            .ok_or_else(|| anyhow::anyhow!("Workflow not found: {}", workflow_id))?;

        Ok(())
    }

    /// Get workflow
    pub async fn get_workflow(&self, workflow_id: &str) -> Result<Workflow> {
        let workflows = self.workflows.read().unwrap();
        workflows
            .get(workflow_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Workflow not found: {}", workflow_id))
    }

    /// List workflows
    pub async fn list_workflows(&self) -> Vec<Workflow> {
        let workflows = self.workflows.read().unwrap();
        workflows.values().cloned().collect()
    }

    /// Delete workflow
    pub async fn delete_workflow(&self, workflow_id: &str) -> Result<()> {
        let mut workflows = self.workflows.write().unwrap();
        workflows
            .remove(workflow_id)
            .ok_or_else(|| anyhow::anyhow!("Workflow not found: {}", workflow_id))?;

        Ok(())
    }

    /// Get execution statistics
    pub async fn get_stats(&self) -> u64 {
        self.stats.read().unwrap().executed_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workflow_creation() {
        let engine = WorkflowEngine::new().await.unwrap();

        let definition = WorkflowDefinition {
            id: "test_workflow".to_string(),
            name: "Test Workflow".to_string(),
            description: "A test workflow".to_string(),
            steps: vec![WorkflowStep {
                id: "step1".to_string(),
                name: "Step 1".to_string(),
                step_type: StepType::CaptureScreen,
                parameters: HashMap::new(),
                conditions: Vec::new(),
                on_failure: FailureAction::Stop,
            }],
            variables: HashMap::new(),
            triggers: Vec::new(),
        };

        let workflow = engine.create_workflow(definition).await.unwrap();
        assert_eq!(workflow.id, "test_workflow");
        assert!(matches!(workflow.status, WorkflowStatus::Created));
    }

    #[tokio::test]
    async fn test_workflow_execution() {
        let engine = WorkflowEngine::new().await.unwrap();

        let definition = WorkflowDefinition {
            id: "test_workflow".to_string(),
            name: "Test Workflow".to_string(),
            description: "A test workflow".to_string(),
            steps: vec![WorkflowStep {
                id: "step1".to_string(),
                name: "Step 1".to_string(),
                step_type: StepType::CaptureScreen,
                parameters: HashMap::new(),
                conditions: Vec::new(),
                on_failure: FailureAction::Stop,
            }],
            variables: HashMap::new(),
            triggers: Vec::new(),
        };

        engine.create_workflow(definition).await.unwrap();
        let result = engine.execute_workflow("test_workflow").await.unwrap();

        assert!(matches!(result.status, WorkflowStatus::Completed));
        assert_eq!(result.steps_executed, 1);
    }
}
