//! WI063 durable Repository Orientation Graph structural validation,
//! integrated into the canonical Rust validation path (ROG-031) rather
//! than a separate Python-only validator. A repository with no durable
//! graph (`rog/manifest.json` absent) remains fully valid -- see Decision
//! 0044 section 9 for the capability contract this enforces.

use repopact_graph::validate::validate_structure;

use crate::Validator;

impl Validator {
    pub(crate) fn validate_graph(&mut self) {
        let diagnostics = validate_structure(self.repository.root());
        for diagnostic in diagnostics {
            if diagnostic.code == "graph.absent" {
                // Graph-disabled repositories are valid RepoPact
                // repositories (Decision 0044 section 9); this is not a
                // reportable diagnostic.
                continue;
            }
            self.push(self.at(diagnostic.code, diagnostic.message, self.repository.root()));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use repopact_repository::Repository;

    fn temp_root(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("repopact-validation-graph-{name}-{suffix}"));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn graph_absent_repository_reports_no_graph_diagnostics() {
        let root = temp_root("absent");
        std::fs::write(root.join("README.md"), "# fixture\n").unwrap();
        let report = crate::validate(&root);
        assert!(
            !report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code.starts_with("graph.")),
            "graph-disabled repository must not report graph.* diagnostics: {:?}",
            report.diagnostics
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn valid_durable_graph_reports_no_graph_diagnostics() {
        let root = temp_root("valid");
        std::fs::write(root.join("README.md"), "# fixture\n").unwrap();
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        repopact_graph::build_and_write(&snapshot).expect("build");

        let report = crate::validate(&root);
        assert!(
            !report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code.starts_with("graph.")),
            "a freshly built, unmodified durable graph must validate cleanly: {:?}",
            report.diagnostics
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn corrupt_durable_graph_is_surfaced_by_canonical_validate() {
        let root = temp_root("corrupt");
        std::fs::write(root.join("README.md"), "# fixture\n").unwrap();
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        repopact_graph::build_and_write(&snapshot).expect("build");

        let manifest_path = root.join("rog").join("manifest.json");
        let mut manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
        manifest["graph_schema_version"] = serde_json::Value::from(999);
        std::fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let report = crate::validate(&root);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "graph.schema-unsupported"),
            "canonical `repopact validate` must surface durable-graph corruption, got: {:?}",
            report.diagnostics
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
