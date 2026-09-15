// WI065 Checkpoint B §23 / Checkpoint D: the Android mobile-acquisition
// entry surface -- Import folder / Import ZIP / Existing workspaces (with
// Open / Export folder / Export ZIP / Remove local copy), shown only when a
// repository is not yet open. Renders nothing at all on desktop:
// `mobileApi.listWorkspaces()` only succeeds where the `mobile_*` Tauri
// commands are actually registered (Decision 0056/0057 §24 -- desktop's
// `select_repository` and its native picker are a completely separate,
// unchanged path), so the very first call this component makes doubles as
// the platform check, without a new platform-detection dependency.
//
// Checkpoint D closes the real product gap Checkpoint B.5 found: every
// long-running operation (import or export) now shows its type, progress,
// status, and a working Cancel button -- there is no hidden/developer-only
// cancellation path.
import { useEffect, useState } from "react";
import type { RepositoryOverview } from "./generated/types";
import { mobileApi } from "./lib/mobile-api";
import type { AcquisitionOperation, SourceStatus, WorkspaceSummary } from "./lib/mobile-types";

const POLL_INTERVAL_MS = 400;

function describeError(error: unknown): string {
  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
}

function describeExportState(state: WorkspaceSummary["lastExportState"]): string {
  switch (state) {
    case "never_exported":
      return "never exported";
    case "exported":
      return "exported";
    case "changed_since_export":
      return "changed since export";
    case "divergence_unknown":
      return "divergence unknown";
    case "diverged":
      return "diverged";
    default:
      return state;
  }
}

function describeSourceStatus(status: SourceStatus): string {
  switch (status) {
    case "unchanged":
      return "source unchanged";
    case "obviously_changed":
      return "source obviously changed";
    case "unavailable":
      return "source unavailable";
    case "permission_lost":
      return "source permission lost";
    case "unknown":
      return "source status unknown";
    default:
      return status;
  }
}

function describePhase(phase: string): string {
  switch (phase) {
    case "importing":
      return "Importing";
    case "validating":
      return "Validating";
    case "publishing":
      return "Publishing";
    case "exporting":
      return "Exporting";
    default:
      return phase;
  }
}

async function pollUntilTerminal(
  operationId: string,
  onProgress: (op: AcquisitionOperation) => void,
): Promise<AcquisitionOperation> {
  for (;;) {
    const status = await mobileApi.operationStatus(operationId);
    onProgress(status);
    if (status.state !== "running") return status;
    await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS));
  }
}

export function MobileAcquisitionPanel({
  onWorkspaceOpened,
}: {
  onWorkspaceOpened: (overview: RepositoryOverview) => void | Promise<void>;
}) {
  const [available, setAvailable] = useState(false);
  const [workspaces, setWorkspaces] = useState<WorkspaceSummary[]>([]);
  const [busy, setBusy] = useState(false);
  const [operationLabel, setOperationLabel] = useState("");
  const [operation, setOperation] = useState<AcquisitionOperation | null>(null);
  const [error, setError] = useState("");
  const [sourceStatusByWorkspace, setSourceStatusByWorkspace] = useState<Record<string, SourceStatus>>({});

  const refreshWorkspaces = async () => {
    try {
      const list = await mobileApi.listWorkspaces();
      setWorkspaces(list);
      setAvailable(true);
    } catch {
      // No mobile_* command surface registered -- this is a desktop build.
      setAvailable(false);
    }
  };

  useEffect(() => {
    void refreshWorkspaces();
  }, []);

  const runOperation = async (label: string, start: () => Promise<string | null>) => {
    setBusy(true);
    setError("");
    setOperation(null);
    setOperationLabel(label);
    try {
      const operationId = await start();
      if (!operationId) return; // user cancelled the picker -- not an error
      const finalState = await pollUntilTerminal(operationId, setOperation);
      if (finalState.state === "failed") {
        setError(finalState.error.message);
      } else if (finalState.state === "succeeded") {
        await refreshWorkspaces();
      }
      // "cancelled" is a distinct terminal state, not an error message.
    } catch (operationError) {
      setError(describeError(operationError));
    } finally {
      setBusy(false);
    }
  };

  const cancelRunningOperation = async () => {
    if (!operation || operation.state !== "running") return;
    try {
      await mobileApi.cancelOperation(operation.progress.operationId);
    } catch (cancelError) {
      setError(describeError(cancelError));
    }
  };

  const openWorkspace = async (workspaceId: string) => {
    setBusy(true);
    setError("");
    try {
      const overview = await mobileApi.openWorkspace(workspaceId);
      await onWorkspaceOpened(overview);
    } catch (openError) {
      setError(describeError(openError));
    } finally {
      setBusy(false);
    }
  };

  const checkSourceStatus = async (workspaceId: string) => {
    try {
      const status = await mobileApi.sourceStatus(workspaceId);
      setSourceStatusByWorkspace((prev) => ({ ...prev, [workspaceId]: status }));
    } catch (statusError) {
      setError(describeError(statusError));
    }
  };

  const removeWorkspace = async (workspace: WorkspaceSummary) => {
    const needsWarning =
      workspace.lastExportState === "never_exported" || workspace.lastExportState === "changed_since_export";
    if (needsWarning) {
      const proceed = window.confirm(
        `"${workspace.displayName}" is ${describeExportState(workspace.lastExportState)}. ` +
          "Removing the local copy will discard any content that has not been explicitly exported. Continue?",
      );
      if (!proceed) return;
    }
    setBusy(true);
    setError("");
    try {
      await mobileApi.removeWorkspace(workspace.workspaceId);
      await refreshWorkspaces();
    } catch (removeError) {
      setError(describeError(removeError));
    } finally {
      setBusy(false);
    }
  };

  if (!available) return null;

  return (
    <section className="panel mobile-acquisition-panel">
      <p className="eyebrow">MOBILE REPOSITORY ACQUISITION</p>
      <h3>Import a repository</h3>
      <div className="mobile-acquisition-actions">
        <button
          className="secondary-button"
          disabled={busy}
          onClick={() => void runOperation("Importing folder", mobileApi.importDirectory)}
        >
          Import folder
        </button>
        <button
          className="secondary-button"
          disabled={busy}
          onClick={() => void runOperation("Importing ZIP", mobileApi.importArchive)}
        >
          Import ZIP
        </button>
      </div>
      {operation && operation.state === "running" && (
        <div className="mobile-operation-status">
          <p className="muted">
            {operationLabel || describePhase(operation.progress.phase)} — {operation.progress.entriesProcessed}{" "}
            entries, {operation.progress.bytesProcessed} bytes
            {operation.progress.currentRelativePath ? ` — ${operation.progress.currentRelativePath}` : ""}
          </p>
          <button className="quiet-button" onClick={() => void cancelRunningOperation()}>
            Cancel
          </button>
        </div>
      )}
      {operation && operation.state === "cancelled" && <p className="muted">Operation cancelled.</p>}
      {error && <p className="error-text">{error}</p>}
      {workspaces.length > 0 && (
        <div className="mobile-workspace-list">
          <p className="eyebrow">EXISTING WORKSPACES</p>
          <ul>
            {workspaces.map((workspace) => (
              <li key={workspace.workspaceId} className="mobile-workspace-row">
                <div className="mobile-workspace-row-header">
                  <span>{workspace.displayName}</span>
                  <span className="muted">{describeExportState(workspace.lastExportState)}</span>
                </div>
                <div className="mobile-workspace-row-actions">
                  <button className="quiet-button" disabled={busy} onClick={() => void openWorkspace(workspace.workspaceId)}>
                    Open
                  </button>
                  <button
                    className="quiet-button"
                    disabled={busy}
                    onClick={() =>
                      void runOperation("Exporting folder", () =>
                        mobileApi.exportWorkspaceDirectory(workspace.workspaceId),
                      )
                    }
                  >
                    Export folder
                  </button>
                  <button
                    className="quiet-button"
                    disabled={busy}
                    onClick={() =>
                      void runOperation("Exporting ZIP", () => mobileApi.exportWorkspaceArchive(workspace.workspaceId))
                    }
                  >
                    Export ZIP
                  </button>
                  {workspace.acquisitionKind === "saf_directory" && (
                    <button className="quiet-button" disabled={busy} onClick={() => void checkSourceStatus(workspace.workspaceId)}>
                      Check source
                    </button>
                  )}
                  <button className="quiet-button" disabled={busy} onClick={() => void removeWorkspace(workspace)}>
                    Remove local copy
                  </button>
                </div>
                {sourceStatusByWorkspace[workspace.workspaceId] && (
                  <p className="muted">{describeSourceStatus(sourceStatusByWorkspace[workspace.workspaceId])}</p>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}
    </section>
  );
}
