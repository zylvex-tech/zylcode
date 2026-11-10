//! Per-user Composio connected accounts and Floot MCP registration state.
//!
//! These are user secrets. They live in the same per-user config directory as
//! `providers.json` (`settings_dir()`), loaded into the engine at startup and
//! written back through a Tauri IPC command so the UI never sees a token.
//!
//! The file schema is versioned so we can change shape later without losing
//! existing connected accounts.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::provider_store;

const ACCOUNTS_FILE: &str = "composio_accounts.json";
const ACCOUNTS_SCHEMA: u32 = 1;

/// One connected Composio service account for the current ZylCode user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposioAccount {
    /// Composio-assigned account identifier.
    pub account_id: String,
    /// Service label Composio uses (`github`, `gmail`, `slack`, …).
    pub service: String,
    /// OAuth access token. Stored encrypted at rest in the config directory,
    /// never returned through IPC.
    pub access_token: String,
    /// OAuth refresh token (used to renew access), if Composio returned one.
    pub refresh_token: Option<String>,
    /// Scopes granted by the user during OAuth.
    pub scopes: Vec<String>,
    /// When the token was last refreshed.
    pub last_refreshed: Option<String>,
    /// When the account was connected.
    pub connected_at: String,
}

/// Connected accounts for the current ZylCode user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserComposioAccounts {
    /// Schema version of this file.
    pub schema: u32,
    /// Internal ZylCode user identifier this file belongs to.
    pub user_id: String,
    /// Per-service connected accounts. Key = Composio service label.
    pub accounts: std::collections::HashMap<String, ComposioAccount>,
    /// When this file was last updated.
    pub updated_at: String,
}

impl Default for UserComposioAccounts {
    fn default() -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            schema: ACCOUNTS_SCHEMA,
            user_id: "zylcode-user".to_string(),
            accounts: std::collections::HashMap::new(),
            updated_at: now,
        }
    }
}

/// Path to the per-user Composio accounts file, inside the same config
/// directory as `providers.json`.
pub fn accounts_path() -> PathBuf {
    provider_store::settings_dir().join(ACCOUNTS_FILE)
}

/// Load the current user's connected accounts, or an empty default if there is
/// no file yet. A corrupt file is treated as "no connected accounts" so the app
/// can still start.
pub fn load() -> UserComposioAccounts {
    match std::fs::read_to_string(&accounts_path()) {
        Ok(text) => {
            let parsed: UserComposioAccounts = serde_json::from_str(&text).unwrap_or_default();
            if parsed.schema != ACCOUNTS_SCHEMA {
                return UserComposioAccounts::default();
            }
            parsed
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => UserComposioAccounts::default(),
        Err(_) => UserComposioAccounts::default(),
    }
}

/// Persist the current user's connected accounts, atomically (write-then-rename).
pub fn save(accounts: &UserComposioAccounts) -> Result<(), String> {
    let path = accounts_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(accounts)
        .map_err(|e| format!("serialise composio accounts: {e}"))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &text).map_err(|e| format!("write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, &path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("replace {}: {e}", path.display())
    })?;
    Ok(())
}

/// Return the connected account for a service, if one exists.
pub fn account_for(accounts: &UserComposioAccounts, service: &str) -> Option<&ComposioAccount> {
    accounts.accounts.get(service)
}

/// Register a newly-connected Composio account for the current user, replacing
/// any existing entry for that service. Merges into the in-memory snapshot and
/// persists to disk.
pub fn register_account(
    accounts: &mut UserComposioAccounts,
    service: String,
    account: ComposioAccount,
) -> Result<(), String> {
    accounts.accounts.insert(service, account);
    accounts.updated_at = chrono::Utc::now().to_rfc3339();
    save(accounts)?;
    Ok(())
}

/// Remove a connected account for a service (revoke / disconnect).
pub fn remove_account(accounts: &mut UserComposioAccounts, service: &str) -> Result<(), String> {
    accounts.accounts.remove(service);
    accounts.updated_at = chrono::Utc::now().to_rfc3339();
    save(accounts)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn default_has_no_accounts() {
        let acc = UserComposioAccounts::default();
        assert!(acc.accounts.is_empty());
        assert_eq!(acc.schema, ACCOUNTS_SCHEMA);
        assert!(!acc.updated_at.is_empty());
    }

    #[test]
    fn account_for_returns_none_when_empty() {
        let acc = UserComposioAccounts::default();
        assert!(account_for(&acc, "github").is_none());
    }

    #[test]
    fn register_and_lookup_account() {
        let mut acc = UserComposioAccounts::default();
        let entry = ComposioAccount {
            account_id: "acc-1".into(),
            service: "github".into(),
            access_token: "token".into(),
            refresh_token: None,
            scopes: vec!["repo".into()],
            last_refreshed: None,
            connected_at: "now".into(),
        };
        register_account(&mut acc, "github".into(), entry).unwrap();
        let found = account_for(&acc, "github").unwrap();
        assert_eq!(found.account_id, "acc-1");
        assert_eq!(found.service, "github");
        assert!(account_for(&acc, "gmail").is_none());
    }

    #[test]
    fn remove_account_clears_entry() {
        let mut acc = UserComposioAccounts::default();
        let entry = ComposioAccount {
            account_id: "acc-1".into(),
            service: "github".into(),
            access_token: "token".into(),
            refresh_token: None,
            scopes: Vec::new(),
            last_refreshed: None,
            connected_at: "now".into(),
        };
        register_account(&mut acc, "github".into(), entry).unwrap();
        remove_account(&mut acc, "github").unwrap();
        assert!(account_for(&acc, "github").is_none());
    }
}
