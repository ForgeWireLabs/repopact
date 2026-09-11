use std::path::{Path, PathBuf};

use jsonschema::Validator;
use repopact_types::Diagnostic;
use serde_json::Value;

const EMBEDDED_SCHEMAS: &[(&str, &str)] = &[
    (
        "evidence-run.schema.json",
        include_str!("../../../../repopact/schemas/evidence-run.schema.json"),
    ),
    (
        "frozen-surface.schema.json",
        include_str!("../../../../repopact/schemas/frozen-surface.schema.json"),
    ),
    (
        "invariants.schema.json",
        include_str!("../../../../repopact/schemas/invariants.schema.json"),
    ),
    (
        "work-item.schema.json",
        include_str!("../../../../repopact/schemas/work-item.schema.json"),
    ),
    (
        "adopter-fleet.schema.json",
        include_str!("../../../../repopact/schemas/adopter-fleet.schema.json"),
    ),
];

#[derive(Debug, Clone)]
pub struct SchemaStore {
    root: PathBuf,
}

impl SchemaStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn source_bytes(&self, name: &str) -> Option<Vec<u8>> {
        let local = self.root.join("schemas").join(name);
        if local.is_file() {
            return std::fs::read(local).ok();
        }
        embedded(name).map(|raw| raw.as_bytes().to_vec())
    }

    pub fn validate(
        &self,
        instance: &Value,
        schema_name: &str,
        record_path: &Path,
        relative_path: impl Fn(&Path) -> String,
    ) -> Vec<Diagnostic> {
        let Some(bytes) = self.source_bytes(schema_name) else {
            return vec![Diagnostic::error(
                "schema.authority.unavailable",
                format!("unsupported schema contract '{schema_name}'"),
            )
            .with_path(relative_path(record_path))];
        };
        let schema: Value = match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(error) => {
                return vec![Diagnostic::error(
                    "schema.authority.invalid",
                    format!("canonical schema '{schema_name}' is not valid JSON: {error}"),
                )
                .with_path(relative_path(record_path))]
            }
        };
        let validator: Validator = match jsonschema::validator_for(&schema) {
            Ok(value) => value,
            Err(error) => {
                return vec![Diagnostic::error(
                    "schema.authority.invalid",
                    format!("canonical schema '{schema_name}' cannot be compiled: {error}"),
                )
                .with_path(relative_path(record_path))]
            }
        };
        let mut diagnostics = validator
            .iter_errors(instance)
            .map(|error| {
                let location = error.instance_path().as_str().trim_start_matches('/');
                let location = if location.is_empty() {
                    "<root>"
                } else {
                    location
                };
                Diagnostic::error(
                    "schema.invalid",
                    format!("schema {location}: {}", error.to_string().replace('"', "'")),
                )
                .with_path(relative_path(record_path))
                .with_field(location)
            })
            .collect::<Vec<_>>();
        diagnostics.sort_by(|left, right| left.message.cmp(&right.message));
        diagnostics
    }
}

pub fn embedded(name: &str) -> Option<&'static str> {
    EMBEDDED_SCHEMAS
        .iter()
        .find_map(|(candidate, content)| (*candidate == name).then_some(*content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_schema_is_the_checked_in_canonical_bytes() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let store = SchemaStore::new(root.join("does-not-exist"));
        for (name, content) in EMBEDDED_SCHEMAS {
            let source = std::fs::read_to_string(root.join("repopact/schemas").join(name)).unwrap();
            assert_eq!(source, *content);
            assert_eq!(store.source_bytes(name).unwrap(), content.as_bytes());
        }
    }
}
