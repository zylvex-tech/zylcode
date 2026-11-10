//! Quick offline check that the new composio modules compile and the Rust-side
//! wiring builds against a healthy workspace. This is not a full integration
//! test against Composio — that ships later — just the compile-level sanity
//! gate for the files added in this stage.

use std::path::PathBuf;

#[test]
fn composio_accounts_module_compiles_and_defaults() {
    let accounts = zylcode_desktop::composio_accounts::UserComposioAccounts::default();
    assert_eq!(accounts.schema, 1);
    assert!(accounts.accounts.is_empty());
    assert!(!accounts.updated_at.is_empty());
    let path = zylcode_desktop::composio_accounts::accounts_path();
    // Must live under the same config directory as providers.json.
    assert!(path.ends_with("composio_accounts.json"));
}

#[test]
fn composio_client_stub_constants_match_plan() {
    assert_eq!(
        zylcode_desktop::composio_client_stub::COMPOSIO_API_BASE,
        "https://api.composio.dev"
    );
    assert_eq!(
        zylcode_desktop::composio_client_stub::COMPOSIO_AUTH_URL,
        "https://auth.composio.dev"
    );
}
