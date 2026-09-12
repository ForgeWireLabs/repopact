//! WI059 CVP-003: local `governance/adopters.json` validation semantics,
//! ported from `repopact.validate_repo.validate_adopter_manifest`.
//!
//! This module owns *local manifest validity* only: schema conformance,
//! unique adopter identities, and vendored-overlay checksum drift against the
//! already-materialized repository snapshot. Cross-repository adopter fleet
//! verification (`fleet_verify.py`) remains Python-owned and is not
//! referenced here; nothing in this module performs network or subprocess
//! access.

use std::collections::HashSet;
use std::path::Path;

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::Validator;

impl Validator {
    pub(crate) fn validate_adopters(&mut self) {
        let path = self.repository.root().join("governance/adopters.json");
        let Some(record) = self.index.adopters.clone() else {
            return;
        };
        let Ok(value) = record.value else {
            self.push(self.at(
                "adopters.unreadable",
                "governance/adopters.json is not valid JSON",
                &path,
            ));
            return;
        };
        self.extend_schema(&value, "adopter-fleet.schema.json", &path);

        let entries: Vec<&Value> = value
            .get("adopters")
            .and_then(Value::as_array)
            .map(|items| items.iter().collect())
            .unwrap_or_default();

        let ids: Vec<String> = entries
            .iter()
            .map(|entry| {
                entry
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned()
            })
            .collect();
        if !all_unique(&ids) {
            self.push(self.at("adopters.duplicate-id", "adopter ids must be unique", &path));
        }

        let repositories: Vec<String> = entries
            .iter()
            .map(|entry| {
                normalize_repository_identity(
                    entry
                        .get("repository")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                )
            })
            .collect();
        if !all_unique(&repositories) {
            self.push(self.at(
                "adopters.duplicate-repository",
                "adopter remote identities must be unique",
                &path,
            ));
        }

        for entry in &entries {
            let Some(consumption) = entry.get("consumption") else {
                continue;
            };
            if consumption.get("type").and_then(Value::as_str) != Some("vendored") {
                continue;
            }
            let Some(files) = consumption.get("files").and_then(Value::as_array) else {
                continue;
            };
            for contract in files {
                if contract.get("mode").and_then(Value::as_str) != Some("overlay") {
                    continue;
                }
                self.validate_overlay_contract(contract, &path);
            }
        }
    }

    fn validate_overlay_contract(&mut self, contract: &Value, manifest_path: &Path) {
        let overlay_relative = contract
            .get("overlay_path")
            .and_then(Value::as_str)
            .unwrap_or("");
        let Some(overlay) =
            repopact_repository::resolve_within_root(self.repository.root(), overlay_relative)
        else {
            self.push(self.at(
                "adopters.overlay-path-escape",
                format!("vendored overlay path escapes the repository: {overlay_relative}"),
                manifest_path,
            ));
            return;
        };
        let Some(bytes) = self.index.overlay_bytes(&overlay) else {
            self.push(self.at(
                "adopters.overlay-missing",
                format!("vendored overlay is unreadable: {overlay_relative}"),
                manifest_path,
            ));
            return;
        };
        let normalized = normalize_crlf(bytes);
        let digest = format!("{:x}", Sha256::digest(&normalized));
        let expected = contract
            .get("overlay_sha256")
            .and_then(Value::as_str)
            .unwrap_or("");
        if digest != expected {
            self.push(self.at(
                "adopters.overlay-checksum-mismatch",
                format!("vendored overlay checksum drift: {overlay_relative}"),
                manifest_path,
            ));
        }
    }
}

fn all_unique(values: &[String]) -> bool {
    let mut seen = HashSet::new();
    values.iter().all(|value| seen.insert(value.clone()))
}

/// Repository-identity normalization: lowercase, trailing `.git` removed.
/// Mirrors Python's `str(repository).lower().removesuffix(".git")`.
fn normalize_repository_identity(repository: &str) -> String {
    let lower = repository.to_ascii_lowercase();
    lower
        .strip_suffix(".git")
        .map(str::to_owned)
        .unwrap_or(lower)
}

/// `bytes.replace(b"\r\n", b"\n")`, matching Python's checksum normalization
/// so a downstream checkout's line endings do not produce checksum drift.
fn normalize_crlf(bytes: &[u8]) -> Vec<u8> {
    let mut normalized = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\r' && bytes.get(index + 1) == Some(&b'\n') {
            index += 1;
            continue;
        }
        normalized.push(bytes[index]);
        index += 1;
    }
    normalized
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use repopact_repository::Repository;
    use serde_json::json;
    use sha2::{Digest, Sha256};

    use crate::Validator;

    fn temp_root(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("repopact-rust-adopters-{name}-{suffix}"));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_json(path: &std::path::Path, value: &serde_json::Value) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    }

    fn codes(root: &std::path::Path) -> Vec<String> {
        Validator::new(Repository::open(root))
            .validate()
            .diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect()
    }

    fn has_code(root: &std::path::Path, code: &str) -> bool {
        codes(root).iter().any(|found| found == code)
    }

    fn pypi_adopter(id: &str, repository: &str) -> serde_json::Value {
        json!({
            "id": id,
            "repository": repository,
            "default_branch": "main",
            "consumption": {"type": "pypi", "package": "repopact", "version_file": "VERSION"},
            "validation_commands": ["repopact validate"],
        })
    }

    fn manifest(adopters: Vec<serde_json::Value>) -> serde_json::Value {
        json!({
            "version": 1,
            "upstream": {"repository": "ForgeWireLabs/repopact", "version_file": "VERSION"},
            "adopters": adopters,
        })
    }

    #[test]
    fn absent_manifest_is_silent() {
        let root = temp_root("absent");
        assert!(!has_code(&root, "adopters.duplicate-id"));
        assert!(!codes(&root)
            .iter()
            .any(|code| code.starts_with("adopters.")));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn valid_manifest_is_silent() {
        let root = temp_root("valid");
        write_json(
            &root.join("governance/adopters.json"),
            &manifest(vec![
                pypi_adopter("adopter-one", "org/repo-one"),
                pypi_adopter("adopter-two", "org/repo-two"),
            ]),
        );
        assert!(!codes(&root)
            .iter()
            .any(|code| code.starts_with("adopters.")));
        assert!(!has_code(&root, "unsupported.semantic-surface"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn schema_invalid_manifest_is_rejected() {
        let root = temp_root("schema-invalid");
        write_json(
            &root.join("governance/adopters.json"),
            &json!({"version": 1, "upstream": {"repository": "org/repo"}, "adopters": []}),
        );
        assert!(has_code(&root, "schema.invalid"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn duplicate_adopter_id_is_rejected() {
        let root = temp_root("duplicate-id");
        write_json(
            &root.join("governance/adopters.json"),
            &manifest(vec![
                pypi_adopter("adopter-one", "org/repo-one"),
                pypi_adopter("adopter-one", "org/repo-two"),
            ]),
        );
        assert!(has_code(&root, "adopters.duplicate-id"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn duplicate_normalized_repository_identity_is_rejected() {
        let root = temp_root("duplicate-repo");
        write_json(
            &root.join("governance/adopters.json"),
            &manifest(vec![
                pypi_adopter("adopter-one", "Org/Repo.git"),
                pypi_adopter("adopter-two", "org/repo"),
            ]),
        );
        assert!(has_code(&root, "adopters.duplicate-repository"));
        fs::remove_dir_all(root).unwrap();
    }

    fn vendored_adopter(
        id: &str,
        repository: &str,
        overlay_path: &str,
        overlay_sha256: &str,
    ) -> serde_json::Value {
        json!({
            "id": id,
            "repository": repository,
            "default_branch": "main",
            "consumption": {
                "type": "vendored",
                "version_file": "VERSION",
                "upstream_version": "1.0.0",
                "upstream_revision": "0".repeat(40),
                "files": [
                    {
                        "upstream_path": "repopact/cli.py",
                        "adopter_path": "vendor/repopact/cli.py",
                        "mode": "overlay",
                        "upstream_sha256": "0".repeat(64),
                        "adopter_sha256": "0".repeat(64),
                        "overlay_path": overlay_path,
                        "overlay_sha256": overlay_sha256,
                    }
                ],
            },
            "validation_commands": ["repopact validate"],
        })
    }

    #[test]
    fn valid_overlay_checksum_is_accepted() {
        let root = temp_root("overlay-valid");
        let overlay_content = b"vendored overlay content\n";
        fs::create_dir_all(root.join("overlays")).unwrap();
        fs::write(root.join("overlays/cli.py"), overlay_content).unwrap();
        let digest = format!("{:x}", Sha256::digest(overlay_content));
        write_json(
            &root.join("governance/adopters.json"),
            &manifest(vec![vendored_adopter(
                "adopter-one",
                "org/repo-one",
                "overlays/cli.py",
                &digest,
            )]),
        );
        assert!(!has_code(&root, "adopters.overlay-checksum-mismatch"));
        assert!(!has_code(&root, "adopters.overlay-missing"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_overlay_checksum_is_rejected() {
        let root = temp_root("overlay-invalid");
        fs::create_dir_all(root.join("overlays")).unwrap();
        fs::write(root.join("overlays/cli.py"), b"vendored overlay content\n").unwrap();
        write_json(
            &root.join("governance/adopters.json"),
            &manifest(vec![vendored_adopter(
                "adopter-one",
                "org/repo-one",
                "overlays/cli.py",
                &"f".repeat(64),
            )]),
        );
        assert!(has_code(&root, "adopters.overlay-checksum-mismatch"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_overlay_is_rejected() {
        let root = temp_root("overlay-missing");
        write_json(
            &root.join("governance/adopters.json"),
            &manifest(vec![vendored_adopter(
                "adopter-one",
                "org/repo-one",
                "overlays/does-not-exist.py",
                &"0".repeat(64),
            )]),
        );
        assert!(has_code(&root, "adopters.overlay-missing"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn overlay_path_escaping_repository_is_rejected() {
        let root = temp_root("overlay-escape");
        write_json(
            &root.join("governance/adopters.json"),
            &manifest(vec![vendored_adopter(
                "adopter-one",
                "org/repo-one",
                "../outside.py",
                &"0".repeat(64),
            )]),
        );
        assert!(has_code(&root, "adopters.overlay-path-escape"));
        fs::remove_dir_all(root).unwrap();
    }
}
