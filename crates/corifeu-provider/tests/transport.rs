//! The HTTP client, against a loopback server this test starts.

use corifeu_provider::http::{Error, get, post_json};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

/// Serves one request with `reply`, then stops. Returns the URL to post to.
fn serving(reply: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/api/chat", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut seen = Vec::new();
        let mut buffer = [0; 4096];
        // Read until the body the Content-Length announced has arrived.
        loop {
            let read = stream.read(&mut buffer).unwrap();
            seen.extend_from_slice(&buffer[..read]);
            let text = String::from_utf8_lossy(&seen);
            if let Some((head, body)) = text.split_once("\r\n\r\n") {
                let want: usize = head
                    .lines()
                    .find_map(|l| l.strip_prefix("Content-Length: "))
                    .and_then(|v| v.trim().parse().ok())
                    .unwrap_or(0);
                if body.len() >= want {
                    break;
                }
            }
            if read == 0 {
                break;
            }
        }
        stream.write_all(reply.as_bytes()).unwrap();
    });
    url
}

#[test]
fn a_content_length_reply_is_read_whole() {
    let url = serving(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 17\r\n\r\n{\"message\":\"ok\"}\n",
    );
    let body = post_json(&url, r#"{"model":"x"}"#, Duration::from_secs(5)).unwrap();
    assert_eq!(body.trim(), r#"{"message":"ok"}"#);
}

/// Ollama answers with `Content-Length`; llama.cpp's server does not always.
#[test]
fn a_chunked_reply_is_reassembled() {
    // `{"a":1,"b":2}` split 9 + 4, then the zero-length terminator.
    let url = serving(
        "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n\
         9\r\n{\"a\":1,\"b\r\n\
         4\r\n\":2}\r\n\
         0\r\n\r\n",
    );
    let body = post_json(&url, "{}", Duration::from_secs(5)).unwrap();
    assert_eq!(body, r#"{"a":1,"b":2}"#);
}

#[test]
fn an_error_status_carries_the_servers_own_explanation() {
    let url = serving(
        "HTTP/1.1 404 Not Found\r\nContent-Length: 29\r\n\r\n{\"error\":\"model not found\"}\r\n",
    );
    let failed = post_json(&url, "{}", Duration::from_secs(5)).unwrap_err();

    let message = failed.to_string();
    assert!(message.contains("404"), "{message}");
    assert!(
        message.contains("model not found"),
        "a status with no body leaves the user guessing: {message}"
    );
}

#[test]
fn https_is_refused_with_the_reason_rather_than_a_connection_error() {
    let failed = post_json("https://api.example.com/v1", "{}", Duration::from_secs(1)).unwrap_err();
    assert!(matches!(failed, Error::NotPlainHttp(_)));
    assert!(failed.to_string().contains("loopback"));
}

#[test]
fn an_unreachable_port_names_the_host() {
    // Port 1 on loopback: nothing listens, and the refusal is immediate.
    let failed =
        post_json("http://127.0.0.1:1/api/chat", "{}", Duration::from_secs(2)).unwrap_err();
    assert!(failed.to_string().contains("127.0.0.1:1"));
}

#[test]
fn a_get_asks_a_router_what_it_serves() {
    let url = serving(
        "HTTP/1.1 200 OK\r\nContent-Length: 34\r\n\r\n{\"data\":[{\"id\":\"qwopus\"}]}\r\n\r\n\r\n\r\n",
    );
    let body = get(&url, Duration::from_secs(5)).unwrap();
    assert!(body.contains("qwopus"), "{body}");
}
