//! Integration tests against a **real, third-party MCP server**.
//!
//! Every other test in this crate talks to `mcp-test-server`, this repository's
//! own fixture binary. That proves the client against our reading of the
//! specification; it does not prove that we can talk to a server somebody else
//! wrote. These tests do. The child process is the published
//! `@modelcontextprotocol/server-filesystem` package (pinned below), started by
//! `npx`, and every assertion is on content that only that server could produce:
//! its identity string, its own tool names, its JSON Schema, its annotations, and
//! bytes read from a real file.
//!
//! They are `#[ignore]`d because they need `node`/`npx` on `PATH` and network
//! access the first time the pinned package is fetched. Once a test starts it
//! never degrades into a skip: a missing `npx`, a handshake that does not
//! complete, a discovered tool list without the expected entries, or a call that
//! returns the wrong bytes all fail it.
//!
//! ```text
//! cargo test -p zylcode-mcp --test mcp_real_server -- --ignored --nocapture
//! ```
//!
//! Override the pinned server with `ZYLCODE_MCP_E2E_SERVER=@scope/name@version`.
//!
//! What this does *not* prove: that the permission gate is consulted, because
//! these tests drive [`McpSession`] directly. The gate is exercised end to end in
//! `tests/mcp_client.rs`, including a refusal that is asserted never to reach the
//! server.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{json, Value};
use zylcode_mcp::mcp_client::config::McpServerConfig;
use zylcode_mcp::mcp_client::session::{ClientInfo, McpSession, SUPPORTED_PROTOCOL_VERSIONS};
use zylcode_mcp::mcp_client::transport::StdioTransport;

/// The server, pinned. A floating `latest` would make this test a report on
/// somebody else's release schedule instead of on our client.
const PINNED_SERVER: &str = "@modelcontextprotocol/server-filesystem@2.3.0";

/// Content that exists nowhere else: if it comes back, a real filesystem read
/// happened.
const MARKER: &str = "zylcode-real-mcp-marker-7f3a91";

/// Content deliberately placed outside the server's allowed roots.
const OUTSIDE_MARKER: &str = "zylcode-outside-root-5c2e08";

/// `npx` may download the package on a cold cache, so the handshake budget is
/// generous. Tool calls keep the per-request budget from the config.
const STARTUP_BUDGET: Duration = Duration::from_secs(120);

fn client() -> ClientInfo {
    ClientInfo {
        name: "zylcode-e2e".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

fn server_spec() -> String {
    std::env::var("ZYLCODE_MCP_E2E_SERVER").unwrap_or_else(|_| PINNED_SERVER.to_string())
}

/// A config that launches the real server, restricted to `root`.
fn config(root: &Path) -> McpServerConfig {
    let mut cfg = McpServerConfig::stdio("real-fs", "npx");
    cfg.args = vec![
        "-y".to_string(),
        server_spec(),
        root.to_string_lossy().to_string(),
    ];
    cfg.timeout_ms = 60_000;
    cfg
}

async fn connect(cfg: &McpServerConfig) -> anyhow::Result<McpSession> {
    let options = cfg.stdio_options()?;
    let transport = Box::new(StdioTransport::spawn(&options)?);
    McpSession::connect(transport, client(), STARTUP_BUDGET).await
}

/// Flattens MCP content blocks into the text they carry.
fn content_text(content: &Value) -> String {
    match content {
        Value::Array(blocks) => blocks
            .iter()
            .filter_map(|b| b.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Two directories: the server's only allowed root, and somewhere outside it.
///
/// Both `TempDir`s are held for the test's lifetime — dropping one deletes the
/// files the assertions read.
struct Fixture {
    _root_dir: tempfile::TempDir,
    _outside_dir: tempfile::TempDir,
    root: PathBuf,
    known: PathBuf,
    outside: PathBuf,
}

fn fixture() -> Fixture {
    let root_dir = tempfile::tempdir().expect("sandbox temp dir");
    let root = root_dir.path().join("root");
    std::fs::create_dir_all(&root).expect("sandbox dir");
    std::fs::write(root.join("known.txt"), format!("{MARKER}\nsecond line\n")).expect("known file");
    std::fs::write(root.join("other.md"), "another file\n").expect("second file");

    let outside_dir = tempfile::tempdir().expect("outside temp dir");
    let outside = outside_dir.path().join("secret.txt");
    std::fs::write(&outside, format!("{OUTSIDE_MARKER}\n")).expect("outside content");

    // Canonicalise so the paths asserted on are the paths the server sees; on
    // Windows the temp directory carries an 8.3 short name.
    let known = root.join("known.txt").canonicalize().expect("known path");
    Fixture {
        root: root.canonicalize().expect("canonical root"),
        known,
        outside: outside.canonicalize().expect("canonical outside"),
        _root_dir: root_dir,
        _outside_dir: outside_dir,
    }
}

// ---------------------------------------------------------------------------
// Handshake, discovery, execution
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "spawns npx and a real MCP server; needs node + network"]
async fn real_server_initializes_discovers_tools_and_executes_a_read() {
    let f = fixture();
    let cfg = config(&f.root);

    let mut session = connect(&cfg)
        .await
        .expect("a real MCP server must accept our initialize");

    // ---- the server is somebody else's program -------------------------
    let handshake = session.handshake();
    assert!(
        handshake
            .server_info
            .name
            .to_lowercase()
            .contains("filesystem"),
        "expected the published filesystem server, got {:?}",
        handshake.server_info
    );
    assert!(
        SUPPORTED_PROTOCOL_VERSIONS.contains(&handshake.protocol_version.as_str()),
        "negotiated a version this client does not implement: {}",
        handshake.protocol_version
    );

    // ---- discovery ------------------------------------------------------
    let tools = session
        .list_all_tools()
        .await
        .expect("tools/list must succeed against a real server");
    assert!(
        tools.len() >= 5,
        "the filesystem server publishes more than five tools, got {}",
        tools.len()
    );

    let read = tools
        .iter()
        .find(|t| t.name == "read_text_file")
        .unwrap_or_else(|| {
            panic!(
                "read_text_file missing from discovery; got {:?}",
                tools.iter().map(|t| &t.name).collect::<Vec<_>>()
            )
        });
    assert!(
        !read.input_schema.is_null(),
        "the server's JSON Schema must be retained, not discarded"
    );
    assert!(
        read.annotations.is_some(),
        "the server publishes annotations for read_text_file; dropping them would \
         lose the read/write distinction the permission gate depends on"
    );
    assert!(tools.iter().any(|t| t.name == "list_directory"));

    // ---- execution: a real file, read by a real server -----------------
    let outcome = session
        .call_tool(
            "read_text_file",
            json!({ "path": f.known.to_string_lossy() }),
        )
        .await
        .expect("tools/call must succeed");
    assert!(!outcome.is_error, "read failed: {:?}", outcome.content);
    let text = content_text(&outcome.content);
    assert!(
        text.contains(MARKER),
        "the returned content is not the file's content: {text}"
    );

    // ---- a second, different tool on the same session ------------------
    let listing = session
        .call_tool(
            "list_directory",
            json!({ "path": f.root.to_string_lossy() }),
        )
        .await
        .expect("list_directory must succeed");
    assert!(!listing.is_error, "listing failed: {:?}", listing.content);
    let listing_text = content_text(&listing.content);
    assert!(
        listing_text.contains("known.txt"),
        "the listing must name the fixture file: {listing_text}"
    );

    // ---- clean close ----------------------------------------------------
    session
        .shutdown()
        .await
        .expect("the session must close cleanly");
}

// ---------------------------------------------------------------------------
// The server's own sandbox, and recovery after a refusal
// ---------------------------------------------------------------------------

#[tokio::test]
#[ignore = "spawns npx and a real MCP server; needs node + network"]
async fn real_server_refuses_a_path_outside_its_roots_and_stays_usable() {
    let f = fixture();
    let cfg = config(&f.root);
    let mut session = connect(&cfg).await.expect("handshake must succeed");

    // The server was started with exactly one allowed root, so a sibling file is
    // outside it. Either the call errors or the server reports isError — both are
    // honest; leaking the bytes is not.
    let attempt = session
        .call_tool(
            "read_text_file",
            json!({ "path": f.outside.to_string_lossy() }),
        )
        .await;

    match attempt {
        Err(error) => {
            let message = error.to_string();
            assert!(
                !message.contains(OUTSIDE_MARKER),
                "an error must not carry the file's contents: {message}"
            );
        }
        Ok(outcome) => {
            let text = content_text(&outcome.content);
            assert!(
                outcome.is_error,
                "a read outside the allowed roots must be an error, not content: {text}"
            );
            assert!(
                !text.contains(OUTSIDE_MARKER),
                "the server leaked content from outside its roots: {text}"
            );
        }
    }

    // A refusal must not desynchronise the session: the same connection still
    // serves a legitimate call.
    let outcome = session
        .call_tool(
            "read_text_file",
            json!({ "path": f.known.to_string_lossy() }),
        )
        .await
        .expect("the session must survive a refused call");
    assert!(!outcome.is_error);
    assert!(content_text(&outcome.content).contains(MARKER));

    // ---- unknown tool: the server's error surfaces, not a hang ---------
    let error = session
        .call_tool("definitely_not_a_tool", json!({}))
        .await
        .expect_err("an unknown tool must fail");
    assert!(
        !error.to_string().is_empty(),
        "the server's diagnostic must be preserved"
    );

    session.shutdown().await.expect("clean close");
}
