//! WI067 item 7 / GH-011, extended by Checkpoint C (GH-007): executable
//! proof that a non-GitHub provider's output flows through the *existing*
//! safe archive materializer, all the way to a published workspace, with
//! zero GitHub-specific branching anywhere on the path:
//!
//!     FakeProvider -> resolve_ref -> describe_snapshot -> open_snapshot
//!         -> SnapshotArtifact.staged_path
//!         -> WorkspaceManager::import_remote_snapshot
//!         -> repopact_mobile_acquisition::archive::import_archive (unmodified)
//!         -> a published, ordinary app-private workspace
//!
//! Nothing in this test, or in the materializer it calls, references
//! GitHub, an HTTP client, or a network connection.

use std::fs::File;
use std::io::Write;

use repopact_mobile_acquisition::archive::import_archive;
use repopact_mobile_acquisition::bounds::ArchiveBounds;
use repopact_mobile_acquisition::operation::CancellationToken;
use repopact_mobile_acquisition::registry::{AcquisitionKind, RemoteSnapshotProvenance};
use repopact_mobile_acquisition::workspace::WorkspaceManager;
use repopact_remote_provider::fake::FakeProvider;
use repopact_remote_provider::provider::{RemoteRepositoryProvider, SnapshotDownloadOptions};
use repopact_remote_provider::snapshot::SnapshotLayout;

fn write_fixture_zip(path: &std::path::Path) {
    let file = File::create(path).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file("README.md", options).unwrap();
    writer.write_all(b"fake snapshot content\n").unwrap();
    writer.start_file("src/example.txt", options).unwrap();
    writer
        .write_all(b"hello from a non-github provider\n")
        .unwrap();
    writer.finish().unwrap();
}

#[test]
fn fake_provider_snapshot_materializes_through_the_existing_archive_importer() {
    let temp = tempfile::tempdir().unwrap();
    let snapshot_path = temp.path().join("fake-snapshot.zip");
    write_fixture_zip(&snapshot_path);

    let provider = FakeProvider::new(
        snapshot_path.to_string_lossy().to_string(),
        SnapshotLayout::Flat,
    );

    // provider-neutral flow: no GitHub type or GitHub string literal
    // appears anywhere below this line.
    provider.begin_authorization().unwrap();
    let accounts = provider.list_accounts().unwrap();
    let repos = provider.list_repositories(&accounts[0]).unwrap();
    let refs = provider.list_refs(&repos[0]).unwrap();
    let resolved = provider.resolve_ref(&repos[0], &refs[0]).unwrap();
    assert_eq!(resolved.immutable_revision_id.len(), 40);

    let descriptor = provider.describe_snapshot(&repos[0], &resolved).unwrap();

    let destination = temp.path().join("downloaded-snapshot.zip");
    let cancelled = false;
    let artifact = provider
        .open_snapshot(
            &descriptor,
            &SnapshotDownloadOptions {
                destination_path: &destination,
                max_compressed_bytes: 10 * 1024 * 1024,
                should_cancel: &|| cancelled,
            },
        )
        .unwrap();
    assert!(artifact.received_bytes > 0);
    assert_eq!(artifact.staged_path, destination.to_string_lossy());

    let staged_file = File::open(&artifact.staged_path).unwrap();
    let cancel = CancellationToken::new();
    let staging_root = temp.path().join("standalone-extraction");
    std::fs::create_dir_all(&staging_root).unwrap();
    let summary = import_archive(
        staged_file,
        &staging_root,
        &ArchiveBounds::default(),
        &cancel,
        |_progress| {},
    )
    .expect("the existing materializer must accept a non-GitHub provider's staged snapshot");
    assert_eq!(summary.files_imported, 2);

    // The stronger, end-to-end Checkpoint C proof: the same staged bytes
    // flow through WorkspaceManager::import_remote_snapshot (the real
    // production orchestration path) and publish an ordinary workspace
    // with credential-free remote-snapshot provenance attached.
    let manager = WorkspaceManager::open(temp.path().join("workspace-root")).unwrap();
    let staged_file = File::open(&artifact.staged_path).unwrap();
    let provenance = RemoteSnapshotProvenance {
        provider: "fake".to_string(),
        provider_repository_id: repos[0].provider_repository_id.clone(),
        owner_label: repos[0].owner_label.clone(),
        repository_name: repos[0].name.clone(),
        selected_ref: resolved.selected_ref.display_name.clone(),
        ref_kind: "branch".to_string(),
        resolved_commit_sha: resolved.immutable_revision_id.clone(),
        acquired_at: "2026-09-16T00:00:00Z".to_string(),
        snapshot_semantics: "immutable_snapshot".to_string(),
    };
    let record = manager
        .import_remote_snapshot(
            staged_file,
            "fake-owner/fake-repo".to_string(),
            "fake:fake-owner/fake-repo".to_string(),
            &ArchiveBounds::default(),
            &CancellationToken::new(),
            |_progress| {},
            provenance,
            false,
        )
        .expect(
            "a non-GitHub provider's snapshot must publish through the real workspace pipeline",
        );

    assert_eq!(record.acquisition_kind, AcquisitionKind::RemoteSnapshot);
    let provenance = record
        .remote_snapshot_provenance
        .expect("provenance must be attached");
    assert_eq!(provenance.provider, "fake");
    assert_eq!(provenance.resolved_commit_sha.len(), 40);
    assert_eq!(provenance.snapshot_semantics, "immutable_snapshot");

    let published_path = manager.repository_path(&record.workspace_id).unwrap();
    assert!(published_path.join("README.md").exists());
    assert!(published_path.join("src/example.txt").exists());
}

#[test]
fn provider_id_is_the_only_place_the_fake_provider_names_itself() {
    let temp = tempfile::tempdir().unwrap();
    let snapshot_path = temp.path().join("unused.zip");
    write_fixture_zip(&snapshot_path);
    let provider = FakeProvider::new(
        snapshot_path.to_string_lossy().to_string(),
        SnapshotLayout::Flat,
    );
    assert_eq!(provider.provider_id(), "fake");
    assert_ne!(provider.provider_id(), "github");
}
