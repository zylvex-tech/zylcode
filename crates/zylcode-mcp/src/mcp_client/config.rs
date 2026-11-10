//! Persisted MCP server configuration (`mcp.servers.json`).
//!
//! # Why a new file
//!
//! The repository already carried `mcp-bridge-config.yaml`, which declared a
//! `bridge:` with "100+ tools across 8 categories" and no reader anywhere in the
//! crates — a decorative config that made MCP look configured. It was removed in
//! the same change that introduced this file. `mcp.tools.yaml` is *not* replaced:
//! it configures ZylCode's own in-process tools, which is a different thing from
//! a list of external servers to connect to.
//!
//! # Shape
//!
//! ```json
//! {
//!   "version": 1,
//!   "servers": [
//!     { "id": "filesystem", "transport": "stdio", "command": "npx",
//!       "args": ["-y", "@modelcontextprotocol/server-filesystem", "."],
//!       "startup": "eager", "timeout_ms": 30000,
//!       "restart": { "max_attempts": 3, "initial_backoff_ms": 500, "max_backoff_ms": 15000 } },
//!     { "id": "local-http", "transport": "http", "url": "http://127.0.0.1:8931/mcp" }
//!   ]
//! }
//! ```
//!
//! # Validation is the security boundary
//!
//! Every field is checked here rather than at use time, and a bad file fails to
//! load with a message naming the server and the field. Two rules matter beyond
//! shape:
//!
//! * an `id` may not contain `__`, because the tool namespace separator is `__`
//!   and an id that contains it could impersonate another server's tool;
//! * plaintext `http://` is only accepted for a loopback host. A remote MCP
//!   endpoint over cleartext leaks the tool arguments and any token in a header,
//!   so it is refused rather than warned about.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::transport::{HttpOptions, StdioOptions};

/// Default config filename, resolved against the process CWD only when no
/// explicit path is given (this is a user-editable config, not runtime state).
pub const CONFIG_FILENAME: &str = "mcp.servers.json";
/// Environment override for the config path.
pub const CONFIG_ENV: &str = "ZYLCODE_MCP_SERVERS";
/// Schema version this build understands.
pub const CONFIG_VERSION: u32 = 1;

/// Transport kind for an external server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServerTransport {
    /// A child process speaking newline-delimited JSON-RPC.
    Stdio,
    /// A Streamable HTTP endpoint.
    Http,
}

impl std::fmt::Display for ServerTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stdio => f.write_str("stdio"),
            Self::Http => f.write_str("http"),
        }
    }
}

/// When the client connects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum StartupPolicy {
    /// Connect at application start.
    #[default]
    Eager,
    /// Connect on first tool call.
    Lazy,
    /// Connect only when the user asks, from the inspector.
    Manual,
}

/// Restart behaviour after a crash or transport failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestartPolicy {
    /// Maximum consecutive attempts before the server is marked failed.
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,
    /// First backoff in milliseconds; doubles each attempt.
    #[serde(default = "default_initial_backoff")]
    pub initial_backoff_ms: u64,
    /// Ceiling for the backoff.
    #[serde(default = "default_max_backoff")]
    pub max_backoff_ms: u64,
}

fn default_max_attempts() -> u32 {
    3
}
fn default_initial_backoff() -> u64 {
    500
}
fn default_max_backoff() -> u64 {
    15_000
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self {
            max_attempts: default_max_attempts(),
            initial_backoff_ms: default_initial_backoff(),
            max_backoff_ms: default_max_backoff(),
        }
    }
}

impl RestartPolicy {
    /// Backoff before attempt `n` (1-based), doubling and capped.
    pub fn backoff_for(&self, attempt: u32) -> std::time::Duration {
        let exp = attempt.saturating_sub(1).min(16);
        let ms = self
            .initial_backoff_ms
            .saturating_mul(1u64 << exp)
            .min(self.max_backoff_ms);
        std::time::Duration::from_millis(ms)
    }
}

/// One configured external MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpServerConfig {
    /// Stable identifier; also the tool namespace prefix.
    pub id: String,
    /// Transport.
    pub transport: ServerTransport,
    /// Whether this server is used at all.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// stdio: program to run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// stdio: arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// stdio: extra environment variables.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// stdio: working directory (absolute, or relative without `..`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// http: endpoint URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// http: extra headers.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// When to connect.
    #[serde(default)]
    pub startup: StartupPolicy,
    /// Per-request timeout.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    /// Restart behaviour.
    #[serde(default)]
    pub restart: RestartPolicy,
}

fn default_true() -> bool {
    true
}
fn default_timeout_ms() -> u64 {
    30_000
}

impl McpServerConfig {
    /// A stdio server with the given program.
    pub fn stdio(id: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            transport: ServerTransport::Stdio,
            enabled: true,
            command: Some(command.into()),
            args: Vec::new(),
            env: BTreeMap::new(),
            cwd: None,
            url: None,
            headers: BTreeMap::new(),
            startup: StartupPolicy::Eager,
            timeout_ms: default_timeout_ms(),
            restart: RestartPolicy::default(),
        }
    }

    /// An HTTP server with the given endpoint.
    pub fn http(id: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            transport: ServerTransport::Http,
            enabled: true,
            command: None,
            args: Vec::new(),
            env: BTreeMap::new(),
            cwd: None,
            url: Some(url.into()),
            headers: BTreeMap::new(),
            startup: StartupPolicy::Eager,
            timeout_ms: default_timeout_ms(),
            restart: RestartPolicy::default(),
        }
    }

    /// Check every field, naming the server and the field on failure.
    pub fn validate(&self) -> Result<()> {
        validate_id(&self.id)?;

        if self.timeout_ms < 1_000 || self.timeout_ms > 600_000 {
            bail!(
                "server `{}`: timeout_ms must be between 1000 and 600000 (was {})",
                self.id,
                self.timeout_ms
            );
        }
        if self.restart.max_attempts > 10 {
            bail!(
                "server `{}`: restart.max_attempts must be <= 10 (was {})",
                self.id,
                self.restart.max_attempts
            );
        }
        if self.restart.initial_backoff_ms < 50 || self.restart.initial_backoff_ms > 60_000 {
            bail!(
                "server `{}`: restart.initial_backoff_ms must be between 50 and 60000",
                self.id
            );
        }
        if self.restart.max_backoff_ms < self.restart.initial_backoff_ms {
            bail!(
                "server `{}`: restart.max_backoff_ms must not be below initial_backoff_ms",
                self.id
            );
        }

        for (key, value) in &self.env {
            if key.trim().is_empty() {
                bail!("server `{}`: env contains an empty key", self.id);
            }
            if key.contains('=') || key.contains('\0') {
                bail!("server `{}`: env key `{key}` contains `=` or NUL", self.id);
            }
            if value.contains('\0') {
                bail!("server `{}`: env value for `{key}` contains NUL", self.id);
            }
        }

        match self.transport {
            ServerTransport::Stdio => {
                let Some(command) = self.command.as_deref() else {
                    bail!(
                        "server `{}`: `command` is required for transport stdio",
                        self.id
                    );
                };
                if command.trim().is_empty() {
                    bail!("server `{}`: `command` must not be empty", self.id);
                }
                if command.contains('\n') {
                    bail!("server `{}`: `command` must not contain a newline", self.id);
                }
                if self.url.is_some() {
                    bail!(
                        "server `{}`: `url` is not valid for transport stdio",
                        self.id
                    );
                }
                if !self.headers.is_empty() {
                    bail!(
                        "server `{}`: `headers` are not valid for transport stdio",
                        self.id
                    );
                }
                for arg in &self.args {
                    if arg.contains('\n') {
                        bail!("server `{}`: args must not contain newlines", self.id);
                    }
                }
                if let Some(cwd) = &self.cwd {
                    validate_cwd(&self.id, cwd)?;
                }
            }
            ServerTransport::Http => {
                let Some(url) = self.url.as_deref() else {
                    bail!("server `{}`: `url` is required for transport http", self.id);
                };
                validate_url(&self.id, url)?;
                if self.command.is_some() {
                    bail!(
                        "server `{}`: `command` is not valid for transport http",
                        self.id
                    );
                }
                if !self.args.is_empty() || !self.env.is_empty() || self.cwd.is_some() {
                    bail!(
                        "server `{}`: args/env/cwd are only valid for transport stdio",
                        self.id
                    );
                }
            }
        }
        Ok(())
    }

    /// Convert to transport options.
    pub fn stdio_options(&self) -> Result<StdioOptions> {
        self.validate()?;
        let command = self
            .command
            .clone()
            .ok_or_else(|| anyhow::anyhow!("server `{}` has no command", self.id))?;
        let mut options = StdioOptions::new(command);
        options.args = self.args.clone();
        options.env = self
            .env
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        options.cwd = self.cwd.as_ref().map(PathBuf::from);
        Ok(options)
    }

    /// Convert to HTTP options.
    pub fn http_options(&self) -> Result<HttpOptions> {
        self.validate()?;
        let url = self
            .url
            .clone()
            .ok_or_else(|| anyhow::anyhow!("server `{}` has no url", self.id))?;
        let mut options = HttpOptions::new(url);
        options.headers = self
            .headers
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        options.timeout = std::time::Duration::from_millis(self.timeout_ms);
        Ok(options)
    }
}

/// Identifier rules, shared with the tool namespace.
pub fn validate_id(id: &str) -> Result<()> {
    if id.trim().is_empty() {
        bail!("server id must not be empty");
    }
    if id.len() > 48 {
        bail!("server id `{id}` is longer than 48 characters");
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    {
        bail!("server id `{id}` may only contain lowercase letters, digits, `-` and `_`");
    }
    if !id.chars().next().is_some_and(|c| c.is_ascii_alphanumeric()) {
        bail!("server id `{id}` must start with a letter or digit");
    }
    if id.contains("__") {
        // `__` separates server id from tool name in a namespaced tool id.
        bail!("server id `{id}` must not contain `__` (reserved as the tool namespace separator)");
    }
    Ok(())
}

fn validate_cwd(server: &str, cwd: &str) -> Result<()> {
    if cwd.trim().is_empty() {
        bail!("server `{server}`: cwd must not be empty");
    }
    if cwd.contains('\0') {
        bail!("server `{server}`: cwd contains NUL");
    }
    let path = Path::new(cwd);
    if path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        bail!("server `{server}`: cwd must not contain `..`");
    }
    Ok(())
}

fn validate_url(server: &str, url: &str) -> Result<()> {
    let lower = url.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        bail!("server `{server}`: url must start with http:// or https://");
    }
    if lower.starts_with("http://") {
        // Cleartext to a non-loopback host leaks tool arguments and headers.
        let rest = lower.strip_prefix("http://").unwrap_or(&lower);
        let host = rest
            .split(['/', ':', '?', '#'])
            .next()
            .unwrap_or_default()
            .trim_start_matches('[')
            .trim_end_matches(']');
        let loopback = matches!(host, "127.0.0.1" | "localhost" | "::1") || host == "0.0.0.0";
        if !loopback {
            bail!(
                "server `{server}`: cleartext http:// is only allowed for a loopback host \
                 (use https:// for {host})"
            );
        }
    }
    Ok(())
}

/// The whole file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpServersFile {
    /// Schema version; must equal [`CONFIG_VERSION`].
    #[serde(default = "default_version")]
    pub version: u32,
    /// Configured servers.
    #[serde(default)]
    pub servers: Vec<McpServerConfig>,
}

fn default_version() -> u32 {
    CONFIG_VERSION
}

impl Default for McpServersFile {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            servers: Vec::new(),
        }
    }
}

impl McpServersFile {
    /// Parse from JSON text and validate every entry.
    pub fn from_json(text: &str) -> Result<Self> {
        let file: Self = serde_json::from_str(text).context("parsing mcp.servers.json")?;
        file.validate()?;
        Ok(file)
    }

    /// Read and validate a file.
    pub fn from_path(path: &Path) -> Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        Self::from_json(&text).with_context(|| format!("validating {}", path.display()))
    }

    /// Write atomically (temp file + rename), so a crash cannot leave a
    /// half-written config that fails the next start.
    pub fn to_path(&self, path: &Path) -> Result<()> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating {}", parent.display()))?;
            }
        }
        let text = serde_json::to_string_pretty(self).context("serialising server config")?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, text).with_context(|| format!("writing {}", tmp.display()))?;
        std::fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))?;
        Ok(())
    }

    /// Validate cross-entry rules.
    pub fn validate(&self) -> Result<()> {
        if self.version != CONFIG_VERSION {
            bail!(
                "unsupported mcp.servers.json version {} (this build reads {})",
                self.version,
                CONFIG_VERSION
            );
        }
        let mut seen = std::collections::HashSet::new();
        for server in &self.servers {
            server.validate()?;
            if !seen.insert(server.id.as_str()) {
                bail!("duplicate server id `{}`", server.id);
            }
        }
        Ok(())
    }

    /// Enabled servers.
    pub fn enabled(&self) -> impl Iterator<Item = &McpServerConfig> {
        self.servers.iter().filter(|s| s.enabled)
    }

    /// Configured path: `ZYLCODE_MCP_SERVERS`, else `./mcp.servers.json`.
    pub fn default_path() -> PathBuf {
        std::env::var(CONFIG_ENV)
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(CONFIG_FILENAME))
    }

    /// Load the default file if it exists; otherwise an empty config.
    ///
    /// A malformed file is an **error**, not an empty config: silently starting
    /// with zero servers would look exactly like "no servers configured".
    pub fn load_default() -> Result<Option<Self>> {
        let path = Self::default_path();
        if !path.exists() {
            return Ok(None);
        }
        Self::from_path(&path).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stdio(id: &str) -> McpServerConfig {
        McpServerConfig::stdio(id, "npx")
    }

    #[test]
    fn a_minimal_stdio_server_validates() {
        stdio("filesystem").validate().unwrap();
    }

    #[test]
    fn an_id_with_the_namespace_separator_is_refused() {
        let err = stdio("bad__id").validate().unwrap_err();
        assert!(err.to_string().contains("reserved"), "{err}");
    }

    #[test]
    fn ids_are_lowercase_and_start_alphanumeric() {
        assert!(stdio("Ok").validate().is_err());
        assert!(stdio("-bad").validate().is_err());
        assert!(stdio("has space").validate().is_err());
        assert!(stdio(&"a".repeat(49)).validate().is_err());
        assert!(stdio("good-id_2").validate().is_ok());
    }

    #[test]
    fn stdio_requires_a_command_and_rejects_url() {
        let mut c = stdio("s");
        c.command = None;
        assert!(c.validate().unwrap_err().to_string().contains("command"));

        let mut c = stdio("s");
        c.command = Some("  ".into());
        assert!(c.validate().is_err());

        let mut c = stdio("s");
        c.url = Some("http://127.0.0.1/mcp".into());
        assert!(c.validate().unwrap_err().to_string().contains("not valid"));
    }

    #[test]
    fn cwd_traversal_is_refused() {
        let mut c = stdio("s");
        c.cwd = Some("../elsewhere".into());
        assert!(c.validate().unwrap_err().to_string().contains(".."));

        let mut c = stdio("s");
        c.cwd = Some("workspace/sub".into());
        assert!(c.validate().is_ok());
    }

    #[test]
    fn cleartext_http_is_loopback_only() {
        McpServerConfig::http("a", "http://127.0.0.1:8931/mcp")
            .validate()
            .unwrap();
        McpServerConfig::http("b", "http://localhost:8931/mcp")
            .validate()
            .unwrap();
        let err = McpServerConfig::http("c", "http://example.com/mcp")
            .validate()
            .unwrap_err();
        assert!(err.to_string().contains("loopback"), "{err}");
        McpServerConfig::http("d", "https://example.com/mcp")
            .validate()
            .unwrap();
    }

    #[test]
    fn http_rejects_stdio_fields() {
        let mut c = McpServerConfig::http("s", "https://example.com/mcp");
        c.command = Some("npx".into());
        assert!(c.validate().unwrap_err().to_string().contains("not valid"));
    }

    #[test]
    fn timeouts_and_restart_bounds_are_enforced() {
        let mut c = stdio("s");
        c.timeout_ms = 10;
        assert!(c.validate().is_err());

        let mut c = stdio("s");
        c.restart.max_attempts = 11;
        assert!(c.validate().is_err());

        let mut c = stdio("s");
        c.restart.max_backoff_ms = 100;
        assert!(c.validate().is_err());
    }

    #[test]
    fn backoff_doubles_and_is_capped() {
        let p = RestartPolicy {
            max_attempts: 5,
            initial_backoff_ms: 500,
            max_backoff_ms: 3000,
        };
        assert_eq!(p.backoff_for(1).as_millis(), 500);
        assert_eq!(p.backoff_for(2).as_millis(), 1000);
        assert_eq!(p.backoff_for(3).as_millis(), 2000);
        assert_eq!(p.backoff_for(4).as_millis(), 3000);
        assert_eq!(p.backoff_for(40).as_millis(), 3000);
    }

    #[test]
    fn duplicate_ids_are_refused_by_the_file() {
        let file = McpServersFile {
            version: CONFIG_VERSION,
            servers: vec![stdio("dup"), stdio("dup")],
        };
        assert!(file
            .validate()
            .unwrap_err()
            .to_string()
            .contains("duplicate"));
    }

    #[test]
    fn an_unknown_field_is_a_loud_error_not_an_ignored_typo() {
        let text = r#"{"version":1,"servers":[{"id":"s","transport":"stdio","comand":"npx"}]}"#;
        let err = McpServersFile::from_json(text).unwrap_err();
        assert!(err.to_string().contains("parsing"), "{err}");
    }

    #[test]
    fn a_wrong_schema_version_is_refused() {
        let text = r#"{"version":99,"servers":[]}"#;
        let err = McpServersFile::from_json(text).unwrap_err();
        assert!(err.to_string().contains("unsupported"), "{err}");
    }

    #[test]
    fn round_trips_through_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mcp.servers.json");
        let file = McpServersFile {
            version: CONFIG_VERSION,
            servers: vec![
                stdio("filesystem"),
                McpServerConfig::http("remote", "https://x.test/mcp"),
            ],
        };
        file.to_path(&path).unwrap();
        let read = McpServersFile::from_path(&path).unwrap();
        assert_eq!(read.servers.len(), 2);
        assert_eq!(read.servers[0].id, "filesystem");
        assert!(
            !path.with_extension("json.tmp").exists(),
            "temp file removed"
        );
    }

    #[test]
    fn disabled_servers_are_skipped_by_enabled() {
        let mut off = stdio("off");
        off.enabled = false;
        let file = McpServersFile {
            version: CONFIG_VERSION,
            servers: vec![stdio("on"), off],
        };
        let ids: Vec<_> = file.enabled().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["on"]);
    }
}
