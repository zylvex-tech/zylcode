//! Re-exports the new composio modules from the desktop crate so integration
//! tests in other crates can reach them without importing private desktop
//! internals. These are the compile-time surfaces the next stage will rely on
//! once a real Composio client lands.
//!
//! This file is intentionally minimal: it only re-exports types/constants the
//! rest of the workspace needs. The real client logic stays in
//! `composio_client_stub.rs` and `composio_accounts.rs` until the next stage.

pub use composio_accounts::{self, ComposioAccount, UserComposioAccounts};
pub use composio_client_stub::{
    self, COMPOSIO_API_BASE, COMPOSIO_AUTH_URL, discover_tools, execute_tool,
};
