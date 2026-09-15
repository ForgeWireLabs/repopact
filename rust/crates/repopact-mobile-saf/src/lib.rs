//! Decision 0056/0057, WI065 Checkpoint B: the narrow Android SAF bridge.
//!
//! This crate exposes **no frontend-invokable commands of its own** -- its
//! `tauri::plugin::Builder` never calls `.invoke_handler(...)`, so nothing
//! here is reachable from JavaScript as `plugin:repopact-mobile-saf|...`.
//! It exists purely so the app's own Rust mobile-acquisition coordinator
//! (in `repopact-desktop`) can call plain Rust methods
//! (`app.saf_acquisition().pick_directory_tree()`, etc.) that internally
//! invoke the Android Kotlin plugin via `PluginHandle::run_mobile_plugin`.
//! The frontend only ever sees the app's own typed `mobile_*` Tauri
//! commands, never this plugin directly and never a raw `content://` URI
//! (Decision 0056 §5/§6).
//!
//! On desktop, this plugin is never registered at all (the app only calls
//! `.plugin(repopact_mobile_saf::init_plugin())` under
//! `#[cfg(target_os = "android")]`, mirroring how `tauri_plugin_dialog` is
//! already desktop-only in this app's `lib.rs`) -- there is no SAF concept
//! on desktop to bridge. `desktop.rs` exists only so this crate still
//! compiles under a plain `cargo check --workspace` on a non-Android host;
//! every one of its methods returns a typed `SourceUnavailable` error and
//! is unreachable in practice.
//!
//! `mobile::AndroidSafSource` implements
//! `repopact_mobile_acquisition::source::AcquisitionSource` against this
//! bridge, so Checkpoint A's bounded importer drives a real SAF tree
//! exactly the way it already drives `FilesystemSource` in tests -- no
//! second, SAF-specific safety engine is introduced here.

use repopact_mobile_acquisition::AcquisitionResult;
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

#[cfg(mobile)]
mod models;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

#[cfg(mobile)]
pub use mobile::{AndroidSafSource, StagingArchiveFile};

/// A picked SAF directory tree, or a picked SAF archive document. The
/// `*_uri` fields are native-owned from here on: they are stored only in
/// Rust structures the frontend never receives (Decision 0056 §6/§9).
#[derive(Debug, Clone)]
pub struct PickedTree {
    pub tree_uri: String,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub struct PickedDocument {
    pub document_uri: String,
    pub display_name: String,
}

pub trait SafAcquisitionExt<R: Runtime> {
    fn saf_acquisition(&self) -> &SafAcquisition<R>;
}

impl<R: Runtime, T: Manager<R>> SafAcquisitionExt<R> for T {
    fn saf_acquisition(&self) -> &SafAcquisition<R> {
        self.state::<SafAcquisition<R>>().inner()
    }
}

#[cfg(mobile)]
pub struct SafAcquisition<R: Runtime>(tauri::plugin::PluginHandle<R>);
// `fn() -> R` (rather than `R` directly) keeps this Send+Sync regardless of
// R, which `Manager::state`/`State::inner` require -- this variant carries
// no actual R-typed value, only a marker for which runtime it would apply
// to if SAF existed on desktop.
#[cfg(desktop)]
pub struct SafAcquisition<R: Runtime>(std::marker::PhantomData<fn() -> R>);

impl<R: Runtime> SafAcquisition<R> {
    pub fn pick_directory_tree(&self) -> AcquisitionResult<Option<PickedTree>> {
        #[cfg(mobile)]
        return mobile::pick_directory_tree(&self.0);
        #[cfg(desktop)]
        return desktop::pick_directory_tree();
    }

    pub fn pick_archive_document(&self) -> AcquisitionResult<Option<PickedDocument>> {
        #[cfg(mobile)]
        return mobile::pick_archive_document(&self.0);
        #[cfg(desktop)]
        return desktop::pick_archive_document();
    }
}

#[cfg(mobile)]
impl<R: Runtime> SafAcquisition<R> {
    /// Constructs a Checkpoint-A `AcquisitionSource` over a picked
    /// directory tree. The `PluginHandle` this clones is native-owned; the
    /// returned source is likewise never given to the frontend, only to
    /// the app's own mobile-acquisition coordinator (Decision 0056 §6).
    pub fn open_directory_source(
        &self,
        tree_uri: String,
    ) -> AcquisitionResult<AndroidSafSource<R>> {
        AndroidSafSource::new(self.0.clone(), tree_uri)
    }

    /// Copies a picked archive document into app-private staging and
    /// returns a seekable handle Checkpoint A's ZIP importer can read
    /// directly.
    pub fn open_archive_document(
        &self,
        document_uri: &str,
    ) -> AcquisitionResult<mobile::StagingArchiveFile> {
        mobile::open_archive_document(&self.0, document_uri)
    }
}

pub fn init_plugin<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("repopact-mobile-saf")
        .setup(|app, _api| {
            #[cfg(mobile)]
            let handle = SafAcquisition(mobile::register(app, _api)?);
            #[cfg(desktop)]
            let handle = SafAcquisition::<R>(std::marker::PhantomData);
            app.manage(handle);
            Ok(())
        })
        .build()
}
