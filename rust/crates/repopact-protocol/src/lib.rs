//! Versioned transport DTOs for the one-request RepoPact engine protocol.
//!
//! This crate intentionally contains no repository or governance semantics. It
//! is shared by the machine engine and protocol-focused tests so that Python
//! and future non-Python clients have one language-neutral envelope contract.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL: &str = "repopact-engine";
pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineRequest {
    pub protocol: String,
    pub protocol_version: u32,
    pub request_id: String,
    pub operation: String,
    #[serde(default)]
    pub root: Option<String>,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineResponse {
    pub protocol: String,
    pub protocol_version: u32,
    pub request_id: String,
    pub engine_version: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<ProtocolDiagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ProtocolError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Capabilities>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolDiagnostic {
    pub code: String,
    pub severity: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_records: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggested_actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    pub operations: Vec<String>,
    pub semantic_surfaces: Vec<String>,
}

impl EngineResponse {
    pub fn success(
        request_id: impl Into<String>,
        engine_version: impl Into<String>,
        result: Value,
    ) -> Self {
        Self {
            protocol: PROTOCOL.to_owned(),
            protocol_version: PROTOCOL_VERSION,
            request_id: request_id.into(),
            engine_version: engine_version.into(),
            ok: true,
            result: Some(result),
            diagnostics: Vec::new(),
            error: None,
            capabilities: None,
        }
    }

    pub fn failure(
        request_id: impl Into<String>,
        engine_version: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            protocol: PROTOCOL.to_owned(),
            protocol_version: PROTOCOL_VERSION,
            request_id: request_id.into(),
            engine_version: engine_version.into(),
            ok: false,
            result: None,
            diagnostics: Vec::new(),
            error: Some(ProtocolError {
                code: code.into(),
                message: message.into(),
            }),
            capabilities: None,
        }
    }
}

pub fn capabilities() -> Capabilities {
    Capabilities {
        operations: vec![
            "handshake".to_owned(),
            "capabilities".to_owned(),
            "validate".to_owned(),
            "dashboard.write".to_owned(),
            "work.create".to_owned(),
            "work.propose".to_owned(),
            "work.amend_proposal".to_owned(),
            "graph".to_owned(),
            "analyze".to_owned(),
        ],
        semantic_surfaces: vec![
            "repository".to_owned(),
            "schema-validation".to_owned(),
            "dashboard".to_owned(),
            "graph".to_owned(),
            "analysis".to_owned(),
            "typed-work-item-mutation".to_owned(),
        ],
    }
}
