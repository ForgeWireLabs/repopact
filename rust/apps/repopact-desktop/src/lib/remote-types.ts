// WI067 Checkpoint B: TypeScript mirror of the Rust DTOs in
// `apps/repopact-desktop/src-tauri/src/remote_provider.rs`. No field here
// is a token/refresh-token/device-code/credential -- the Rust command
// layer never serializes one to the frontend by construction (see
// `repopact_remote_provider::redact::Secret`, which has no `Serialize`
// impl at all).

export interface ProviderCapabilities {
  provider: "github";
  configured: boolean;
  supportsPublicWithoutAuth: boolean;
  supportsPrivateRepositories: boolean;
  supportsOrganizations: boolean;
}

// Modeled directly on `repopact_remote_provider::auth::AuthState`, which
// serializes with `#[serde(tag = "state"/"status", rename_all =
// "snake_case")]` applied uniformly to both variant names AND field names
// -- unlike every other DTO in this file, this one's fields are
// snake_case on the wire, not camelCase.

export type RemoteErrorCode =
  | "not_connected"
  | "provider_not_configured"
  | "authorization_pending"
  | "authorization_cancelled"
  | "authorization_expired"
  | "authorization_denied"
  | "credential_unavailable"
  | "credential_expired"
  | "refresh_failed"
  | "provider_rate_limited"
  | "provider_forbidden"
  | "provider_not_found"
  | "installation_revoked"
  | "organization_authorization_required"
  | "network_unavailable"
  | "tls_failure"
  | "provider_protocol_error"
  | "snapshot_too_large"
  | "download_cancelled"
  | "materialization_failed";

export interface RemoteProviderError {
  code: RemoteErrorCode;
  detail: string;
}

export type ConnectionStatus =
  | { status: "disconnected" }
  | { status: "awaiting_user"; user_code: string; verification_uri: string; expires_at: string }
  | { status: "connected"; login: string }
  | { status: "cancelled" }
  | { status: "failed"; code: RemoteErrorCode };

export interface RemoteAccount {
  connectionId: string;
  label: string;
  scopeLabel: string;
  includesPrivateRepositories: boolean;
}

export interface RemoteRepository {
  repositoryId: string;
  owner: string;
  name: string;
  fullName: string;
  private: boolean;
  defaultBranch: string | null;
}

export type RemoteRefKind = "branch" | "tag" | "commit";

export interface RemoteRef {
  displayName: string;
  kind: RemoteRefKind;
  refId: string;
}

export interface ResolvedRevision {
  selectedRefDisplayName: string;
  resolvedCommitSha: string;
}

// Only what a command needs to identify a repository/ref -- never a raw
// provider URL.
export interface RemoteRepositoryRef {
  repositoryId: string;
  owner: string;
  name: string;
}

export interface RemoteRefRef {
  displayName: string;
  kind: RemoteRefKind;
  refId: string;
}

// WI067 Checkpoint C.
export interface RemoteImportResult {
  workspaceId: string;
  displayName: string;
  resolvedCommitSha: string;
}
