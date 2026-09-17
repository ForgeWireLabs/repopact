//! Decision 0062: a one-shot native loopback HTTP listener for the desktop
//! authorization-code callback. Never a general HTTP server -- it accepts
//! exactly one terminal callback at exactly one path, bounded in request
//! size, bound only to `127.0.0.1` (never `0.0.0.0`), and shuts down
//! immediately after resolving (success, rejection, cancellation, or
//! timeout).
//!
//! `state` validation happens one layer up
//! (`provider::GitHubProvider`/its authorization session), not here: this
//! module's job is transport only -- accept a request at the expected path,
//! extract `code`/`state`/`error` from the query string, and respond with a
//! minimal, secret-free HTML page. It never echoes `code`, `state`, a
//! token, or a GitHub API response back into that page.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

pub const CALLBACK_PATH: &str = "/repopact/github/callback";
/// Generous enough for a real browser's request line + headers + a query
/// string carrying a `code`/`state` pair; small enough that nothing but a
/// deliberately hostile request would ever approach it.
const MAX_REQUEST_BYTES: usize = 8 * 1024;
const POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawCallback {
    /// Raw, not-yet-validated query parameters exactly as received.
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallbackWaitOutcome {
    Received(RawCallback),
    Cancelled,
    TimedOut,
}

pub struct CallbackListener {
    listener: TcpListener,
    port: u16,
}

impl CallbackListener {
    /// Binds an ephemeral loopback-only port. Never `0.0.0.0` -- a
    /// non-loopback bind would accept a callback from another host on the
    /// network, which this flow must never do.
    pub fn bind() -> std::io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();
        Ok(Self { listener, port })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn redirect_uri(&self) -> String {
        format!("http://127.0.0.1:{}{}", self.port, CALLBACK_PATH)
    }

    /// Blocks (via a short poll loop, since the socket is nonblocking) until
    /// a request at exactly `CALLBACK_PATH` arrives, `should_cancel` returns
    /// true, or `timeout` elapses. Any request at a different path, or a
    /// request that exceeds the bounded size, receives a response but does
    /// NOT terminate the wait -- only a well-formed request at the expected
    /// path is treated as the one terminal callback (Decision 0062: "accept
    /// exactly the expected path" / "bound request/header size" / "accept
    /// only one terminal callback").
    pub fn wait_for_callback(
        self,
        timeout: Duration,
        should_cancel: &dyn Fn() -> bool,
    ) -> CallbackWaitOutcome {
        let deadline = Instant::now() + timeout;
        loop {
            if should_cancel() {
                return CallbackWaitOutcome::Cancelled;
            }
            if Instant::now() >= deadline {
                return CallbackWaitOutcome::TimedOut;
            }
            match self.listener.accept() {
                Ok((stream, _addr)) => {
                    if let Some(callback) = Self::handle_connection(stream) {
                        return CallbackWaitOutcome::Received(callback);
                    }
                    // Non-terminal request (wrong path / oversized /
                    // unparsable): keep waiting for the real callback.
                    continue;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(POLL_INTERVAL);
                }
                Err(_) => {
                    std::thread::sleep(POLL_INTERVAL);
                }
            }
        }
    }

    /// Reads one bounded HTTP/1.1 request and responds. Returns `Some` only
    /// if the request was a syntactically valid GET at `CALLBACK_PATH`;
    /// every other case (wrong path, wrong method, oversized, malformed) is
    /// answered and treated as non-terminal noise.
    fn handle_connection(mut stream: TcpStream) -> Option<RawCallback> {
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let request_line = match Self::read_bounded_request_line(&mut stream) {
            Some(line) => line,
            None => {
                Self::respond(&mut stream, 400, FAILURE_PAGE);
                return None;
            }
        };
        // Drain (bounded) and discard headers/body -- this listener never
        // needs them, but it must not leave the client hanging or accept an
        // unbounded stream.
        Self::drain_bounded_headers(&mut stream);

        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap_or("");
        let target = parts.next().unwrap_or("");
        if method != "GET" {
            Self::respond(&mut stream, 405, FAILURE_PAGE);
            return None;
        }

        let (path, query) = match target.split_once('?') {
            Some((path, query)) => (path, query),
            None => (target, ""),
        };
        if path != CALLBACK_PATH {
            Self::respond(&mut stream, 404, FAILURE_PAGE);
            return None;
        }

        let callback = parse_callback_query(query);
        let page = if callback.error.is_none() && callback.code.is_some() {
            SUCCESS_PAGE
        } else {
            FAILURE_PAGE
        };
        Self::respond(&mut stream, 200, page);
        Some(callback)
    }

    fn read_bounded_request_line(stream: &mut TcpStream) -> Option<String> {
        let mut buffer = Vec::with_capacity(512);
        let mut byte = [0u8; 1];
        loop {
            if buffer.len() >= MAX_REQUEST_BYTES {
                return None;
            }
            match stream.read(&mut byte) {
                Ok(0) => return None,
                Ok(_) => {
                    if byte[0] == b'\n' {
                        break;
                    }
                    buffer.push(byte[0]);
                }
                Err(_) => return None,
            }
        }
        String::from_utf8(buffer)
            .ok()
            .map(|line| line.trim_end_matches('\r').to_string())
    }

    fn drain_bounded_headers(stream: &mut TcpStream) {
        let mut total_read = 0usize;
        let mut buffer = [0u8; 512];
        let mut consecutive_blank = 0u8;
        // Bounded: stop after MAX_REQUEST_BYTES total or two blank lines in
        // a row (end of headers), whichever comes first. A slow/hostile
        // client that never sends a blank line is bounded by the byte cap,
        // not left to stream forever.
        loop {
            if total_read >= MAX_REQUEST_BYTES {
                break;
            }
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    total_read += read;
                    if buffer[..read].windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                    consecutive_blank += 1;
                    if consecutive_blank > 4 {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    }

    fn respond(stream: &mut TcpStream, status: u16, body: &str) {
        let status_text = match status {
            200 => "OK",
            400 => "Bad Request",
            404 => "Not Found",
            405 => "Method Not Allowed",
            _ => "Error",
        };
        let response = format!(
            "HTTP/1.1 {status} {status_text}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
    }
}

/// Never echoes `code`, `state`, a token, or any GitHub response data.
const SUCCESS_PAGE: &str = "<!doctype html><html><head><title>RepoPact</title></head><body><p>RepoPact connected successfully. You can return to RepoPact.</p></body></html>";
const FAILURE_PAGE: &str = "<!doctype html><html><head><title>RepoPact</title></head><body><p>RepoPact could not complete the GitHub connection. You can return to RepoPact and try again.</p></body></html>";

fn parse_callback_query(query: &str) -> RawCallback {
    let mut callback = RawCallback {
        code: None,
        state: None,
        error: None,
        error_description: None,
    };
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        let value = percent_decode(value);
        match key {
            "code" => callback.code = Some(value),
            "state" => callback.state = Some(value),
            "error" => callback.error = Some(value),
            "error_description" => callback.error_description = Some(value),
            _ => {}
        }
    }
    callback
}

/// A minimal percent-decoder for query-string values -- no dependency
/// needed for this narrow, well-bounded use.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
                if let Some(value) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                    out.push(value);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufRead;
    use std::io::BufReader;
    use std::sync::Arc;

    fn send_raw_request(port: u16, raw: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream.write_all(raw.as_bytes()).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
        let mut reader = BufReader::new(stream);
        let mut status_line = String::new();
        let _ = reader.read_line(&mut status_line);
        status_line
    }

    #[test]
    fn parses_a_successful_callback_query() {
        let callback = parse_callback_query("code=abc123&state=xyz789");
        assert_eq!(callback.code.as_deref(), Some("abc123"));
        assert_eq!(callback.state.as_deref(), Some("xyz789"));
        assert!(callback.error.is_none());
    }

    #[test]
    fn parses_a_github_error_callback() {
        let callback =
            parse_callback_query("error=access_denied&error_description=The+user+denied&state=xyz");
        assert_eq!(callback.error.as_deref(), Some("access_denied"));
        assert_eq!(
            callback.error_description.as_deref(),
            Some("The user denied")
        );
        assert!(callback.code.is_none());
    }

    #[test]
    fn a_real_callback_request_is_received_and_gets_a_success_page() {
        let listener = CallbackListener::bind().unwrap();
        let port = listener.port();
        let redirect_uri = listener.redirect_uri();
        assert!(redirect_uri.contains(CALLBACK_PATH));

        let handle = std::thread::spawn(move || {
            listener.wait_for_callback(Duration::from_secs(5), &|| false)
        });
        std::thread::sleep(Duration::from_millis(100));
        let status = send_raw_request(
            port,
            &format!(
                "GET {}?code=real-code&state=real-state HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
                CALLBACK_PATH
            ),
        );
        assert!(status.contains("200"));
        let outcome = handle.join().unwrap();
        match outcome {
            CallbackWaitOutcome::Received(callback) => {
                assert_eq!(callback.code.as_deref(), Some("real-code"));
                assert_eq!(callback.state.as_deref(), Some("real-state"));
            }
            other => panic!("expected Received, got {other:?}"),
        }
    }

    #[test]
    fn a_request_to_the_wrong_path_does_not_terminate_the_wait() {
        let listener = CallbackListener::bind().unwrap();
        let port = listener.port();
        let handle = std::thread::spawn(move || {
            listener.wait_for_callback(Duration::from_millis(600), &|| false)
        });
        std::thread::sleep(Duration::from_millis(50));
        let status = send_raw_request(port, "GET /favicon.ico HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
        assert!(status.contains("404"));
        // The wrong-path request must not be treated as the terminal
        // callback -- the listener keeps waiting until it times out.
        let outcome = handle.join().unwrap();
        assert_eq!(outcome, CallbackWaitOutcome::TimedOut);
    }

    #[test]
    fn cancellation_stops_the_wait_without_a_callback() {
        let listener = CallbackListener::bind().unwrap();
        let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancelled_clone = cancelled.clone();
        let handle = std::thread::spawn(move || {
            listener.wait_for_callback(Duration::from_secs(30), &move || {
                cancelled_clone.load(std::sync::atomic::Ordering::SeqCst)
            })
        });
        std::thread::sleep(Duration::from_millis(100));
        cancelled.store(true, std::sync::atomic::Ordering::SeqCst);
        let outcome = handle.join().unwrap();
        assert_eq!(outcome, CallbackWaitOutcome::Cancelled);
    }

    #[test]
    fn a_timeout_with_no_request_reports_timed_out() {
        let listener = CallbackListener::bind().unwrap();
        let outcome = listener.wait_for_callback(Duration::from_millis(150), &|| false);
        assert_eq!(outcome, CallbackWaitOutcome::TimedOut);
    }

    #[test]
    fn an_oversized_request_line_is_rejected_and_non_terminal() {
        let listener = CallbackListener::bind().unwrap();
        let port = listener.port();
        let handle = std::thread::spawn(move || {
            listener.wait_for_callback(Duration::from_millis(600), &|| false)
        });
        std::thread::sleep(Duration::from_millis(50));
        let oversized_query = "a=".to_string() + &"x".repeat(MAX_REQUEST_BYTES + 100);
        let raw = format!(
            "GET {}?{} HTTP/1.1\r\nHost: x\r\n\r\n",
            CALLBACK_PATH, oversized_query
        );
        let _ = send_raw_request(port, &raw);
        let outcome = handle.join().unwrap();
        assert_eq!(outcome, CallbackWaitOutcome::TimedOut);
    }

    #[test]
    fn binds_only_to_loopback() {
        let listener = CallbackListener::bind().unwrap();
        let local_addr = listener.listener.local_addr().unwrap();
        assert!(local_addr.ip().is_loopback());
    }

    #[test]
    fn each_listener_gets_a_fresh_ephemeral_port() {
        let a = CallbackListener::bind().unwrap();
        let b = CallbackListener::bind().unwrap();
        assert_ne!(a.port(), b.port());
    }
}
