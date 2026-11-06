//! Deterministic loopback HTTP server for provider streaming tests.
//!
//! Ordinary CI must not depend on a real provider or the public internet, so these
//! tests speak the real wire protocol to `127.0.0.1` instead. The server is
//! deliberately dumb: it reads a request head, then writes whatever the scenario
//! tells it to write, in whatever pieces it is told, with an explicit flush between
//! pieces so client-side chunking is exercised.
//!
//! This is not a mock of the transport — `reqwest`, HTTP framing, socket delivery,
//! and connection teardown are all real. Only the provider is local.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

/// What the server should do after receiving one request.
pub enum Scenario {
    /// Write these byte slices, flushing between each, then close.
    Body(Vec<Vec<u8>>),
    /// Write these pieces, then keep the connection open until the client goes away.
    ///
    /// Used to prove cancellation aborts the request: the server reports through the
    /// returned receiver as soon as a write fails or the socket reports EOF.
    BodyThenHold(Vec<Vec<u8>>),
}

/// A running loopback server for one request.
pub struct TestServer {
    pub url: String,
    /// Set for [`Scenario::BodyThenHold`]: `true` once the client disconnected.
    pub disconnected: Receiver<bool>,
}

impl TestServer {
    /// Start a server that answers exactly one request on an ephemeral port.
    pub fn start(scenario: Scenario) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let port = listener.local_addr().expect("local addr").port();
        let (tx, rx): (Sender<bool>, Receiver<bool>) = channel();

        std::thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
            read_request_head(&mut stream);

            match scenario {
                Scenario::Body(pieces) => {
                    write_head(&mut stream, "200 OK", "text/event-stream");
                    for piece in pieces {
                        if stream.write_all(&piece).is_err() {
                            break;
                        }
                        let _ = stream.flush();
                        std::thread::sleep(Duration::from_millis(5));
                    }
                }
                Scenario::BodyThenHold(pieces) => {
                    write_head(&mut stream, "200 OK", "text/event-stream");
                    for piece in pieces {
                        if stream.write_all(&piece).is_err() {
                            let _ = tx.send(true);
                            return;
                        }
                        let _ = stream.flush();
                    }
                    // Keep pushing data until the peer closes. A successful write to a
                    // closed socket eventually fails; a read returning Ok(0) means the
                    // peer closed cleanly.
                    for _ in 0..200 {
                        std::thread::sleep(Duration::from_millis(10));
                        let heartbeat = b": keep-alive\n\n".to_vec();
                        if stream.write_all(&heartbeat).is_err() || stream.flush().is_err() {
                            let _ = tx.send(true);
                            return;
                        }
                        let mut probe = [0u8; 1];
                        match stream.read(&mut probe) {
                            Ok(0) => {
                                let _ = tx.send(true);
                                return;
                            }
                            Ok(_) => {}
                            Err(_) => {
                                let _ = tx.send(true);
                                return;
                            }
                        }
                    }
                    let _ = tx.send(false);
                }
            }
        });

        Self {
            url: format!("http://127.0.0.1:{port}"),
            disconnected: rx,
        }
    }

    /// Start a server that answers with an error status and body.
    pub fn start_error(status: &'static str, body: &'static str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let port = listener.local_addr().expect("local addr").port();
        // Error responses complete immediately, so nothing reports a disconnect.
        let (_tx, rx) = channel();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                read_request_head(&mut stream);
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });
        Self {
            url: format!("http://127.0.0.1:{port}"),
            disconnected: rx,
        }
    }
}

fn read_request_head(stream: &mut std::net::TcpStream) {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while stream.read(&mut byte).unwrap_or(0) == 1 {
        head.push(byte[0]);
        if head.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    // Drain the body so the client is not blocked writing it.
    let text = String::from_utf8_lossy(&head).to_string();
    let content_length = text
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("content-length") {
                value.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0);
    let mut remaining = vec![0u8; content_length];
    let mut read = 0;
    while read < content_length {
        match stream.read(&mut remaining[read..]) {
            Ok(0) => break,
            Ok(n) => read += n,
            Err(_) => break,
        }
    }
}

fn write_head(stream: &mut std::net::TcpStream, status: &str, content_type: &str) {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n"
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.flush();
}

/// Build one SSE `data:` frame from a JSON value.
pub fn sse_json(value: &serde_json::Value) -> Vec<u8> {
    format!("data: {value}\n\n").into_bytes()
}

/// Build one SSE `data:` frame from a raw string.
pub fn sse_data(data: &str) -> Vec<u8> {
    format!("data: {data}\n\n").into_bytes()
}
