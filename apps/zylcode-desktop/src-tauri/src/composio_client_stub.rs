// Stubbed Composio HTTP client + tool discovery/execution sketch.
//
// This module is intentionally a NO-OP today: the real Composio SDK / OAuth
// plumbing ships later. Keeping the file here (compile-checked, wired behind
// the `get_integrations_status` Tauri command) means the wiring has a clear
// landing place and the rest of the app compiles against it.
//
// # Security notes (enforced when the real calls ship)
// - All API calls pass through the permission gate before reaching Composio.
// - Evidence is recorded for every tool invocation.
// - User tokens stay in the per-user `composio_accounts.json` file; nothing
//   leaves through IPC except a masked hint exactly like `provider_store`.
// - OAuth never hands a token to React / localStorage; the browser only opens
//   the Composio auth URL, and the callback lands back in the desktop app.
//
// NOTE: this file is deliberately named with a `_stub` suffix so it is clearly
// a placeholder. Once the real client lands, this file is the first candidate
// to be replaced/renamed; do not promote it to a real client by merely editing
// in place without a separate review.

use anyhow::Result;

/// Composio API base, matching the plan doc.
pub const COMPOSIO_API_BASE: &str = "https://api.composio.dev";

/// OAuth authorization host, matching the plan doc.
pub const COMPOSIO_AUTH_URL: &str = "https://auth.composio.dev";

/// Discover tools available for a connected Composio account.
///
/// Today returns an empty vec. When the real client ships this is the place
/// that hits Composio's tool-discovery endpoint for the given account and
/// returns schemas the permission gate can classify.
pub async fn discover_tools(_account_id: &str) -> Result<Vec<serde_json::Value>> {
    Ok(Vec::new())
}

/// Execute a single Composio tool for a connected account.
///
/// Parameters:
/// - `account_id`: Composio account to act through.
/// - `tool_name`: Composio operation identifier.
/// - `params`: JSON parameters for the operation.
///
/// Today returns an error indicating the feature is not wired yet. When wired,
/// this is the place that:
/// 1. Looks up the per-user account (never trusts a caller-supplied token).
/// 2. Passes the requested operation through the permission gate.
/// 3. Records evidence.
/// 4. Calls Composio and returns the result.
pub async fn execute_tool(
    _account_id: &str,
    _tool_name: &str,
    _params: serde_json::Value,
) -> Result<serde_json::Value> {
    anyhow::bail!("composio execution not wired yet")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_plan_values() {
        // Keep these in sync with composio_integration_plan.md.
        assert_eq!(COMPOSIO_API_BASE, "https://api.composio.dev");
        assert_eq!(COMPOSIO_AUTH_URL, "https://auth.composio.dev");
    }

    #[tokio::test]
    async fn discover_tools_returns_empty_vec_today() {
        let tools = discover_tools("ignored").await.unwrap();
        assert!(tools.is_empty());
    }
}
