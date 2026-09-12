use std::collections::BTreeSet;
use std::path::{Component, Path};

use repopact_repository::resolve_within_root;
use repopact_types::Diagnostic;
use serde_json::Value;

use crate::Validator;

const VERIFICATION_PATH: &str = "governance/verification.json";
const VERIFICATION_SCHEMA: &str = "verification-profile.schema.json";
const PLACEHOLDERS: [&str; 3] = ["{python}", "{repopact}", "{root}"];

pub(super) fn validate(validator: &mut Validator) {
    let path = validator.repository.root().join(VERIFICATION_PATH);
    let Some(text) = validator.index.text(&path).map(str::to_owned) else {
        return;
    };
    let value: Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => {
            validator.push(validator.at(
                "verification.json-invalid",
                format!("verification contract is not valid JSON: {error}"),
                &path,
            ));
            return;
        }
    };

    validator.extend_schema(&value, VERIFICATION_SCHEMA, &path);
    for diagnostic in validate_semantics(validator, &value, &path) {
        validator.push(diagnostic);
    }
}

fn validate_semantics(validator: &Validator, value: &Value, path: &Path) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let Some(profiles) = value.get("profiles").and_then(Value::as_object) else {
        return diagnostics;
    };

    if let Some(default) = value.get("default_profile").and_then(Value::as_str) {
        if !profiles.contains_key(default) {
            diagnostics.push(validator.at(
                "verification.default-profile-missing",
                format!("default_profile {default:?} does not name a declared verification profile"),
                path,
            ));
        }
    }

    for (profile_name, profile) in profiles {
        let Some(steps) = profile.get("steps").and_then(Value::as_array) else {
            continue;
        };
        let mut seen = BTreeSet::new();
        for step in steps {
            let Some(object) = step.as_object() else {
                continue;
            };
            let step_id = object.get("id").and_then(Value::as_str).unwrap_or("");
            if !seen.insert(step_id.to_owned()) {
                diagnostics.push(validator.at(
                    "verification.step-duplicate",
                    format!("profile {profile_name:?} contains duplicate step id {step_id:?}"),
                    path,
                ));
            }

            let cwd = object.get("cwd").and_then(Value::as_str).unwrap_or(".");
            if !safe_cwd(validator, cwd) {
                diagnostics.push(validator.at(
                    "verification.cwd-escape",
                    format!(
                        "profile {profile_name:?} step {step_id:?} cwd must stay inside the repository: {cwd:?}"
                    ),
                    path,
                ));
            }

            if let Some(argv) = object.get("argv").and_then(Value::as_array) {
                for token in argv.iter().filter_map(Value::as_str) {
                    if token.starts_with('{')
                        && token.ends_with('}')
                        && !PLACEHOLDERS.contains(&token)
                    {
                        diagnostics.push(validator.at(
                            "verification.placeholder-unknown",
                            format!(
                                "profile {profile_name:?} step {step_id:?} uses unknown placeholder {token:?}"
                            ),
                            path,
                        ));
                    }
                }
            }
        }
    }
    diagnostics
}

fn safe_cwd(validator: &Validator, value: &str) -> bool {
    let relative = Path::new(value);
    if relative.is_absolute() {
        return false;
    }
    let mut depth = 0usize;
    for component in relative.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(_) => depth += 1,
            Component::ParentDir if depth > 0 => depth -= 1,
            Component::ParentDir => return false,
            Component::RootDir | Component::Prefix(_) => return false,
        }
    }
    resolve_within_root(validator.repository.root(), value).is_some()
}
