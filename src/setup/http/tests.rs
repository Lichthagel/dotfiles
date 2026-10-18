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
            // Windows may carry the listener's nonblocking mode to accepted sockets.
            stream.set_nonblocking(false).unwrap();
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

fn client() -> CurlHttpClient {
    CurlHttpClient::new()
}

fn client_with_timeout(timeout: Duration) -> CurlHttpClient {
    CurlHttpClient {
        timeout,
        ..client()
    }
}

#[test]
fn command_preserves_download_policy_without_a_shell() {
    let url = "https://example.test/{theme}.json?x=1&y=$(echo unsafe)";
    let command = client().command(url, MAX_THEME_FILE_BYTES);
    assert_eq!(
        command.get_program(),
        if cfg!(windows) { "curl.exe" } else { "curl" }
    );
    let arguments: Vec<_> = command
        .get_args()
        .map(|arg| arg.to_str().unwrap())
        .collect();
    assert_eq!(
        arguments,
        [
            "--disable",
            "--silent",
            "--show-error",
            "--fail",
            "--location",
            "--max-redirs",
            "10",
            "--connect-timeout",
            "10",
            "--globoff",
            "--proto",
            "=http,https",
            "--proto-redir",
            "=http,https",
            "--user-agent",
            "dotfiles-rust-installer",
            "--max-time",
            "60.000",
            "--max-filesize",
            "1048576",
            "--url",
            url,
        ]
    );
}

#[test]
fn diagnostics_are_bounded_and_the_remaining_stream_is_drained() {
    let mut reader = io::Cursor::new(vec![b'x'; MAX_DIAGNOSTIC_BYTES as usize * 2]);
    let bytes = read_diagnostics(&mut reader);
    assert_eq!(bytes, vec![b'x'; MAX_DIAGNOSTIC_BYTES as usize]);
    assert_eq!(reader.position(), MAX_DIAGNOSTIC_BYTES * 2);
}

#[test]
fn missing_curl_returns_an_actionable_setup_error() {
    let client = CurlHttpClient {
        program: "dotfiles-test-nonexistent-curl-command".into(),
        ..client()
    };
    let error = client
        .get_bytes("https://example.test/theme.json")
        .unwrap_err();
    assert!(matches!(error, InstallError::Setup(_)));
    assert!(error.to_string().contains("install curl"));
}

#[test]
fn rejects_non_http_protocols() {
    let error = client()
        .get_bytes("file:///dotfiles-test-not-a-theme")
        .unwrap_err();
    assert!(error.to_string().contains("curl failed"));
}

#[test]
fn does_not_expand_url_globs() {
    let (url, server) = serve(vec![response("Content-Length: 2\r\n", b"{}")]);
    let bytes = client()
        .get_bytes(&format!("{url}/{{one,two}}.json"))
        .unwrap();
    let requests = server.join().unwrap();
    assert_eq!(bytes, b"{}");
    assert_eq!(requests.len(), 1);
    assert!(requests[0].starts_with("GET /{one,two}.json HTTP/1.1\r\n"));
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

#[cfg(unix)]
#[test]
fn rust_body_limit_kills_and_reaps_a_downloader_that_ignores_size_flags() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempfile::tempdir().unwrap();
    let program = directory.path().join("fake-curl");
    std::fs::write(&program, "#!/bin/sh\nprintf '0123456789'\nexec sleep 5\n").unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    let client = CurlHttpClient {
        program,
        ..client()
    };
    let started = Instant::now();
    let error = client
        .get_limited("https://example.test/theme.json", 8)
        .unwrap_err();
    assert!(error.to_string().contains("exceeds the 8-byte limit"));
    assert!(started.elapsed() < Duration::from_secs(3));
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
    assert!(error.to_string().contains("curl failed"));
    assert!(error.to_string().to_ascii_lowercase().contains("timed out"));
}

#[test]
fn rejects_truncated_bodies() {
    let (url, server) = serve(vec![response("Content-Length: 10\r\n", b"{}")]);
    let error = client().get_bytes(&url).unwrap_err();
    server.join().unwrap();
    assert!(error.to_string().contains("curl failed"));
    assert!(error.to_string().contains("18"));
}
