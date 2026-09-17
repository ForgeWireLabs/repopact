// WI067 Checkpoint B: the frontend's only entry point into the GitHub
// remote-provider command surface (item 34). Every call here is a typed,
// narrow `remote_*` Tauri command (Decision 0061) -- there is no generic
// proxy/passthrough command that takes a caller-supplied method, URL, or
// header map, and none should ever be added here. This module never calls
// the browser's own fetch API against GitHub itself; all network I/O
// happens natively.
import { invoke } from "@tauri-apps/api/core";
import type {
  ConnectionStatus,
  ProviderCapabilities,
  RemoteAccount,
  RemoteImportResult,
  RemoteRef,
  RemoteRefRef,
  RemoteRepository,
  RemoteRepositoryRef,
  ResolvedRevision,
} from "./remote-types";

export const remoteApi = {
  capabilities: () => invoke<ProviderCapabilities>("remote_provider_capabilities"),
  connectionStatus: () => invoke<ConnectionStatus>("remote_connections"),
  // Decision 0062: one native, typed operation. The backend generates the
  // authorization session (state + PKCE + loopback listener) and opens the
  // system browser itself -- there is no separate "open verification URL"
  // command for the frontend to call.
  connectStart: () => invoke<ConnectionStatus>("remote_connect_start"),
  // Native polling only decides *when* to call this; the frontend never
  // sets its own interval -- it calls this once per its own UI-refresh
  // tick and displays whatever status comes back.
  connectStatus: () => invoke<ConnectionStatus>("remote_connect_status"),
  connectCancel: () => invoke<void>("remote_connect_cancel"),
  // Opens GitHub's own installation/configuration page for the RepoPact
  // GitHub App, built entirely from native app-slug configuration -- never
  // a frontend-supplied URL.
  openInstallationPage: () => invoke<void>("remote_open_installation_page"),
  disconnect: () => invoke<void>("remote_disconnect"),
  accounts: () => invoke<RemoteAccount[]>("remote_accounts"),
  repositories: (connectionId: string) =>
    invoke<RemoteRepository[]>("remote_repositories", { connectionId }),
  repositoryRefs: (repository: RemoteRepositoryRef) =>
    invoke<RemoteRef[]>("remote_repository_refs", { repository }),
  resolveRef: (repository: RemoteRepositoryRef, reference: RemoteRefRef) =>
    invoke<ResolvedRevision>("remote_resolve_ref", { repository, reference }),
  // WI067 Checkpoint C. Accepts only typed repository/ref identifiers --
  // the native backend re-resolves the exact commit SHA itself rather
  // than trusting any SHA this frontend already displayed (item 4/9/31).
  importSnapshot: (repository: RemoteRepositoryRef, reference: RemoteRefRef) =>
    invoke<RemoteImportResult>("remote_import_snapshot", { repository, reference }),
  importCancel: () => invoke<void>("remote_import_cancel"),
};
