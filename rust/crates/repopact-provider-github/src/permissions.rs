//! WI067 GH-002, item 9: the exact GitHub App permission/endpoint matrix
//! for v1 snapshot import. This is the durable record Decision 0061
//! references -- do not add an endpoint here without also justifying its
//! permission in the decision.
//!
//! v1 requests exactly two repository-adjacent permissions, both
//! read-only, and nothing else:
//!
//! - `contents: read` -- required to list branches/tags, resolve a ref to
//!   a commit SHA, and download a repository archive.
//! - `metadata: read` -- GitHub's baseline permission for any repository
//!   the installation can see at all (repository existence, visibility,
//!   default branch); every GitHub App implicitly has this for its
//!   accessible repositories, and it is recorded explicitly here rather
//!   than left implicit.
//!
//! v1 explicitly does NOT request: `contents: write`, `administration`,
//! `actions` (any level), `workflows`, or `pull_requests: write`. None of
//! v1's endpoints require them, and WI067 GH-014 requires that any future
//! request for write/administrative permission carry its own separate
//! acceptance/security evidence.

/// One row of the endpoint -> required-permission matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndpointPermission {
    pub endpoint: &'static str,
    pub required_permission: &'static str,
}

pub const PERMISSION_MATRIX: &[EndpointPermission] = &[
    EndpointPermission {
        endpoint: "GET /user",
        required_permission: "none (user identity from the user access token itself)",
    },
    EndpointPermission {
        endpoint: "GET /user/installations",
        required_permission: "none (enumerates installations the authorized user granted)",
    },
    EndpointPermission {
        endpoint: "GET /user/installations/{installation_id}/repositories",
        required_permission: "metadata: read",
    },
    EndpointPermission {
        endpoint: "GET /repos/{owner}/{repo}",
        required_permission: "metadata: read",
    },
    EndpointPermission {
        endpoint: "GET /repos/{owner}/{repo}/branches",
        required_permission: "contents: read",
    },
    EndpointPermission {
        endpoint: "GET /repos/{owner}/{repo}/tags",
        required_permission: "contents: read",
    },
    EndpointPermission {
        endpoint: "GET /repos/{owner}/{repo}/commits/{ref}",
        required_permission: "contents: read",
    },
    EndpointPermission {
        endpoint: "GET /repos/{owner}/{repo}/zipball/{ref}",
        required_permission: "contents: read",
    },
];

/// The exact GitHub App permission set requested at installation time.
pub const REQUESTED_APP_PERMISSIONS: &[(&str, &str)] =
    &[("contents", "read"), ("metadata", "read")];

/// Permissions v1 explicitly does not request. Kept as a literal, checked
/// list (rather than only prose) so a future PR that adds one of these
/// must touch this file and its accompanying justification.
pub const EXPLICITLY_NOT_REQUESTED: &[&str] = &[
    "contents:write",
    "administration",
    "actions:write",
    "workflows",
    "pull_requests:write",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_matrix_entry_requires_only_read_level_or_no_repository_permission() {
        for row in PERMISSION_MATRIX {
            assert!(
                !row.required_permission.contains("write"),
                "endpoint {} must not require a write permission",
                row.endpoint
            );
        }
    }

    #[test]
    fn requested_permissions_are_exactly_contents_and_metadata_read() {
        assert_eq!(
            REQUESTED_APP_PERMISSIONS,
            &[("contents", "read"), ("metadata", "read")]
        );
    }
}
