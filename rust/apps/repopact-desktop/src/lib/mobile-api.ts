// WI065 Checkpoint B: the frontend's only entry point into the Android
// mobile acquisition command surface. Every call here is a typed, narrow
// `mobile_*` Tauri command (Decision 0056/0057) -- there is no
// `read_uri`/`write_uri`/`list_uri`/arbitrary-path command anywhere in this
// file, and none should ever be added here.
import { invoke } from "@tauri-apps/api/core";
import type { RepositoryOverview } from "../generated/types";
import type {
  AcquisitionOperation,
  MobileAcquisitionError,
  MobileCapabilityStatus,
  SourceStatus,
  WorkspaceSummary,
} from "./mobile-types";

export type MobileFailure = MobileAcquisitionError;

export const mobileApi = {
  listWorkspaces: () => invoke<WorkspaceSummary[]>("mobile_workspace_list"),
  gitCapabilities: () => invoke<MobileCapabilityStatus>("mobile_git_capabilities"),
  // `null` means the user cancelled the OS picker -- not a failure the
  // caller needs to display as an error.
  importDirectory: () => invoke<string | null>("mobile_import_directory"),
  importArchive: () => invoke<string | null>("mobile_import_archive"),
  operationStatus: (operationId: string) =>
    invoke<AcquisitionOperation>("mobile_operation_status", { operationId }),
  cancelOperation: (operationId: string) =>
    invoke<void>("mobile_operation_cancel", { operationId }),
  // Accepts only an opaque workspace id -- never a path or URI.
  openWorkspace: (workspaceId: string) =>
    invoke<RepositoryOverview>("mobile_workspace_open", { workspaceId }),
  // WI065 Checkpoint D: explicit export/share-back. `null` means the user
  // cancelled the SAF destination picker -- not a failure.
  exportWorkspaceDirectory: (workspaceId: string) =>
    invoke<string | null>("mobile_export_workspace_directory", { workspaceId }),
  exportWorkspaceArchive: (workspaceId: string) =>
    invoke<string | null>("mobile_export_workspace_archive", { workspaceId }),
  sourceStatus: (workspaceId: string) =>
    invoke<SourceStatus>("mobile_workspace_source_status", { workspaceId }),
  removeWorkspace: (workspaceId: string) =>
    invoke<void>("mobile_workspace_remove", { workspaceId }),
};
