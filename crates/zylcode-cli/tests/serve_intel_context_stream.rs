// P15 W4 — the streaming observation surface, exercised against the **real**
// `zylcode serve-intel` binary over its real HTTP routes:
//
//   * `GET /api/repo-context` walks the honest state machine on a real
//     repository: fresh (after the warm index) → stale (a real content
//     change, counted) → fresh again (a real query performs the rebuild);
//   * `GET /api/repo-context-stream` emits an initial `state` event, emits a
//     `stale` event when the tree changes underneath it, and stays **silent**
//     while nothing changes — transitions only, no repeat spam;
//   * the banner must announce the *bound* port (the test relies on
//     `--port 0`, which only works because the printed address is the real
//     one).
//
// Nothing here is simulated: a spawned server, real files on disk, real
// HTTP, server-sent events.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use futures::StreamExt;
use tokio::time::{sleep, timeout, Instant};

/// The spawned `serve-intel` process; killed and reaped on drop.
struct Server {
    child: Child,
    base: String,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Spawn `zylcode serve-intel --repo <root> --port 0` and read the bound
/// address back off stdout.
fn start_server(repo: &Path) -> Server {
    let mut child = Command::new(env!("CARGO_BIN_EXE_zylcode"))
        .args([
            "serve-intel",
            "--repo",
            &repo.to_string_lossy(),
            "--port",
            "0",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("serve-intel must start");

    let stdout = child.stdout.take().expect("stdout is piped");
    let reader = BufReader::new(stdout);
    let mut base = None;
    for line in reader.lines() {
        let Ok(line) = line else { break };
        if let Some(rest) = line.split_once("http://") {
            let addr = rest.1.split_whitespace().next().unwrap_or_default();
            if addr.contains(':') {
                base = Some(format!("http://{addr}"));
                break;
            }
        }
    }
    let base = base.expect("the banner must announce the bound address");
    Server { child, base }
}

async fn get_json(client: &reqwest::Client, url: &str) -> serde_json::Value {
    let res = match client.get(url).send().await {
        Ok(res) => res,
        Err(e) => return serde_json::json!({ "transport_error": e.to_string() }),
    };
    let status = res.status().as_u16();
    let body = res.text().await.unwrap_or_default();
    match serde_json::from_str::<serde_json::Value>(&body) {
        Ok(mut value) => {
            if let Some(obj) = value.as_object_mut() {
                obj.entry("http").or_insert(status.into());
            }
            value
        }
        Err(_) => serde_json::json!({ "http": status, "raw": body }),
    }
}

/// Poll `path` until its `state` equals `want` — or panic with the last
/// observation, never silently pass.
async fn wait_for_state(
    client: &reqwest::Client,
    base: &str,
    path: &str,
    want: &str,
    within: Duration,
) -> serde_json::Value {
    let deadline = Instant::now() + within;
    let mut last = serde_json::Value::Null;
    while Instant::now() < deadline {
        last = get_json(client, &format!("{base}{path}")).await;
        if last["state"] == want {
            return last;
        }
        sleep(Duration::from_millis(200)).await;
    }
    panic!("`{path}` never reached `{want}` within {within:?}; last observation: {last}");
}

/// Drain the next `state`-bearing SSE frame from the buffer/stream pair.
async fn next_state_opt<S, B>(
    stream: &mut S,
    buf: &mut String,
    within: Duration,
) -> Option<serde_json::Value>
where
    S: futures::Stream<Item = reqwest::Result<B>> + Unpin,
    B: AsRef<[u8]>,
{
    let deadline = Instant::now() + within;
    loop {
        // Complete frames only: an event is everything up to the blank line.
        while let Some(idx) = buf.find("\n\n") {
            let frame: String = buf.drain(..idx + 2).collect();
            for line in frame.lines() {
                let Some(data) = line
                    .strip_prefix("data: ")
                    .or_else(|| line.strip_prefix("data:"))
                else {
                    continue;
                };
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(data) {
                    if value.get("state").is_some() {
                        return Some(value);
                    }
                }
            }
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return None;
        }
        match timeout(remaining, stream.next()).await {
            Ok(Some(Ok(chunk))) => buf.push_str(&String::from_utf8_lossy(chunk.as_ref())),
            Ok(Some(Err(e))) => panic!("the stream failed mid-flight: {e}"),
            Ok(None) => panic!("the stream ended without another event; buffer: {buf:?}"),
            Err(_) => return None,
        }
    }
}

fn write_readme(root: &Path, body: &str) {
    std::fs::write(root.join("README.md"), body).expect("README must be writable");
}

#[tokio::test]
async fn the_context_stream_reports_fresh_stale_and_back_again() {
    let dir = tempfile::tempdir().unwrap();
    let root: PathBuf = dir.path().to_path_buf();
    std::fs::create_dir(root.join("src")).unwrap();
    std::fs::write(root.join("src/lib.rs"), "pub fn alpha() {}\n").unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"x\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    write_readme(&root, "# x\n");

    let server = start_server(&root);
    let client = reqwest::Client::new();

    // Readiness: the service answers at all.
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let health = get_json(&client, &format!("{}/healthz", server.base)).await;
        if health["status"] == "ok" {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "serve-intel never became healthy: {health}"
        );
        sleep(Duration::from_millis(200)).await;
    }

    // 1. The warm index settles: fresh, both hashes agreeing, zero changes.
    let fresh = wait_for_state(
        &client,
        &server.base,
        "/api/repo-context",
        "fresh",
        Duration::from_secs(30),
    )
    .await;
    assert_eq!(fresh["changed_files"], 0, "{fresh}");
    let first_fingerprint = fresh["fingerprint"].as_str().expect("hash").to_string();
    assert_eq!(
        first_fingerprint.len(),
        12,
        "short hash, full stop: {fresh}"
    );
    assert_eq!(
        fresh["fingerprint"], fresh["indexed_fingerprint"],
        "fresh means measured == indexed: {fresh}"
    );

    // 2. A real content change: stale, hashes diverge, one change counted.
    write_readme(&root, "# x\n\nreal content changed\n");
    let stale = wait_for_state(
        &client,
        &server.base,
        "/api/repo-context",
        "stale",
        Duration::from_secs(10),
    )
    .await;
    assert_ne!(
        stale["fingerprint"], stale["indexed_fingerprint"],
        "the tree changed, so the hashes must differ: {stale}"
    );
    assert_eq!(stale["changed_files"], 1, "{stale}");

    // 3. A real query performs the rebuild; the standing returns to fresh
    //    under the *new* fingerprint (and this repository's rebuild goes
    //    through the reindex guard the stream observes).
    let query = client
        .get(format!(
            "{}/api/repo-query?method=architecture",
            server.base
        ))
        .send()
        .await
        .expect("query response");
    assert!(
        query.status().is_success(),
        "the rebuild query must run: {}",
        query.status()
    );
    let fresh_again = wait_for_state(
        &client,
        &server.base,
        "/api/repo-context",
        "fresh",
        Duration::from_secs(10),
    )
    .await;
    assert_eq!(fresh_again["changed_files"], 0, "{fresh_again}");
    assert_ne!(
        fresh_again["fingerprint"], first_fingerprint,
        "the new index covers the changed tree: {fresh_again}"
    );

    // 4. The stream: an initial state event, then a `stale` event the moment
    //    the tree changes — and silence while nothing changes.
    let response = client
        .get(format!("{}/api/repo-context-stream", server.base))
        .header(reqwest::header::ACCEPT, "text/event-stream")
        .send()
        .await
        .expect("stream response");
    assert_eq!(response.status().as_u16(), 200);
    let mut stream = response.bytes_stream();
    let mut buf = String::new();

    let initial = next_state_opt(&mut stream, &mut buf, Duration::from_secs(15))
        .await
        .expect("the stream must open with its current state");
    assert_eq!(
        initial["state"], "fresh",
        "nothing is happening yet: {initial}"
    );

    write_readme(&root, "# x\n\nreal content changed, a second time\n");
    let transition = next_state_opt(&mut stream, &mut buf, Duration::from_secs(15))
        .await
        .expect("a content change must surface as a transition");
    assert_eq!(transition["state"], "stale", "{transition}");
    assert_ne!(
        transition["fingerprint"], initial["fingerprint"],
        "the fingerprint transition must be visible: {transition}"
    );

    // Standing unchanged → no further events. Two probe ticks of quiet prove
    // the stream emits transitions, not periodic repeats.
    let quiet = next_state_opt(&mut stream, &mut buf, Duration::from_millis(1600)).await;
    assert_eq!(
        quiet, None,
        "an unchanged standing must not re-emit: {quiet:?}"
    );
}
