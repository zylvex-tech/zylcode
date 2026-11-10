//! Composio external integration support.
//!
//! Wraps the Composio HTTP API for tool discovery, authentication, and
//! external service connection status.  This module is thin: the real work
//! (actual API calls, OAuth tokens) happens in the web layer through the
//! MCP client transport.  Here we publish the types the desktop UI and the
//! engine need to render an integrations panel and, later, route tool calls
//! through Composio.
//!
//! # Ownership
//!
//! * Types in this module are serializable for IPC.
//! * No tokens are stored here — the credential store owns them.
//! * Every Composio-backed call still passes through [`crate::permission`].

use serde::{Deserialize, Serialize};

/// Services the desktop integrations panel currently knows about.
///
/// Keeping this list here (not in the UI) means the backend can extend it
/// without a frontend release: new entries appear as "unsupported" until the
/// corresponding MCP bridge/infra ships.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComposioService {
    GitHub,
    Gmail,
    Slack,
    Linear,
    GoogleDrive,
    Jira,
    Notion,
}

impl ComposioService {
    pub fn label(&self) -> &'static str {
        match self {
            ComposioService::GitHub => "GitHub",
            ComposioService::Gmail => "Gmail",
            ComposioService::Slack => "Slack",
            ComposioService::Linear => "Linear",
            ComposioService::GoogleDrive => "Google Drive",
            ComposioService::Jira => "Jira",
            ComposioService::Notion => "Notion",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ComposioService::GitHub => "💻",
            ComposioService::Gmail => "📧",
            ComposioService::Slack => "💬",
            ComposioService::Linear => "📋",
            ComposioService::GoogleDrive => "☁️",
            ComposioService::Jira => "🗂️",
            ComposioService::Notion => "📝",
        }
    }
}

/// Status of one external integration for the current user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationStatus {
    /// Service this entry describes.
    pub service: ComposioService,
    /// Whether the current user has connected this service.
    pub connected: bool,
    /// Human-readable status if not connected (e.g. "not setup", "token expired").
    pub status: String,
    /// When the connection was established / last refreshed, if connected.
    pub connected_at: Option<String>,
}

/// Snapshot of all known integrations for the current user.
///
/// Returned by the desktop IPC command `get_integrations_status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationsSnapshot {
    pub services: Vec<IntegrationStatus>,
    /// Whether the Composio backend is reachable at all from this desktop.
    pub composio_reachable: bool,
    /// Whether Floot MCP is currently registered as a bridge.
    pub floot_registered: bool,
}

impl Default for IntegrationsSnapshot {
    fn default() -> Self {
        Self {
            services: vec![
                IntegrationStatus {
                    service: ComposioService::GitHub,
                    connected: false,
                    status: "not_setup".to_string(),
                    connected_at: None,
                },
                IntegrationStatus {
                    service: ComposioService::Gmail,
                    connected: false,
                    status: "not_setup".to_string(),
                    connected_at: None,
                },
                IntegrationStatus {
                    service: ComposioService::Slack,
                    connected: false,
                    status: "not_setup".to_string(),
                    connected_at: None,
                },
                IntegrationStatus {
                    service: ComposioService::Linear,
                    connected: false,
                    status: "not_setup".to_string(),
                    connected_at: None,
                },
                IntegrationStatus {
                    service: ComposioService::GoogleDrive,
                    connected: false,
                    status: "not_setup".to_string(),
                    connected_at: None,
                },
                IntegrationStatus {
                    service: ComposioService::Jira,
                    connected: false,
                    status: "not_setup".to_string(),
                    connected_at: None,
                },
                IntegrationStatus {
                    service: ComposioService::Notion,
                    connected: false,
                    status: "not_setup".to_string(),
                    connected_at: None,
                },
            ],
            composio_reachable: false,
            floot_registered: false,
        }
    }
}

/// Build the integrations snapshot from whatever the engine currently knows.
///
/// Today that is: "no integrations connected".  When the MCP bridges and OAuth
/// plumbing ship, this function is the single place to fold real state in
/// without touching the UI.
pub fn build_snapshot(floot_registered: bool) -> IntegrationsSnapshot {
    let mut snap = IntegrationsSnapshot::default();
    snap.floot_registered = floot_registered;
    snap
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_snapshot_lists_all_known_services() {
        let snap = IntegrationsSnapshot::default();
        assert_eq!(snap.services.len(), 7);
        assert!(!snap.services.iter().any(|s| s.connected));
        assert!(!snap.floot_registered);
    }

    #[test]
    fn service_labels_are_stable() {
        assert_eq!(ComposioService::GitHub.label(), "GitHub");
        assert_eq!(ComposioService::GoogleDrive.label(), "Google Drive");
        assert_eq!(ComposioService::Notion.label(), "Notion");
    }

    #[test]
    fn snapshot_can_mark_floot_registered() {
        let mut snap = IntegrationsSnapshot::default();
        snap.floot_registered = true;
        assert!(snap.floot_registered);
    }
}
