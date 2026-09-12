use std::collections::BTreeSet;
use std::path::{Component, Path};

use repopact_repository::{path_string, resolve_within_root, RepositorySnapshot};
use repopact_schema::SchemaStore;
use repopact_types::Diagnostic;
use serde_json::Value;

const VERIFICATION_PATH: &str = "governance/verification.json";
const VERIFICATION_SCHEMA: &str = "verification-profile.schema.json";
const PLACEHOLDERS: [&str; 3] = ["{python}", "{repopact}", "{root}"];

pub(crate) fn validate_snapshot(snapshot: &RepositorySnapshot) -> Vec<Diagnostic> {
    let root = snapshot.repository().root();
    let path = root.join(VERIFICATION_PATH);
    let Some(text) = snapshot.index().text(&path) else {
        return Vec::new();
    };
    let value: Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(error) => {
            return vec![Diagnostic::error(
                "verification.json-invalid",
                format!("verification contract is not valid JSON: {error}"),
            )
            .with_path(VERIFICATION_PATH)];
        }
    };

    let schemas = SchemaStore::new(root);
    let mut diagnostics = schemas.validate(&value, VERIFICATION_SCHEMA, &path, |candidate| {
        snapshot.repository().relative_path(candidate)
    });
    diagnostics.extend(validate_semantics(snapshot, &value));
    diagnostics
}

fn validate_semantics(snapshot: &RepositorySnapshot, value: &Value) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let Some(profiles) = value.get("profiles").and_then(Value::as_object) else {
        return diagnostics;
    };

    if let Some(default) = value.get("default_profile").and_then(Value::as_str) {
        if !profiles.contains_key(default) {
            diagnostics.push(at(
                "verification.default-profile-missing",
                format!("default_profile {default:?} does not name a declared verification profile"),
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
                diagnostics.push(at(
                    "verification.step-duplicate",
                    format!("profile {profile_name:?} contains duplicate step id {step_id:?}"),
                ));
            }

            let cwd = object.get("cwd").and_then(Value::as_str).unwrap_or(".");
            if !safe_cwd(snapshot, cwd) {
                diagnostics.push(at(
                    "verification.cwd-escape",
                    format!(
                        "profile {profile_name:?} step {step_id:?} cwd must stay inside the repository: {cwd:?}"
                    ),
                ));
            }

            if let Some(argv) = object.get("argv").and_then(Value::as_array) {
                for token in argv.iter().filter_map(Value::as_str) {
                    if token.starts_with('{')
                        && token.ends_with('}')
                        && !PLACEHOLDERS.contains(&token)
                    {
                        diagnostics.push(at(
                            "verification.placeholder-unknown",
                            format!(
                                "profile {profile_name:?} step {step_id:?} uses unknown placeholder {token:?}"
                            ),
                        ));
                    }
                }
            }
        }
    }
    diagnostics
}

fn safe_cwd(snapshot: &RepositorySnapshot, value: &str) -> bool {
    let relative = Path::new(value);
    if relative.is_absolute() {
        return false;
    }
    // Reject an obvious lexical escape even when the path does not exist. For
    // existing paths, resolve_within_root also follows symlinks/reparse points
    // through the repository crate's canonical containment boundary.
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
    resolve_within_root(snapshot.repository().root(), value).is_some()
}

fn at(code: impl Into<String>, message: impl Into<String>) -> Diagnostic {
    Diagnostic::error(code, message).with_path(path_string(Path::new(VERIFICATION_PATH)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use repopact_repository::Repository;
    use std::fs;
    use tempfile::TempDir;

    fn snapshot(contract: &str) -> (TempDir, RepositorySnapshot) {
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join("governance")).unwrap();
        fs::write(temp.path().join(VERIFICATION_PATH), contract).unwrap();
        let snapshot = Repository::open(temp.path()).session().snapshot();
        (temp, snapshot)
    }

    #[test]
    fn snapshot_contract_accepts_local_first_profile() {
        let (_temp, snapshot) = snapshot(
            r#"{
              "$schema":"../schemas/verification-profile.schema.json",
              "version":1,
              "default_profile":"quick",
              "execution_policy":{"local_primary":true,"hosted_ci_default":false,"hosted_cd_default":false},
              "profiles":{"quick":{"description":"quick","coverage":"host","steps":[{"id":"validate","argv":["{repopact}","validate"]}]}}
            }"#,
        );
        assert!(validate_snapshot(&snapshot).is_empty());
    }

    #[test]
    fn snapshot_contract_rejects_duplicate_step_and_unknown_placeholder() {
        let (_temp, snapshot) = snapshot(
            r#"{
              "$schema":"../schemas/verification-profile.schema.json",
              "version":1,
              "execution_policy":{"local_primary":true,"hosted_ci_default":false,"hosted_cd_default":false},
              "profiles":{"quick":{"description":"quick","steps":[
                {"id":"same","argv":["{provider_secret}"]},
                {"id":"same","argv":["{python}","-c","pass"]}
              ]}}
            }"#,
        );
        let diagnostics = validate_snapshot(&snapshot);
        assert!(diagnostics.iter().any(|item| item.code == "verification.step-duplicate"));
        assert!(diagnostics.iter().any(|item| item.code == "verification.placeholder-unknown"));
    }

    #[test]
    fn snapshot_contract_rejects_cwd_escape() {
        let (_temp, snapshot) = snapshot(
            r#"{
              "$schema":"../schemas/verification-profile.schema.json",
              "version":1,
              "execution_policy":{"local_primary":true,"hosted_ci_default":false,"hosted_cd_default":false},
              "profiles":{"quick":{"description":"quick","steps":[{"id":"escape","cwd":"../outside","argv":["{python}"]}]}}
            }"#,
        );
        assert!(validate_snapshot(&snapshot)
            .iter()
            .any(|item| item.code == "verification.cwd-escape"));
    }
}
