// WI065 Checkpoint B §23: the minimal Android mobile-acquisition entry
// surface -- Import folder / Import ZIP / Existing workspaces, shown only
// when a repository is not yet open. Renders nothing at all on desktop:
// `mobileApi.listWorkspaces()` only succeeds where the `mobile_*` Tauri
// commands are actually registered (Decision 0056/0057 §24 -- desktop's
// `select_repository` and its native picker are a completely separate,
// unchanged path), so the very first call this component makes doubles as
// the platform check, without a new platform-detection dependency.
import { useEffect, useState } from "react";
import type { RepositoryOverview } from "./generated/types";
import { mobileApi } from "./lib/mobile-api";
import type { AcquisitionOperation, WorkspaceSummary } from "./lib/mobile-types";

const POLL_INTERVAL_MS = 400;

function describeError(error: unknown): string {
  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
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
  const [operation, setOperation] = useState<AcquisitionOperation | null>(null);
  const [error, setError] = useState("");

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

  const runImport = async (pick: () => Promise<string | null>) => {
    setBusy(true);
    setError("");
    setOperation(null);
    try {
      const operationId = await pick();
      if (!operationId) return; // user cancelled the picker -- not an error
      const finalState = await pollUntilTerminal(operationId, setOperation);
      if (finalState.state === "failed") {
        setError(finalState.error.message);
      } else if (finalState.state === "succeeded") {
        await refreshWorkspaces();
      }
      // "cancelled" is a distinct terminal state, not an error message.
    } catch (importError) {
      setError(describeError(importError));
    } finally {
      setBusy(false);
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

  if (!available) return null;

  return (
    <section className="panel mobile-acquisition-panel">
      <p className="eyebrow">MOBILE REPOSITORY ACQUISITION</p>
      <h3>Import a repository</h3>
      <div className="mobile-acquisition-actions">
        <button className="secondary-button" disabled={busy} onClick={() => void runImport(mobileApi.importDirectory)}>
          Import folder
        </button>
        <button className="secondary-button" disabled={busy} onClick={() => void runImport(mobileApi.importArchive)}>
          Import ZIP
        </button>
      </div>
      {operation && operation.state === "running" && (
        <p className="muted">
          Importing… {operation.progress.entriesProcessed} entries, {operation.progress.bytesProcessed} bytes
        </p>
      )}
      {error && <p className="error-text">{error}</p>}
      {workspaces.length > 0 && (
        <div className="mobile-workspace-list">
          <p className="eyebrow">EXISTING WORKSPACES</p>
          <ul>
            {workspaces.map((workspace) => (
              <li key={workspace.workspaceId}>
                <span>{workspace.displayName}</span>
                <button className="quiet-button" disabled={busy} onClick={() => void openWorkspace(workspace.workspaceId)}>
                  Open
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}
    </section>
  );
}
