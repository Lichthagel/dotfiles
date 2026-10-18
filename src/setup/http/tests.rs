use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::thread::{self, JoinHandle};
use std::time::Instant;

use super::*;

fn serve(responses: Vec<Vec<u8>>) -> (String, JoinHandle<Vec<String>>) {
    serve_with_body_delay(responses, Duration::ZERO)
}

fn serve_with_body_delay(
    responses: Vec<Vec<u8>>,
    delay: Duration,
) -> (String, JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let handle = thread::spawn(move || {
        let mut requests = Vec::new();
        for response in responses {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "HTTP fixture timed out");
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("accept fixture connection: {error}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = String::new();
            let mut reader = BufReader::new(&stream);
            loop {
                let mut line = String::new();
                assert_ne!(reader.read_line(&mut line).unwrap(), 0);
                request.push_str(&line);
                if line == "\r\n" {
                    break;
                }
            }
            requests.push(request);
            let body_start = response
                .windows(4)
                .position(|part| part == b"\r\n\r\n")
                .unwrap()
                + 4;
            // Oversized responses may be rejected before the server finishes writing.
            let _ = stream.write_all(&response[..body_start]);
            thread::sleep(delay);
            let _ = stream.write_all(&response[body_start..]);
        }
        requests
    });
    (url, handle)
}

fn response(headers: &str, body: &[u8]) -> Vec<u8> {
    let mut response =
        format!("HTTP/1.1 200 OK\r\nConnection: close\r\n{headers}\r\n").into_bytes();
    response.extend_from_slice(body);
    response
}

fn client() -> UreqHttpClient {
    client_with_timeout(Duration::from_secs(60))
}

fn client_with_timeout(timeout: Duration) -> UreqHttpClient {
    let defaults = UreqHttpClient::new();
    let config = defaults.client.config();
    UreqHttpClient {
        client: ureq::Agent::config_builder()
            .proxy(None)
            .tls_config(config.tls_config().clone())
            .timeout_connect(config.timeouts().connect)
            .timeout_global(Some(timeout))
            .max_redirects(config.max_redirects())
            .user_agent(config.user_agent().clone())
            .build()
            .into(),
    }
}

#[test]
fn defaults_preserve_timeouts_redirects_and_tls_verification() {
    let client = UreqHttpClient::new();
    let config = client.client.config();
    assert_eq!(config.timeouts().connect, Some(Duration::from_secs(10)));
    assert_eq!(config.timeouts().global, Some(Duration::from_secs(60)));
    assert_eq!(config.max_redirects(), 10);
    assert!(config.max_redirects_will_error());
    assert!(config.http_status_as_error());
    assert!(!config.tls_config().disable_verification());
    assert!(config.tls_config().use_sni());
    assert_eq!(
        config.tls_config().provider(),
        ureq::tls::TlsProvider::NativeTls
    );
    assert!(matches!(
        config.tls_config().root_certs(),
        ureq::tls::RootCerts::PlatformVerifier
    ));
}

#[test]
fn downloads_json_and_preserves_the_user_agent() {
    let (url, server) = serve(vec![response("Content-Length: 11\r\n", br#"{"ok":true}"#)]);
    let value = client().get_json(&url).unwrap();
    let requests = server.join().unwrap();
    assert_eq!(value, serde_json::json!({"ok": true}));
    assert!(requests[0]
        .to_ascii_lowercase()
        .contains("user-agent: dotfiles-rust-installer\r\n"));
}

#[test]
fn rejects_invalid_json() {
    let (url, server) = serve(vec![response("Content-Length: 1\r\n", b"{")]);
    let error = client().get_json(&url).unwrap_err();
    server.join().unwrap();
    assert!(error.to_string().contains("parse JSON"));
}

#[test]
fn follows_relative_redirects() {
    let (url, server) = serve(vec![
        b"HTTP/1.1 302 Found\r\nLocation: /theme.json\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            .to_vec(),
        response("Content-Length: 2\r\n", b"{}"),
    ]);
    let bytes = client().get_bytes(&url).unwrap();
    let requests = server.join().unwrap();
    assert_eq!(bytes, b"{}");
    assert!(requests[1].starts_with("GET /theme.json HTTP/1.1\r\n"));
}

#[test]
fn rejects_redirect_loops() {
    let redirect =
        b"HTTP/1.1 302 Found\r\nLocation: /loop\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    let (url, server) = serve(vec![redirect.to_vec(); 11]);
    let error = client().get_bytes(&url).unwrap_err();
    server.join().unwrap();
    assert!(error.to_string().contains("redirect"));
}

#[test]
fn rejects_http_error_statuses() {
    for status in ["404 Not Found", "500 Internal Server Error"] {
        let (url, server) = serve(vec![format!(
            "HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )
        .into_bytes()]);
        let error = client().get_bytes(&url).unwrap_err();
        server.join().unwrap();
        assert!(error.to_string().contains(&status[..3]));
    }
}

#[test]
fn accepts_a_body_exactly_at_the_limit() {
    let body = vec![b' '; MAX_THEME_FILE_BYTES];
    let headers = format!("Content-Length: {}\r\n", body.len());
    let (url, server) = serve(vec![response(&headers, &body)]);
    let bytes = client().get_bytes(&url).unwrap();
    server.join().unwrap();
    assert_eq!(bytes, body);
}

#[test]
fn rejects_oversized_content_length_without_reading_the_body() {
    let headers = format!("Content-Length: {}\r\n", MAX_THEME_FILE_BYTES + 1);
    let (url, server) = serve(vec![response(&headers, b"")]);
    let error = client().get_bytes(&url).unwrap_err();
    server.join().unwrap();
    assert!(error.to_string().contains("exceeds the 1048576-byte limit"));
}

#[test]
fn rejects_oversized_bodies_without_content_length() {
    let body = vec![b' '; MAX_THEME_FILE_BYTES + 1];
    let (url, server) = serve(vec![response("", &body)]);
    let error = client().get_bytes(&url).unwrap_err();
    server.join().unwrap();
    assert!(error.to_string().contains("exceeds the 1048576-byte limit"));
}

#[test]
fn rejects_oversized_chunked_json() {
    let mut chunks = format!("{:x}\r\n", MAX_THEME_TREE_BYTES + 1).into_bytes();
    chunks.extend(vec![b' '; MAX_THEME_TREE_BYTES + 1]);
    chunks.extend_from_slice(b"\r\n0\r\n\r\n");
    let (url, server) = serve(vec![response("Transfer-Encoding: chunked\r\n", &chunks)]);
    let error = client().get_json(&url).unwrap_err();
    server.join().unwrap();
    assert!(error.to_string().contains("exceeds the 1048576-byte limit"));
}

#[test]
fn global_timeout_includes_reading_the_body() {
    let (url, server) = serve_with_body_delay(
        vec![response("Content-Length: 2\r\n", b"{}")],
        Duration::from_millis(750),
    );
    let client = client_with_timeout(Duration::from_millis(250));
    let error = client.get_bytes(&url).unwrap_err();
    server.join().unwrap();
    assert!(error.to_string().contains("read response"));
    assert!(error.to_string().to_ascii_lowercase().contains("timeout"));
}

#[test]
fn rejects_truncated_bodies() {
    let (url, server) = serve(vec![response("Content-Length: 10\r\n", b"{}")]);
    let error = client().get_bytes(&url).unwrap_err();
    server.join().unwrap();
    assert!(error.to_string().contains("read response"));
}
