//! Input validation for the two untrusted entry points into the engine.
//!
//! ZylCode takes input from places it does not control: 37 Tauri IPC commands
//! invoked by the renderer, and the `serve-intel` HTTP surface that the
//! browser preview (and anyone who can reach the port) can POST to. Before
//! this module existed both boundaries accepted whatever serde produced —
//! a non-string `prompt` silently became an empty string, an oversized body
//! was only bounded by axum's default 2 MiB limit, and a caller had no way to
//! tell "you sent that wrong" apart from "the engine broke".
//!
//! The design rules here:
//!
//! * **Reject before work starts.** Every check runs ahead of engine
//!   construction, so a malformed request costs a length comparison rather
//!   than an index build.
//! * **A typed error, never a panic and never a silent default.** Callers get
//!   [`InputError`] with a stable machine-readable [`InputError::code`] and a
//!   human [`std::fmt::Display`], so both the HTTP JSON and the desktop error
//!   string can report the same thing.
//! * **Limits are named constants, not literals at the call sites.** Each one
//!   is documented with why it is that number.
//! * **Two untrusted entry points share this boundary.** The Tauri IPC layer
//!   and the `serve-intel` HTTP surface both POST untrusted JSON here, so a
//!   malformed request must be rejected identically on both. `Url` parses an
//!   endpoint field; `Value` owns the structured-argument tree check. Keeping
//!   the imports narrow like this keeps the dependency surface small — the
//!   module documents its contract, and the rest of the crate imports
//!   `zylcode_core::input_validation` rather than reaching into the detail.
//!
//! ## Commands that reach the operating system
//!
//! Most commands take text and hand it to the engine. The four below go further
//! — they spawn a child process, write a file whose name they derive from
//! input, open a socket, or scan the extension registry — so each one is gated
//! here *before* the work starts, with a bound derived from what that command is
//! actually for rather than from one shared default.
//!
//! | Command | Reaches | Validators | Bounds |
//! | --- | --- | --- | --- |
//! | `terminal_exec` | spawns a shell child process | [`validate_terminal_request_in`] (command, cwd, shell, timeout, session id) | 4 096-char command with no NUL/escape controls; `cwd` confined to the workspace; 1 s – 24 h timeout; 128-char session id |
//! | `package_start` | names the release artifact it writes | [`validate_release_version`] | 128 chars; no path separator or `..`; portable version charset |
//! | `register_mcp_bridge` | opens a transport to an endpoint | [`validate_tool_id`], [`validate_endpoint`], [`validate_transport`] | 128-char id, 512-char URL, `stdio`/`sse`/`websocket` |
//! | `marketplace_search` | scans the extension registry | [`validate_marketplace_query`] | 1 024 chars |
//! | `test_provider_connection` | sends a real request to a model provider | [`validate_provider_endpoint`], [`validate_model`] | 512-char http(s) URL with no embedded credentials; 256-char model id; 1 s – 300 s timeout |
//!
//! Confinement for `terminal_exec` lives in [`validate_terminal_cwd`]: the
//! command itself is a shell command line by contract, so the property with
//! security weight is the directory the child starts in.

use crate::terminal::{ShellMode, TerminalRequest, DEFAULT_TIMEOUT_SECS};
use serde_json::Value;
use std::path::{Component, Path, PathBuf};

// ---------------------------------------------------------------------------
// Limits
// ---------------------------------------------------------------------------

pub const MAX_PROMPT_CHARS: usize = 32_768;

/// 16 384 characters for a queued mission task — the same order of magnitude
/// as a prompt, bounded separately so one limit can be tuned without moving
/// the other.
pub const MAX_TASK_CHARS: usize = 16_384;

/// 1 024 characters for a search query. A search box has no use for more,
/// and query length multiplies directly into index work.
pub const MAX_QUERY_CHARS: usize = 1_024;

/// 512 characters for a URL endpoint.
pub const MAX_URL_CHARS: usize = 512;

/// 256 characters for a model identifier such as
/// `anthropic/claude-3.5-sonnet`. Real identifiers are far shorter; the
/// bound exists so an identifier cannot be used as an injection channel.
pub const MAX_MODEL_ID_CHARS: usize = 256;

/// 128 characters for a correlation id.
pub const MAX_CORRELATION_ID_CHARS: usize = 128;

/// 128 characters for an MCP tool id.
pub const MAX_TOOL_ID_CHARS: usize = 128;

/// Validate a workspace-relative path.
///
/// At 4 096 characters this sits above every supported filesystem's
/// maximum path length while remaining far below any allocation bound the
/// engine would otherwise reach. The comment below the constant keeps the
/// rationale on the definition line it explains.
pub const MAX_PATH_CHARS: usize = 4_096;

/// 4 096 characters for an artifact label (the editor's display name).
pub const MAX_LABEL_CHARS: usize = 4_096;

/// 4 MiB for artifact content or a patch body. A workspace file the editor
/// can plausibly hold open is well under this; above it the save path is
/// being used as a bulk transfer, which is not what it is for.
pub const MAX_ARTIFACT_BYTES: usize = 4 * 1024 * 1024;

/// 64 KiB for a structured tool-argument object.
pub const MAX_ARGS_BYTES: usize = 64 * 1024;

/// 4 096 characters for a terminal command. This is far above any shell
/// command the engine will honestly run and below anything that can turn the
/// terminal into a storage path for arbitrary content. The limit is on the
/// length of the command string, not on what the child process reads from
/// its own stdin.
pub const MAX_TERMINAL_COMMAND_CHARS: usize = 4_096;

/// 128 characters for a terminal session id. The id is a client-chosen nonce
/// used to group output for a single shell session; it is not trusted as an
/// identifier for any server-side resource.
pub const MAX_TERMINAL_SESSION_ID_CHARS: usize = 128;

/// 86 400 seconds (24 hours) for a terminal request timeout. This is the
/// maximum wall-clock time a single terminal_exec call may block before the
/// engine gives up the child process. Ordinary commands return far faster; the
/// cap exists so a hung or accidentally interactive shell has a ceiling and
/// does not hold a worker indefinitely.
pub const MAX_TERMINAL_TIMEOUT_SECS: usize = 86_400;

// ---------------------------------------------------------------------------
// Typed error
// ---------------------------------------------------------------------------

/// Why a request was rejected before it reached the engine.
///
/// Variants map one-to-one onto [`InputError::code`], which is stable: HTTP
/// responses and desktop error strings both carry it, so a caller can branch
/// on the code without parsing prose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputError {
    /// A required field was absent from the request.
    Missing { field: &'static str },
    /// The field was present but of the wrong JSON type.
    WrongType {
        field: &'static str,
        expected: &'static str,
    },
    /// The field was present but blank after trimming.
    Empty { field: &'static str },
    /// The field exceeded its character bound.
    TooLong {
        field: &'static str,
        chars: usize,
        max: usize,
    },
    /// The field exceeded its byte bound.
    TooLarge {
        field: &'static str,
        bytes: usize,
        max: usize,
    },
    /// The field was well-formed but held an unacceptable value.
    Invalid { field: &'static str, detail: String },
}

impl InputError {
    /// Stable machine-readable code. Never changes wording; callers may rely
    /// on it.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Missing { .. } => "missing_field",
            Self::WrongType { .. } => "wrong_type",
            Self::Empty { .. } => "empty_field",
            Self::TooLong { .. } => "too_long",
            Self::TooLarge { .. } => "too_large",
            Self::Invalid { .. } => "invalid_value",
        }
    }

    /// The offending field, for a response body that can point at it.
    pub fn field(&self) -> &'static str {
        match self {
            Self::Missing { field }
            | Self::WrongType { field, .. }
            | Self::Empty { field }
            | Self::TooLong { field, .. }
            | Self::TooLarge { field, .. }
            | Self::Invalid { field, .. } => field,
        }
    }
}

impl std::fmt::Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing { field } => write!(f, "missing required field '{field}'"),
            Self::WrongType { field, expected } => {
                write!(f, "field '{field}' must be {expected}")
            }
            Self::Empty { field } => write!(f, "field '{field}' must not be empty"),
            Self::TooLong { field, chars, max } => {
                write!(
                    f,
                    "field '{field}' is {chars} characters; the limit is {max}"
                )
            }
            Self::TooLarge { field, bytes, max } => {
                write!(f, "field '{field}' is {bytes} bytes; the limit is {max}")
            }
            Self::Invalid { field, detail } => write!(f, "field '{field}': {detail}"),
        }
    }
}

impl std::error::Error for InputError {}

// ---------------------------------------------------------------------------
// Field extraction
// ---------------------------------------------------------------------------

/// Read a required string field, distinguishing absent from wrong-typed.
///
/// `serde_json`'s `as_str()` collapses both into `None`, which is why the
/// handlers used to fall through to `unwrap_or("")` — a missing prompt and a
/// prompt that was accidentally an object became the same empty string.
pub fn required_str<'a>(body: &'a Value, field: &'static str) -> Result<&'a str, InputError> {
    match body.get(field) {
        None => Err(InputError::Missing { field }),
        Some(Value::String(s)) => Ok(s),
        Some(_) => Err(InputError::WrongType {
            field,
            expected: "a string",
        }),
    }
}

/// Read an optional string field. `null` is absent; any other non-string is
/// a type error rather than a silent drop.
pub fn optional_str<'a>(
    body: &'a Value,
    field: &'static str,
) -> Result<Option<&'a str>, InputError> {
    match body.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        Some(_) => Err(InputError::WrongType {
            field,
            expected: "a string",
        }),
    }
}

/// Read a required field that may be any JSON value (structured arguments).
pub fn required_value<'a>(body: &'a Value, field: &'static str) -> Result<&'a Value, InputError> {
    body.get(field).ok_or(InputError::Missing { field })
}

// ---------------------------------------------------------------------------
// Field rules
// ---------------------------------------------------------------------------

/// Reject a field that is blank after trimming.
pub fn ensure_not_blank(field: &'static str, value: &str) -> Result<(), InputError> {
    if value.trim().is_empty() {
        return Err(InputError::Empty { field });
    }
    Ok(())
}

/// Reject a field longer than `max` characters.
///
/// Counted in characters, not bytes: a limit is a statement about how much
/// text a human pasted, and `char` count is what they perceive.
pub fn ensure_max_chars(field: &'static str, value: &str, max: usize) -> Result<(), InputError> {
    let chars = value.chars().count();
    if chars > max {
        return Err(InputError::TooLong { field, chars, max });
    }
    Ok(())
}

/// Reject a payload larger than `max` bytes.
pub fn ensure_max_bytes(field: &'static str, value: &[u8], max: usize) -> Result<(), InputError> {
    let bytes = value.len();
    if bytes > max {
        return Err(InputError::TooLarge { field, bytes, max });
    }
    Ok(())
}

/// Reject control characters (including NUL and newlines) in a field whose
/// value is an identifier or single-line token.
fn ensure_single_line(field: &'static str, value: &str) -> Result<(), InputError> {
    if value.chars().any(|c| c.is_control()) {
        return Err(InputError::Invalid {
            field,
            detail: "control characters are not allowed".to_string(),
        });
    }
    Ok(())
}

fn ensure_charset(
    field: &'static str,
    value: &str,
    allowed: fn(char) -> bool,
) -> Result<(), InputError> {
    if let Some(bad) = value.chars().find(|c| !allowed(*c)) {
        return Err(InputError::Invalid {
            field,
            detail: format!("unexpected character {bad:?}"),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Domain validators
// ---------------------------------------------------------------------------

/// Validate a free-text prompt bound for the intent pipeline.
pub fn validate_prompt(prompt: &str) -> Result<(), InputError> {
    ensure_not_blank("prompt", prompt)?;
    ensure_max_chars("prompt", prompt, MAX_PROMPT_CHARS)?;
    Ok(())
}

/// Validate a mission task bound for the queue.
pub fn validate_task(task: &str) -> Result<(), InputError> {
    ensure_not_blank("task", task)?;
    ensure_max_chars("task", task, MAX_TASK_CHARS)?;
    Ok(())
}

/// Validate a repository search query.
pub fn validate_query(query: &str) -> Result<(), InputError> {
    ensure_not_blank("q", query)?;
    ensure_max_chars("q", query, MAX_QUERY_CHARS)?;
    Ok(())
}

/// Validate an optional model override.
///
/// `None` (no override) is always valid. A present identifier must be
/// non-blank, single-line and drawn from the characters a provider slug
/// actually uses — so a prompt-shaped string cannot be smuggled through a
/// field that is only ever meant to select a model.
pub fn validate_model(model: Option<&str>) -> Result<(), InputError> {
    let Some(model) = model else { return Ok(()) };
    ensure_not_blank("model", model)?;
    ensure_max_chars("model", model, MAX_MODEL_ID_CHARS)?;
    ensure_single_line("model", model)?;
    ensure_charset("model", model, |c| {
        c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_' | ':')
    })?;
    Ok(())
}

/// Validate a correlation id: an opaque token, so same charset as a model id
/// but a tighter bound.
pub fn validate_correlation_id(id: Option<&str>) -> Result<(), InputError> {
    let Some(id) = id else { return Ok(()) };
    ensure_not_blank("correlation_id", id)?;
    ensure_max_chars("correlation_id", id, MAX_CORRELATION_ID_CHARS)?;
    ensure_single_line("correlation_id", id)?;
    ensure_charset("correlation_id", id, |c| {
        c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')
    })?;
    Ok(())
}

/// Validate a workspace-relative path: non-blank, bounded, and free of the
/// NUL and newline characters that make a path ambiguous to the OS layer.
///
/// Containment (does it stay inside the workspace root?) is deliberately
/// *not* checked here — that policy already lives with the code that opens
/// the file, and duplicating it at the boundary would let the two drift.
pub fn validate_path(path: &str) -> Result<(), InputError> {
    ensure_not_blank("path", path)?;
    ensure_max_chars("path", path, MAX_PATH_CHARS)?;
    if path.chars().any(|c| c == '\0' || c == '\n' || c == '\r') {
        return Err(InputError::Invalid {
            field: "path",
            detail: "NUL and newline are not allowed in a path".to_string(),
        });
    }
    Ok(())
}

/// Validate a URL endpoint (http/https, http://127.0.0.1:3000/
/// style). Reject non-URL, malformed, or scheme/port-mismatched values.
pub fn validate_endpoint(endpoint: &str) -> Result<(), InputError> {
    if endpoint.trim().is_empty() {
        return Err(InputError::Empty { field: "endpoint" });
    }
    if endpoint.len() > MAX_URL_CHARS {
        return Err(InputError::TooLong {
            field: "endpoint",
            chars: endpoint.len(),
            max: MAX_URL_CHARS,
        });
    }
    let parsed = match url::Url::parse(endpoint) {
        Ok(u) => u,
        Err(_) => {
            return Err(InputError::Invalid {
                field: "endpoint",
                detail: "must be a valid http:// or https:// URL".to_string(),
            });
        }
    };
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(InputError::Invalid {
            field: "endpoint",
            detail: "only http:// and https:// schemes are allowed".to_string(),
        });
    }
    // Loopback-only dev surface: block public-hostnames that can be used to
    // reach services outside the local machine.
    if parsed.host_str() == Some("127.0.0.1") || parsed.host_str() == Some("[::1]") {
        return Ok(());
    }
    if let Some(host) = parsed.host_str() {
        if host.contains('.') || host == "localhost" {
            return Err(InputError::Invalid {
                field: "endpoint",
                detail: "loopback host only (127.0.0.1 or ::1)".to_string(),
            });
        }
    }
    Ok(())
}

/// Validate a client-sent transport name for MCP bridges: stdio, sse,
/// websocket.
pub fn validate_transport(transport: Option<&str>) -> Result<(), InputError> {
    match transport {
        None => Ok(()),
        Some(t) => {
            ensure_not_blank("transport", t)?;
            ensure_max_chars("transport", t, MAX_TOOL_ID_CHARS)?;
            ensure_charset("transport", t, |c| {
                c.is_ascii_alphanumeric() || matches!(c, '-' | '_')
            })?;
            match t {
                "stdio" | "sse" | "websocket" => Ok(()),
                other => Err(InputError::Invalid {
                    field: "transport",
                    detail: format!("expected 'stdio', 'sse' or 'websocket', got {other:?}"),
                }),
            }
        }
    }
}

/// Validate a model-provider base URL typed by the operator (the "Base URL"
/// field of provider settings).
///
/// This is deliberately **not** [`validate_endpoint`]: that one guards an MCP
/// bridge endpoint — a local process the app launches — and therefore admits
/// loopback only. A model provider is the opposite case: `api.anthropic.com`
/// and `https://api.deepseek.com/v1` are the normal, intended values, so
/// rejecting public hosts there would make the feature unusable while
/// protecting nothing (the request already leaves the machine by design).
///
/// What is rejected is what makes a URL ambiguous rather than remote: a
/// non-http(s) scheme, embedded credentials, a fragment, control characters
/// or whitespace, a missing host, and anything over the URL bound.
pub fn validate_provider_endpoint(endpoint: &str) -> Result<(), InputError> {
    if endpoint.trim().is_empty() {
        return Err(InputError::Empty { field: "endpoint" });
    }
    if endpoint.len() > MAX_URL_CHARS {
        return Err(InputError::TooLong {
            field: "endpoint",
            chars: endpoint.len(),
            max: MAX_URL_CHARS,
        });
    }
    if endpoint.chars().any(|c| c.is_control() || c == ' ') {
        return Err(InputError::Invalid {
            field: "endpoint",
            detail: "whitespace and control characters are not allowed in a URL".to_string(),
        });
    }
    let parsed = match url::Url::parse(endpoint) {
        Ok(u) => u,
        Err(_) => {
            return Err(InputError::Invalid {
                field: "endpoint",
                detail: "must be a valid http:// or https:// URL".to_string(),
            });
        }
    };
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(InputError::Invalid {
            field: "endpoint",
            detail: "only http:// and https:// schemes are allowed".to_string(),
        });
    }
    if parsed.host_str().unwrap_or("").is_empty() {
        return Err(InputError::Invalid {
            field: "endpoint",
            detail: "the URL must name a host".to_string(),
        });
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(InputError::Invalid {
            field: "endpoint",
            detail: "credentials must not be embedded in the URL; use an API key field".to_string(),
        });
    }
    if parsed.fragment().is_some() {
        return Err(InputError::Invalid {
            field: "endpoint",
            detail: "a fragment is not part of a request URL".to_string(),
        });
    }
    Ok(())
}

/// Validate a marketplace search query.
pub fn validate_marketplace_query(query: &str) -> Result<(), InputError> {
    validate_query(query)
}

/// Validate an MCP tool id.
pub fn validate_tool_id(id: &str) -> Result<(), InputError> {
    ensure_not_blank("id", id)?;
    ensure_max_chars("id", id, MAX_TOOL_ID_CHARS)?;
    ensure_single_line("id", id)?;
    ensure_charset("id", id, |c| {
        c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':')
    })?;
    Ok(())
}

/// Validate a mission mode. Unknown modes are rejected rather than coerced
/// to `build`: silently running a build when the caller asked for something
/// else would be the worst possible default.
pub fn validate_mode(mode: Option<&str>) -> Result<(), InputError> {
    match mode {
        None => Ok(()),
        Some("build") | Some("plan") => Ok(()),
        Some(other) => Err(InputError::Invalid {
            field: "mode",
            detail: format!("expected 'build' or 'plan', got {other:?}"),
        }),
    }
}

// ---------------------------------------------------------------------------
// Terminal requests and release versions
// ---------------------------------------------------------------------------

/// Control characters a shell command line legitimately carries: horizontal
/// tab, line feed, and carriage return.
///
/// They are line structure, not payload. [`validate_terminal_request_command`]
/// accepts them because the engine hands the whole string to a real shell
/// (`sh -c`, `cmd.exe /C`, PowerShell `-EncodedCommand`), where a newline
/// separates statements exactly like `;` does. Every other control character —
/// NUL, escape, BEL, DEL and the rest — is rejected there.
const COMMAND_LINE_CONTROLS: [char; 3] = ['\t', '\n', '\r'];

/// One component of a path, folded for comparison.
///
/// Windows paths are case-insensitive, so the drive letter and directory names
/// are compared case-folded there. ASCII folding covers the realistic
/// differences (a caller typing `c:\projects` for `C:\Projects`) and can
/// never make two genuinely different components compare equal.
fn path_component_key(component: Component<'_>) -> String {
    let part = component.as_os_str().to_string_lossy().into_owned();
    if cfg!(windows) {
        part.to_ascii_lowercase()
    } else {
        part
    }
}

/// Resolve `.` and `..` in `path` without touching the filesystem.
///
/// A working directory that does not exist yet must still be judged on where it
/// points, so the decision cannot depend on `is_dir`.
fn normalize_lexically(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out.push(std::path::MAIN_SEPARATOR_STR),
            Component::Normal(part) => out.push(part),
        }
    }
    out
}

/// Resolve a path to the absolute location it names, without requiring the
/// whole thing to exist.
///
/// The filesystem's own answer is preferred: canonicalize resolves symlinks and
/// junctions, so a link planted inside the workspace cannot be used to step
/// outside it, and it returns one canonical spelling of each existing directory.
///
/// A directory that does not exist yet is resolved as far as it does. Falling
/// back to a purely lexical answer for the whole path would compare two
/// different spellings of the same place: on Windows the workspace root
/// canonicalizes to `\\?\C:\…` while the not-yet-created child is spelled
/// `C:\…`, and an 8.3 short name such as `MBARIE~1` is canonicalized to its long
/// form. Either mismatch made a legitimate in-workspace directory look like
/// traversal. Canonicalizing the deepest existing ancestor and re-appending the
/// missing tail keeps the comparison honest in both directions: an inside path
/// compares equal, and a sibling of the workspace still does not.
fn resolve_for_confinement(path: &Path) -> PathBuf {
    let normalized = normalize_lexically(path);
    if let Ok(canonical) = std::fs::canonicalize(&normalized) {
        return canonical;
    }
    let mut missing: Vec<std::ffi::OsString> = Vec::new();
    let mut cursor = normalized.as_path();
    while let Some(name) = cursor.file_name() {
        missing.push(name.to_os_string());
        cursor = match cursor.parent() {
            Some(parent) => parent,
            None => break,
        };
        if let Ok(base) = std::fs::canonicalize(cursor) {
            let mut out = base;
            for part in missing.iter().rev() {
                out.push(part);
            }
            return out;
        }
    }
    normalized
}

/// Resolve a request `cwd` to the absolute directory it names.
///
/// A relative `cwd` is workspace-relative, which is what the contract says: the
/// desktop terminal shows the workspace root as its "here".
fn resolve_terminal_cwd(root: &Path, cwd: &str) -> PathBuf {
    let raw = if Path::new(cwd).is_absolute() {
        PathBuf::from(cwd)
    } else {
        root.join(cwd)
    };
    resolve_for_confinement(&raw)
}

/// True when `candidate` is `root` or lives underneath it, compared by whole
/// path components — so `…/zylcode-evil` is not inside `…/zylcode`.
fn path_is_within(root: &Path, candidate: &Path) -> bool {
    let root_parts: Vec<String> = root.components().map(path_component_key).collect();
    let candidate_parts: Vec<String> = candidate.components().map(path_component_key).collect();
    candidate_parts.len() >= root_parts.len()
        && candidate_parts[..root_parts.len()] == root_parts[..]
}

/// Characters allowed in a terminal session id: a portable identifier with no
/// whitespace, quotes, control characters, or path separators.
fn is_session_id_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':')
}

/// Characters allowed in a release version: ASCII alphanumerics plus the four
/// separators semver and its build metadata actually use.
fn is_release_version_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+' | '~')
}

/// Reject path separators and relative-traversal components in a small
/// machine-readable token such as a release version.
fn ensure_no_path_traversal(field: &'static str, value: &str) -> Result<(), InputError> {
    if value.contains('/') || value.contains('\\') {
        return Err(InputError::Invalid {
            field,
            detail: "path separators are not allowed".to_string(),
        });
    }
    if value.contains("..") {
        return Err(InputError::Invalid {
            field,
            detail: "relative traversal is not allowed".to_string(),
        });
    }
    Ok(())
}

/// Validate a `cwd` string on its own: a usable path, no control characters,
/// inside the path length cap.
fn validate_cwd_field(cwd: &str) -> Result<(), InputError> {
    ensure_not_blank("cwd", cwd)?;
    ensure_max_chars("cwd", cwd, MAX_PATH_CHARS)?;
    if let Some(c) = cwd.chars().find(|c| c.is_control()) {
        return Err(InputError::Invalid {
            field: "cwd",
            detail: format!(
                "control character U+{:04X} is not allowed in a path",
                c as u32
            ),
        });
    }
    Ok(())
}

/// Validate a terminal working directory against the workspace it must stay in.
///
/// This is the real security boundary for `terminal_exec`: the command itself
/// is a shell command line by design, so the property worth enforcing is
/// *where* the process runs. Relative paths inside the workspace — `src`,
/// `crates/x`, `src/../src/lib` — are legitimate and pass, as does an absolute
/// path that happens to point inside the workspace. Anything resolving above
/// `root` is refused, including an absolute path elsewhere on the machine and a
/// sibling directory whose name merely starts with the workspace's.
pub fn validate_terminal_cwd(root: &Path, cwd: &str) -> Result<(), InputError> {
    validate_cwd_field(cwd)?;
    let root = resolve_for_confinement(root);
    let candidate = resolve_terminal_cwd(&root, cwd);
    if !path_is_within(&root, &candidate) {
        return Err(InputError::Invalid {
            field: "cwd",
            detail: "must stay inside the permitted workspace directory".to_string(),
        });
    }
    Ok(())
}

/// Validate the fields of a [`TerminalRequest`] that do not depend on a
/// workspace root, and validate `cwd` as a path string.
///
/// Callers that know the workspace should use [`validate_terminal_request_in`]
/// instead: this one cannot tell whether `cwd` points at the workspace or at
/// `C:\Windows`.
pub fn validate_terminal_request(req: &TerminalRequest) -> Result<(), InputError> {
    validate_terminal_request_command(&req.command)?;

    if let Some(cwd) = req.cwd.as_deref() {
        validate_cwd_field(cwd)?;
    }

    if let Some(shell) = req.shell.as_deref() {
        ShellMode::parse(shell).map_err(|e| InputError::Invalid {
            field: "shell",
            detail: e.to_string(),
        })?;
    }

    validate_terminal_request_timeout(req.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS))?;

    if let Some(session_id) = req.session_id.as_deref() {
        validate_terminal_request_session_id(session_id)?;
    }
    Ok(())
}

/// Validate a [`TerminalRequest`] in full against the workspace it may run in.
///
/// Both untrusted surfaces call this one. The command is a shell command line,
/// so the boundary that carries security weight is the directory the child
/// process is started in; [`validate_terminal_cwd`] is what keeps it inside
/// `root`.
pub fn validate_terminal_request_in(root: &Path, req: &TerminalRequest) -> Result<(), InputError> {
    validate_terminal_request(req)?;
    if let Some(cwd) = req.cwd.as_deref() {
        validate_terminal_cwd(root, cwd)?;
    }
    Ok(())
}

/// Validate the `command` field of a [`TerminalRequest`].
///
/// The command is a **shell command line**, not an argument vector: the engine
/// hands it to `sh -c`, `cmd.exe /C` or PowerShell `-EncodedCommand` as
/// written. Pipes, redirection, quoting, `$VAR` / `%VAR%`, globbing, `&&` and
/// `;` are therefore part of the contract and are accepted. A charset filter
/// over them would break every real workflow while stopping nothing, because a
/// shell command that survives a filter is still a shell command.
///
/// What is enforced here is integrity, not sanitisation: at most
/// [`MAX_TERMINAL_COMMAND_CHARS`] characters, not blank, and no control
/// character other than the line structure listed in
/// `COMMAND_LINE_CONTROLS` (tab, line feed, carriage return).
/// NUL cannot cross into a child process, and escape and the remaining C0/DEL
/// codes have no legitimate place in a command while being exactly what
/// terminal-escape injection is built from.
///
/// Confinement lives in [`validate_terminal_cwd`], and the child inherits only
/// the operator's own environment and privileges.
pub fn validate_terminal_request_command(command: &str) -> Result<(), InputError> {
    ensure_not_blank("command", command)?;
    ensure_max_chars("command", command, MAX_TERMINAL_COMMAND_CHARS)?;
    if let Some(c) = command
        .chars()
        .find(|c| c.is_control() && !COMMAND_LINE_CONTROLS.contains(c))
    {
        return Err(InputError::Invalid {
            field: "command",
            detail: format!(
                "control character U+{:04X} is not allowed in a command (NUL, escape and the other C0/DEL codes are rejected)",
                c as u32
            ),
        });
    }
    Ok(())
}

/// Validate the `timeout_secs` field of a [`TerminalRequest`].
///
/// The timeout is bounded into a hard range so a caller cannot hand the engine
/// a zero-second wait or a multi-day one. The ceiling is the public constant, so
/// a future review of the worst acceptable case changes one number.
pub fn validate_terminal_request_timeout(timeout_secs: u64) -> Result<(), InputError> {
    if timeout_secs == 0 {
        return Err(InputError::Invalid {
            field: "timeout_secs",
            detail: "timeout must be at least 1 second".to_string(),
        });
    }
    if timeout_secs > MAX_TERMINAL_TIMEOUT_SECS as u64 {
        return Err(InputError::TooLong {
            field: "timeout_secs",
            chars: timeout_secs as usize,
            max: MAX_TERMINAL_TIMEOUT_SECS,
        });
    }
    Ok(())
}

/// Validate the optional `session_id` field of a [`TerminalRequest`].
///
/// The id is not a trusted key for anything on the server, but letting it carry
/// control characters or separators would be letting an untrusted value transit
/// the boundary unchecked.
pub fn validate_terminal_request_session_id(session_id: &str) -> Result<(), InputError> {
    ensure_max_chars("session_id", session_id, MAX_TERMINAL_SESSION_ID_CHARS)?;
    ensure_single_line("session_id", session_id)?;
    ensure_charset("session_id", session_id, is_session_id_char)?;
    Ok(())
}

/// Validate a release version string for the `package_start` command.
///
/// A release version is a small machine-readable label, not free text: it must
/// be non-blank, free of path separators and relative traversal, and limited to
/// the portable version charset (ASCII alphanumerics plus `.` `-` `_` `+` `~`).
/// The length bound is the small-token cap, since a version is a label and never
/// a payload.
pub fn validate_release_version(version: &str) -> Result<(), InputError> {
    ensure_not_blank("version", version)?;
    ensure_max_chars("version", version, MAX_TERMINAL_SESSION_ID_CHARS)?;
    ensure_no_path_traversal("version", version)?;
    ensure_single_line("version", version)?;
    ensure_charset("version", version, is_release_version_char)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Transport-wide caps
// ---------------------------------------------------------------------------

/// Whole-request body cap for the HTTP surface: 8 MiB.
///
/// The precise limits are per field — [`MAX_ARTIFACT_BYTES`] and the rest.
/// This one exists so a body that could not possibly pass a field rule is
/// never buffered in the first place. It is 8 MiB because the largest
/// accepted field is 4 MiB of content and ordinary JSON escaping of that
/// stays well inside 8 MiB, so no request the field rules accept is refused
/// here; anything refused at this level was already unacceptable and simply
/// fails earlier and cheaper.
///
/// Axum's built-in default is 2 MiB, which is *below* the documented
/// artifact limit — before this constant existed a legitimate large save was
/// refused by the transport while the code claimed to accept 4 MiB.
pub const MAX_HTTP_BODY_BYTES: usize = 8 * 1024 * 1024;

// ---------------------------------------------------------------------------
// Artifact writes
// ---------------------------------------------------------------------------

/// Validate an editor save: the workspace-relative label and the bytes that
/// will be written under it.
///
/// The label is blank-checked; the content deliberately is not — saving an
/// empty file is a legitimate operation, and an *absent* content field is a
/// different failure that belongs to [`required_str`] at the boundary.
pub fn validate_artifact(label: &str, content: &str) -> Result<(), InputError> {
    ensure_not_blank("label", label)?;
    ensure_max_chars("label", label, MAX_LABEL_CHARS)?;
    ensure_max_bytes("content", content.as_bytes(), MAX_ARTIFACT_BYTES)?;
    Ok(())
}

/// Validate a unified-diff patch body before it reaches `git apply`.
///
/// Bounded like artifact content: a patch is a file-shaped payload, and the
/// apply path is where an unbounded body would do the most damage.
pub fn validate_patch(patch: &str) -> Result<(), InputError> {
    ensure_not_blank("patch", patch)?;
    ensure_max_bytes("patch", patch.as_bytes(), MAX_ARTIFACT_BYTES)?;
    Ok(())
}

/// Validate the structured argument object handed to an MCP tool.
///
/// Measured on its serialized form, because that is what the tool actually
/// receives — a small-looking object of ten 6 MiB strings is 60 MiB of work
/// no matter how few keys it has.
pub fn validate_tool_args(args: &Value) -> Result<(), InputError> {
    if !args.is_object() {
        return Err(InputError::WrongType {
            field: "params",
            expected: "an object",
        });
    }
    // `Value` maps always have string keys, so serialization cannot fail.
    let bytes = serde_json::to_vec(args).unwrap_or_default();
    ensure_max_bytes("params", &bytes, MAX_ARGS_BYTES)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // -------------------------------------------------------------------------
    // Field extraction: absent and wrong-typed must stay distinguishable.
    // -------------------------------------------------------------------------

    #[test]
    fn a_missing_field_is_missing_not_empty() {
        let body = json!({});
        let err = required_str(&body, "prompt").unwrap_err();
        assert_eq!(err.code(), "missing_field");
        assert_eq!(err.field(), "prompt");
        assert_eq!(err.to_string(), "missing required field 'prompt'");
    }

    #[test]
    fn a_non_string_field_is_a_type_error_not_a_default() {
        // The previous behaviour was `body["prompt"].as_str().unwrap_or("")`,
        // which turned this into an empty prompt the engine would then reject
        // with a message about the prompt rather than about the request.
        let body = json!({ "prompt": 42 });
        let err = required_str(&body, "prompt").unwrap_err();
        assert_eq!(err.code(), "wrong_type");
        assert!(err.to_string().contains("must be a string"));
    }

    #[test]
    fn an_array_field_is_a_type_error_too() {
        let body = json!({ "prompt": ["do", "this"] });
        assert_eq!(
            required_str(&body, "prompt").unwrap_err().code(),
            "wrong_type"
        );
    }

    #[test]
    fn a_present_string_is_read_verbatim() {
        let body = json!({ "prompt": "fix the bug" });
        assert_eq!(
            required_str(&body, "prompt").expect("a present string must parse"),
            "fix the bug"
        );
    }

    #[test]
    fn an_optional_field_absent_or_null_is_none() {
        assert_eq!(optional_str(&json!({}), "model").unwrap(), None);
        assert_eq!(
            optional_str(&json!({ "model": null }), "model").unwrap(),
            None
        );
    }

    #[test]
    fn an_optional_field_of_the_wrong_type_is_still_an_error() {
        let body = json!({ "model": true });
        assert_eq!(
            optional_str(&body, "model").unwrap_err().code(),
            "wrong_type"
        );
    }

    #[test]
    fn a_required_value_must_be_present() {
        let err = required_value(&json!({}), "params").unwrap_err();
        assert_eq!(err.code(), "missing_field");
        assert_eq!(err.field(), "params");
    }

    // ----
    // Terminal requests and release versions
    // ----

    /// A workspace root that deliberately does not exist on disk: traversal must
    /// be judged on where a path points, never on whether `is_dir` happens to
    /// answer yes.
    fn workspace_root() -> PathBuf {
        std::env::temp_dir().join("zylcode-r05-workspace")
    }

    fn terminal_request(command: &str, cwd: Option<&str>) -> TerminalRequest {
        TerminalRequest {
            command: command.to_string(),
            cwd: cwd.map(str::to_string),
            shell: Some("sh".to_string()),
            timeout_secs: Some(60),
            session_id: None,
        }
    }

    /// Regression: the terminal is a real command runner. A validator that
    /// rejects a path separator breaks `cat src/lib.rs` and every relative
    /// command a user types, so these must all be accepted.
    #[test]
    fn legitimate_commands_with_relative_paths_are_accepted() {
        for command in [
            "cat src/lib.rs",
            "ls ../src",
            r"Get-Content .\src\lib.rs",
            "cargo test -p zylcode-core",
            "git status",
            "npm run build",
        ] {
            assert!(
                validate_terminal_request_command(command).is_ok(),
                "{command} must be accepted"
            );
        }
    }

    /// Regression: commands run through a shell, so its syntax is the contract.
    #[test]
    fn shell_syntax_is_part_of_the_command_contract() {
        for command in [
            "ls -la | head -20",
            "cargo test 2>&1 | tail -5",
            "echo hi > out.txt",
            "echo a && echo b",
            "FOO=1 cargo run",
            "echo $HOME",
            "echo $(pwd)",
            "echo %PATH%",
            r"Select-String -Path .\src\*.rs -Pattern fn",
            "echo one\necho two",
        ] {
            assert!(
                validate_terminal_request_command(command).is_ok(),
                "{command} must be accepted"
            );
        }
    }

    #[test]
    fn a_normal_terminal_request_is_valid() {
        assert!(validate_terminal_request_in(
            &workspace_root(),
            &terminal_request("cargo test --quiet", Some("crates/zylcode-core")),
        )
        .is_ok());
    }

    #[test]
    fn relative_cwds_inside_the_workspace_are_accepted() {
        let root = workspace_root();
        for cwd in [
            ".",
            "./src",
            "crates/zylcode-core",
            "src/../src/lib",
            "target/debug",
        ] {
            assert!(
                validate_terminal_request_in(&root, &terminal_request("pwd", Some(cwd))).is_ok(),
                "{cwd} must be accepted"
            );
        }
    }

    #[test]
    fn an_absolute_cwd_inside_the_workspace_is_accepted() {
        let root = workspace_root();
        let inside = root.join("crates").join("zylcode-core");
        assert!(validate_terminal_cwd(&root, &inside.to_string_lossy()).is_ok());
    }

    #[test]
    fn a_request_without_a_cwd_is_valid() {
        assert!(validate_terminal_request(&terminal_request("pwd", None)).is_ok());
    }

    /// A real directory on disk, removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "zylcode-r05-{tag}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Regression: the workspace root is a real directory, so it canonicalizes
    /// (on Windows to a `\\?\C:\...` verbatim path), while a subdirectory of it
    /// that does not exist yet can only be judged lexically (`C:\...`). Both
    /// name the same place, so the request must be accepted and the hub left to
    /// report "does not exist" — the confinement check must not blame traversal
    /// for a path that is inside the workspace.
    #[test]
    fn a_cwd_inside_a_real_workspace_is_accepted_when_it_does_not_exist_yet() {
        let root = TempDir::new("existing-root");
        assert!(
            validate_terminal_cwd(&root.0, "not-yet-created").is_ok(),
            "a relative missing child of a real workspace must be accepted"
        );
        let absolute = root.0.join("not-yet-created");
        assert!(
            validate_terminal_cwd(&root.0, &absolute.to_string_lossy()).is_ok(),
            "an absolute missing child of a real workspace must be accepted"
        );
    }

    /// The counterpart: a real workspace root must not widen the boundary. A
    /// sibling of it is still outside.
    #[test]
    fn a_sibling_of_a_real_workspace_is_still_rejected() {
        let root = TempDir::new("real-sibling");
        let sibling = root.0.with_file_name(format!(
            "{}-evil",
            root.0
                .file_name()
                .expect("the temp child has a file name")
                .to_string_lossy()
        ));
        assert_eq!(
            validate_terminal_cwd(&root.0, &sibling.to_string_lossy())
                .unwrap_err()
                .field(),
            "cwd"
        );
    }

    #[test]
    fn a_cwd_that_climbs_out_of_the_workspace_is_rejected() {
        let root = workspace_root();
        for cwd in ["..", "../..", "../../elsewhere", "src/../../.."] {
            let err = validate_terminal_request_in(&root, &terminal_request("pwd", Some(cwd)))
                .unwrap_err();
            assert_eq!(err.field(), "cwd", "{cwd}");
            assert_eq!(err.code(), "invalid_value", "{cwd}");
        }
    }

    #[test]
    fn an_absolute_cwd_outside_the_workspace_is_rejected() {
        let root = workspace_root();
        let outside = root.parent().expect("the temp directory has a parent");
        let err = validate_terminal_cwd(&root, &outside.to_string_lossy()).unwrap_err();
        assert_eq!(err.field(), "cwd");
    }

    #[test]
    fn a_sibling_directory_sharing_the_workspace_name_prefix_is_rejected() {
        let root = workspace_root();
        let sibling = root
            .parent()
            .expect("the temp directory has a parent")
            .join("zylcode-r05-workspace-evil");
        let err = validate_terminal_cwd(&root, &sibling.to_string_lossy()).unwrap_err();
        assert_eq!(err.code(), "invalid_value");
    }

    #[test]
    fn a_cwd_with_a_control_character_is_rejected() {
        let root = workspace_root();
        let err = validate_terminal_request_in(&root, &terminal_request("pwd", Some("src\nlib")))
            .unwrap_err();
        assert_eq!(err.field(), "cwd");
        assert_eq!(err.code(), "invalid_value");
    }

    #[test]
    fn a_terminal_command_may_not_be_blank() {
        assert_eq!(
            validate_terminal_request_command("").unwrap_err().code(),
            "empty_field"
        );
        assert_eq!(
            validate_terminal_request_command("   ").unwrap_err().code(),
            "empty_field"
        );
    }

    #[test]
    fn a_terminal_command_over_the_limit_is_rejected() {
        let over = "x".repeat(MAX_TERMINAL_COMMAND_CHARS + 1);
        assert_eq!(
            validate_terminal_request_command(&over).unwrap_err().code(),
            "too_long"
        );
        let at_limit = "x".repeat(MAX_TERMINAL_COMMAND_CHARS);
        assert!(validate_terminal_request_command(&at_limit).is_ok());
    }

    #[test]
    fn control_characters_in_a_command_are_rejected() {
        for command in ["ls \0/tmp", "ls \u{1b}[31m", "echo \u{7}", "rm -rf x\u{7f}"] {
            let err = validate_terminal_request_command(command).unwrap_err();
            assert_eq!(err.code(), "invalid_value", "{command:?}");
            assert_eq!(err.field(), "command");
        }
    }

    #[test]
    fn a_terminal_timeout_of_zero_is_rejected() {
        let err = validate_terminal_request(&TerminalRequest {
            command: "pwd".into(),
            cwd: Some(".".into()),
            shell: Some("sh".into()),
            timeout_secs: Some(0),
            session_id: None,
        })
        .unwrap_err();
        assert_eq!(err.field(), "timeout_secs");
    }

    #[test]
    fn a_terminal_timeout_over_the_maximum_is_rejected() {
        assert_eq!(
            validate_terminal_request_timeout(MAX_TERMINAL_TIMEOUT_SECS as u64 + 1)
                .unwrap_err()
                .code(),
            "too_long"
        );
    }

    #[test]
    fn a_terminal_timeout_at_the_maximum_is_accepted() {
        assert!(validate_terminal_request_timeout(MAX_TERMINAL_TIMEOUT_SECS as u64).is_ok());
    }

    #[test]
    fn a_terminal_request_with_an_invalid_shell_is_rejected() {
        let err = validate_terminal_request(&TerminalRequest {
            command: "echo hi".into(),
            cwd: Some(".".into()),
            shell: Some("nuke".into()),
            timeout_secs: Some(10),
            session_id: None,
        })
        .unwrap_err();
        assert_eq!(err.field(), "shell");
        let text = err.to_string();
        assert!(text.contains("expected one of"), "{text}");
        assert!(text.contains("pwsh"), "{text}");
    }

    #[test]
    fn an_optional_session_id_may_be_absent() {
        assert!(validate_terminal_request(&TerminalRequest {
            command: "pwd".into(),
            cwd: Some(".".into()),
            shell: Some("sh".into()),
            timeout_secs: Some(10),
            session_id: None,
        })
        .is_ok());
    }

    #[test]
    fn a_session_id_over_the_limit_is_rejected() {
        assert_eq!(
            validate_terminal_request_session_id(&"x".repeat(MAX_TERMINAL_SESSION_ID_CHARS + 1))
                .unwrap_err()
                .code(),
            "too_long"
        );
    }

    #[test]
    fn a_session_id_with_control_characters_is_rejected() {
        assert_eq!(
            validate_terminal_request_session_id("abc\nxyz")
                .unwrap_err()
                .code(),
            "invalid_value"
        );
    }

    #[test]
    fn a_session_id_with_a_path_separator_is_rejected() {
        assert_eq!(
            validate_terminal_request_session_id("../../etc/passwd")
                .unwrap_err()
                .code(),
            "invalid_value"
        );
    }

    #[test]
    fn a_valid_session_id_is_accepted() {
        assert!(validate_terminal_request_session_id("session-1").is_ok());
        assert!(
            validate_terminal_request_session_id("9f1c2a44-1f0e-4c2b-9d55-2a0d6c3f8e11").is_ok()
        );
    }

    #[test]
    fn a_release_version_may_not_be_blank() {
        assert_eq!(
            validate_release_version("").unwrap_err().code(),
            "empty_field"
        );
        assert_eq!(
            validate_release_version("   ").unwrap_err().code(),
            "empty_field"
        );
    }

    #[test]
    fn a_release_version_may_not_carry_path_separators() {
        assert_eq!(
            validate_release_version("1.0/../oops").unwrap_err().code(),
            "invalid_value"
        );
        assert_eq!(
            validate_release_version("v1/2").unwrap_err().code(),
            "invalid_value"
        );
        assert_eq!(
            validate_release_version(r"v1\2").unwrap_err().code(),
            "invalid_value"
        );
    }

    #[test]
    fn a_release_version_must_match_the_allowed_charset() {
        assert_eq!(
            validate_release_version("1.0 beta").unwrap_err().code(),
            "invalid_value"
        );
        assert_eq!(
            validate_release_version("v1.0\n").unwrap_err().code(),
            "invalid_value"
        );
    }

    #[test]
    fn a_normal_release_version_is_valid() {
        assert!(validate_release_version("1.2.3").is_ok());
        assert!(validate_release_version("v0.1.0-rc.1").is_ok());
        assert!(validate_release_version("1.0.0+build.123").is_ok());
    }

    // -------------------------------------------------------------------------
    // Bounds
    // -------------------------------------------------------------------------

    #[test]
    fn a_blank_field_is_empty_not_valid() {
        let err = validate_prompt("   \n\t ").unwrap_err();
        assert_eq!(err.code(), "empty_field");
        assert_eq!(err.field(), "prompt");
    }

    #[test]
    fn an_empty_prompt_is_rejected() {
        assert_eq!(validate_prompt("").unwrap_err().code(), "empty_field");
    }

    #[test]
    fn a_prompt_over_the_limit_is_rejected_with_both_numbers() {
        let over = "x".repeat(MAX_PROMPT_CHARS + 1);
        let err = validate_prompt(&over).unwrap_err();
        assert_eq!(err.code(), "too_long");
        let rendered = err.to_string();
        assert!(rendered.contains(&(MAX_PROMPT_CHARS + 1).to_string()));
        assert!(rendered.contains(&MAX_PROMPT_CHARS.to_string()));
    }

    #[test]
    fn a_prompt_at_the_limit_is_accepted() {
        let exact = "x".repeat(MAX_PROMPT_CHARS);
        assert!(validate_prompt(&exact).is_ok());
    }

    #[test]
    fn limits_are_counted_in_characters_not_bytes() {
        // A limit is about how much text someone pasted; a multi-byte glyph
        // must not be able to fail a length check the user can see they pass.
        let accented = "é".repeat(10);
        assert_eq!(accented.chars().count(), 10);
        // `str::len` is the byte length — this is the gap the char-counted
        // rule must not collapse into.
        assert!(accented.len() > 10);
        assert!(ensure_max_chars("f", &accented, 10).is_ok());
        assert!(ensure_max_chars("f", &accented, 9).is_err());
    }

    #[test]
    fn byte_bound_rejects_an_oversized_payload() {
        let big = vec![0u8; MAX_ARTIFACT_BYTES + 1];
        let err = ensure_max_bytes("content", &big, MAX_ARTIFACT_BYTES).unwrap_err();
        assert_eq!(err.code(), "too_large");
        assert_eq!(err.field(), "content");
    }

    #[test]
    fn task_and_query_have_their_own_bounds() {
        assert!(validate_task(&"x".repeat(MAX_TASK_CHARS)).is_ok());
        assert_eq!(
            validate_task(&"x".repeat(MAX_TASK_CHARS + 1))
                .unwrap_err()
                .field(),
            "task"
        );
        assert_eq!(
            validate_query(&"x".repeat(MAX_QUERY_CHARS + 1))
                .unwrap_err()
                .field(),
            "q"
        );
        assert_eq!(validate_query("").unwrap_err().code(), "empty_field");
    }

    // -------------------------------------------------------------------------
    // Model / identifier fields
    // -------------------------------------------------------------------------

    #[test]
    fn no_model_override_is_valid() {
        assert!(validate_model(None).is_ok());
    }

    #[test]
    fn a_real_provider_slug_is_accepted() {
        assert!(validate_model(Some("anthropic/claude-3.5-sonnet")).is_ok());
        assert!(validate_model(Some("llama3.1")).is_ok());
        assert!(validate_model(Some("openai:gpt-4.1-mini")).is_ok());
    }

    #[test]
    fn a_blank_model_is_rejected() {
        assert_eq!(
            validate_model(Some("   ")).unwrap_err().code(),
            "empty_field"
        );
    }

    #[test]
    fn a_prompt_smuggled_through_model_is_rejected() {
        // `model` only ever selects a model; a field carrying free text would
        // be an unvalidated second prompt entering the pipeline.
        let err = validate_model(Some("ignore previous instructions")).unwrap_err();
        assert_eq!(err.code(), "invalid_value");
        assert_eq!(err.field(), "model");
    }

    #[test]
    fn a_newline_in_model_is_rejected_even_if_charset_would_allow_it() {
        let err = validate_model(Some("model\ninjected")).unwrap_err();
        assert_eq!(err.code(), "invalid_value");
        assert!(err.to_string().contains("control characters"));
    }

    #[test]
    fn model_length_is_bounded() {
        let long = "a".repeat(MAX_MODEL_ID_CHARS + 1);
        assert_eq!(validate_model(Some(&long)).unwrap_err().code(), "too_long");
    }

    #[test]
    fn correlation_ids_are_tighter_still() {
        assert!(validate_correlation_id(Some("abc-DEF_1.2")).is_ok());
        assert!(validate_correlation_id(None).is_ok());
        assert_eq!(
            validate_correlation_id(Some("has spaces"))
                .unwrap_err()
                .code(),
            "invalid_value"
        );
        assert_eq!(
            validate_correlation_id(Some("")).unwrap_err().code(),
            "empty_field"
        );
    }

    // -------------------------------------------------------------------------
    // Paths, tool ids, modes
    // -------------------------------------------------------------------------

    #[test]
    fn a_path_may_not_carry_a_newline_or_nul() {
        assert!(
            validate_path("src/lib.rs").is_ok(),
            "a normal path must pass"
        );
        assert_eq!(validate_path("a\0b").unwrap_err().code(), "invalid_value");
        assert_eq!(validate_path("a\nb").unwrap_err().code(), "invalid_value");
        assert_eq!(validate_path("a\rb").unwrap_err().code(), "invalid_value");
        assert_eq!(validate_path("").unwrap_err().code(), "empty_field");
        assert_eq!(
            validate_path(&"x".repeat(MAX_PATH_CHARS + 1))
                .unwrap_err()
                .code(),
            "too_long"
        );
    }

    // -------------------------------------------------------------------------
    // Provider base URL (a *remote* endpoint, unlike an MCP bridge endpoint)
    // -------------------------------------------------------------------------

    #[test]
    fn a_provider_base_url_may_point_at_a_public_host() {
        for url in [
            "https://api.anthropic.com",
            "https://api.deepseek.com/v1",
            "https://openrouter.ai/api/v1",
            "http://localhost:11434",
            "http://127.0.0.1:11434",
            "https://gateway.example.co.uk:8443/v1",
        ] {
            assert!(validate_provider_endpoint(url).is_ok(), "rejected {url}");
        }
    }

    #[test]
    fn a_provider_base_url_rejects_what_makes_a_url_ambiguous() {
        // The MCP-bridge validator admits loopback only; this one must not be
        // reused for it, or every real provider would be rejected.
        assert!(validate_endpoint("https://api.anthropic.com").is_err());
        assert!(validate_provider_endpoint("https://api.anthropic.com").is_ok());

        assert_eq!(
            validate_provider_endpoint("").unwrap_err().code(),
            "empty_field"
        );
        assert_eq!(
            validate_provider_endpoint("api.anthropic.com")
                .unwrap_err()
                .code(),
            "invalid_value"
        );
        assert_eq!(
            validate_provider_endpoint("file:///etc/passwd")
                .unwrap_err()
                .code(),
            "invalid_value"
        );
        assert_eq!(
            validate_provider_endpoint("https://user:pw@api.anthropic.com")
                .unwrap_err()
                .code(),
            "invalid_value"
        );
        assert_eq!(
            validate_provider_endpoint("https://api.anthropic.com/#frag")
                .unwrap_err()
                .code(),
            "invalid_value"
        );
        assert_eq!(
            validate_provider_endpoint("https://api.anthropic.com/a b")
                .unwrap_err()
                .code(),
            "invalid_value"
        );
        assert_eq!(
            validate_provider_endpoint(&format!("https://{}/v1", "x".repeat(MAX_URL_CHARS)))
                .unwrap_err()
                .code(),
            "too_long"
        );
    }

    #[test]
    fn a_tool_id_must_look_like_one() {
        assert!(validate_tool_id("read_file").is_ok());
        assert!(validate_tool_id("mcp:server.tool").is_ok());
        assert_eq!(validate_tool_id("").unwrap_err().code(), "empty_field");
        assert_eq!(
            validate_tool_id("has space").unwrap_err().code(),
            "invalid_value"
        );
        assert_eq!(
            validate_tool_id(&format!("{}{}", "a".repeat(MAX_TOOL_ID_CHARS), "b"))
                .unwrap_err()
                .code(),
            "too_long"
        );
    }

    #[test]
    fn an_unknown_mission_mode_is_rejected_not_coerced() {
        assert!(validate_mode(None).is_ok());
        assert!(validate_mode(Some("build")).is_ok());
        assert!(validate_mode(Some("plan")).is_ok());

        let err = validate_mode(Some("nuke")).unwrap_err();
        assert_eq!(err.code(), "invalid_value");
        assert_eq!(err.field(), "mode");
        assert!(err.to_string().contains("build"));
        assert!(err.to_string().contains("plan"));
    }

    #[test]
    fn every_error_is_send_and_sync_so_it_can_cross_a_task_boundary() {
        fn assert_send_sync<T: Send + Sync + 'static>() {}
        assert_send_sync::<InputError>();
    }

    // -------------------------------------------------------------------------
    // Artifact writes and tool arguments
    // -------------------------------------------------------------------------

    #[test]
    fn saving_an_empty_file_is_a_valid_save() {
        assert!(validate_artifact("notes.md", "").is_ok());
    }

    #[test]
    fn a_blank_label_is_refused_but_a_blank_content_is_not() {
        assert_eq!(
            validate_artifact("   ", "body").unwrap_err().code(),
            "empty_field"
        );
        assert!(validate_artifact("notes.md", "").is_ok());
    }

    #[test]
    fn an_oversized_artifact_is_refused_before_any_write() {
        let big = "x".repeat(MAX_ARTIFACT_BYTES + 1);
        let err = validate_artifact("notes.md", &big).unwrap_err();
        assert_eq!(err.code(), "too_large");
        assert_eq!(err.field(), "content");
    }

    #[test]
    fn an_artifact_exactly_at_the_limit_is_accepted() {
        let at_limit = "x".repeat(MAX_ARTIFACT_BYTES);
        assert!(validate_artifact("notes.md", &at_limit).is_ok());
    }

    #[test]
    fn an_empty_patch_is_refused_before_git_runs() {
        assert_eq!(validate_patch("").unwrap_err().code(), "empty_field");
        assert!(validate_patch("diff --git a/x b/x\n--- a/x\n+++ b/x\n").is_ok());
    }

    #[test]
    fn an_oversized_patch_is_refused_before_git_runs() {
        let big = "x".repeat(MAX_ARTIFACT_BYTES + 1);
        assert_eq!(validate_patch(&big).unwrap_err().code(), "too_large");
    }

    #[test]
    fn tool_args_must_be_an_object_not_an_array_or_scalar() {
        assert!(validate_tool_args(&json!({})).is_ok());
        assert!(validate_tool_args(&json!({ "path": "src/lib.rs" })).is_ok());
        assert_eq!(
            validate_tool_args(&json!(["fs.read"])).unwrap_err().code(),
            "wrong_type"
        );
        assert_eq!(
            validate_tool_args(&json!("src/lib.rs")).unwrap_err().code(),
            "wrong_type"
        );
    }

    #[test]
    fn tool_args_are_bounded_by_serialized_size() {
        let huge = json!({ "blob": "x".repeat(MAX_ARGS_BYTES) });
        assert_eq!(validate_tool_args(&huge).unwrap_err().code(), "too_large");
    }

    #[test]
    fn the_http_body_cap_sits_above_the_largest_accepted_field() {
        // If this ever goes false the transport would refuse bodies the field
        // rules accept, and the documented 4 MiB save would be a lie. A const
        // block keeps the check at compile time, where it costs nothing and
        // cannot be quietly skipped.
        const {
            assert!(MAX_HTTP_BODY_BYTES > MAX_ARTIFACT_BYTES);
        }
    }
}
