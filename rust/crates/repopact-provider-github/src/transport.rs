//! WI067 item 38: a narrow HTTP transport seam so the device-flow state
//! machine (and, later, the REST client) is fully testable without a real
//! network connection. Selecting a concrete HTTP client library (reqwest
//! or otherwise) for the real backend is deliberately deferred past
//! Checkpoint A -- nothing in this crate depends on one yet.
//!
//! This trait is deliberately narrow: it is not `request(method, url,
//! body)` exposed to callers/frontend (item 22); it is an internal seam
//! between the device-flow/REST logic and whatever eventually performs
//! real TLS-verified HTTP I/O (item 24 -- no `danger_accept_invalid_certs`,
//! explicit timeouts, explicit redirect policy).

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
