//! WI067 Checkpoint A defined this as a narrow HTTP transport seam so the
//! device-flow state machine was fully testable without a real network
//! connection. Checkpoint B (item 4) adds the real implementation,
//! [`ReqwestTransport`], built on `reqwest`'s blocking client: rustls-tls
//! (no OpenSSL/native-tls cross-compilation burden for Android), explicit
//! connect/overall timeouts, and -- critically -- automatic redirect
//! following turned OFF so this module can enforce Decision 0061's
//! `Authorization`-header redirect policy itself (item 5) rather than
//! trusting a generic HTTP client's redirect defaults.
//!
//! This trait is deliberately narrow: it is not `request(method, url,
//! body)` exposed to callers/frontend (item 22 and item 33) -- it is an
//! internal seam between the device-flow/REST logic and the concrete
//! network client.

use std::io::Read;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct FormRequest {
    pub url: String,
    pub fields: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct TransportError {
    pub message: String,
}

/// Internal transport seam. Real implementations must use normal TLS
/// certificate verification and explicit timeouts (item 24); this trait
/// carries no knob to disable either.
pub trait HttpTransport: Send + Sync {
    fn post_form(&self, request: &FormRequest) -> Result<HttpResponse, TransportError>;
}

/// A GET request against the GitHub REST API. `url` is the full request
/// URL (including any query string); `headers` is built centrally by
/// `crate::headers::RequestHeaders` -- this module never decides on its
/// own whether to include `Authorization`.
#[derive(Debug, Clone)]
pub struct RestRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
}

/// Only the response metadata callers actually need: pagination (`link`),
/// rate-limit accounting, and retry timing. Never the full raw header
/// set -- there is no reason to carry an arbitrary GitHub response header
/// past this boundary.
#[derive(Debug, Clone, Default)]
pub struct RestResponse {
    pub status: u16,
    pub link_header: Option<String>,
    pub retry_after_seconds: Option<u64>,
    pub rate_limit_remaining: Option<u64>,
    pub rate_limit_reset_epoch_seconds: Option<u64>,
    pub body: Vec<u8>,
}

pub trait RestTransport: Send + Sync {
    fn get(&self, request: &RestRequest) -> Result<RestResponse, TransportError>;
}

/// Maximum response body this adapter will read for a REST/JSON API call.
/// Enforced while streaming (via a bounded `Read::take`), not merely by
/// distrusting a declared `Content-Length` -- a real GitHub API JSON
/// response (even a full page of 100 repositories) is a few hundred KB at
/// most; this bound exists to fail closed on a misbehaving or malicious
/// response rather than to accommodate a large payload. Archive downloads
/// (Checkpoint C) use their own, much larger, streaming-to-disk bound and
/// never go through this path.
pub const MAX_REST_RESPONSE_BYTES: u64 = 8 * 1024 * 1024;

const MAX_REDIRECTS: u8 = 5;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const OVERALL_TIMEOUT: Duration = Duration::from_secs(30);
/// WI067 Checkpoint C (Phase 5): a repository archive download is
/// legitimately much larger and slower than a REST/JSON call -- reusing
/// the 30s API-call timeout here would abort a real download on an
/// ordinary connection. A deliberately longer deadline, still finite (this
/// is a whole-request timeout, not an unbounded wait): long enough for a
/// several-hundred-MB archive on a slow link, short enough to fail closed
/// rather than hang indefinitely.
const DOWNLOAD_OVERALL_TIMEOUT: Duration = Duration::from_secs(600);
/// Chunk size for streaming a download to disk. Small enough to check
/// `should_cancel`/the byte bound frequently; large enough not to dominate
/// download time with syscall overhead.
const DOWNLOAD_CHUNK_BYTES: usize = 256 * 1024;

/// A GET request whose response body must be streamed to a local file
/// rather than buffered in memory (item: Phase 5 -- "do not load a
/// potentially large repository archive wholly into memory").
#[derive(Debug, Clone)]
pub struct StreamingRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default)]
pub struct StreamingDownloadOutcome {
    pub bytes_written: u64,
}

pub trait StreamingDownloadTransport: Send + Sync {
    /// Streams the response body to `destination` (created fresh; any
    /// pre-existing file at that path is an implementation-detail
    /// overwrite, not append), enforcing `max_bytes` while streaming and
    /// polling `should_cancel` between chunks. On any error (including
    /// cancellation and the byte bound), the partial destination file is
    /// removed before returning -- the caller's own staging cleanup is a
    /// backstop, not the primary mechanism.
    fn download(
        &self,
        request: &StreamingRequest,
        destination: &std::path::Path,
        max_bytes: u64,
        should_cancel: &dyn Fn() -> bool,
    ) -> Result<StreamingDownloadOutcome, TransportError>;
}

/// The production transport. Built once and reused (a fresh
/// `reqwest::blocking::Client` per call would rebuild the connection
/// pool/TLS session cache every time).
pub struct ReqwestTransport {
    client: reqwest::blocking::Client,
    download_client: reqwest::blocking::Client,
}

impl ReqwestTransport {
    pub fn new() -> Result<Self, TransportError> {
        let client = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(OVERALL_TIMEOUT)
            .build()
            .map_err(|error| TransportError {
                message: format!("failed to construct HTTP client: {error}"),
            })?;
        let download_client = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(DOWNLOAD_OVERALL_TIMEOUT)
            .build()
            .map_err(|error| TransportError {
                message: format!("failed to construct download HTTP client: {error}"),
            })?;
        Ok(Self {
            client,
            download_client,
        })
    }

    fn read_bounded_body(response: reqwest::blocking::Response) -> Result<Vec<u8>, TransportError> {
        let mut limited = response.take(MAX_REST_RESPONSE_BYTES + 1);
        let mut buffer = Vec::new();
        limited
            .read_to_end(&mut buffer)
            .map_err(|error| TransportError {
                message: format!("failed to read response body: {error}"),
            })?;
        if buffer.len() as u64 > MAX_REST_RESPONSE_BYTES {
            return Err(TransportError {
                message: format!(
                    "response body exceeded the {}-byte bound",
                    MAX_REST_RESPONSE_BYTES
                ),
            });
        }
        Ok(buffer)
    }
}

impl HttpTransport for ReqwestTransport {
    fn post_form(&self, request: &FormRequest) -> Result<HttpResponse, TransportError> {
        let mut builder = self.client.post(&request.url).form(&request.fields);
        for (name, value) in &request.headers {
            builder = builder.header(name, value);
        }
        let response = builder.send().map_err(|error| TransportError {
            message: format!("request failed: {error}"),
        })?;
        let status = response.status().as_u16();
        let body = Self::read_bounded_body(response)?;
        let body = String::from_utf8_lossy(&body).into_owned();
        Ok(HttpResponse { status, body })
    }
}

impl RestTransport for ReqwestTransport {
    fn get(&self, request: &RestRequest) -> Result<RestResponse, TransportError> {
        let mut current_url =
            reqwest::Url::parse(&request.url).map_err(|error| TransportError {
                message: format!("invalid URL: {error}"),
            })?;
        // Only forward Authorization on the redirect chain if the ORIGINAL
        // request carried one -- a caller making an unauthenticated public
        // request never gains one just because a redirect target happens
        // to be on the allowlist.
        let had_authorization = request
            .headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("authorization"));
        let mut headers = request.headers.clone();

        for _ in 0..=MAX_REDIRECTS {
            let mut builder = self.client.get(current_url.clone());
            for (name, value) in &headers {
                builder = builder.header(name, value);
            }
            let response = builder.send().map_err(|error| TransportError {
                message: format!("request failed: {error}"),
            })?;
            let status = response.status();

            if status.is_redirection() {
                let location = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .ok_or_else(|| TransportError {
                        message: format!("redirect status {status} had no Location header"),
                    })?;
                let next_url = current_url.join(location).map_err(|error| TransportError {
                    message: format!("invalid redirect Location: {error}"),
                })?;
                let next_host = next_url.host_str().unwrap_or("");
                if !authorization_survives_redirect(had_authorization, next_host) {
                    headers.retain(|(name, _)| !name.eq_ignore_ascii_case("authorization"));
                }
                current_url = next_url;
                continue;
            }

            let link_header = response
                .headers()
                .get(reqwest::header::LINK)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string);
            let retry_after_seconds = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok());
            let rate_limit_remaining = header_u64(&response, "x-ratelimit-remaining");
            let rate_limit_reset_epoch_seconds = header_u64(&response, "x-ratelimit-reset");
            let status_code = status.as_u16();
            let body = Self::read_bounded_body(response)?;

            return Ok(RestResponse {
                status: status_code,
                link_header,
                retry_after_seconds,
                rate_limit_remaining,
                rate_limit_reset_epoch_seconds,
                body,
            });
        }

        Err(TransportError {
            message: format!("exceeded {MAX_REDIRECTS} redirects"),
        })
    }
}

impl StreamingDownloadTransport for ReqwestTransport {
    fn download(
        &self,
        request: &StreamingRequest,
        destination: &std::path::Path,
        max_bytes: u64,
        should_cancel: &dyn Fn() -> bool,
    ) -> Result<StreamingDownloadOutcome, TransportError> {
        let outcome = self.download_inner(request, destination, max_bytes, should_cancel);
        if outcome.is_err() {
            let _ = std::fs::remove_file(destination);
        }
        outcome
    }
}

impl ReqwestTransport {
    fn download_inner(
        &self,
        request: &StreamingRequest,
        destination: &std::path::Path,
        max_bytes: u64,
        should_cancel: &dyn Fn() -> bool,
    ) -> Result<StreamingDownloadOutcome, TransportError> {
        let mut current_url =
            reqwest::Url::parse(&request.url).map_err(|error| TransportError {
                message: format!("invalid URL: {error}"),
            })?;
        // Phase 6: reject a redirect that downgrades from https to http.
        // Compared against the *original* request's own scheme, not a
        // fixed "https required" rule -- production always builds
        // `https://api.github.com/...` itself, so any redirect away from
        // https is a genuine downgrade there, while this module's own
        // local-socket tests intentionally use plain `http://127.0.0.1`
        // test servers end-to-end (never claiming to be secure to begin
        // with, so there is no downgrade to detect).
        let original_scheme_was_https = current_url.scheme() == "https";
        let had_authorization = request
            .headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("authorization"));
        let mut headers = request.headers.clone();

        for hop in 0..=MAX_REDIRECTS {
            if should_cancel() {
                return Err(TransportError {
                    message: "cancelled".to_string(),
                });
            }
            if hop > 0
                && redirect_is_insecure_downgrade(original_scheme_was_https, current_url.scheme())
            {
                return Err(TransportError {
                    message: format!(
                        "refusing a redirect that downgrades to an insecure ({}) URL",
                        current_url.scheme()
                    ),
                });
            }
            let mut builder = self.download_client.get(current_url.clone());
            for (name, value) in &headers {
                builder = builder.header(name, value);
            }
            let response = builder.send().map_err(|error| TransportError {
                message: format!("request failed: {error}"),
            })?;
            let status = response.status();

            if status.is_redirection() {
                let location = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .ok_or_else(|| TransportError {
                        message: format!("redirect status {status} had no Location header"),
                    })?;
                let next_url = current_url.join(location).map_err(|error| TransportError {
                    message: format!("invalid redirect Location: {error}"),
                })?;
                let next_host = next_url.host_str().unwrap_or("");
                if !authorization_survives_redirect(had_authorization, next_host) {
                    headers.retain(|(name, _)| !name.eq_ignore_ascii_case("authorization"));
                }
                current_url = next_url;
                continue;
            }

            if !status.is_success() {
                let retry_after = header_u64(&response, "retry-after");
                // Phase 10: a hostile or merely verbose error body must
                // never leak a credential-shaped string into an error
                // message that could reach evidence/logs/UI, and must
                // never be embedded in full -- bounded to a short,
                // redacted snippet, mirroring rest.rs's own error-body
                // handling for the REST GET path.
                let mut raw_snippet = Vec::new();
                let _ = response.take(2048).read_to_end(&mut raw_snippet);
                let redacted = repopact_remote_provider::redact::redact(&String::from_utf8_lossy(
                    &raw_snippet,
                ));
                let bounded: String = redacted.chars().take(200).collect();
                let retry_suffix = retry_after
                    .map(|seconds| format!(" (retry after {seconds}s)"))
                    .unwrap_or_default();
                return Err(TransportError {
                    message: format!(
                        "download failed with status {status}{retry_suffix}: {bounded}"
                    ),
                });
            }

            return self.stream_to_file(response, destination, max_bytes, should_cancel);
        }

        Err(TransportError {
            message: format!("exceeded {MAX_REDIRECTS} redirects"),
        })
    }

    fn stream_to_file(
        &self,
        mut response: reqwest::blocking::Response,
        destination: &std::path::Path,
        max_bytes: u64,
        should_cancel: &dyn Fn() -> bool,
    ) -> Result<StreamingDownloadOutcome, TransportError> {
        use std::io::Write;
        let mut file = std::fs::File::create(destination).map_err(|error| TransportError {
            message: format!("failed to create staging file: {error}"),
        })?;
        let mut buffer = vec![0u8; DOWNLOAD_CHUNK_BYTES];
        let mut total: u64 = 0;
        loop {
            if should_cancel() {
                return Err(TransportError {
                    message: "cancelled mid-download".to_string(),
                });
            }
            let read = response.read(&mut buffer).map_err(|error| TransportError {
                message: format!("download read failed: {error}"),
            })?;
            if read == 0 {
                break;
            }
            total += read as u64;
            if total > max_bytes {
                return Err(TransportError {
                    message: format!("download exceeded the {max_bytes}-byte compressed bound"),
                });
            }
            file.write_all(&buffer[..read])
                .map_err(|error| TransportError {
                    message: format!("failed to write staging file: {error}"),
                })?;
        }
        file.flush().map_err(|error| TransportError {
            message: format!("failed to flush staging file: {error}"),
        })?;
        if total == 0 {
            return Err(TransportError {
                message: "download produced an empty body".to_string(),
            });
        }
        Ok(StreamingDownloadOutcome {
            bytes_written: total,
        })
    }
}

/// WI067 item 5: the exact policy decision the redirect loop applies at
/// every hop. An `Authorization` header only ever survives a redirect if
/// the request had one to begin with (an unauthenticated request never
/// gains one) AND the next hop's host is on Decision 0061's explicit
/// allowlist (`crate::redirect_policy::may_receive_authorization_header`).
fn authorization_survives_redirect(had_authorization: bool, next_host: &str) -> bool {
    had_authorization && crate::redirect_policy::may_receive_authorization_header(next_host)
}

/// WI067 Checkpoint C, Phase 6: a redirect only counts as an insecure
/// *downgrade* -- and is rejected -- if the original request was itself
/// https. An already-insecure chain (only ever exercised by this module's
/// own local-socket tests; production always starts from
/// `https://api.github.com/...`) has nothing to downgrade from.
fn redirect_is_insecure_downgrade(original_scheme_was_https: bool, next_scheme: &str) -> bool {
    original_scheme_was_https && next_scheme != "https"
}

fn header_u64(response: &reqwest::blocking::Response, name: &str) -> Option<u64> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
}

/// A scripted transport for tests: returns queued responses in order, and
/// records every request it received so a test can assert on the exact
/// URL/fields/headers sent (e.g. that no `client_secret` field was ever
/// included in a device-flow request).
#[derive(Default)]
pub struct ScriptedTransport {
    responses: std::sync::Mutex<Vec<Result<HttpResponse, TransportError>>>,
    received: std::sync::Mutex<Vec<FormRequest>>,
}

impl ScriptedTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_response(&self, response: Result<HttpResponse, TransportError>) {
        self.responses.lock().unwrap().push(response);
    }

    pub fn received_requests(&self) -> Vec<FormRequest> {
        self.received.lock().unwrap().clone()
    }
}

impl HttpTransport for ScriptedTransport {
    fn post_form(&self, request: &FormRequest) -> Result<HttpResponse, TransportError> {
        self.received.lock().unwrap().push(request.clone());
        let mut responses = self.responses.lock().unwrap();
        if responses.is_empty() {
            return Err(TransportError {
                message: "ScriptedTransport exhausted: no more queued responses".into(),
            });
        }
        responses.remove(0)
    }
}

/// Builds a JSON object body for tests. A value that parses as an integer
/// is emitted as a JSON number (matching GitHub's real response shape for
/// fields like `expires_in`); everything else is emitted as a JSON string.
pub fn json_response(status: u16, fields: &[(&str, &str)]) -> HttpResponse {
    let mut map = serde_json::Map::new();
    for (key, value) in fields {
        let json_value = match value.parse::<u64>() {
            Ok(number) => serde_json::Value::Number(number.into()),
            Err(_) => serde_json::Value::String((*value).to_string()),
        };
        map.insert((*key).to_string(), json_value);
    }
    HttpResponse {
        status,
        body: serde_json::to_string(&serde_json::Value::Object(map)).unwrap(),
    }
}

/// A scripted `RestTransport` for GET-based REST client tests: returns
/// queued responses in order, records every request received.
#[derive(Default)]
pub struct ScriptedRestTransport {
    responses: std::sync::Mutex<Vec<Result<RestResponse, TransportError>>>,
    received: std::sync::Mutex<Vec<RestRequest>>,
}

impl ScriptedRestTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_response(&self, response: Result<RestResponse, TransportError>) {
        self.responses.lock().unwrap().push(response);
    }

    pub fn received_requests(&self) -> Vec<RestRequest> {
        self.received.lock().unwrap().clone()
    }
}

impl RestTransport for ScriptedRestTransport {
    fn get(&self, request: &RestRequest) -> Result<RestResponse, TransportError> {
        self.received.lock().unwrap().push(request.clone());
        let mut responses = self.responses.lock().unwrap();
        if responses.is_empty() {
            return Err(TransportError {
                message: "ScriptedRestTransport exhausted: no more queued responses".into(),
            });
        }
        responses.remove(0)
    }
}

pub fn rest_json_response(status: u16, json: serde_json::Value) -> RestResponse {
    RestResponse {
        status,
        body: serde_json::to_vec(&json).unwrap(),
        ..Default::default()
    }
}

/// A scripted `StreamingDownloadTransport` for `GitHubProvider`-level unit
/// tests (auth-header construction, error-code mapping) that do not need a
/// real socket -- the streaming mechanics themselves (redirect handling,
/// byte bounding, cancellation) are proven against real local sockets in
/// this module's own `#[cfg(test)]` suite below, not re-mocked here.
#[derive(Default)]
pub struct ScriptedStreamingTransport {
    responses: std::sync::Mutex<Vec<Result<StreamingDownloadOutcome, TransportError>>>,
    received: std::sync::Mutex<Vec<StreamingRequest>>,
}

impl ScriptedStreamingTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_response(&self, response: Result<StreamingDownloadOutcome, TransportError>) {
        self.responses.lock().unwrap().push(response);
    }

    pub fn received_requests(&self) -> Vec<StreamingRequest> {
        self.received.lock().unwrap().clone()
    }
}

impl StreamingDownloadTransport for ScriptedStreamingTransport {
    fn download(
        &self,
        request: &StreamingRequest,
        _destination: &std::path::Path,
        _max_bytes: u64,
        _should_cancel: &dyn Fn() -> bool,
    ) -> Result<StreamingDownloadOutcome, TransportError> {
        self.received.lock().unwrap().push(request.clone());
        let mut responses = self.responses.lock().unwrap();
        if responses.is_empty() {
            return Err(TransportError {
                message: "ScriptedStreamingTransport exhausted: no more queued responses".into(),
            });
        }
        responses.remove(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::Arc;

    #[test]
    fn authorization_survives_redirect_only_to_an_allowlisted_host() {
        assert!(authorization_survives_redirect(true, "api.github.com"));
        assert!(authorization_survives_redirect(true, "codeload.github.com"));
        assert!(!authorization_survives_redirect(true, "evil.example"));
        assert!(!authorization_survives_redirect(true, "127.0.0.1"));
        // An unauthenticated request never gains a header just because the
        // redirect target happens to be allowlisted.
        assert!(!authorization_survives_redirect(false, "api.github.com"));
    }

    fn read_request_headers(stream: &TcpStream) -> Vec<String> {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut lines = Vec::new();
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                break;
            }
            let trimmed = line.trim_end().to_string();
            if trimmed.is_empty() {
                break;
            }
            lines.push(trimmed);
        }
        lines
    }

    /// Real integration proof of item 5: two independent local TCP servers
    /// stand in for two different hosts (a redirect from one 127.0.0.1
    /// port to a *different* 127.0.0.1 port is, from `Url::host_str`'s
    /// perspective, the same host -- so this test instead proves the
    /// underlying mechanism precisely by observing what the *second*
    /// server actually receives on the wire after a real HTTP redirect
    /// response from the first, which is the exact code path
    /// `ReqwestTransport::get` executes; the host-based allow/deny
    /// decision itself is exhaustively covered above and in
    /// `redirect_policy`'s own tests against the real GitHub-host
    /// allowlist.
    #[test]
    fn a_real_redirect_to_a_non_allowlisted_host_does_not_carry_the_authorization_header() {
        let server_b = TcpListener::bind("127.0.0.1:0").unwrap();
        let port_b = server_b.local_addr().unwrap().port();
        let received_by_b: Arc<std::sync::Mutex<Option<Vec<String>>>> =
            Arc::new(std::sync::Mutex::new(None));
        let received_by_b_clone = received_by_b.clone();
        let handle_b = std::thread::spawn(move || {
            let (stream, _) = server_b.accept().unwrap();
            let headers = read_request_headers(&stream);
            *received_by_b_clone.lock().unwrap() = Some(headers);
            let mut stream = stream;
            let body = b"ok";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
            stream.write_all(body).unwrap();
        });

        let server_a = TcpListener::bind("127.0.0.1:0").unwrap();
        let port_a = server_a.local_addr().unwrap().port();
        let handle_a = std::thread::spawn(move || {
            let (mut stream, _) = server_a.accept().unwrap();
            let _headers = read_request_headers(&stream);
            let location = format!("http://127.0.0.1:{port_b}/target");
            let response = format!("HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            stream.write_all(response.as_bytes()).unwrap();
        });

        let transport = ReqwestTransport::new().unwrap();
        let result = transport.get(&RestRequest {
            url: format!("http://127.0.0.1:{port_a}/start"),
            headers: vec![
                (
                    "Authorization".to_string(),
                    "Bearer super-secret-token".to_string(),
                ),
                (
                    "Accept".to_string(),
                    "application/vnd.github+json".to_string(),
                ),
            ],
        });

        handle_a.join().unwrap();
        handle_b.join().unwrap();

        let response =
            result.expect("the redirect chain should complete against the local mock servers");
        assert_eq!(response.status, 200);

        let headers_b = received_by_b
            .lock()
            .unwrap()
            .clone()
            .expect("server B should have received a request");
        let saw_authorization = headers_b
            .iter()
            .any(|line| line.to_ascii_lowercase().starts_with("authorization:"));
        assert!(
            !saw_authorization,
            "the Authorization header must not survive a redirect to a non-allowlisted host, but server B received: {headers_b:?}"
        );
    }

    /// Spawns a one-shot local HTTP/1.1 server that replies to the first
    /// connection with exactly `raw_response` and then exits. Used for the
    /// Checkpoint C streaming-download failure matrix (WI067 item Phase
    /// 13) -- real sockets, not a mocked transport.
    fn spawn_one_shot_server(raw_response: Vec<u8>) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let _ = read_request_headers(&stream);
                let _ = stream.write_all(&raw_response);
            }
        });
        port
    }

    fn ok_zip_body_response(body: &[u8]) -> Vec<u8> {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/zip\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .into_bytes()
        .into_iter()
        .chain(body.iter().copied())
        .collect()
    }

    #[test]
    fn download_streams_a_successful_response_to_the_destination_file() {
        let body = b"pretend-zip-bytes-0123456789".to_vec();
        let port = spawn_one_shot_server(ok_zip_body_response(&body));
        let transport = ReqwestTransport::new().unwrap();
        let dest = std::env::temp_dir().join(format!("repopact-dl-ok-{}", std::process::id()));
        let cancelled = false;
        let outcome = transport
            .download(
                &StreamingRequest {
                    url: format!("http://127.0.0.1:{port}/archive.zip"),
                    headers: vec![],
                },
                &dest,
                1024,
                &|| cancelled,
            )
            .unwrap();
        assert_eq!(outcome.bytes_written, body.len() as u64);
        assert_eq!(std::fs::read(&dest).unwrap(), body);
        let _ = std::fs::remove_file(&dest);
    }

    #[test]
    fn download_rejects_a_response_exceeding_the_compressed_byte_bound_while_streaming() {
        let body = vec![b'x'; 5000];
        let port = spawn_one_shot_server(ok_zip_body_response(&body));
        let transport = ReqwestTransport::new().unwrap();
        let dest =
            std::env::temp_dir().join(format!("repopact-dl-oversized-{}", std::process::id()));
        let cancelled = false;
        let result = transport.download(
            &StreamingRequest {
                url: format!("http://127.0.0.1:{port}/archive.zip"),
                headers: vec![],
            },
            &dest,
            1000,
            &|| cancelled,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("compressed bound"));
        // The partial file must not survive a failed download.
        assert!(!dest.exists());
    }

    #[test]
    fn download_honors_cancellation_and_leaves_no_partial_file() {
        let body = vec![b'y'; 500];
        let port = spawn_one_shot_server(ok_zip_body_response(&body));
        let transport = ReqwestTransport::new().unwrap();
        let dest = std::env::temp_dir().join(format!("repopact-dl-cancel-{}", std::process::id()));
        let result = transport.download(
            &StreamingRequest {
                url: format!("http://127.0.0.1:{port}/archive.zip"),
                headers: vec![],
            },
            &dest,
            1024,
            &|| true, // cancel immediately
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("cancel"));
        assert!(!dest.exists());
    }

    #[test]
    fn download_rejects_a_404() {
        let port = spawn_one_shot_server(
            b"HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\nConnection: close\r\n\r\nnot found"
                .to_vec(),
        );
        let transport = ReqwestTransport::new().unwrap();
        let dest = std::env::temp_dir().join(format!("repopact-dl-404-{}", std::process::id()));
        let cancelled = false;
        let result = transport.download(
            &StreamingRequest {
                url: format!("http://127.0.0.1:{port}/archive.zip"),
                headers: vec![],
            },
            &dest,
            1024,
            &|| cancelled,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("404"));
        assert!(!dest.exists());
    }

    #[test]
    fn download_rejects_a_redirect_with_a_missing_location() {
        let port = spawn_one_shot_server(
            b"HTTP/1.1 302 Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
        );
        let transport = ReqwestTransport::new().unwrap();
        let dest = std::env::temp_dir().join(format!("repopact-dl-noloc-{}", std::process::id()));
        let cancelled = false;
        let result = transport.download(
            &StreamingRequest {
                url: format!("http://127.0.0.1:{port}/archive.zip"),
                headers: vec![],
            },
            &dest,
            1024,
            &|| cancelled,
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .to_lowercase()
            .contains("location"));
        assert!(!dest.exists());
    }

    #[test]
    fn redirect_is_insecure_downgrade_only_when_the_original_request_was_https() {
        // A real https-original download redirected to http is a genuine
        // downgrade and must be rejected.
        assert!(redirect_is_insecure_downgrade(true, "http"));
        assert!(!redirect_is_insecure_downgrade(true, "https"));
        // This module's own local-socket tests intentionally run an
        // all-http chain end-to-end (see
        // `a_real_download_redirect_does_not_carry_authorization_to_a_non_allowlisted_host`
        // below) -- that is not a downgrade, since there was nothing
        // secure to downgrade from.
        assert!(!redirect_is_insecure_downgrade(false, "http"));
    }

    #[test]
    fn download_rejects_an_empty_body() {
        let port = spawn_one_shot_server(
            b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
        );
        let transport = ReqwestTransport::new().unwrap();
        let dest = std::env::temp_dir().join(format!("repopact-dl-empty-{}", std::process::id()));
        let cancelled = false;
        let result = transport.download(
            &StreamingRequest {
                url: format!("http://127.0.0.1:{port}/archive.zip"),
                headers: vec![],
            },
            &dest,
            1024,
            &|| cancelled,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("empty"));
        assert!(!dest.exists());
    }

    /// WI067 Checkpoint D, Phase 2: a server that declares a
    /// `Content-Length` larger than what it actually sends, then closes
    /// the connection -- a genuine truncated transfer, not a scripted
    /// error. Proves the production HTTP stack itself detects the short
    /// body (HTTP/1.1 length-framed bodies are a hard protocol violation
    /// when the connection closes early) rather than this module silently
    /// treating early EOF as a normal, if short, success.
    #[test]
    fn download_rejects_a_truncated_body() {
        let declared_len = 5000;
        let actual_body = vec![b'z'; 200]; // far short of the declared length
        let mut raw = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {declared_len}\r\nConnection: close\r\n\r\n"
        )
        .into_bytes();
        raw.extend_from_slice(&actual_body);
        let port = spawn_one_shot_server(raw);
        let transport = ReqwestTransport::new().unwrap();
        let dest =
            std::env::temp_dir().join(format!("repopact-dl-truncated-{}", std::process::id()));
        let cancelled = false;
        let result = transport.download(
            &StreamingRequest {
                url: format!("http://127.0.0.1:{port}/archive.zip"),
                headers: vec![],
            },
            &dest,
            1_000_000,
            &|| cancelled,
        );
        assert!(
            result.is_err(),
            "a truncated transfer must be a typed download failure, not a false success"
        );
        assert!(
            !dest.exists(),
            "no partial archive may survive a truncated download"
        );
    }

    #[test]
    fn download_rejects_a_500() {
        let port = spawn_one_shot_server(
            b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 5\r\nConnection: close\r\n\r\noops!"
                .to_vec(),
        );
        let transport = ReqwestTransport::new().unwrap();
        let dest = std::env::temp_dir().join(format!("repopact-dl-500-{}", std::process::id()));
        let cancelled = false;
        let result = transport.download(
            &StreamingRequest {
                url: format!("http://127.0.0.1:{port}/archive.zip"),
                headers: vec![],
            },
            &dest,
            1024,
            &|| cancelled,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("500"));
        assert!(!dest.exists());
    }

    #[test]
    fn download_rejects_a_503() {
        let port = spawn_one_shot_server(
            b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                .to_vec(),
        );
        let transport = ReqwestTransport::new().unwrap();
        let dest = std::env::temp_dir().join(format!("repopact-dl-503-{}", std::process::id()));
        let cancelled = false;
        let result = transport.download(
            &StreamingRequest {
                url: format!("http://127.0.0.1:{port}/archive.zip"),
                headers: vec![],
            },
            &dest,
            1024,
            &|| cancelled,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("503"));
        assert!(!dest.exists());
    }

    #[test]
    fn download_reports_retry_after_on_a_secondary_rate_limit_403() {
        let port = spawn_one_shot_server(
            b"HTTP/1.1 403 Forbidden\r\nRetry-After: 30\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                .to_vec(),
        );
        let transport = ReqwestTransport::new().unwrap();
        let dest =
            std::env::temp_dir().join(format!("repopact-dl-secondary-rl-{}", std::process::id()));
        let cancelled = false;
        let result = transport.download(
            &StreamingRequest {
                url: format!("http://127.0.0.1:{port}/archive.zip"),
                headers: vec![],
            },
            &dest,
            1024,
            &|| cancelled,
        );
        let error = result.unwrap_err();
        assert!(error.message.contains("status 403"));
        assert!(error.message.contains("retry after"));
        assert!(!dest.exists());
    }

    /// WI067 Checkpoint D, Phase 6/10: a hostile error-response body must
    /// never leak a credential-shaped token, must not blow past the
    /// bounded snippet length, and must be redacted -- proven against the
    /// *production* download path, not a unit test of the redact()
    /// function in isolation.
    #[test]
    fn download_error_body_is_bounded_and_redacted_even_when_hostile() {
        let canary = "ghu_REPOPACT_CANARY_dO_NOT_USE_0000000000";
        let hostile_body = format!(
            "<html><body>Error: token {canary} rejected. Authorization: Bearer {canary}. {}</body></html>",
            "A".repeat(10_000)
        );
        let response = format!(
            "HTTP/1.1 403 Forbidden\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{hostile_body}",
            hostile_body.len()
        );
        let port = spawn_one_shot_server(response.into_bytes());
        let transport = ReqwestTransport::new().unwrap();
        let dest = std::env::temp_dir().join(format!("repopact-dl-hostile-{}", std::process::id()));
        let cancelled = false;
        let result = transport.download(
            &StreamingRequest {
                url: format!("http://127.0.0.1:{port}/archive.zip"),
                headers: vec![],
            },
            &dest,
            1024,
            &|| cancelled,
        );
        let error = result.unwrap_err();
        assert!(
            !error.message.contains(canary),
            "the canary token must never appear in a transport error message"
        );
        assert!(
            !error.message.contains("Bearer"),
            "an Authorization-shaped value must never survive into the error message"
        );
        assert!(
            error.message.len() < 500,
            "the error message must stay bounded even when the hostile server sends a huge body: got {} chars",
            error.message.len()
        );
        assert!(!dest.exists());
    }

    /// WI067 Checkpoint D, Phase 2/3: a real redirect loop -- a persistent
    /// (not one-shot) local server that answers every connection with a
    /// 302 pointing back at itself -- proving `MAX_REDIRECTS` is actually
    /// enforced by the production redirect loop against a server that
    /// never stops redirecting, not merely by a documented constant.
    #[test]
    fn download_rejects_a_real_redirect_loop_at_the_max_redirect_bound() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let accept_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let accept_count_clone = accept_count.clone();
        let handle = std::thread::spawn(move || {
            // A correctly-bounded client makes at most MAX_REDIRECTS + 1
            // requests total (the initial request plus MAX_REDIRECTS
            // follow-ups) before giving up -- exactly this many accepts,
            // so the server thread completes normally rather than blocking
            // forever on a connection that (correctly) never arrives.
            for _ in 0..(MAX_REDIRECTS as u32 + 1) {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                accept_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let _ = read_request_headers(&stream);
                let location = format!("http://127.0.0.1:{port}/loop");
                let response = format!(
                    "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });

        let transport = ReqwestTransport::new().unwrap();
        let dest = std::env::temp_dir().join(format!("repopact-dl-loop-{}", std::process::id()));
        let cancelled = false;
        let result = transport.download(
            &StreamingRequest {
                url: format!("http://127.0.0.1:{port}/loop"),
                headers: vec![],
            },
            &dest,
            1024,
            &|| cancelled,
        );
        handle.join().unwrap();

        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("redirect"));
        assert!(!dest.exists());
        assert!(
            accept_count.load(std::sync::atomic::Ordering::SeqCst) <= MAX_REDIRECTS as u32 + 1,
            "the client must stop following redirects at MAX_REDIRECTS, not loop indefinitely"
        );
    }

    #[test]
    fn download_rejects_a_connection_refused_host() {
        let transport = ReqwestTransport::new().unwrap();
        let dest = std::env::temp_dir().join(format!("repopact-dl-refused-{}", std::process::id()));
        let cancelled = false;
        // Port 1 is reserved and nothing listens on it locally.
        let result = transport.download(
            &StreamingRequest {
                url: "http://127.0.0.1:1/archive.zip".to_string(),
                headers: vec![],
            },
            &dest,
            1024,
            &|| cancelled,
        );
        assert!(result.is_err());
        assert!(!dest.exists());
    }

    /// WI067 Checkpoint D, Phase 2: a genuine TLS certificate-validation
    /// failure through the *production* client (rustls-backed, default
    /// certificate verification -- no insecure-TLS bypass exists anywhere
    /// in this codebase to disable for the test). `self-signed.badssl.com`
    /// is a long-standing, purpose-built public test service maintained
    /// specifically for this class of test (the same convention this
    /// module already uses for live GitHub proof against
    /// octocat/Hello-World). Not run by default; run explicitly with
    /// `--ignored` for checkpoint evidence.
    #[test]
    #[ignore]
    fn live_download_rejects_a_self_signed_certificate() {
        let transport = ReqwestTransport::new().unwrap();
        let dest = std::env::temp_dir().join(format!("repopact-dl-tls-{}", std::process::id()));
        let cancelled = false;
        let result = transport.download(
            &StreamingRequest {
                url: "https://self-signed.badssl.com/".to_string(),
                headers: vec![],
            },
            &dest,
            1024,
            &|| cancelled,
        );
        assert!(
            result.is_err(),
            "a self-signed certificate must be rejected, not silently accepted"
        );
        assert!(
            !dest.exists(),
            "no partial file may survive a TLS validation failure"
        );
    }

    /// The same proof against the plain REST GET path (used for API
    /// calls, not archive downloads) -- both HTTP client instances inside
    /// `ReqwestTransport` share the same rustls default verification, but
    /// this proves it explicitly for the path GH-010's "API-error"
    /// category actually exercises.
    #[test]
    #[ignore]
    fn live_rest_get_rejects_a_self_signed_certificate() {
        let transport = ReqwestTransport::new().unwrap();
        let result = transport.get(&RestRequest {
            url: "https://self-signed.badssl.com/".to_string(),
            headers: vec![],
        });
        assert!(
            result.is_err(),
            "a self-signed certificate must be rejected on the REST path too"
        );
    }

    #[test]
    fn a_real_download_redirect_does_not_carry_authorization_to_a_non_allowlisted_host() {
        let body = b"redirected-zip-bytes".to_vec();
        let server_b_response = ok_zip_body_response(&body);
        let server_b = TcpListener::bind("127.0.0.1:0").unwrap();
        let port_b = server_b.local_addr().unwrap().port();
        let received_by_b: Arc<std::sync::Mutex<Option<Vec<String>>>> =
            Arc::new(std::sync::Mutex::new(None));
        let received_by_b_clone = received_by_b.clone();
        let handle_b = std::thread::spawn(move || {
            let (stream, _) = server_b.accept().unwrap();
            let headers = read_request_headers(&stream);
            *received_by_b_clone.lock().unwrap() = Some(headers);
            let mut stream = stream;
            stream.write_all(&server_b_response).unwrap();
        });

        let server_a = TcpListener::bind("127.0.0.1:0").unwrap();
        let port_a = server_a.local_addr().unwrap().port();
        let handle_a = std::thread::spawn(move || {
            let (mut stream, _) = server_a.accept().unwrap();
            let _ = read_request_headers(&stream);
            let location = format!("http://127.0.0.1:{port_b}/target.zip");
            let response = format!(
                "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            stream.write_all(response.as_bytes()).unwrap();
        });

        let transport = ReqwestTransport::new().unwrap();
        let dest =
            std::env::temp_dir().join(format!("repopact-dl-redirect-auth-{}", std::process::id()));
        let cancelled = false;
        let result = transport.download(
            &StreamingRequest {
                url: format!("http://127.0.0.1:{port_a}/start.zip"),
                headers: vec![(
                    "Authorization".to_string(),
                    "Bearer super-secret".to_string(),
                )],
            },
            &dest,
            1024,
            &|| cancelled,
        );

        handle_a.join().unwrap();
        handle_b.join().unwrap();

        assert!(
            result.is_ok(),
            "download should succeed following the local redirect"
        );
        let _ = std::fs::remove_file(&dest);

        let headers_b = received_by_b.lock().unwrap().clone().unwrap();
        assert!(
            headers_b.iter().all(|line| !line.to_ascii_lowercase().starts_with("authorization:")),
            "Authorization must not survive a download redirect to a non-allowlisted host: {headers_b:?}"
        );
    }
}
