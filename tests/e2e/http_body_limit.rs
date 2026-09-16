//! Regression: the HTTP server rejects request bodies larger than 10 MB
//! with `413` before the handler runs (4.4 fix). Declared Content-Length is
//! rejected without reading the body; a body within the limit is served.

use crate::common;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

const LIMIT: usize = 10 * 1024 * 1024; // server.rs: MAX_REQUEST_BODY_BYTES
const CANDIDATE_PORTS: [u16; 5] = [47613, 47623, 47633, 47643, 47653];

fn server_source(port: u16) -> String {
    format!(
        r#"
fiber handle(json: req -> json) {{
    yield net.respond(200, <<< {{"ok": true}} >>>);
    return <<< {{}} >>>;
}};

serve: app {{
    host = "127.0.0.1",
    port = {port},
    routes = ["POST /upload" :: handle, "*" :: handle]
}};
"#
    )
}

/// Spawns the server on the first candidate port it can actually bind and
/// serve on. Kills and retries the next port if the process exits early.
fn spawn_server(name: &str) -> (std::process::Child, u16) {
    for &port in &CANDIDATE_PORTS {
        let (_file, mut child) = common::spawn_program(name, &server_source(port));
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut ready = false;
        while Instant::now() < deadline {
            if let Some(_status) = child.try_wait().expect("poll server") {
                break; // exited early — port was taken by something else
            }
            if TcpStream::connect_timeout(
                &format!("127.0.0.1:{port}").parse().unwrap(),
                Duration::from_millis(250),
            )
            .is_ok()
            {
                ready = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if ready {
            return (child, port);
        }
        let _ = child.kill();
        let _ = child.wait();
    }
    panic!("could not start the regression HTTP server on any candidate port");
}

fn post(port: u16, content_length: usize, body_bytes_to_send: usize) -> (u16, String) {
    let mut stream =
        TcpStream::connect(("127.0.0.1", port)).expect("connect to regression server");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(10)))
        .unwrap();

    let head = format!(
        "POST /upload HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {content_length}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(head.as_bytes()).unwrap();
    let chunk = [b'x'; 4096];
    let mut sent = 0usize;
    while sent < body_bytes_to_send {
        let n = chunk.len().min(body_bytes_to_send - sent);
        stream.write_all(&chunk[..n]).unwrap();
        sent += n;
    }

    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);
    let status = response
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);
    (status, response)
}

#[test]
fn oversized_body_is_rejected_with_413_before_handler() {
    let (mut child, port) = spawn_server("http_413_over");
    let (status, body) = post(port, LIMIT + 1, 1024);
    let _ = child.kill();
    let _ = child.wait();

    assert_eq!(
        status, 413,
        "a body 1 byte over the 10 MB limit must be rejected with 413"
    );
    assert!(
        body.contains("payload too large"),
        "expected the documented rejection body, got: {body}"
    );
}

#[test]
fn body_within_limit_is_served() {
    let (mut child, port) = spawn_server("http_413_under");
    let (status, body) = post(port, 1024, 1024);
    let _ = child.kill();
    let _ = child.wait();

    assert_eq!(
        status, 200,
        "a 1 KB body must be served normally, got: {body}"
    );
    assert!(body.contains("\"ok\": true") || body.contains("\"ok\":true"));
}
