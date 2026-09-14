//! Typed graph freshness/capability status (WI063 ROG-010, Decision 0044
//! section 10).

use repopact_repository::Repository;
use serde::{Deserialize, Serialize};

use crate::capability::{self, CapabilityState};
use crate::durable;
use crate::projection::SourceProjection;
use crate::validate::{self, GraphDiagnostic};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    /// No `rog/manifest.json` exists. The repository remains a fully
    /// valid RepoPact repository (Decision 0044 section 9).
    Absent,
    Fresh,
    /// Reserved for the future watcher/dirty-tree overlay (ROG-013).
    /// Never emitted by this session's implementation.
    WorkingOverlay,
    /// Structurally valid and fingerprint-fresh, but at least one
    /// supported source file could not be fully processed (parse error,
    /// cancellation, or adapter-internal failure -- not merely an
    /// unsupported language or a policy exclusion, which are expected,
    /// not gaps). The durable data is genuinely usable; the coverage gap
    /// is explicit rather than silently absorbed into a plain `Fresh`
    /// verdict (WI063 semantic-checkpoint requirement).
    Partial,
    Stale,
    Unsupported,
    Corrupt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStatus {
    pub freshness: Freshness,
    /// The five-state ROG capability (Decision 0051, ROG-039), computed
    /// independently of `freshness`/`manifest`/`diagnostics`. In
    /// particular, `EnabledMissing` is a hard binding failure that is
    /// never collapsed into `Freshness::Absent` -- callers that only
    /// inspect `freshness` for backward compatibility still see a
    /// non-`Absent`, non-`Fresh` value (`Corrupt`) in that case, and
    /// callers that want the precise reason inspect `capability_state`.
    pub capability_state: CapabilityState,
    pub manifest: Option<durable::Manifest>,
    pub diagnostics: Vec<GraphDiagnosticView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphDiagnosticView {
    pub code: String,
    pub message: String,
}

impl From<GraphDiagnostic> for GraphDiagnosticView {
    fn from(diagnostic: GraphDiagnostic) -> Self {
        Self {
            code: diagnostic.code,
            message: diagnostic.message,
        }
    }
}

/// Compute current graph status for `repository`. This recomputes the
/// source projection fingerprint (a full projection walk) when the
/// durable graph is otherwise structurally sound, in order to detect
/// staleness -- callers that only need structural validity without a
/// fresh walk should use [`crate::validate::validate_structure`] directly.
pub fn status(repository: &Repository) -> GraphStatus {
    // Read the capability declaration once; a malformed record is
    // itself reported as a diagnostic below rather than silently
    // treated as absent (Decision 0051 section 2 -- an adopter that
    // committed a capability record at all made an explicit claim).
    let declaration_result = capability::read_declaration(repository.root());
    let declaration = declaration_result.as_ref().ok().copied().flatten();

    let diagnostics = validate::validate_structure(repository.root());
    let rog_dir_exists = !diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "graph.absent");

    // A malformed capability declaration is always reported, regardless
    // of whether `rog/` exists -- an adopter that committed a
    // capability record at all made an explicit claim, and a broken
    // claim must never be silently treated as "no claim" just because
    // the repository also happens to have no graph yet.
    if let Err(error) = &declaration_result {
        return GraphStatus {
            freshness: Freshness::Corrupt,
            capability_state: capability::resolve_state(None, rog_dir_exists),
            manifest: durable::read_manifest(repository.root()).ok().flatten(),
            diagnostics: vec![GraphDiagnosticView {
                code: "graph.capability-malformed".to_owned(),
                message: error.to_string(),
            }],
        };
    }

    if !rog_dir_exists {
        let capability_state = capability::resolve_state(declaration, false);
        // ROG-039: `enabled` but `rog/` is absent is a hard, binding
        // failure -- never reported as `Freshness::Absent` ("no graph,
        // valid"), and never silently collapsed into it.
        if capability_state == CapabilityState::EnabledMissing {
            return GraphStatus {
                freshness: Freshness::Corrupt,
                capability_state,
                manifest: None,
                diagnostics: vec![GraphDiagnosticView {
                    code: "graph.capability-enabled-but-missing".to_owned(),
                    message: "capability declares rog=enabled but no durable graph exists; \
                              run `repopact graph build` or `repopact graph disable`"
                        .to_owned(),
                }],
            };
        }
        return GraphStatus {
            freshness: Freshness::Absent,
            capability_state,
            manifest: None,
            diagnostics: Vec::new(),
        };
    }
    let capability_state = capability::resolve_state(declaration, true);
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "graph.schema-unsupported")
    {
        return GraphStatus {
            freshness: Freshness::Unsupported,
            capability_state,
            manifest: durable::read_manifest(repository.root()).ok().flatten(),
            diagnostics: diagnostics
                .into_iter()
                .map(GraphDiagnosticView::from)
                .collect(),
        };
    }
    if !diagnostics.is_empty() {
        return GraphStatus {
            freshness: Freshness::Corrupt,
            capability_state,
            manifest: durable::read_manifest(repository.root()).ok().flatten(),
            diagnostics: diagnostics
                .into_iter()
                .map(GraphDiagnosticView::from)
                .collect(),
        };
    }

    let manifest = match durable::read_manifest(repository.root()) {
        Ok(Some(manifest)) => manifest,
        _ => {
            return GraphStatus {
                freshness: Freshness::Corrupt,
                capability_state,
                manifest: None,
                diagnostics: vec![GraphDiagnosticView {
                    code: "graph.manifest-missing".to_owned(),
                    message: "manifest disappeared between structural checks".to_owned(),
                }],
            }
        }
    };

    let topology = repository.topology();
    let current_fingerprint = SourceProjection::build(repository, &topology).fingerprint();
    let freshness = if current_fingerprint != manifest.source_projection_fingerprint {
        Freshness::Stale
    } else if has_semantic_coverage_gap(&manifest) {
        Freshness::Partial
    } else {
        Freshness::Fresh
    };
    GraphStatus {
        freshness,
        capability_state,
        manifest: Some(manifest),
        diagnostics: Vec::new(),
    }
}

/// A coverage gap is a supported file that could not be fully processed
/// (`files_partial`/`files_failed`) -- never merely an unsupported
/// language or a policy exclusion, both of which are expected outcomes,
/// not gaps in what the graph should have covered.
pub(crate) fn has_semantic_coverage_gap(manifest: &durable::Manifest) -> bool {
    manifest
        .semantic_coverage
        .as_ref()
        .is_some_and(|coverage| coverage.files_partial > 0 || coverage.files_failed > 0)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("repopact-status-capability-{name}-{suffix}"));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn seeded(name: &str) -> PathBuf {
        let root = temp_root(name);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/lib.rs"), "pub fn hello() {}\n").unwrap();
        root
    }

    // ---- ROG-039 required state matrix (Decision 0051 section 1) ----

    #[test]
    fn legacy_absent_is_valid_and_disabled() {
        let root = seeded("legacy-absent");
        let repository = Repository::open(&root);
        let result = status(&repository);
        assert_eq!(result.capability_state, CapabilityState::LegacyAbsent);
        assert_eq!(result.freshness, Freshness::Absent);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn legacy_enabled_stays_binding_with_no_capability_record() {
        let root = seeded("legacy-enabled");
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        crate::build_and_write(&snapshot).expect("build");
        // Simulate a graph produced under Decisions 0044-0050, before
        // Decision 0051 existed: strip the capability record this
        // checkpoint's own build just wrote.
        std::fs::remove_file(root.join(capability::CAPABILITY_RECORD_RELATIVE_PATH)).unwrap();

        let result = status(&repository);
        assert_eq!(result.capability_state, CapabilityState::LegacyEnabled);
        assert_eq!(result.freshness, Freshness::Fresh);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn explicit_disabled_is_valid_without_a_graph() {
        let root = seeded("explicit-disabled");
        capability::persist_disabled(&root).unwrap();
        let repository = Repository::open(&root);
        let result = status(&repository);
        assert_eq!(result.capability_state, CapabilityState::ExplicitDisabled);
        assert_eq!(result.freshness, Freshness::Absent);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn explicit_enabled_with_a_fresh_graph_is_valid() {
        let root = seeded("explicit-enabled-fresh");
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        crate::build_and_write(&snapshot).expect("build");

        let result = status(&repository);
        assert_eq!(result.capability_state, CapabilityState::ExplicitEnabled);
        assert_eq!(result.freshness, Freshness::Fresh);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn enabled_but_missing_is_a_hard_failure_never_absent() {
        let root = seeded("enabled-missing");
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        crate::build_and_write(&snapshot).expect("build");
        // Delete the durable graph but leave the capability declaration
        // claiming `enabled` -- exactly the clone-swallowed-by-
        // .gitignore / manually-deleted rog/ scenario ROG-039 names.
        std::fs::remove_dir_all(durable::rog_root(&root)).unwrap();

        let result = status(&repository);
        assert_eq!(result.capability_state, CapabilityState::EnabledMissing);
        assert_ne!(
            result.freshness,
            Freshness::Absent,
            "enabled-but-missing must never be reported as plain Absent"
        );
        assert_ne!(result.freshness, Freshness::Fresh);
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == "graph.capability-enabled-but-missing"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn enabled_with_stale_graph_is_still_capability_enabled() {
        let root = seeded("enabled-stale");
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        crate::build_and_write(&snapshot).expect("build");
        std::fs::write(
            root.join("src/lib.rs"),
            "pub fn hello() {}\npub fn added() {}\n",
        )
        .unwrap();

        let result = status(&repository);
        assert_eq!(result.capability_state, CapabilityState::ExplicitEnabled);
        assert_eq!(result.freshness, Freshness::Stale);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn enabled_with_corrupt_graph_is_still_capability_enabled() {
        let root = seeded("enabled-corrupt");
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        crate::build_and_write(&snapshot).expect("build");
        let manifest_path = durable::manifest_path(&root);
        let mut manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
        manifest["node_shards"][0]["sha256"] = serde_json::Value::from("0".repeat(64));
        std::fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let result = status(&repository);
        assert_eq!(result.capability_state, CapabilityState::ExplicitEnabled);
        assert_eq!(result.freshness, Freshness::Corrupt);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn enabled_with_unsupported_schema_is_still_capability_enabled() {
        let root = seeded("enabled-unsupported");
        let repository = Repository::open(&root);
        let snapshot = repository.session().snapshot();
        crate::build_and_write(&snapshot).expect("build");
        let manifest_path = durable::manifest_path(&root);
        let mut manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
        manifest["graph_schema_version"] = serde_json::Value::from(999);
        std::fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let result = status(&repository);
        assert_eq!(result.capability_state, CapabilityState::ExplicitEnabled);
        assert_eq!(result.freshness, Freshness::Unsupported);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn malformed_capability_declaration_is_reported_not_silently_ignored() {
        let root = seeded("malformed-capability");
        std::fs::create_dir_all(root.join("governance")).unwrap();
        std::fs::write(
            root.join(capability::CAPABILITY_RECORD_RELATIVE_PATH),
            "{ not valid json",
        )
        .unwrap();
        let repository = Repository::open(&root);
        let result = status(&repository);
        assert_eq!(result.freshness, Freshness::Corrupt);
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == "graph.capability-malformed"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
