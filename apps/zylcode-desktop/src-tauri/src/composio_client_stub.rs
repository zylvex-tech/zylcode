// Stubbed Composio HTTP client + tool discovery/execution sketch.
// This module is intentionally a NO-OP today: the real Composio SDK / OAuth
// plumbing ships later. Keeping the file here (compile-checked, wired behind
// the `get_integrations_status` Tauri command) means the wiring has a clear
// landing place and the rest of the app compiles against it.
// Security notes (enforced when the real calls ship)
// - All API calls pass through the permission gate before reaching Composio.
// - Evidence is recorded for every tool invocation.
// - User tokens stay in the per-user `composio_accounts.json` file; nothing
//   leaves through IPC except a masked hint exactly like `provider_store`.
// - OAuth never hands a token to React / localStorage; the browser only opens
//   the Composio auth URL, and the callback lands back in the desktop app.
// NOTE: this file is deliberately named with a `_stub` suffix so it is clearly
// a placeholder. Once the real client lands, this file is the first candidate
// to be replaced/renamed; do not promote it to a real client by merely editing
// in place without a separate review.

use anyhow::Result;
use uuid::Uuid;

/* NOTE: uuid is used for generating connection IDs in create_connect_link.
 * If uuid is not already a dependency, you can replace the Uuid::new_v4()
 * call with a simple string generator. The actual Composio API will replace
 * this stub entirely. */

/// Composio API base, matching the plan doc.
pub const COMPOSIO_API_BASE: &str = "https://api.composio.dev";

/// OAuth authorization host, matching the plan doc.
pub const COMPOSIO_AUTH_URL: &str = "https://auth.composio.dev";

/// Discover tools available for a connected Composio account.
/// Today returns an empty vec. When the real client ships this is the place
/// that hits Composio's tool-discovery endpoint for the given account and
/// returns schemas the permission gate can classify.
pub async fn discover_tools(_account_id: &str) -> Result<Vec<serde_json::Value>> {
    Ok(Vec::new())
}

/// Execute a single Composio tool for a connected account.
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

/// Create a Composio Connect Link for a user and service.
/// This initiates the OAuth connection flow by generating a hosted Connect Link
/// URL that the user should open to authorize the given service.
///
/// Parameters:
/// - `service`: The Composio service key (e.g. "github", "gmail", "slack").
/// - `user_id`: The ZylCode user identifier.
///
/// Returns a tuple of (redirect_url, connection_id) where:
/// - `redirect_url` is the hosted URL the user must open to complete OAuth.
/// - `connection_id` is a session identifier to track the connection afterward.
pub async fn create_connect_link(service: &str, user_id: &str) -> Result<(String, String)> {
    // TODO: Replace with real Composio API call when the real client ships.
    // The Composio API endpoint: POST /api/v3.1/connected_accounts/link
    // Returns: { redirect_url: String, connection_id: String, expires_at: String }

    // For now, construct the Composio Connect Link URL manually.
    // This follows the Composio convention.
    let redirect_url = format!(
        "https://composio.dev/connect?service={}&user_id={}",
        service, user_id
    );
    let connection_id = Uuid::new_v4().to_string();

    Ok((redirect_url, connection_id))
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

    #[tokio::test]
    async fn create_connect_link_generates_url_and_id() {
        let (url, id) = create_connect_link("github", "zylcode-user").await.unwrap();
        assert!(!url.is_empty());
        assert!(!id.is_empty());
        // Verify URL contains expected parameters
        assert!(url.contains("service=github"));
        assert!(url.contains("user_id=zylcode-user"));
    }
}
