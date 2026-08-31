use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

use crate::tool::Tool;

#[derive(Debug, Clone)]
pub struct ExecuteOptions {
    pub timeout: Duration,
    pub max_retries: u32,
    pub retry_backoff: Duration,
}

impl Default for ExecuteOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_retries: 2,
            retry_backoff: Duration::from_millis(250),
        }
    }
}

fn is_transient(err: &anyhow::Error) -> bool {
    let s = err.to_string().to_lowercase();
    s.contains("timeout")
        || s.contains("deadline")
        || s.contains("temporarily")
        || s.contains("connection")
        || s.contains("transient")
        || s.contains("busy")
}

/// Execute a tool with structured logging, 30s timeout, and up to 2 transient retries.
pub async fn execute_with_recovery(
    tool: Arc<dyn Tool>,
    params: Value,
    opts: ExecuteOptions,
) -> Result<Value> {
    let id = tool.id().to_string();
    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
        info!(tool = %id, attempt, "executing tool");
        let fut = tool.call(params.clone());
        let res = tokio::time::timeout(opts.timeout, fut).await;
        match res {
            Ok(Ok(val)) => {
                info!(tool = %id, attempt, "tool succeeded");
                return Ok(val);
            }
            Ok(Err(e)) => {
                let transient = is_transient(&e);
                if transient && attempt <= opts.max_retries {
                    warn!(tool = %id, attempt, error = %e, "transient tool error, retrying");
                    tokio::time::sleep(opts.retry_backoff * attempt).await;
                    continue;
                }
                error!(tool = %id, attempt, error = %e, transient, "tool failed");
                return Err(e);
            }
            Err(_elapsed) => {
                let e = anyhow::anyhow!("tool {id} timed out after {:?}", opts.timeout);
                if attempt <= opts.max_retries {
                    warn!(tool = %id, attempt, error = %e, "tool timeout, retrying");
                    tokio::time::sleep(opts.retry_backoff * attempt).await;
                    continue;
                }
                error!(tool = %id, attempt, error = %e, "tool timed out");
                return Err(e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{McpToolConfig, McpTransport};
    use crate::tool::DynamicTool;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug)]
    struct FlakyTool {
        id: String,
        fails: usize,
        counter: AtomicUsize,
    }
    #[async_trait::async_trait]
    impl Tool for FlakyTool {
        fn id(&self) -> &str {
            &self.id
        }
        fn descriptor(&self) -> crate::tool::ToolDescriptor {
            crate::tool::ToolDescriptor {
                id: self.id.clone(),
                transport: "stdio".into(),
                command: "flaky".into(),
                env_keys: vec![],
                description: None,
            }
        }
        async fn call(&self, _params: Value) -> anyhow::Result<Value> {
            let n = self.counter.fetch_add(1, Ordering::SeqCst);
            if n < self.fails {
                anyhow::bail!("transient failure {}", n)
            }
            Ok(serde_json::json!({"ok": true}))
        }
    }

    #[tokio::test]
    async fn retries_transient_then_succeeds() {
        let t: Arc<dyn Tool> = Arc::new(FlakyTool {
            id: "flaky".into(),
            fails: 1,
            counter: AtomicUsize::new(0),
        });
        let v = execute_with_recovery(t, Value::Null, ExecuteOptions::default())
            .await
            .unwrap();
        assert_eq!(v["ok"], true);
    }

    #[tokio::test]
    async fn succeeds_without_retry_for_stable_tool() {
        let cfg = McpToolConfig {
            id: "stable".into(),
            command: "echo".into(),
            transport: McpTransport::Stdio,
            env: Default::default(),
            enabled: true,
            description: None,
        };
        let t: Arc<dyn Tool> = Arc::new(DynamicTool::new(cfg));
        let v = execute_with_recovery(t, serde_json::json!({"a":1}), ExecuteOptions::default())
            .await
            .unwrap();
        assert_eq!(v["tool"], "stable");
    }
}
