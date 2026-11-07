//! Durable provider configuration and credential plumbing.
//!
//! Two different kinds of state live here, and they have different rules:
//!
//! * **Settings** — which providers exist, their order, model, endpoint,
//!   timeout and enabled flag — are user choices. They belong in a file under
//!   the per-user config directory and must survive a restart.
//! * **Credentials** — API keys — are secrets. They go to the OS credential
//!   store ([`crate::secrets`]) and are never written to the settings file,
//!   never returned through IPC, and never logged.
//!
//! At startup the two are merged in one direction: the file supplies the
//! configuration, the store supplies keys, and the result becomes the live
//! router configuration that dispatch reads. Within that configuration a stored
//! key wins over the provider's environment variable (the variable is the
//! fallback), which is why the panel names the active source.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use zylcode_core::router::{ProviderKind, RouterConfig};

use crate::secrets;

const FILE_NAME: &str = "providers.json";
const SCHEMA: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct ProviderFile {
    schema: u32,
    config: RouterConfig,
}

/// What the settings surface is allowed to know about a credential.
///
/// Deliberately excludes the key itself. `hint` is four trailing characters and
/// only exists for store-held keys, so the operator can tell two keys apart
/// without either being rendered.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderKeyStatus {
    /// `ProviderKind` display name (`anthropic`, `custom`, …).
    pub kind: String,
    /// Dispatch provider name, i.e. the credential account name.
    pub provider: Option<String>,
    pub requires_key: bool,
    pub configured: bool,
    /// `credential_store` | `environment` | `config` | `none` | `not_required`
    pub source: String,
    /// Environment variable that could supply this key.
    pub env_var: Option<String>,
    /// Masked tail of a store-held key.
    pub hint: Option<String>,
    /// Present when this platform cannot store keys at all.
    pub store_error: Option<String>,
}

/// Where `providers.json` lives.
///
/// Resolved without a platform crate: `%APPDATA%\ZylCode` on Windows, then the
/// XDG location, then `~/.config/zylcode`. `ZYLCODE_CONFIG_DIR` overrides it,
/// which is what the tests and a portable install use.
pub fn settings_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("ZYLCODE_CONFIG_DIR") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir);
        }
    }
    if cfg!(windows) {
        if let Ok(appdata) = std::env::var("APPDATA") {
            if !appdata.trim().is_empty() {
                return PathBuf::from(appdata).join("ZylCode");
            }
        }
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.trim().is_empty() {
            return PathBuf::from(xdg).join("zylcode");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.trim().is_empty() {
            return PathBuf::from(home).join(".config").join("zylcode");
        }
    }
    PathBuf::from(".zylcode")
}

pub fn settings_path() -> PathBuf {
    settings_dir().join(FILE_NAME)
}

/// Load a persisted configuration, if there is a readable one.
///
/// A corrupt file is reported as `None` rather than as an error: the app has a
/// working default chain, and refusing to start because a settings file was
/// truncated would be a worse failure than starting with defaults.
pub fn load() -> Option<RouterConfig> {
    load_from(&settings_path())
        .map_err(|e| tracing::warn!(error = %e, "ignoring unreadable provider settings"))
        .ok()
        .flatten()
}

pub fn load_from(path: &Path) -> Result<Option<RouterConfig>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("read {}: {e}", path.display())),
    };
    let parsed: ProviderFile =
        serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;
    if parsed.schema != SCHEMA {
        return Err(format!(
            "{} has schema {}, this build understands {SCHEMA}",
            path.display(),
            parsed.schema
        ));
    }
    Ok(Some(parsed.config))
}

/// Persist a configuration, minus anything secret.
pub fn save(config: &RouterConfig) -> Result<(), String> {
    save_to(&settings_path(), config)
}

pub fn save_to(path: &Path, config: &RouterConfig) -> Result<(), String> {
    let mut stored = config.clone();
    // The file is user-readable; a key is not a setting. Keys live in the
    // credential store and are re-injected in memory at startup.
    stored.api_keys.clear();

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    let payload = serde_json::to_string_pretty(&ProviderFile {
        schema: SCHEMA,
        config: stored,
    })
    .map_err(|e| format!("serialise provider settings: {e}"))?;

    // Write-then-rename so an interrupted save cannot leave half a file behind
    // (which `load` would then have to ignore).
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, payload).map_err(|e| format!("write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("replace {}: {e}", path.display())
    })
}

/// Move stored keys into the in-memory configuration that dispatch reads.
///
/// Returns warnings for credentials that exist but could not be read. A missing
/// entry is not a warning: "no stored key" is the normal state, and the
/// environment may still supply one at request time.
pub fn inject_secrets(config: &mut RouterConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    for kind in ProviderKind::all() {
        if !kind.requires_api_key() {
            continue;
        }
        let Some(name) = kind.dispatch_name() else {
            continue;
        };
        match secrets::get(&name) {
            Ok(Some(secret)) => {
                config.api_keys.insert(name, secret);
            }
            Ok(None) => {}
            Err(secrets::StoreError::Unsupported) => {}
            Err(e) => warnings.push(format!("{name}: {e}")),
        }
    }
    warnings
}

/// Report where each provider's credential comes from, never what it is.
pub fn key_status(kind: &ProviderKind, config: &RouterConfig) -> ProviderKeyStatus {
    let provider = kind.dispatch_name();
    let env_var = kind.env_var().map(str::to_string);
    let mut store_error = None;

    let stored = match &provider {
        Some(name) => match secrets::get(name) {
            Ok(found) => found,
            Err(secrets::StoreError::Unsupported) => {
                store_error = Some(secrets::StoreError::Unsupported.to_string());
                None
            }
            Err(e) => {
                store_error = Some(e.to_string());
                None
            }
        },
        None => None,
    };

    let in_memory = provider
        .as_ref()
        .and_then(|name| config.api_keys.get(name))
        .filter(|key| !key.is_empty())
        .cloned();

    let env_present = env_var
        .as_ref()
        .and_then(|name| std::env::var(name).ok())
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);

    let (source, hint) = if !kind.requires_api_key() {
        ("not_required".to_string(), None)
    } else if let Some(secret) = stored {
        ("credential_store".to_string(), Some(secrets::hint(&secret)))
    } else if let Some(secret) = in_memory {
        ("credential_store".to_string(), Some(secrets::hint(&secret)))
    } else if env_present {
        ("environment".to_string(), None)
    } else {
        ("none".to_string(), None)
    };

    ProviderKeyStatus {
        kind: kind.to_string(),
        provider,
        requires_key: kind.requires_api_key(),
        configured: source != "none",
        source,
        env_var,
        hint,
        store_error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zylcode_core::router::{ModelProvider, ProviderConfig};

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "zylcode-provider-store-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn config_with_key() -> RouterConfig {
        let mut cfg = RouterConfig {
            primary_provider: ModelProvider::Anthropic,
            ..RouterConfig::default()
        };
        cfg.api_keys
            .insert("anthropic".to_string(), "sk-ant-secret-value".to_string());
        cfg
    }

    #[test]
    fn settings_round_trip_through_the_file() {
        let dir = temp_dir("roundtrip");
        let path = dir.join("providers.json");
        let mut cfg = config_with_key();
        cfg.provider_configs[0].model = "claude-sonnet-4-5".to_string();
        cfg.provider_configs[1].enabled = false;
        cfg.provider_configs[0].fallback_order = 4;

        save_to(&path, &cfg).expect("save");
        let loaded = load_from(&path).expect("load").expect("present");

        assert_eq!(loaded.provider_configs[0].model, "claude-sonnet-4-5");
        assert!(!loaded.provider_configs[1].enabled);
        assert_eq!(loaded.provider_configs[0].fallback_order, 4);
    }

    #[test]
    fn the_settings_file_never_contains_a_key() {
        let dir = temp_dir("nokey");
        let path = dir.join("providers.json");
        save_to(&path, &config_with_key()).expect("save");
        let text = std::fs::read_to_string(&path).expect("read");
        assert!(
            !text.contains("sk-ant-secret-value"),
            "a key must not be written to the settings file"
        );
        assert!(
            !text.contains("\"api_keys\": {\n    \"anthropic\""),
            "{text}"
        );
        let loaded = load_from(&path).expect("load").expect("present");
        assert!(
            loaded.api_keys.is_empty(),
            "keys are not restorable from the file"
        );
    }

    #[test]
    fn a_missing_file_is_not_an_error() {
        let dir = temp_dir("missing");
        assert!(load_from(&dir.join("nope.json"))
            .expect("no error")
            .is_none());
    }

    #[test]
    fn a_corrupt_file_is_rejected_rather_than_partially_applied() {
        let dir = temp_dir("corrupt");
        let path = dir.join("providers.json");
        std::fs::write(&path, "{\"schema\":1,\"config\":{\"prim").unwrap();
        assert!(load_from(&path).is_err());
    }

    #[test]
    fn key_status_reports_a_source_and_never_a_key() {
        let mut cfg = config_with_key();
        let status = key_status(&ProviderKind::Anthropic, &cfg);
        assert_eq!(status.kind, "anthropic");
        assert!(status.requires_key);
        assert!(status.configured);
        assert_eq!(status.source, "credential_store");
        let rendered = serde_json::to_string(&status).unwrap();
        assert!(!rendered.contains("sk-ant-secret-value"), "{rendered}");
        assert!(
            !rendered.contains("sk-ant"),
            "hint must be masked: {rendered}"
        );

        // Ollama needs no credential and must not claim one is missing.
        cfg.api_keys.clear();
        let ollama = key_status(&ProviderKind::Ollama, &cfg);
        assert_eq!(ollama.source, "not_required");
        assert!(ollama.configured);
    }

    #[test]
    fn custom_provider_reports_the_fields_it_requires() {
        let cfg = RouterConfig::default();
        let custom = cfg
            .provider_configs
            .iter()
            .find(|c: &&ProviderConfig| c.kind == ProviderKind::Custom)
            .expect("custom entry exists");
        assert!(!custom.enabled, "a custom endpoint is opt-in");
        assert!(custom.endpoint.is_empty());
        assert!(custom.model.is_empty());
    }
}
