use std::path::Path;

use repopact_repository::{Repository, RepositorySnapshot};
use repopact_types::ValidationReport;

mod verification;

pub use repopact_validation_base::{render_dashboard, render_dashboard_snapshot, Validator};

/// Canonical semantic validation entry point for repository roots.
///
/// The lower-level validator owns the established repository rules. Additive
/// semantic contracts such as WI046 verification profiles are composed here so
/// every consumer sees one final validation result.
pub fn validate(root: impl AsRef<Path>) -> ValidationReport {
    let repository = Repository::open(root);
    let snapshot = repository.session().snapshot();
    validate_snapshot(&snapshot)
}

/// Canonical semantic validation entry point for an immutable repository
/// generation. Workbench, mutation, engine, and core callers should all consume
/// this surface instead of remembering additive validators independently.
pub fn validate_snapshot(snapshot: &RepositorySnapshot) -> ValidationReport {
    let mut report = repopact_validation_base::validate_snapshot(snapshot);
    report
        .diagnostics
        .extend(verification::validate_snapshot(snapshot));
    report.diagnostics.sort_by(|left, right| {
        (left.path.as_deref(), &left.message).cmp(&(right.path.as_deref(), &right.message))
    });
    report
}
