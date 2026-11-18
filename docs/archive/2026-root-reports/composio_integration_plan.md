# Composio Integration Plan — ZylCode AI Software Creation OS

## 1. ARCHITECTURE OVERVIEW

### User Identity Layer

```text
ZylCode User (stable internal ID)
    |
    +----------------------+----------------------+
    |                      |                      |
   user_id                composio_user_id        (mapped from OAuth)
    |                      |
    |                      v
    |              Connected Accounts
    |              GitHub, Gmail, Slack, Linear,
    |              Google Drive, Jira, Notion, etc.
    |
    +----------------------+----------------------+
                Account Isolation
    User A must never receive User B's
    connected account or tool access.
```

### Composio Integration Flow

```text
User clicks "Connect GitHub" in Settings
    ↓
OAuth redirect → Composio API
    ↓
Composio returns: access_token, refresh_token, account_id, scopes
    ↓
Store per-user: {service: "github", user_id: "...", tokens: {...}, scopes: [...]}
    ↓
AI can now request: "List my repositories"
    ↓
Tool router looks up: "github" → connected account → compose tool call
    ↓
Permission gate: risk=GitRead → decision based on policy
    ↓
If allowed: execute via Composio API
    ↓
Evidence ledger records: {tool_id: "github.list_repos", decision, result}
    ↓
Result returned to model → continuation
```

### Security Model Preservation

Composio must NOT bypass:

- ✅ Approval gate (PermissionGate.decide())
- ✅ Permission classification (RiskLevel: Read/Write/Execute/Network/Destructive)
- ✅ Workspace boundaries (working_directory, environment)
- ✅ Evidence logging (ToolEvidence::approval_decision + execution evidence)
- ✅ User identity (per-user tokens, never shared)
- ✅ Auditability (every call records who, what, when, decision)
- ✅ Destructive-action controls (highest risk class requires explicit allow)

### Tool Discovery (Dynamic, Not Eager)

```text
DO NOT load 1000+ tools into AI context.

AI reasons: "I need to update a GitHub issue."
    ↓
Tool router: "github" service → need list_repos or search_repos
    ↓
Composio: discover tools for connected GitHub account
    ↓
Retrieve schema (dynamic, per-account)
    ↓
Request authorization if needed (OAuth scope check)
    ↓
Execute single tool (not all tools at once)
    ↓
Return result to model
```

### Per-User Identity

```text
Do NOT use:
  - email as primary identity
  - mutable display name
  - temporary session IDs

Define:
  ZylCode user_id → mapped to Composio user_id
  Each user has their own connected accounts
  Account isolation: User A ≠ User B's access
```

## 2. IMPLEMENTATION COMPONENTS

### 2.1 Composio Client Library

New file: `crates/zylcode-mcp/src/composio_client.rs`

```rust
//! Composio external integration client.
//! 
//! Wraps the Composio HTTP API for tool discovery, authentication, and execution.
//! 
//! # Security
//! - All API calls pass through the PermissionGate
//! - Evidence is logged via the runtime evidence sink
//! - User tokens are per-user, never committed or logged
//! - OAuth flow handled through loopback redirect

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Composio API base endpoints
pub const COMPOSIO_API_BASE: &str = "https://api.composio.dev";

/// OAuth authorization URL
pub const COMPOSIO_AUTH_URL: &str = "https://auth.composio.dev";

/// One connected service account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposioAccount {
    /// Composio-assigned account identifier
    pub account_id: String,
    /// Service name (github, gmail, slack, linear, etc.)
    pub service: String,
    /// OAuth access token (encrypted at rest)
    pub access_token: String,
    /// OAuth refresh token (used to renew access)
    pub refresh_token: Option<String>,
    /// Scopes granted by the user
    pub scopes: Vec<String>,
    /// When the token was last refreshed
    pub last_refreshed: Option<String>,
    /// When the account was connected
    pub connected_at: String,
}

/// Connected account for a user
#[derive(Debug, Clone)]
pub struct UserComposioAccount {
    /// Internal ZylCode user identifier
    pub user_id: String,
    /// Composio account
    pub account: ComposioAccount,
}

/// Tool schema from Composio (dynamic, per-account)
#[derive(Debug, Clone)]
pub struct ComposioToolSchema {
    /// Tool/operation name
    pub name: String,
    /// JSON schema for parameters
    pub input_schema: serde_json::Value,
    /// Risk level of the operation
    pub risk_level: RiskLevel,
    /// Bound operation (what this tool actually does)
    pub bound_operation: Option<String>,
    /// Required OAuth scopes
    pub required_scopes: Vec<String>,
}

/// Discover tools for a connected account
pub async fn discover_tools(
    account: &ComposioAccount,
) -> Result<Vec<ComposioToolSchema>> {
    // Call Composio's tool discovery endpoint
    // Returns only tools the account can access
    // Dynamically loaded, not all at once
}

/// Execute a Composio tool
pub async fn execute_tool(
    account: &ComposioAccount,
    tool_name: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    // Build the API call to Composio
    // Pass through the same PermissionGate + evidence sink
    // Return result or permission denial
}
```

### 2.2 User Identity Management

New file: `crates/zylcode-core/src/user_identity.rs`

```rust
//! Stable user identity for per-user integrations.
//! 
//! Maps ZylCode internal user_id to external service identities.
//! 
//! # Design
//! - user_id is a stable, persistent identifier
//! - Maps to Composio user_id via OAuth completion
//! - Account isolation: each user has their own connected accounts
//! - Persisted in user preferences, never in chat/logs

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Stable ZylCode user identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserId(pub String);

impl UserId {
    /// Create a new user ID from a stable source
    /// 
    /// In production this might derive from the auth system.
    /// For now, it's generated from the session.
    pub fn new() -> Self {
        // Use a hash of the machine + session info for stability
        // In production: derive from authenticated user session
        Self("zylcode-user".to_string()) 
    }
    
    /// Get the inner string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// User's connected accounts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConnectedAccounts {
    /// User's stable ID
    pub user_id: UserId,
    /// Per-service connected accounts
    pub accounts: HashMap<String, ComposioAccount>,
    /// When this was last updated
    pub updated_at: String,
}

/// Load user's connected accounts from persistent storage
pub fn load_user_accounts(
    base_dir: &PathBuf,
) -> anyhow::Result<UserConnectedAccounts> {
    let path = base_dir.join("user_composio_accounts.json");
    if !path.exists() {
        return Ok(UserConnectedAccounts {
            user_id: UserId::new(),
            accounts: HashMap::new(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        });
    }
    let text = std::fs::read_to_string(&path)?;
    serde_json::from_str(&text).context("parsing user composio accounts")
}

/// Save user's connected accounts to persistent storage
pub fn save_user_accounts(
    base_dir: &PathBuf,
    accounts: &UserConnectedAccounts,
) -> anyhow::Result<()> {
    let path = base_dir.join("user_composio_accounts.json");
    // Atomic write: temp file + rename
    let text = serde_json::to_string_pretty(accounts)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &text)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}
```

### 2.3 Settings → Integrations UI

Modify: `apps/zylcode-desktop/src/components/SettingsSurface.tsx` (or create new component)

Add Integrations section with connected accounts cards:

```tsx
// Settings → Integrations panel
<IntegrationsPanel>
  <ConnectCard
    service="github"
    name="GitHub"
    icon="github"
    descriptio
