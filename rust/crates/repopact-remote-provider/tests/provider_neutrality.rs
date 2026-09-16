//! WI067 item 7 / GH-011: executable proof that a non-GitHub provider's
//! output flows through the *existing* safe archive materializer
//! (`repopact_mobile_acquisition::archive::import_archive`, unmodified by
//! WI067) with zero GitHub-specific branching anywhere on the path:
//!
//!     FakeProvider -> resolve_ref -> describe_snapshot -> open_snapshot
//!         -> SnapshotArtifact.staged_path
//!         -> repopact_mobile_acquisition::archive::import_archive
//!         -> ordinary extracted workspace tree
//!
//! Nothing in this test, or in the materializer it calls, references
//! GitHub, an HTTP client, or a network connection.

use std::fs::File;
use std::io::Write;

use repopact_mobile_acquisition::archive::import_archive;
use repopact_mobile_acquisition::bounds::ArchiveBounds;
use repopact_mobile_acquisition::operation::CancellationToken;
use repopact_remote_provider::fake::FakeProvider;
use repopact_remote_provider::provider::RemoteRepositoryProvider;
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
    let artifact = provider.open_snapshot(&descriptor).unwrap();
    assert!(artifact.received_bytes > 0);

    let staging_root = temp.path().join("workspace-staging");
    std::fs::create_dir_all(&staging_root).unwrap();

    let staged_file = File::open(&artifact.staged_path).unwrap();
    let cancel = CancellationToken::new();
    let summary = import_archive(
        staged_file,
        &staging_root,
        &ArchiveBounds::default(),
        &cancel,
        |_progress| {},
    )
    .expect("the existing materializer must accept a non-GitHub provider's staged snapshot");

    assert_eq!(summary.files_imported, 2);
    assert!(staging_root.join("README.md").exists());
    assert!(staging_root.join("src/example.txt").exists());
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
