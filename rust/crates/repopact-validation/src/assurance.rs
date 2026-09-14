//! Assurance/control mapping diagnostics beyond structural validation
//! (WI051 phase 2, Decision 0055): bounded sensitive-evidence guardrails
//! (ACM-004), review/drift semantics and documentation claim-basis
//! validation (ACM-005).
//!
//! Every diagnostic here is scoped to content an assurance mapping already
//! explicitly references -- never a whole-repository scan -- and every
//! `Warning`/`Info` diagnostic is a bounded heuristic or coverage note, never
//! a legal or data-protection classification (Decision 0055 section 2).

use std::fs;
use std::path::Path;

use repopact_repository::{resolve_within_root, IndexedRecord};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::Validator;

/// Bounded artifact inspection ceiling (Decision 0055 section 3): large
/// enough for any genuinely bounded/redacted/synthetic evidence artifact,
/// small enough that this validator can never become an unbounded content
/// scanner.
const MAX_ARTIFACT_BYTES: u64 = 262_144;

const IDENTITY_LABELS: &[&str] = &[
    "patient_id",
    "medical_record_number",
    "mrn",
    "passport_number",
    "social_security_number",
    "ssn",
    "bank_account_number",
    "routing_number",
];

const SECRET_ASSIGNMENT_LABELS: &[&str] = &["password", "secret", "api_key", "access_token"];

const STRONG_CLAIM_PHRASES: &[&str] = &[
    "certified",
    "fully compliant",
    "complies with",
    "meets all requirements",
];

// --- ACM-004: bounded sensitive-evidence guardrails ------------------------

impl Validator {
    pub(crate) fn validate_sensitive_evidence(&mut self) {
        if self.index.assurance_mappings.is_empty() {
            return;
        }
        let root = self.repository.root().to_path_buf();
        for record in self.index.assurance_mappings.clone() {
            let Ok(data) = record.value.clone() else {
                continue;
            };
            let Some(object) = data.as_object() else {
                continue;
            };
            let mapping_id = object
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            for evidence_ref in object
                .get("evidence_refs")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                self.scan_evidence_ref(evidence_ref, &mapping_id, &record.path, &root);
            }
            for dependency in object
                .get("third_party_dependencies")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let Some(dependency) = dependency.as_object() else {
                    continue;
                };
                for evidence_ref in dependency
                    .get("evidence_refs")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    self.scan_evidence_ref(evidence_ref, &mapping_id, &record.path, &root);
                }
            }
        }
    }

    fn scan_evidence_ref(
        &mut self,
        evidence_ref: &Value,
        mapping_id: &str,
        path: &Path,
        root: &Path,
    ) {
        let Some(evidence_ref) = evidence_ref.as_object() else {
            return;
        };
        let kind = evidence_ref
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("");
        let sensitivity = evidence_ref
            .get("sensitivity")
            .and_then(Value::as_str)
            .unwrap_or("");
        if kind == "repository_artifact" {
            if sensitivity == "restricted" {
                self.push(self.warn_at(
                    "assurance.evidence-restricted-repository-artifact",
                    format!(
                        "mapping '{mapping_id}' declares a 'restricted' evidence artifact stored directly as a repository file; restricted evidence should normally be represented by a hash, controlled external reference, redacted artifact, synthetic artifact, or bounded metadata instead of a raw repository artifact"
                    ),
                    path,
                ));
            }
            let relative = evidence_ref
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or("");
            if let Some(resolved) = resolve_within_root(root, relative) {
                self.scan_repository_artifact(&resolved, mapping_id, path);
            }
        } else if kind == "external_reference" {
            if let Some(reference) = evidence_ref
                .get("external")
                .and_then(Value::as_object)
                .and_then(|external| external.get("reference"))
                .and_then(Value::as_str)
            {
                self.scan_external_reference(reference, mapping_id, path);
            }
        }
    }

    fn scan_repository_artifact(&mut self, resolved: &Path, mapping_id: &str, path: &Path) {
        let metadata = match fs::metadata(resolved) {
            Ok(metadata) => metadata,
            Err(_) => {
                self.push(self.info_at(
                    "assurance.evidence-artifact-unreadable",
                    format!(
                        "mapping '{mapping_id}' evidence artifact could not be read for sensitive-evidence inspection"
                    ),
                    path,
                ));
                return;
            }
        };
        if metadata.len() > MAX_ARTIFACT_BYTES {
            self.push(self.info_at(
                "assurance.evidence-artifact-oversize-not-inspected",
                format!(
                    "mapping '{mapping_id}' evidence artifact exceeds the {MAX_ARTIFACT_BYTES}-byte bounded inspection limit ({} bytes); not inspected for sensitive-evidence indicators",
                    metadata.len()
                ),
                path,
            ));
            return;
        }
        let bytes = match fs::read(resolved) {
            Ok(bytes) => bytes,
            Err(_) => {
                self.push(self.info_at(
                    "assurance.evidence-artifact-unreadable",
                    format!(
                        "mapping '{mapping_id}' evidence artifact could not be read for sensitive-evidence inspection"
                    ),
                    path,
                ));
                return;
            }
        };
        let Some(text) = decode_text(&bytes) else {
            self.push(self.info_at(
                "assurance.evidence-artifact-binary-not-inspected",
                format!(
                    "mapping '{mapping_id}' evidence artifact is binary or not valid UTF-8; not inspected for sensitive-evidence indicators"
                ),
                path,
            ));
            return;
        };
        for diagnostic in scan_text_for_hazards(&text, mapping_id) {
            self.push(diagnostic.with_path(self.rel(path)));
        }
    }

    fn scan_external_reference(&mut self, reference: &str, mapping_id: &str, path: &Path) {
        if has_credential_in_uri(reference) {
            self.push(self.at(
                "assurance.evidence-external-reference-credential",
                format!(
                    "mapping '{mapping_id}' external evidence reference embeds userinfo credentials in its URI; use a credential-free reference"
                ),
                path,
            ));
        }
        if has_secret_query_param(reference) {
            self.push(self.warn_at(
                "assurance.evidence-external-reference-secret-query-param",
                format!(
                    "mapping '{mapping_id}' external evidence reference has a query parameter shaped like a secret/token/credential; verify it does not leak one"
                ),
                path,
            ));
        }
    }
}

/// Decode `bytes` as text only if it is valid UTF-8 and contains no NUL byte
/// in the inspected window; anything else is treated as binary and skipped
/// (Decision 0055 section 3).
fn decode_text(bytes: &[u8]) -> Option<String> {
    if bytes.contains(&0) {
        return None;
    }
    std::str::from_utf8(bytes).ok().map(str::to_owned)
}

fn scan_text_for_hazards(text: &str, mapping_id: &str) -> Vec<repopact_types::Diagnostic> {
    use repopact_types::Diagnostic;
    let mut diagnostics = Vec::new();
    if let Some(marker) = detect_private_key_marker(text) {
        diagnostics.push(Diagnostic::error(
            "assurance.evidence-private-key-material",
            format!(
                "mapping '{mapping_id}' evidence artifact contains a private-key marker ('{marker}'); private-key material must never be committed as evidence"
            ),
        ));
    }
    if detect_secret_assignment(text) {
        diagnostics.push(Diagnostic::warning(
            "assurance.evidence-secret-like-content",
            format!(
                "mapping '{mapping_id}' evidence artifact contains a secret-assignment-shaped pattern (e.g. password=/secret=/api_key=/access_token=); potential secret/credential material, not a confirmed classification"
            ),
        ));
    }
    if detect_luhn_valid_pan(text) {
        diagnostics.push(Diagnostic::warning(
            "assurance.evidence-cardholder-shaped-content",
            format!(
                "mapping '{mapping_id}' evidence artifact contains a Luhn-valid, 13-19 digit sequence shaped like a primary account number; potential cardholder-data-shaped evidence, not a confirmed PCI scope determination"
            ),
        ));
    }
    if detect_identity_markers(text) {
        diagnostics.push(Diagnostic::warning(
            "assurance.evidence-restricted-identity-shaped-content",
            format!(
                "mapping '{mapping_id}' evidence artifact contains a high-signal health/identity/financial label (e.g. patient_id/ssn/passport_number/bank_account_number); potential restricted health/identity evidence, not a confirmed classification"
            ),
        ));
    }
    diagnostics
}

fn detect_private_key_marker(text: &str) -> Option<&'static str> {
    const MARKERS: [&str; 3] = [
        "-----BEGIN PRIVATE KEY-----",
        "-----BEGIN RSA PRIVATE KEY-----",
        "-----BEGIN OPENSSH PRIVATE KEY-----",
    ];
    MARKERS
        .iter()
        .find(|marker| text.contains(*marker))
        .copied()
}

fn detect_secret_assignment(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    SECRET_ASSIGNMENT_LABELS
        .iter()
        .any(|label| find_label_assignment(&lower, label))
}

/// True when `label` appears immediately followed by optional whitespace and
/// a `=` or `:` and then a non-whitespace value, on a word boundary before
/// the label (so "password" matches but "mypasswordfield" does not).
fn find_label_assignment(lower_text: &str, label: &str) -> bool {
    let mut search_from = 0usize;
    while let Some(offset) = lower_text[search_from..].find(label) {
        let start = search_from + offset;
        let boundary_ok = start == 0
            || !lower_text.as_bytes()[start - 1].is_ascii_alphanumeric()
                && lower_text.as_bytes()[start - 1] != b'_';
        let after = &lower_text[start + label.len()..];
        let trimmed = after.trim_start_matches([' ', '\t']);
        let has_assignment = trimmed.starts_with('=') || trimmed.starts_with(':');
        if boundary_ok && has_assignment {
            let value = trimmed.trim_start_matches([':', '=']).trim_start();
            if value
                .chars()
                .next()
                .is_some_and(|c| !c.is_whitespace() && c != '\n' && c != '\r')
            {
                return true;
            }
        }
        search_from = start + label.len();
        if search_from >= lower_text.len() {
            break;
        }
    }
    false
}

fn detect_identity_markers(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    IDENTITY_LABELS
        .iter()
        .any(|label| find_label_assignment(&lower, label))
}

/// Bounded scan for a 13-19 digit sequence (allowing interior spaces/dashes,
/// as real PANs are often displayed) that passes the Luhn checksum. This is
/// a shape heuristic, never proof of cardholder data or PCI scope.
fn detect_luhn_valid_pan(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index].is_ascii_digit() {
            let start = index;
            let mut digits: Vec<u8> = Vec::new();
            let mut cursor = index;
            while cursor < bytes.len() {
                let byte = bytes[cursor];
                if byte.is_ascii_digit() {
                    digits.push(byte - b'0');
                } else if byte == b' ' || byte == b'-' {
                    // separators inside a candidate number are tolerated
                } else {
                    break;
                }
                cursor += 1;
                if digits.len() > 19 {
                    break;
                }
            }
            if (13..=19).contains(&digits.len()) && luhn_valid(&digits) {
                return true;
            }
            index = if cursor > start { cursor } else { start + 1 };
        } else {
            index += 1;
        }
    }
    false
}

fn luhn_valid(digits: &[u8]) -> bool {
    let mut sum = 0u32;
    let mut double = false;
    for &digit in digits.iter().rev() {
        let mut value = u32::from(digit);
        if double {
            value *= 2;
            if value > 9 {
                value -= 9;
            }
        }
        sum += value;
        double = !double;
    }
    sum % 10 == 0
}

/// True when `uri` embeds `user:password@` userinfo credentials before its
/// host (`scheme://user:password@host/...`). Structural, no network access.
fn has_credential_in_uri(uri: &str) -> bool {
    let Some(after_scheme) = uri.split_once("://").map(|(_, rest)| rest) else {
        return false;
    };
    let authority = after_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(after_scheme);
    match authority.rsplit_once('@') {
        Some((userinfo, _host)) => userinfo.contains(':') && !userinfo.is_empty(),
        None => false,
    }
}

/// True when `uri`'s query string has a parameter shaped like a secret
/// (`token`, `api_key`, `password`, `secret`) with a non-empty value.
fn has_secret_query_param(uri: &str) -> bool {
    let Some((_, query)) = uri.split_once('?') else {
        return false;
    };
    let query = query.split('#').next().unwrap_or(query);
    const SECRET_PARAMS: [&str; 4] = ["token", "api_key", "password", "secret"];
    query.split('&').any(|pair| {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or("").to_ascii_lowercase();
        let value = parts.next().unwrap_or("");
        SECRET_PARAMS.contains(&key.as_str()) && !value.is_empty()
    })
}

// --- ACM-005: canonical review projection, digests, snapshots --------------

/// The mapping's canonical review projection (Decision 0055 section 5): the
/// full record with `review.snapshot`, `created`, and `updated` excluded --
/// bookkeeping timestamps are not review-relevant semantics, everything else
/// (including `notes`) participates.
fn canonical_review_projection(mapping: &Value) -> Value {
    let mut projection = mapping.clone();
    if let Value::Object(object) = &mut projection {
        object.remove("created");
        object.remove("updated");
        if let Some(Value::Object(review)) = object.get_mut("review") {
            review.remove("snapshot");
        }
    }
    projection
}

/// Recursively sort object keys (workspace `serde_json` uses `preserve_order`,
/// so `Value::Object` is insertion-ordered, not alphabetical) so serialization
/// is deterministic regardless of the source file's own key order. Arrays are
/// never reordered: their order is semantic content.
fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut sorted = Map::new();
            for key in keys {
                sorted.insert(key.clone(), canonicalize(&map[key]));
            }
            Value::Object(sorted)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize).collect()),
        other => other.clone(),
    }
}

fn canonical_bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(&canonicalize(value)).unwrap_or_default()
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    repopact_types::hex_digest(Sha256::digest(bytes))
}

fn digest_file(path: &Path) -> String {
    match fs::read(path) {
        Ok(bytes) => sha256_hex(&bytes),
        Err(_) => "absent".to_owned(),
    }
}

fn digest_repo_relative(root: &Path, relative: &str) -> String {
    let Some(resolved) = resolve_within_root(root, relative) else {
        return "unresolvable".to_owned();
    };
    if !resolved.exists() {
        return "absent".to_owned();
    }
    if !resolved.is_file() {
        return "unresolvable".to_owned();
    }
    digest_file(&resolved)
}

fn digest_indexed_by_id<'a>(records: impl Iterator<Item = &'a IndexedRecord>, id: &str) -> String {
    for record in records {
        if record.reference.id == id {
            return digest_file(&record.path);
        }
    }
    "absent".to_owned()
}

impl Validator {
    /// Compute the deterministic, read-only review snapshot for the
    /// assurance mapping identified by `mapping_id` (Decision 0055 section
    /// 5). Never mutates the mapping; the engine's `assurance.snapshot`
    /// operation and `repopact assurance snapshot` CLI surface this for the
    /// adopter to merge into their own record.
    pub fn compute_review_snapshot(&self, mapping_id: &str) -> Result<Value, String> {
        let record = self
            .index
            .assurance_mappings
            .iter()
            .find(|record| record.reference.id == mapping_id)
            .ok_or_else(|| format!("no assurance mapping with id '{mapping_id}'"))?;
        let data = record.value.clone().map_err(|error| {
            format!("assurance mapping '{mapping_id}' is not valid JSON: {error}")
        })?;
        let object = data
            .as_object()
            .ok_or_else(|| format!("assurance mapping '{mapping_id}' is not a JSON object"))?;
        let root = self.repository.root();

        let projection = canonical_review_projection(&data);
        let mapping_digest = sha256_hex(&canonical_bytes(&projection));

        let mut references = Vec::new();
        for control_ref in object
            .get("control_refs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(entry) = self.snapshot_control_ref(control_ref) {
                references.push(entry);
            }
        }
        for implementation_ref in object
            .get("implementation_refs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(entry) = self.snapshot_implementation_ref(implementation_ref, root) {
                references.push(entry);
            }
        }
        for evidence_ref in object
            .get("evidence_refs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(entry) = self.snapshot_evidence_ref(evidence_ref, root) {
                references.push(entry);
            }
        }
        for documentation_ref in object
            .get("documentation_refs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(entry) = self.snapshot_documentation_ref(documentation_ref, root) {
                references.push(entry);
            }
        }

        Ok(serde_json::json!({
            "mapping_digest": mapping_digest,
            "references": references,
        }))
    }

    fn snapshot_control_ref(&self, control_ref: &Value) -> Option<Value> {
        let control_ref = control_ref.as_object()?;
        let kind = control_ref.get("kind").and_then(Value::as_str)?;
        let reference = control_ref.get("ref").and_then(Value::as_str)?;
        let digest = match kind {
            "policy" => digest_indexed_by_id(self.index.policies.iter(), reference),
            "decision" => digest_indexed_by_id(self.index.decisions.iter(), reference),
            "work_item" => digest_indexed_by_id(self.index.work_items.iter(), reference),
            "invariant" => digest_file(&self.repository.root().join("governance/invariants.json")),
            "contract" => digest_repo_relative(self.repository.root(), reference),
            // "role" (and any future kind) has no canonical file to
            // fingerprint (WI051 phase-2 architecture review, coverage
            // limitation): recorded as unresolvable, never as drift.
            _ => "unresolvable".to_owned(),
        };
        Some(
            serde_json::json!({"category": "control", "kind": kind, "ref": reference, "digest": digest}),
        )
    }

    fn snapshot_implementation_ref(
        &self,
        implementation_ref: &Value,
        root: &Path,
    ) -> Option<Value> {
        let implementation_ref = implementation_ref.as_object()?;
        let kind = implementation_ref.get("kind").and_then(Value::as_str)?;
        let reference = implementation_ref.get("ref").and_then(Value::as_str)?;
        let digest = match kind {
            "decision" => digest_indexed_by_id(self.index.decisions.iter(), reference),
            "work_item" => digest_indexed_by_id(self.index.work_items.iter(), reference),
            "source" | "configuration" | "workflow" | "test" | "runtime_surface" => {
                digest_repo_relative(root, reference)
            }
            _ => "unresolvable".to_owned(),
        };
        Some(
            serde_json::json!({"category": "implementation", "kind": kind, "ref": reference, "digest": digest}),
        )
    }

    fn snapshot_evidence_ref(&self, evidence_ref: &Value, root: &Path) -> Option<Value> {
        let evidence_ref = evidence_ref.as_object()?;
        let kind = evidence_ref.get("kind").and_then(Value::as_str)?;
        match kind {
            "evidence_run" => {
                let reference = evidence_ref
                    .get("evidence_run_id")
                    .and_then(Value::as_str)?;
                let digest = digest_indexed_by_id(self.index.evidence.iter(), reference);
                Some(
                    serde_json::json!({"category": "evidence", "kind": kind, "ref": reference, "digest": digest}),
                )
            }
            "repository_artifact" => {
                let reference = evidence_ref.get("path").and_then(Value::as_str)?;
                let digest = digest_repo_relative(root, reference);
                Some(
                    serde_json::json!({"category": "evidence", "kind": kind, "ref": reference, "digest": digest}),
                )
            }
            // "hash" and "external_reference" evidence carry no locally
            // resolvable content of their own (Decision 0055 section 5/6);
            // their declared metadata still participates in mapping_digest.
            _ => None,
        }
    }

    fn snapshot_documentation_ref(&self, documentation_ref: &Value, root: &Path) -> Option<Value> {
        let documentation_ref = documentation_ref.as_object()?;
        let reference = documentation_ref.get("path").and_then(Value::as_str)?;
        let digest = digest_repo_relative(root, reference);
        Some(
            serde_json::json!({"category": "documentation", "kind": "documentation", "ref": reference, "digest": digest}),
        )
    }
}

// --- ACM-005: freshness, drift, and documentation claim-basis diagnostics --

fn parse_date_prefix(value: &str) -> Option<(i64, i64, i64)> {
    if value.len() < 10 {
        return None;
    }
    let year = value.get(0..4)?.parse::<i64>().ok()?;
    let month = value.get(5..7)?.parse::<i64>().ok()?;
    let day = value.get(8..10)?.parse::<i64>().ok()?;
    Some((year, month, day))
}

impl Validator {
    pub(crate) fn validate_review_and_claims(&mut self) {
        if self.index.assurance_mappings.is_empty() {
            return;
        }
        let root = self.repository.root().to_path_buf();
        let today = self.today.clone().unwrap_or_else(crate::today_utc);
        for record in self.index.assurance_mappings.clone() {
            let Ok(data) = record.value.clone() else {
                continue;
            };
            let Some(object) = data.as_object().cloned() else {
                continue;
            };
            let mapping_id = object
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();

            self.check_framework_version_stale(&object, &mapping_id, &record.path);
            self.check_review_due(&object, &mapping_id, &record.path, &today);
            if let Some(snapshot) = object
                .get("review")
                .and_then(Value::as_object)
                .and_then(|review| review.get("snapshot"))
            {
                self.check_review_drift(&mapping_id, &data, snapshot, &record.path, &root);
            }
            self.check_documentation_claims(&object, &mapping_id, &record.path, &root);
        }
    }

    fn check_framework_version_stale(
        &mut self,
        object: &Map<String, Value>,
        mapping_id: &str,
        path: &Path,
    ) {
        let framework_version = object
            .get("framework")
            .and_then(Value::as_object)
            .and_then(|framework| framework.get("version"))
            .and_then(Value::as_str);
        let reviewed_version = object
            .get("review")
            .and_then(Value::as_object)
            .and_then(|review| review.get("framework_version_reviewed"))
            .and_then(Value::as_str);
        if let (Some(current), Some(reviewed)) = (framework_version, reviewed_version) {
            if current != reviewed {
                self.push(self.warn_at(
                    "assurance.review-framework-version-stale",
                    format!(
                        "mapping '{mapping_id}' framework version '{current}' differs from the reviewed version '{reviewed}'; the review has not accounted for this framework revision"
                    ),
                    path,
                ));
            }
        }
    }

    fn check_review_due(
        &mut self,
        object: &Map<String, Value>,
        mapping_id: &str,
        path: &Path,
        today: &str,
    ) {
        let Some(review) = object.get("review").and_then(Value::as_object) else {
            return;
        };
        let Some(today_ymd) = parse_date_prefix(today) else {
            return;
        };
        let due_at = review.get("review_due_at").and_then(Value::as_str);
        let reviewed_at = review.get("reviewed_at").and_then(Value::as_str);
        let interval_days = review.get("review_interval_days").and_then(Value::as_i64);

        // Decision 0055 section 4: an explicit review_due_at wins; otherwise
        // a computed due date from reviewed_at + review_interval_days;
        // otherwise there is no automatic time-expiry rule.
        let due_ymd = if let Some(due_at) = due_at {
            parse_date_prefix(due_at)
        } else if let (Some(reviewed_at), Some(interval_days)) = (reviewed_at, interval_days) {
            parse_date_prefix(reviewed_at)
                .map(|(year, month, day)| add_days(year, month, day, interval_days))
        } else {
            None
        };
        let Some(due_ymd) = due_ymd else {
            return;
        };
        if days_since_epoch(due_ymd) < days_since_epoch(today_ymd) {
            self.push(self.warn_at(
                "assurance.review-overdue",
                format!(
                    "mapping '{mapping_id}' control review is overdue (due {}-{:02}-{:02}, today {today})",
                    due_ymd.0, due_ymd.1, due_ymd.2
                ),
                path,
            ));
        }
    }

    fn check_review_drift(
        &mut self,
        mapping_id: &str,
        data: &Value,
        snapshot: &Value,
        path: &Path,
        root: &Path,
    ) {
        let Some(snapshot) = snapshot.as_object() else {
            return;
        };
        let stored_mapping_digest = snapshot.get("mapping_digest").and_then(Value::as_str);
        let current_projection = canonical_review_projection(data);
        let current_mapping_digest = sha256_hex(&canonical_bytes(&current_projection));
        if let Some(stored) = stored_mapping_digest {
            if stored != current_mapping_digest {
                self.push(self.warn_at(
                    "assurance.review-mapping-drift",
                    format!(
                        "mapping '{mapping_id}' has changed since its last review snapshot (reviewed digest {stored}, current digest {current_mapping_digest}); re-review and take a fresh snapshot"
                    ),
                    path,
                ));
            }
        }
        for entry in snapshot
            .get("references")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(entry) = entry.as_object() else {
                continue;
            };
            let category = entry.get("category").and_then(Value::as_str).unwrap_or("");
            let kind = entry.get("kind").and_then(Value::as_str).unwrap_or("");
            let reference = entry.get("ref").and_then(Value::as_str).unwrap_or("");
            let stored_digest = entry.get("digest").and_then(Value::as_str).unwrap_or("");
            if stored_digest == "unresolvable" {
                // A permanently unfingerprintable reference (e.g. a "role"
                // control ref): never compared, never reported as drift.
                continue;
            }
            let current_digest = self.recompute_reference_digest(category, kind, reference, root);
            if current_digest == "absent" || current_digest == "unresolvable" {
                if stored_digest != current_digest {
                    self.push(self.warn_at(
                        "assurance.review-reference-missing",
                        format!(
                            "mapping '{mapping_id}' {category} reference '{reference}' ({kind}) was reviewed but is now {current_digest}"
                        ),
                        path,
                    ));
                }
            } else if current_digest != stored_digest {
                self.push(self.warn_at(
                    "assurance.review-reference-drift",
                    format!(
                        "mapping '{mapping_id}' {category} reference '{reference}' ({kind}) has changed since review (reviewed digest {stored_digest}, current digest {current_digest})"
                    ),
                    path,
                ));
            }
        }
    }

    fn recompute_reference_digest(
        &self,
        category: &str,
        kind: &str,
        reference: &str,
        root: &Path,
    ) -> String {
        match category {
            "control" => match kind {
                "policy" => digest_indexed_by_id(self.index.policies.iter(), reference),
                "decision" => digest_indexed_by_id(self.index.decisions.iter(), reference),
                "work_item" => digest_indexed_by_id(self.index.work_items.iter(), reference),
                "invariant" => digest_file(&root.join("governance/invariants.json")),
                "contract" => digest_repo_relative(root, reference),
                _ => "unresolvable".to_owned(),
            },
            "implementation" => match kind {
                "decision" => digest_indexed_by_id(self.index.decisions.iter(), reference),
                "work_item" => digest_indexed_by_id(self.index.work_items.iter(), reference),
                _ => digest_repo_relative(root, reference),
            },
            "evidence" => match kind {
                "evidence_run" => digest_indexed_by_id(self.index.evidence.iter(), reference),
                _ => digest_repo_relative(root, reference),
            },
            "documentation" => digest_repo_relative(root, reference),
            _ => "unresolvable".to_owned(),
        }
    }

    fn check_documentation_claims(
        &mut self,
        object: &Map<String, Value>,
        mapping_id: &str,
        path: &Path,
        root: &Path,
    ) {
        let has_control_refs = non_empty_array(object, "control_refs");
        let has_implementation_refs = non_empty_array(object, "implementation_refs");
        let has_evidence_refs = non_empty_array(object, "evidence_refs");
        let has_attestation = object.get("attestation").is_some_and(Value::is_object);
        let has_open_gap = non_empty_array(object, "gaps");

        for documentation_ref in object
            .get("documentation_refs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(documentation_ref) = documentation_ref.as_object() else {
                continue;
            };
            let claim_basis = documentation_ref
                .get("claim_basis")
                .and_then(Value::as_str)
                .unwrap_or("");
            let doc_path = documentation_ref
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or("");
            let supported = match claim_basis {
                "mapping" => true,
                "control_reference" => has_control_refs,
                "implementation_reference" => has_implementation_refs,
                "evidence_reference" => has_evidence_refs,
                "external_attestation_reference" => has_attestation,
                _ => true,
            };
            if !supported {
                self.push(self.at(
                    "assurance.documentation-claim-basis-unsupported",
                    format!(
                        "mapping '{mapping_id}' documentation '{doc_path}' declares claim_basis '{claim_basis}' but the mapping has no corresponding data to support it"
                    ),
                    path,
                ));
                continue;
            }
            if let Some(resolved) = resolve_within_root(root, doc_path) {
                if resolved.is_file() {
                    if let Ok(bytes) = fs::read(&resolved) {
                        if bytes.len() as u64 <= MAX_ARTIFACT_BYTES {
                            if let Some(text) = decode_text(&bytes) {
                                let lower = text.to_ascii_lowercase();
                                let strong_claim = STRONG_CLAIM_PHRASES
                                    .iter()
                                    .any(|phrase| lower.contains(phrase));
                                let weak_support =
                                    has_open_gap || !has_evidence_refs && !has_attestation;
                                if strong_claim && weak_support {
                                    self.push(self.warn_at(
                                        "assurance.documentation-claim-exceeds-support",
                                        format!(
                                            "mapping '{mapping_id}' documentation '{doc_path}' may claim stronger assurance than this record's evidence/attestation/gap state supports; this is an advisory heuristic over the declared documentation file only, not a legal or classification determination"
                                        ),
                                        path,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn non_empty_array(object: &Map<String, Value>, key: &str) -> bool {
    object
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|array| !array.is_empty())
}

fn days_since_epoch(ymd: (i64, i64, i64)) -> i64 {
    crate::days_from_civil(ymd.0, ymd.1, ymd.2)
}

fn add_days(year: i64, month: i64, day: i64, days: i64) -> (i64, i64, i64) {
    crate::civil_from_days(crate::days_from_civil(year, month, day) + days)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luhn_detects_a_valid_test_card_number() {
        // Well-known synthetic Visa test PAN (never a real card).
        assert!(detect_luhn_valid_pan("card: 4111 1111 1111 1111 end"));
    }

    #[test]
    fn luhn_rejects_an_arbitrary_16_digit_number() {
        assert!(!detect_luhn_valid_pan("id: 1234567890123456"));
    }

    #[test]
    fn luhn_ignores_short_digit_runs() {
        assert!(!detect_luhn_valid_pan("port 8080, count 42"));
    }

    #[test]
    fn private_key_marker_is_detected() {
        assert_eq!(
            detect_private_key_marker("-----BEGIN OPENSSH PRIVATE KEY-----\nabc"),
            Some("-----BEGIN OPENSSH PRIVATE KEY-----")
        );
        assert_eq!(detect_private_key_marker("no key here"), None);
    }

    #[test]
    fn secret_assignment_pattern_matches_common_forms() {
        assert!(detect_secret_assignment(
            "api_key=synthetic-test-value-only"
        ));
        assert!(detect_secret_assignment("password: synthetic-test-value"));
        assert!(!detect_secret_assignment("this password field is empty"));
        assert!(!detect_secret_assignment("mypasswordfield=x"));
    }

    #[test]
    fn identity_markers_require_assignment_shape() {
        assert!(detect_identity_markers("ssn=000-00-0000"));
        assert!(!detect_identity_markers("no ssn present here"));
    }

    #[test]
    fn credential_in_uri_requires_userinfo_colon() {
        assert!(has_credential_in_uri("https://user:pass@example.invalid/x"));
        assert!(!has_credential_in_uri("https://example.invalid/x"));
        assert!(!has_credential_in_uri("https://user@example.invalid/x"));
    }

    #[test]
    fn secret_query_param_is_detected() {
        assert!(has_secret_query_param(
            "https://example.invalid/x?token=abc123"
        ));
        assert!(!has_secret_query_param(
            "https://example.invalid/x?ref=abc123"
        ));
        assert!(!has_secret_query_param("https://example.invalid/x?token="));
    }

    #[test]
    fn canonical_projection_excludes_snapshot_and_timestamps() {
        let mapping = serde_json::json!({
            "id": "example",
            "created": "2026-01-01",
            "updated": "2026-01-02",
            "review": {"reviewed_at": "2026-01-01T00:00:00Z", "snapshot": {"mapping_digest": "x"}},
        });
        let projection = canonical_review_projection(&mapping);
        assert_eq!(projection.get("created"), None);
        assert_eq!(projection.get("updated"), None);
        assert_eq!(
            projection
                .get("review")
                .and_then(|review| review.get("snapshot")),
            None
        );
        assert!(projection
            .get("review")
            .and_then(|review| review.get("reviewed_at"))
            .is_some());
    }

    #[test]
    fn canonical_digest_is_stable_regardless_of_source_key_order() {
        let a = serde_json::json!({"b": 1, "a": 2});
        let b = serde_json::json!({"a": 2, "b": 1});
        assert_eq!(
            sha256_hex(&canonical_bytes(&a)),
            sha256_hex(&canonical_bytes(&b))
        );
    }
}
