//! Opaque, bounded pagination cursors (Decision 0050 sections 5/16). A
//! cursor is a base64url-encoded canonical JSON payload binding the
//! query contract version, the graph fingerprint, the operation name, a
//! hash of the normalized request, and a continuation position. No
//! secrets, no absolute paths, no executable content, and never a
//! serialized Rust memory layout.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::bounds::MAX_CURSOR_LEN;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CursorPayload {
    /// Query contract version this cursor was minted under.
    v: u32,
    /// Graph fingerprint (or effective generation) this cursor was
    /// minted against.
    fp: String,
    /// The operation name (e.g. `"graph.neighbors"`).
    op: String,
    /// SHA-256 hex of the normalized request identity (selector +
    /// filters, excluding the cursor/page position themselves).
    req: String,
    /// Continuation position: an index into the deterministic, sorted
    /// result set this operation would otherwise produce in full.
    pos: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorError {
    TooLong,
    Malformed,
    VersionMismatch,
    FingerprintMismatch,
    OperationMismatch,
    RequestMismatch,
}

impl std::fmt::Display for CursorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::TooLong => "cursor exceeds the maximum allowed length",
            Self::Malformed => "cursor is not a valid opaque cursor",
            Self::VersionMismatch => "cursor was minted under a different query contract version",
            Self::FingerprintMismatch => "cursor was minted against a different graph fingerprint",
            Self::OperationMismatch => "cursor was minted for a different operation",
            Self::RequestMismatch => "cursor was minted for a different selector/filter request",
        };
        write!(f, "{message}")
    }
}

/// Hash a normalized request identity (any `Serialize` value fully
/// controlled by this crate, so field order is already deterministic --
/// no separate canonicalization pass is needed).
pub fn request_identity(value: &impl Serialize) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let digest = Sha256::digest(&bytes);
    hex_encode(&digest)
}

pub fn encode_cursor(
    query_contract_version: u32,
    graph_fingerprint: &str,
    operation: &str,
    request_hash: &str,
    position: u64,
) -> String {
    let payload = CursorPayload {
        v: query_contract_version,
        fp: graph_fingerprint.to_owned(),
        op: operation.to_owned(),
        req: request_hash.to_owned(),
        pos: position,
    };
    let json = serde_json::to_vec(&payload).unwrap_or_default();
    base64url_encode(&json)
}

/// Decode and validate a cursor against the expected context, returning
/// the continuation position on success.
pub fn decode_and_validate_cursor(
    cursor: &str,
    expected_version: u32,
    expected_fingerprint: &str,
    expected_operation: &str,
    expected_request_hash: &str,
) -> Result<u64, CursorError> {
    if cursor.len() > MAX_CURSOR_LEN {
        return Err(CursorError::TooLong);
    }
    let bytes = base64url_decode(cursor).ok_or(CursorError::Malformed)?;
    let payload: CursorPayload =
        serde_json::from_slice(&bytes).map_err(|_| CursorError::Malformed)?;
    if payload.v != expected_version {
        return Err(CursorError::VersionMismatch);
    }
    if payload.fp != expected_fingerprint {
        return Err(CursorError::FingerprintMismatch);
    }
    if payload.op != expected_operation {
        return Err(CursorError::OperationMismatch);
    }
    if payload.req != expected_request_hash {
        return Err(CursorError::RequestMismatch);
    }
    Ok(payload.pos)
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn base64url_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied();
        let b2 = chunk.get(2).copied();
        let n = (b0 as u32) << 16 | (b1.unwrap_or(0) as u32) << 8 | (b2.unwrap_or(0) as u32);
        out.push(ALPHABET[((n >> 18) & 0x3f) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 0x3f) as usize] as char);
        if b1.is_some() {
            out.push(ALPHABET[((n >> 6) & 0x3f) as usize] as char);
        }
        if b2.is_some() {
            out.push(ALPHABET[(n & 0x3f) as usize] as char);
        }
    }
    out
}

fn base64url_decode(input: &str) -> Option<Vec<u8>> {
    if !input.is_ascii() {
        return None;
    }
    let mut reverse = [255u8; 256];
    for (index, &symbol) in ALPHABET.iter().enumerate() {
        reverse[symbol as usize] = index as u8;
    }
    let symbols: Vec<u8> = input.bytes().collect();
    if symbols.iter().any(|&byte| reverse[byte as usize] == 255) {
        return None;
    }
    let mut out = Vec::with_capacity(symbols.len() * 3 / 4 + 3);
    for chunk in symbols.chunks(4) {
        let v0 = reverse[chunk[0] as usize] as u32;
        let v1 = *chunk.get(1).map(|b| &reverse[*b as usize])? as u32;
        let n = (v0 << 18) | (v1 << 12);
        out.push((n >> 16) as u8);
        if let Some(&b2) = chunk.get(2) {
            let v2 = reverse[b2 as usize] as u32;
            let n = n | (v2 << 6);
            out.push((n >> 8) as u8);
            if let Some(&b3) = chunk.get(3) {
                let v3 = reverse[b3 as usize] as u32;
                let n = n | v3;
                out.push(n as u8);
            }
        }
    }
    Some(out)
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64url_round_trips_arbitrary_bytes() {
        for len in 0..40 {
            let input: Vec<u8> = (0..len as u8).collect();
            let encoded = base64url_encode(&input);
            assert!(!encoded.contains('+') && !encoded.contains('/') && !encoded.contains('='));
            let decoded = base64url_decode(&encoded).unwrap();
            assert_eq!(decoded, input);
        }
    }

    #[test]
    fn cursor_round_trips_and_validates() {
        let request_hash = request_identity(&"selector-and-filters");
        let cursor = encode_cursor(1, "fingerprint-a", "graph.neighbors", &request_hash, 42);
        let position = decode_and_validate_cursor(
            &cursor,
            1,
            "fingerprint-a",
            "graph.neighbors",
            &request_hash,
        )
        .unwrap();
        assert_eq!(position, 42);
    }

    #[test]
    fn cursor_rejects_mismatched_context() {
        let request_hash = request_identity(&"selector-and-filters");
        let cursor = encode_cursor(1, "fingerprint-a", "graph.neighbors", &request_hash, 5);

        assert_eq!(
            decode_and_validate_cursor(
                &cursor,
                2,
                "fingerprint-a",
                "graph.neighbors",
                &request_hash
            ),
            Err(CursorError::VersionMismatch)
        );
        assert_eq!(
            decode_and_validate_cursor(
                &cursor,
                1,
                "fingerprint-b",
                "graph.neighbors",
                &request_hash
            ),
            Err(CursorError::FingerprintMismatch)
        );
        assert_eq!(
            decode_and_validate_cursor(
                &cursor,
                1,
                "fingerprint-a",
                "graph.dependencies",
                &request_hash
            ),
            Err(CursorError::OperationMismatch)
        );
        let other_hash = request_identity(&"different-request");
        assert_eq!(
            decode_and_validate_cursor(&cursor, 1, "fingerprint-a", "graph.neighbors", &other_hash),
            Err(CursorError::RequestMismatch)
        );
    }

    #[test]
    fn malformed_and_oversized_cursors_are_rejected() {
        assert_eq!(
            decode_and_validate_cursor("not valid base64url!!", 1, "fp", "op", "hash"),
            Err(CursorError::Malformed)
        );
        let oversized = "A".repeat(MAX_CURSOR_LEN + 1);
        assert_eq!(
            decode_and_validate_cursor(&oversized, 1, "fp", "op", "hash"),
            Err(CursorError::TooLong)
        );
    }
}
