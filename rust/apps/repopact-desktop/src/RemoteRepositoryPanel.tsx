// WI067 Checkpoint B (item 35/36): the first-class GitHub entry point in
// the repository acquisition UX, alongside local-folder/archive/mobile
// acquisition. Renders only "Connect GitHub" / connection state / device
// code / Cancel / connected account / Disconnect / account+repository+ref
// browsing / a resolved-SHA display. Deliberately does NOT execute
// snapshot materialization yet -- "Import snapshot" is a disabled
// next-step control until Checkpoint C, and every label below says
// "Snapshot"/"Import snapshot"/"Resolved commit", never "Clone"/"Pull"/
// "Push"/"Sync" (item 37 -- true Git is WI068, not this).
import { useEffect, useState } from "react";
import { remoteApi } from "./lib/remote-api";
import type {
  ConnectionStatus,
  ProviderCapabilities,
  RemoteAccount,
  RemoteRef,
  RemoteRepository,
  ResolvedRevision,
} from "./lib/remote-types";

const STATUS_POLL_INTERVAL_MS = 1500;

function describeError(error: unknown): string {
  if (error && typeof error === "object" && "detail" in error) {
    return String((error as { detail: unknown }).detail);
  }
  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
}

export function RemoteRepositoryPanel() {
  const [capabilities, setCapabilities] = useState<ProviderCapabilities | null>(null);
  const [status, setStatus] = useState<ConnectionStatus>({ status: "disconnected" });
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  const [accounts, setAccounts] = useState<RemoteAccount[]>([]);
  const [selectedConnectionId, setSelectedConnectionId] = useState<string | null>(null);
  const [repositories, setRepositories] = useState<RemoteRepository[]>([]);
  const [repositoryQuery, setRepositoryQuery] = useState("");
  const [selectedRepository, setSelectedRepository] = useState<RemoteRepository | null>(null);
  const [refs, setRefs] = useState<RemoteRef[]>([]);
  const [selectedRef, setSelectedRef] = useState<RemoteRef | null>(null);
  const [resolved, setResolved] = useState<ResolvedRevision | null>(null);

  useEffect(() => {
    remoteApi
      .capabilities()
      .then(setCapabilities)
      .catch(() => setCapabilities(null));
    remoteApi
      .connectionStatus()
      .then(setStatus)
      .catch(() => {});
  }, []);

  // Native polling drives the device-flow status; this effect only
  // decides *how often to ask*, never the server-side interval (item 20).
  useEffect(() => {
    if (status.status !== "awaiting_user") return;
    const timer = setInterval(async () => {
      try {
        const next = await remoteApi.connectStatus();
        setStatus(next);
        if (next.status === "connected") {
          await loadAccounts();
        }
      } catch (statusError) {
        setError(describeError(statusError));
      }
    }, STATUS_POLL_INTERVAL_MS);
    return () => clearInterval(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [status.status]);

  if (!capabilities) return null; // command surface unavailable (e.g. Android build).

  const loadAccounts = async () => {
    try {
      const list = await remoteApi.accounts();
      setAccounts(list);
      if (list.length > 0) setSelectedConnectionId(list[0].connectionId);
    } catch (accountsError) {
      setError(describeError(accountsError));
    }
  };

  const connect = async () => {
    setBusy(true);
    setError("");
    try {
      const next = await remoteApi.connectStart();
      setStatus(next);
    } catch (connectError) {
      setError(describeError(connectError));
    } finally {
      setBusy(false);
    }
  };

  const cancelConnect = async () => {
    try {
      await remoteApi.connectCancel();
      setStatus({ status: "cancelled" });
    } catch (cancelError) {
      setError(describeError(cancelError));
    }
  };

  const disconnect = async () => {
    try {
      await remoteApi.disconnect();
      setStatus({ status: "disconnected" });
      setAccounts([]);
      setRepositories([]);
      setSelectedRepository(null);
      setRefs([]);
      setSelectedRef(null);
      setResolved(null);
    } catch (disconnectError) {
      setError(describeError(disconnectError));
    }
  };

  const loadRepositories = async (connectionId: string) => {
    setBusy(true);
    setError("");
    try {
      setRepositories(await remoteApi.repositories(connectionId));
    } catch (reposError) {
      setError(describeError(reposError));
    } finally {
      setBusy(false);
    }
  };

  const selectRepository = async (repo: RemoteRepository) => {
    setSelectedRepository(repo);
    setSelectedRef(null);
    setResolved(null);
    setBusy(true);
    setError("");
    try {
      setRefs(
        await remoteApi.repositoryRefs({
          repositoryId: repo.repositoryId,
          owner: repo.owner,
          name: repo.name,
        }),
      );
    } catch (refsError) {
      setError(describeError(refsError));
    } finally {
      setBusy(false);
    }
  };

  const resolveSelectedRef = async () => {
    if (!selectedRepository || !selectedRef) return;
    setBusy(true);
    setError("");
    try {
      const revision = await remoteApi.resolveRef(
        { repositoryId: selectedRepository.repositoryId, owner: selectedRepository.owner, name: selectedRepository.name },
        { displayName: selectedRef.displayName, kind: selectedRef.kind, refId: selectedRef.refId },
      );
      setResolved(revision);
    } catch (resolveError) {
      setError(describeError(resolveError));
    } finally {
      setBusy(false);
    }
  };

  const filteredRepositories = repositoryQuery
    ? repositories.filter(
        (repo) =>
          repo.name.toLowerCase().includes(repositoryQuery.toLowerCase()) ||
          repo.fullName.toLowerCase().includes(repositoryQuery.toLowerCase()),
      )
    : repositories;

  return (
    <div className="remote-repository-panel">
      <h3>GitHub</h3>
      {error && <p className="error-text">{error}</p>}

      {status.status === "disconnected" && (
        <button className="secondary-button" onClick={connect} disabled={busy || !capabilities.configured}>
          Connect GitHub
        </button>
      )}
      {!capabilities.configured && status.status === "disconnected" && (
        <p className="hint-text">
          No GitHub App is configured yet. An operator must register one (see docs/guides/github-app-setup.md)
          and set REPOPACT_GITHUB_CLIENT_ID before Connect GitHub can start.
        </p>
      )}

      {status.status === "awaiting_user" && (
        <div>
          <p>
            Enter code <strong>{status.user_code}</strong> at <code>{status.verification_uri}</code>
          </p>
          <button
            className="secondary-button"
            onClick={() => remoteApi.openVerificationUrl().catch((openError) => setError(describeError(openError)))}
          >
            Open GitHub authorization
          </button>
          <button className="secondary-button" onClick={cancelConnect}>
            Cancel
          </button>
        </div>
      )}

      {status.status === "connected" && (
        <div>
          <p>Connected as {status.login}</p>
          <button className="secondary-button" onClick={disconnect}>
            Disconnect
          </button>
          {accounts.length === 0 && (
            <button className="secondary-button" onClick={loadAccounts} disabled={busy}>
              Load accounts
            </button>
          )}
          {accounts.length > 0 && (
            <select
              value={selectedConnectionId ?? ""}
              onChange={(event) => {
                setSelectedConnectionId(event.target.value);
                void loadRepositories(event.target.value);
              }}
            >
              {accounts.map((account) => (
                <option key={account.connectionId} value={account.connectionId}>
                  {account.label} ({account.scopeLabel})
                </option>
              ))}
            </select>
          )}

          {repositories.length > 0 && (
            <div>
              <input
                placeholder="Search repositories"
                value={repositoryQuery}
                onChange={(event) => setRepositoryQuery(event.target.value)}
              />
              <ul>
                {filteredRepositories.map((repo) => (
                  <li key={repo.repositoryId}>
                    <button className="link-button" onClick={() => selectRepository(repo)}>
                      {repo.fullName} {repo.private ? "(private)" : ""}
                    </button>
                  </li>
                ))}
              </ul>
            </div>
          )}

          {selectedRepository && refs.length > 0 && (
            <div>
              <select
                value={selectedRef?.refId ?? ""}
                onChange={(event) => setSelectedRef(refs.find((r) => r.refId === event.target.value) ?? null)}
              >
                <option value="" disabled>
                  Select a branch or tag
                </option>
                {refs.map((ref) => (
                  <option key={ref.refId} value={ref.refId}>
                    {ref.displayName} ({ref.kind})
                  </option>
                ))}
              </select>
              <button className="secondary-button" onClick={resolveSelectedRef} disabled={!selectedRef || busy}>
                Resolve
              </button>
            </div>
          )}

          {resolved && (
            <div>
              <p>Snapshot at {resolved.resolvedCommitSha.slice(0, 12)}</p>
              <button className="primary-button" disabled title="Import snapshot lands in WI067 Checkpoint C">
                Import snapshot (coming soon)
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
