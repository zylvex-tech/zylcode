use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::tool::{Tool, ToolDescriptor};

#[derive(Default)]
pub struct ToolRegistry {
    tools: RwLock<HashMap<String, Arc<dyn Tool>>>,
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Avoid locking in Debug
        f.debug_struct("ToolRegistry").finish_non_exhaustive()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn register(&self, tool: Arc<dyn Tool>) {
        let id = tool.id().to_string();
        self.tools.write().await.insert(id, tool);
    }

    pub async fn register_many(&self, tools: Vec<Arc<dyn Tool>>) {
        let mut g = self.tools.write().await;
        for t in tools {
            g.insert(t.id().to_string(), t);
        }
    }

    pub async fn unregister(&self, id: &str) -> bool {
        self.tools.write().await.remove(id).is_some()
    }

    pub async fn get(&self, id: &str) -> Option<Arc<dyn Tool>> {
        self.tools.read().await.get(id).cloned()
    }

    pub async fn list(&self) -> Vec<ToolDescriptor> {
        let g = self.tools.read().await;
        let mut out: Vec<ToolDescriptor> = g.values().map(|t| t.descriptor()).collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    pub async fn len(&self) -> usize {
        self.tools.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.tools.read().await.is_empty()
    }

    pub async fn clear(&self) {
        self.tools.write().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{McpToolConfig, McpTransport};
    use crate::tool::DynamicTool;

    #[tokio::test]
    async fn registry_register_and_list() {
        let reg = ToolRegistry::new();
        let cfg = McpToolConfig {
            id: "a".into(),
            command: "echo".into(),
            transport: McpTransport::Stdio,
            env: Default::default(),
            enabled: true,
            description: None,
        };
        reg.register(Arc::new(DynamicTool::new(cfg))).await;
        assert_eq!(reg.len().await, 1);
        let list = reg.list().await;
        assert_eq!(list[0].id, "a");
    }
}
