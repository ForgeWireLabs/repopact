// WI065 Checkpoint B: hand-maintained mirror of the `mobile_*` Tauri
// command DTOs in `rust/apps/repopact-desktop/src-tauri/src/mobile_acquisition.rs`.
//
// Unlike `src/generated/types.ts`, these are not produced by
// `repopact-desktop-api`'s `generate-types` binary -- the mobile acquisition
// DTOs live in `repopact-mobile-acquisition`/`repopact-desktop`, outside
// that generator's current scope. Extending the generator to a second crate
// is a larger change than this checkpoint's narrow SAF-bridge scope
// justifies; these types are kept intentionally small and are exercised by
// `mobile_acquisition.rs`'s own host-side serialization tests (WI065
// Checkpoint B §35), which pin down the exact JSON shape this file mirrors.
// If the Rust shape changes, these fall out of sync until updated by hand.

export type AcquisitionKind = "saf_directory" | "saf_archive" | "remote_git";
export type GitState = "non_git" | "git_metadata_present" | "embedded_git_managed";
export type LifecycleState = "allocating" | "importing" | "ready" | "exporting" | "failed";
export type ExportState =
  | "never_exported"
  | "exported"
  | "changed_since_export"
  | "divergence_unknown"
  | "diverged";

export interface WorkspaceSummary {
  workspaceId: string;
  displayName: string;
  acquisitionKind: AcquisitionKind;
  gitState: GitState;
  lifecycleState: LifecycleState;
  lastExportState: ExportState;
  createdAt: string;
}

// Mirrors repopact-mobile-acquisition's ErrorCode (Decision 0057's typed
// error taxonomy). The frontend must switch on this, never parse `message`.
export type MobileErrorCode =
  | "selection_cancelled"
  | "permission_denied"
  | "source_unavailable"
  | "unsupported_entry"
  | "path_escape"
  | "duplicate_path"
  | "case_conflict"
  | "resource_limit"
  | "archive_invalid"
  | "archive_symlink"
  | "operation_cancelled"
  | "workspace_not_found"
  | "workspace_not_ready"
  | "export_conflict"
  | "source_diverged"
  | "internal_io";

export interface MobileAcquisitionError {
  code: MobileErrorCode;
  message: string;
}

export type OperationPhase = "importing" | "validating" | "publishing" | "exporting";

export interface AcquisitionProgress {
  operationId: string;
  phase: OperationPhase;
  entriesProcessed: number;
  bytesProcessed: number;
  totalEntries: number | null;
  currentRelativePath: string | null;
}

export type AcquisitionOperation =
  | { state: "running"; progress: AcquisitionProgress }
  | { state: "succeeded"; workspace: WorkspaceSummary }
  | { state: "failed"; error: MobileAcquisitionError }
  | { state: "cancelled" };

export type Stage2Status = "unsupported_stage2";

export interface MobileCapabilityStatus {
  cloneRepository: Stage2Status;
  pullRepository: Stage2Status;
  pushRepository: Stage2Status;
}
