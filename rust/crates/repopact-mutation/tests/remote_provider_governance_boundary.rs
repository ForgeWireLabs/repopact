//! WI067 Checkpoint D (GH-013 "no elevation of GitHub metadata into
//! governance/mutation approval authority", Phase 8): a structural,
//! executable regression proof rather than a one-off manual check.
//!
//! `repopact_mutation::MutationRequest -> MutationPlan -> MutationResult`
//! and `repopact_graph`'s durable/ROG authority never read
//! `WorkspaceRecord::remote_snapshot_provenance` (or anything from
//! `repopact-mobile-acquisition`/`repopact-remote-provider`/
//! `repopact-provider-github` at all) for the simplest possible reason:
//! neither crate depends on either of those crates, so no such value is
//! even reachable at compile time, let alone at runtime. This test fails
//! loudly if a future change ever adds that dependency edge, which would
//! be the first step toward accidentally letting remote/provider metadata
//! influence governance decisions.

use std::fs;
use std::path::Path;

fn manifest_forbids(crate_dir: &str, forbidden: &[&str]) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(crate_dir)
        .join("Cargo.toml");
    let manifest = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    for name in forbidden {
        assert!(
            !manifest.contains(name),
            "{} must never depend on {name} -- doing so would create a path for remote-provider/mobile-acquisition metadata to reach governance/mutation authority",
            path.display()
        );
    }
}

const FORBIDDEN: &[&str] = &[
    "repopact-mobile-acquisition",
    "repopact-remote-provider",
    "repopact-provider-github",
];

#[test]
fn mutation_crate_never_depends_on_remote_provider_or_mobile_acquisition() {
    manifest_forbids("repopact-mutation", FORBIDDEN);
}

#[test]
fn graph_crate_never_depends_on_remote_provider_or_mobile_acquisition() {
    manifest_forbids("repopact-graph", FORBIDDEN);
}

#[test]
fn core_crate_never_depends_on_remote_provider_or_mobile_acquisition() {
    manifest_forbids("repopact-core", FORBIDDEN);
}
