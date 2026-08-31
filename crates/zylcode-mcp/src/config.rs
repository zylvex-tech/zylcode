use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum McpTransport {
    #[default]
    Stdio,
    Sse,
    Websocket,
}

impl std::fmt::Display for McpTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stdio => write!(f, "stdio"),
            Self::Sse => write!(f, "sse"),
            Self::Websocket => write!(f, "websocket"),
        }
    }
}

impl std::str::FromStr for McpTransport {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "stdio" => Ok(Self::Stdio),
            "sse" => Ok(Self::Sse),
            "websocket" | "ws" => Ok(Self::Websocket),
            _ => anyhow::bail!("unknown transport: {s}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolConfig {
    pub id: String,
    pub command: String,
    #[serde(default)]
    pub transport: McpTransport,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub description: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpConfigFile {
    #[serde(default)]
    pub tools: Vec<McpToolConfig>,
    #[serde(default)]
    pub watch: bool,
}

impl McpConfigFile {
    pub fn from_path(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read MCP config {}", path.display()))?;
        Self::from_str(&raw)
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(raw: &str) -> Result<Self> {
        // Accept YAML or JSON (JSON is valid YAML)
        let cfg: Self = serde_yaml::from_str(raw).context("failed to parse MCP config (expected YAML/JSON)")?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<()> {
        let mut seen = std::collections::HashSet::new();
        for t in &self.tools {
            if t.id.trim().is_empty() {
                anyhow::bail!("MCP tool id must not be empty");
            }
            if t.command.trim().is_empty() {
                anyhow::bail!("MCP tool command must not be empty for id={}", t.id);
            }
            if !seen.insert(t.id.clone()) {
                anyhow::bail!("duplicate MCP tool id: {}", t.id);
            }
        }
        Ok(())
    }

    pub fn enabled_tools(&self) -> Vec<&McpToolConfig> {
        self.tools.iter().filter(|t| t.enabled).collect()
    }
}

// ---------------------------------------------------------------------------
// Hot-reload watcher — calls callback on file change (debounced)
// ---------------------------------------------------------------------------

pub type ReloadCallback = Arc<dyn Fn(McpConfigFile) + Send + Sync + 'static>;

pub struct ConfigWatcher {
    _watcher: notify::RecommendedWatcher,
    _path: PathBuf,
    // Keep callback alive
    _cb: ReloadCallback,
}

impl ConfigWatcher {
    pub fn watch(path: PathBuf, cb: ReloadCallback) -> Result<Self> {
        use notify::{RecursiveMode, Watcher};
        let cb_clone = Arc::clone(&cb);
        let path_clone = path.clone();
        let mut watcher = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
            match res {
                Ok(evt) if is_write_event(&evt) => {
                    match McpConfigFile::from_path(&path_clone) {
                        Ok(cfg) => {
                            info!(path = %path_clone.display(), tools = cfg.tools.len(), "MCP config reloaded");
                            cb_clone(cfg);
                        }
                        Err(e) => warn!(error = %e, path = %path_clone.display(), "failed to reload MCP config"),
                    }
                }
                Ok(_) => {}
                Err(e) => warn!(error = %e, "notify watcher error"),
            }
        })
        .context("failed to create file watcher")?;
        watcher
            .watch(&path, RecursiveMode::NonRecursive)
            .with_context(|| format!("failed to watch {}", path.display()))?;
        info!(path = %path.display(), "watching MCP config for hot-reload");
        Ok(Self {
            _watcher: watcher,
            _path: path,
            _cb: cb,
        })
    }
}

fn is_write_event(evt: &notify::Event) -> bool {
    use notify::EventKind;
    matches!(evt.kind, EventKind::Modify(_) | EventKind::Create(_))
}

// Keep a process-global watcher handle so it isn't dropped.
static WATCHER_HOLDER: Mutex<Option<ConfigWatcher>> = Mutex::new(None);

pub fn install_global_watcher(path: PathBuf, cb: ReloadCallback) -> Result<()> {
    let watcher = ConfigWatcher::watch(path, cb)?;
    *WATCHER_HOLDER.lock().unwrap() = Some(watcher);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_yaml_config() {
        let yaml = r#"
tools:
  - id: fs
    command: npx fs-mcp
    transport: stdio
    enabled: true
  - id: web
    command: https://example.com/sse
    transport: sse
"#;
        let cfg = McpConfigFile::from_str(yaml).unwrap();
        assert_eq!(cfg.tools.len(), 2);
        assert_eq!(cfg.tools[0].id, "fs");
        assert_eq!(cfg.tools[1].transport, McpTransport::Sse);
    }
    #[test]
    fn rejects_duplicate_id() {
        let yaml = r#"
tools:
  - id: dup
    command: a
  - id: dup
    command: b
"#;
        assert!(McpConfigFile::from_str(yaml).is_err());
    }
}
