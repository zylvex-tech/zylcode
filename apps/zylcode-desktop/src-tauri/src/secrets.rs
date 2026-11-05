//! Provider API keys in the OS credential store.
//!
//! A key is a secret, not a setting: writing it to a JSON file next to the
//! workspace would leave it in any backup, sync folder or screenshot of the
//! directory. The durable home is the platform's credential vault — Windows
//! Credential Manager today, with the same three-call interface that Keychain
//! (`SecItemAdd`/`SecItemCopyMatching`) and Secret Service
//! (`org.freedesktop.Secret`) expose, so those can be added without the caller
//! changing.
//!
//! Nothing here returns a key to a caller that would render it: the only
//! consumer is [`crate::provider_store`], which moves a key into the in-memory
//! router configuration and reports a masked hint upwards.
//!
//! Environment variables remain a first-class source. Resolution order is
//! config (where stored keys are injected) and then the environment, so a key
//! saved here is used first and the variable is the fallback; a missing entry
//! simply means "no stored key" and leaves the variable to supply one.

use std::fmt;

/// Account namespace inside the vault. One account per dispatch provider name
/// (`zylcode/provider/anthropic`), which is the same string used as the key in
/// `RouterConfig::api_keys`.
pub const ACCOUNT_PREFIX: &str = "zylcode/provider";

/// Largest secret the vault accepts for a generic credential (Win32 limit is
/// 5 * 512 bytes; keys are far smaller, but the bound is enforced here so a
/// pasted file cannot be written as an "API key").
const MAX_SECRET_BYTES: usize = 2048;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    /// This build has no implementation for the host platform.
    Unsupported,
    /// The operating system refused the operation.
    Os(String),
    /// The name or secret failed validation before reaching the OS.
    Invalid(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => write!(
                f,
                "no credential store implementation for this platform yet — \
                 set the provider's environment variable instead"
            ),
            Self::Os(msg) => write!(f, "credential store error: {msg}"),
            Self::Invalid(msg) => write!(f, "invalid credential request: {msg}"),
        }
    }
}

impl std::error::Error for StoreError {}

/// Whether this build can store keys durably.
pub fn is_supported() -> bool {
    cfg!(windows)
}

fn account(provider_name: &str) -> Result<String, StoreError> {
    let valid = !provider_name.is_empty()
        && provider_name.len() <= 64
        && provider_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid {
        return Err(StoreError::Invalid(format!(
            "provider name {provider_name:?} must be 1..=64 chars of [a-z0-9_-]"
        )));
    }
    Ok(format!("{ACCOUNT_PREFIX}/{provider_name}"))
}

fn validate_secret(secret: &str) -> Result<&[u8], StoreError> {
    let bytes = secret.as_bytes();
    if bytes.is_empty() {
        return Err(StoreError::Invalid("secret is empty".to_string()));
    }
    if bytes.len() > MAX_SECRET_BYTES {
        return Err(StoreError::Invalid(format!(
            "secret is {} bytes, limit is {MAX_SECRET_BYTES}",
            bytes.len()
        )));
    }
    if secret.chars().any(|c| c.is_control()) {
        return Err(StoreError::Invalid(
            "secret contains a control character (paste a single line)".to_string(),
        ));
    }
    Ok(bytes)
}

/// Store (or replace) a provider's key.
pub fn set(provider_name: &str, secret: &str) -> Result<(), StoreError> {
    let account = account(provider_name)?;
    let bytes = validate_secret(secret)?;
    platform::set(&account, bytes)
}

/// Read a provider's key, if the vault holds one.
pub fn get(provider_name: &str) -> Result<Option<String>, StoreError> {
    let account = account(provider_name)?;
    platform::get(&account)
}

/// Remove a provider's key. `Ok(false)` means there was nothing to remove.
pub fn delete(provider_name: &str) -> Result<bool, StoreError> {
    let account = account(provider_name)?;
    platform::delete(&account)
}

/// Last four characters of a key, for "which key is this?" without showing it.
///
/// Only used for keys the vault holds; an environment key is never read for
/// display, because the operator already knows where they exported it.
pub fn hint(secret: &str) -> String {
    let tail: String = secret
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if secret.chars().count() <= 4 {
        "…".to_string()
    } else {
        format!("…{tail}")
    }
}

#[cfg(windows)]
mod platform {
    //! Windows Credential Manager via `advapi32`.
    //!
    //! Written against the documented Win32 ABI rather than a crate: the three
    //! calls needed (`CredWriteW`, `CredReadW`, `CredDeleteW`) are stable, and a
    //! new dependency would have to be vendored into a workspace that builds
    //! offline. `CREDENTIALW` is `#[repr(C)]` and matches the SDK layout field
    //! for field.

    use super::StoreError;
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::{null, null_mut};

    const CRED_TYPE_GENERIC: u32 = 1;
    /// Survive a reboot; the credential follows the Windows user, not the
    /// process, so it can never end up in the repository.
    const CRED_PERSIST_LOCAL_MACHINE: u32 = 2;
    const ERROR_NOT_FOUND: i32 = 1168;

    #[repr(C)]
    struct FileTime {
        low_date_time: u32,
        high_date_time: u32,
    }

    #[repr(C)]
    struct CredentialW {
        flags: u32,
        type_: u32,
        target_name: *mut u16,
        comment: *mut u16,
        last_written: FileTime,
        credential_blob_size: u32,
        credential_blob: *mut u8,
        persist: u32,
        attribute_count: u32,
        attributes: *mut c_void,
        target_alias: *mut u16,
        user_name: *mut u16,
    }

    #[link(name = "advapi32")]
    extern "system" {
        fn CredWriteW(credential: *const CredentialW, flags: u32) -> i32;
        fn CredReadW(
            target: *const u16,
            type_: u32,
            flags: u32,
            credential: *mut *mut CredentialW,
        ) -> i32;
        fn CredDeleteW(target: *const u16, type_: u32, flags: u32) -> i32;
        fn CredFree(buffer: *mut c_void);
    }

    fn wide(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    pub fn set(account: &str, secret: &[u8]) -> Result<(), StoreError> {
        let mut target = wide(account);
        let mut user = wide("zylcode");
        let mut blob = secret.to_vec();
        let credential = CredentialW {
            flags: 0,
            type_: CRED_TYPE_GENERIC,
            target_name: target.as_mut_ptr(),
            comment: null_mut(),
            last_written: FileTime {
                low_date_time: 0,
                high_date_time: 0,
            },
            credential_blob_size: blob.len() as u32,
            credential_blob: blob.as_mut_ptr(),
            persist: CRED_PERSIST_LOCAL_MACHINE,
            attribute_count: 0,
            attributes: null_mut(),
            target_alias: null_mut(),
            user_name: user.as_mut_ptr(),
        };
        // SAFETY: every pointer in `credential` refers to a live local buffer
        // for the duration of the call, and the sizes describe those buffers.
        let ok = unsafe { CredWriteW(&credential, 0) };
        if ok == 0 {
            return Err(StoreError::Os(std::io::Error::last_os_error().to_string()));
        }
        Ok(())
    }

    pub fn get(account: &str) -> Result<Option<String>, StoreError> {
        let target = wide(account);
        let mut out: *mut CredentialW = null_mut();
        // SAFETY: `target` is a NUL-terminated UTF-16 buffer; `out` receives an
        // allocation owned by the credential API, freed below with CredFree.
        let ok = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut out) };
        if ok == 0 {
            let err = std::io::Error::last_os_error();
            return if err.raw_os_error() == Some(ERROR_NOT_FOUND) {
                Ok(None)
            } else {
                Err(StoreError::Os(err.to_string()))
            };
        }
        if out.is_null() {
            return Ok(None);
        }
        // SAFETY: a successful CredReadW returns a non-null pointer to a valid
        // CREDENTIALW whose blob lives until CredFree.
        let secret = unsafe {
            let credential = &*out;
            if credential.credential_blob.is_null() || credential.credential_blob_size == 0 {
                None
            } else {
                let bytes = std::slice::from_raw_parts(
                    credential.credential_blob,
                    credential.credential_blob_size as usize,
                );
                Some(String::from_utf8_lossy(bytes).trim().to_string())
            }
        };
        // SAFETY: the pointer came from CredReadW and is freed exactly once.
        unsafe { CredFree(out as *mut c_void) };
        Ok(secret.filter(|s| !s.is_empty()))
    }

    pub fn delete(account: &str) -> Result<bool, StoreError> {
        let target = wide(account);
        // SAFETY: NUL-terminated UTF-16 buffer, no out-parameters.
        let ok = unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) };
        if ok != 0 {
            return Ok(true);
        }
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(ERROR_NOT_FOUND) {
            Ok(false)
        } else {
            Err(StoreError::Os(err.to_string()))
        }
    }

    // Keep the unused-import check honest on Windows.
    #[allow(dead_code)]
    fn _assert_null_is_used() {
        let _ = null::<u16>();
    }
}

#[cfg(not(windows))]
mod platform {
    use super::StoreError;

    pub fn set(_account: &str, _secret: &[u8]) -> Result<(), StoreError> {
        Err(StoreError::Unsupported)
    }

    pub fn get(_account: &str) -> Result<Option<String>, StoreError> {
        Err(StoreError::Unsupported)
    }

    pub fn delete(_account: &str) -> Result<bool, StoreError> {
        Err(StoreError::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_names_that_could_escape_the_namespace() {
        for bad in ["", "../etc", "with space", "slash/name", "nul\0byte"] {
            assert!(
                matches!(account(bad), Err(StoreError::Invalid(_))),
                "{bad:?} must be rejected"
            );
        }
        assert_eq!(
            account("openai-compatible").unwrap(),
            "zylcode/provider/openai-compatible"
        );
    }

    #[test]
    fn rejects_empty_multiline_and_oversized_secrets() {
        assert!(validate_secret("").is_err());
        assert!(validate_secret("line1\nline2").is_err());
        assert!(validate_secret(&"k".repeat(MAX_SECRET_BYTES + 1)).is_err());
        assert!(validate_secret("sk-abc123").is_ok());
    }

    #[test]
    fn hint_never_reveals_more_than_four_trailing_characters() {
        assert_eq!(hint("sk-ant-api03-SECRETVALUE"), "…ALUE");
        assert_eq!(hint("abcd"), "…");
        assert_eq!(hint("a"), "…");
    }
}
