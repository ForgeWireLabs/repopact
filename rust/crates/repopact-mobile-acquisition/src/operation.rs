//! Decision 0057 §"Cancellation and progress" / §"Concurrency": a native-
//! owned, cooperative cancellation flag and a throttled progress reporter,
//! plus the single-active-operation coordinator.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use uuid::Uuid;

use crate::error::{AcquisitionError, AcquisitionResult, ErrorCode};

/// A cooperative cancellation flag checked between bounded units of work
/// (never inferred from the frontend disappearing).
#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    flag: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    /// Returns `Err(OperationCancelled)` if cancellation has been
    /// requested; otherwise `Ok(())`. Call this between bounded units of
    /// work (e.g. after each source entry, or after each fixed-size chunk).
    pub fn check(&self) -> AcquisitionResult<()> {
        if self.is_cancelled() {
            Err(AcquisitionError::new(
                ErrorCode::OperationCancelled,
                "operation was cancelled",
            ))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct OperationProgress {
    pub operation_id: String,
    pub phase: OperationPhase,
    pub entries_processed: u64,
    pub bytes_processed: u64,
    pub total_entries: Option<u64>,
    /// Workspace-relative current path only -- never a raw external URI,
    /// never file content (Decision 0057 / privacy §63).
    pub current_relative_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationPhase {
    Importing,
    Validating,
    Publishing,
    Exporting,
}

/// Throttles progress callbacks so a large source cannot flood the WebView
/// IPC channel with one event per file (Decision 0057 §"Cancellation and
/// progress").
pub struct ProgressThrottle {
    operation_id: String,
    phase: OperationPhase,
    min_interval: Duration,
    min_entry_step: u64,
    last_emit: Instant,
    last_emitted_entries: u64,
    entries_processed: AtomicU64,
    bytes_processed: AtomicU64,
}

impl ProgressThrottle {
    pub fn new(operation_id: impl Into<String>, phase: OperationPhase) -> Self {
        Self {
            operation_id: operation_id.into(),
            phase,
            min_interval: Duration::from_millis(150),
            min_entry_step: 25,
            last_emit: Instant::now() - Duration::from_secs(3600),
            last_emitted_entries: 0,
            entries_processed: AtomicU64::new(0),
            bytes_processed: AtomicU64::new(0),
        }
    }

    pub fn record(&mut self, bytes_delta: u64) {
        self.entries_processed.fetch_add(1, Ordering::Relaxed);
        self.bytes_processed
            .fetch_add(bytes_delta, Ordering::Relaxed);
    }

    /// Calls `emit` only if enough time or enough entries have passed since
    /// the last emission, or `force` is set (used for the final event).
    pub fn maybe_emit(
        &mut self,
        current_relative_path: Option<&str>,
        total_entries: Option<u64>,
        force: bool,
        mut emit: impl FnMut(OperationProgress),
    ) {
        let entries = self.entries_processed.load(Ordering::Relaxed);
        let due_by_time = self.last_emit.elapsed() >= self.min_interval;
        let due_by_count = entries.saturating_sub(self.last_emitted_entries) >= self.min_entry_step;
        if !force && !due_by_time && !due_by_count {
            return;
        }
        self.last_emit = Instant::now();
        self.last_emitted_entries = entries;
        emit(OperationProgress {
            operation_id: self.operation_id.clone(),
            phase: self.phase,
            entries_processed: entries,
            bytes_processed: self.bytes_processed.load(Ordering::Relaxed),
            total_entries,
            current_relative_path: current_relative_path.map(str::to_owned),
        });
    }
}

pub fn new_operation_id() -> String {
    Uuid::new_v4().to_string()
}

/// Decision 0057 §"Concurrency": exactly one acquisition/export operation
/// may be active at a time for v1. A second concurrent request is rejected
/// rather than queued or interleaved.
#[derive(Default)]
pub struct OperationCoordinator {
    active: Mutex<Option<ActiveOperation>>,
}

struct ActiveOperation {
    operation_id: String,
    cancel: CancellationToken,
}

impl OperationCoordinator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Attempts to begin a new operation, returning its id and a
    /// cancellation token the caller must poll. Fails with a typed error if
    /// another operation is already active.
    pub fn begin(&self) -> AcquisitionResult<(String, CancellationToken)> {
        let mut guard = self.active.lock().expect("operation coordinator poisoned");
        if guard.is_some() {
            return Err(AcquisitionError::new(
                ErrorCode::ResourceLimit,
                "another mobile acquisition/export operation is already in progress",
            ));
        }
        let operation_id = new_operation_id();
        let cancel = CancellationToken::new();
        *guard = Some(ActiveOperation {
            operation_id: operation_id.clone(),
            cancel: cancel.clone(),
        });
        Ok((operation_id, cancel))
    }

    /// Ends whichever operation is active, regardless of outcome. Safe to
    /// call even if `operation_id` does not match (defensive; the caller is
    /// still expected to always end what it began).
    pub fn end(&self, operation_id: &str) {
        let mut guard = self.active.lock().expect("operation coordinator poisoned");
        if guard
            .as_ref()
            .is_some_and(|op| op.operation_id == operation_id)
        {
            *guard = None;
        }
    }

    pub fn cancel(&self, operation_id: &str) -> AcquisitionResult<()> {
        let guard = self.active.lock().expect("operation coordinator poisoned");
        match guard.as_ref() {
            Some(op) if op.operation_id == operation_id => {
                op.cancel.cancel();
                Ok(())
            }
            _ => Err(AcquisitionError::new(
                ErrorCode::WorkspaceNotFound,
                format!("no active operation with id '{operation_id}'"),
            )),
        }
    }
}

/// RAII guard so `OperationCoordinator::end` is always called, even on an
/// early return/panic unwind through `?`.
pub struct OperationGuard<'a> {
    coordinator: &'a OperationCoordinator,
    operation_id: String,
}

impl<'a> OperationGuard<'a> {
    pub fn begin(coordinator: &'a OperationCoordinator) -> AcquisitionResult<Self> {
        let (operation_id, _cancel) = coordinator.begin()?;
        Ok(Self {
            coordinator,
            operation_id,
        })
    }

    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }
}

impl Drop for OperationGuard<'_> {
    fn drop(&mut self) {
        self.coordinator.end(&self.operation_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinator_rejects_concurrent_operation() {
        let coordinator = OperationCoordinator::new();
        let (id, _cancel) = coordinator.begin().unwrap();
        let err = coordinator.begin().unwrap_err();
        assert_eq!(err.code, ErrorCode::ResourceLimit);
        coordinator.end(&id);
        coordinator.begin().unwrap();
    }

    #[test]
    fn cancellation_token_is_cooperative() {
        let token = CancellationToken::new();
        token.check().unwrap();
        token.cancel();
        assert_eq!(
            token.check().unwrap_err().code,
            ErrorCode::OperationCancelled
        );
    }

    #[test]
    fn operation_guard_releases_on_drop() {
        let coordinator = OperationCoordinator::new();
        {
            let _guard = OperationGuard::begin(&coordinator).unwrap();
            assert!(coordinator.begin().is_err());
        }
        coordinator.begin().unwrap();
    }
}
