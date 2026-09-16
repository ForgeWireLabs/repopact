//! WI067 Checkpoint B: the narrow, typed GitHub REST client. Every
//! function here maps to exactly one documented endpoint from
//! `crate::permissions::PERMISSION_MATRIX`; nothing upward of this module
//! (the provider, the Tauri commands, the frontend) ever sees a raw
//! GitHub JSON payload or constructs a GitHub URL itself.

use repopact_remote_provider::error::{ErrorCode, RemoteProviderError, RemoteProviderResult};
use repopact_remote_provider::redact::Secret;
use serde::Deserialize;

use crate::headers::RequestHeaders;
use crate::transport::{RestRequest, RestTransport};

const API_BASE: &str = "https://api.github.com";
const MAX_TAG_PEEL_DEPTH: u8 = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubUser {
    pub id: u64,
    pub login: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubInstallation {
    pub installation_id: u64,
    pub account_login: String,
    /// `"User"` or `"Organization"`, from GitHub's own `account.type`.
    pub account_type: String,
    /// `"all"` or `"selected"` -- whether this installation grants every
    /// repository under the account or only an explicitly chosen subset.
    pub repository_selection: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubRepository {
    pub id: u64,
    pub owner_login: String,
    pub name: String,
    pub full_name: String,
    pub private: bool,
    pub default_branch: Option<String>,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubBranch {
    pub name: String,
    /// A branch always points directly at a commit -- no annotated-tag
    /// peeling is possible or needed here. Still treated as a display-only
    /// value; `resolve_ref` (not this listing) is the item-30 authority
    /// for materialization.
    pub commit_sha: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubTag {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page<T> {
    pub items: Vec<T>,
    /// Opaque -- the exact next-page URL GitHub's own `Link` header
    /// supplied. Never reconstructed from a page number this crate
    /// invents, and never exposed to the frontend as a raw URL (the Tauri
    /// command layer re-wraps this as a provider-neutral cursor string).
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

fn parse_link_header_next(link_header: &Option<String>) -> Option<String> {
    let header = link_header.as_ref()?;
    for part in header.split(',') {
        let mut segments = part.split(';');
        let url_part = segments.next()?.trim();
        let is_next = segments.any(|s| s.trim() == "rel=\"next\"");
        if is_next {
            let url = url_part.trim_start_matches('<').trim_end_matches('>');
            return Some(url.to_string());
        }
    }
    None
}

fn map_status_to_error(
    status: u16,
    retry_after_seconds: Option<u64>,
    body: &[u8],
) -> RemoteProviderError {
    let snippet = String::from_utf8_lossy(&body[..body.len().min(200)]);
    match status {
        401 => RemoteProviderError::new(
            ErrorCode::CredentialExpired,
            "GitHub rejected the access token (401)",
        ),
        403 if retry_after_seconds.is_some() => RemoteProviderError::new(
            ErrorCode::ProviderRateLimited,
            format!(
                "GitHub secondary rate limit; retry after {}s",
                retry_after_seconds.unwrap_or(60)
            ),
        ),
        403 => RemoteProviderError::new(
            ErrorCode::ProviderForbidden,
            format!("GitHub forbade the request: {snippet}"),
        ),
        404 => RemoteProviderError::new(
            ErrorCode::ProviderNotFound,
            "GitHub reported the resource does not exist or is not visible to this token",
        ),
        422 => RemoteProviderError::new(
            ErrorCode::ProviderProtocolError,
            format!("GitHub rejected the request as unprocessable: {snippet}"),
        ),
        429 => RemoteProviderError::new(
            ErrorCode::ProviderRateLimited,
            "GitHub primary rate limit exceeded",
        ),
        other => RemoteProviderError::new(
            ErrorCode::ProviderProtocolError,
            format!("unexpected GitHub status {other}: {snippet}"),
        ),
    }
}

fn get_json(
    transport: &dyn RestTransport,
    token: Option<&Secret>,
    url: &str,
) -> RemoteProviderResult<(serde_json::Value, crate::transport::RestResponse)> {
    let headers = match token {
        Some(token) => RequestHeaders::with_bearer_token(token),
        None => RequestHeaders::new(),
    }
    .as_pairs()
    .into_iter()
    .map(|(name, value)| (name.to_string(), value))
    .collect();
    let response = transport
        .get(&RestRequest {
            url: url.to_string(),
            headers,
        })
        .map_err(|error| RemoteProviderError::new(ErrorCode::NetworkUnavailable, error.message))?;

    if response.rate_limit_remaining == Some(0) {
        let reset = response.rate_limit_reset_epoch_seconds.unwrap_or(0);
        return Err(RemoteProviderError::new(
            ErrorCode::ProviderRateLimited,
            format!("GitHub primary rate limit exhausted; resets at epoch {reset}"),
        ));
    }
    if !(200..300).contains(&response.status) {
        return Err(map_status_to_error(
            response.status,
            response.retry_after_seconds,
            &response.body,
        ));
    }
    let value: serde_json::Value = serde_json::from_slice(&response.body).map_err(|error| {
        RemoteProviderError::new(
            ErrorCode::ProviderProtocolError,
            format!("malformed JSON from GitHub: {error}"),
        )
    })?;
    Ok((value, response))
}

pub fn get_current_user(
    transport: &dyn RestTransport,
    token: &Secret,
) -> RemoteProviderResult<GitHubUser> {
    let (value, _) = get_json(transport, Some(token), &format!("{API_BASE}/user"))?;
    #[derive(Deserialize)]
    struct Response {
        id: u64,
        login: String,
    }
    let parsed: Response = serde_json::from_value(value).map_err(|error| {
        RemoteProviderError::new(
            ErrorCode::ProviderProtocolError,
            format!("malformed /user response: {error}"),
        )
    })?;
    Ok(GitHubUser {
        id: parsed.id,
        login: parsed.login,
    })
}

pub fn list_installations(
    transport: &dyn RestTransport,
    token: &Secret,
    cursor: Option<&str>,
) -> RemoteProviderResult<Page<GitHubInstallation>> {
    let url = cursor
        .map(str::to_string)
        .unwrap_or_else(|| format!("{API_BASE}/user/installations?per_page=100"));
    let (value, response) = get_json(transport, Some(token), &url)?;
    #[derive(Deserialize)]
    struct Account {
        login: String,
        #[serde(rename = "type")]
        account_type: String,
    }
    #[derive(Deserialize)]
    struct Installation {
        id: u64,
        account: Account,
        repository_selection: String,
    }
    #[derive(Deserialize)]
    struct Response {
        installations: Vec<Installation>,
    }
    let parsed: Response = serde_json::from_value(value).map_err(|error| {
        RemoteProviderError::new(
            ErrorCode::ProviderProtocolError,
            format!("malformed /user/installations response: {error}"),
        )
    })?;
    let items = parsed
        .installations
        .into_iter()
        .map(|installation| GitHubInstallation {
            installation_id: installation.id,
            account_login: installation.account.login,
            account_type: installation.account.account_type,
            repository_selection: installation.repository_selection,
        })
        .collect();
    let next_cursor = parse_link_header_next(&response.link_header);
    Ok(Page {
        items,
        has_more: next_cursor.is_some(),
        next_cursor,
    })
}

pub fn list_installation_repositories(
    transport: &dyn RestTransport,
    token: &Secret,
    installation_id: u64,
    cursor: Option<&str>,
) -> RemoteProviderResult<Page<GitHubRepository>> {
    let url = cursor.map(str::to_string).unwrap_or_else(|| {
        format!("{API_BASE}/user/installations/{installation_id}/repositories?per_page=100")
    });
    let (value, response) = get_json(transport, Some(token), &url)?;
    #[derive(Deserialize)]
    struct Owner {
        login: String,
    }
    #[derive(Deserialize)]
    struct Repository {
        id: u64,
        name: String,
        full_name: String,
        owner: Owner,
        private: bool,
        default_branch: Option<String>,
        #[serde(default)]
        archived: bool,
    }
    #[derive(Deserialize)]
    struct Response {
        repositories: Vec<Repository>,
    }
    let parsed: Response = serde_json::from_value(value).map_err(|error| {
        RemoteProviderError::new(
            ErrorCode::ProviderProtocolError,
            format!("malformed installation repositories response: {error}"),
        )
    })?;
    let items = parsed
        .repositories
        .into_iter()
        .map(|repo| GitHubRepository {
            id: repo.id,
            owner_login: repo.owner.login,
            name: repo.name,
            full_name: repo.full_name,
            private: repo.private,
            default_branch: repo.default_branch,
            archived: repo.archived,
        })
        .collect();
    let next_cursor = parse_link_header_next(&response.link_header);
    Ok(Page {
        items,
        has_more: next_cursor.is_some(),
        next_cursor,
    })
}

pub fn list_branches(
    transport: &dyn RestTransport,
    token: Option<&Secret>,
    owner: &str,
    repo: &str,
    cursor: Option<&str>,
) -> RemoteProviderResult<Page<GitHubBranch>> {
    let url = cursor
        .map(str::to_string)
        .unwrap_or_else(|| format!("{API_BASE}/repos/{owner}/{repo}/branches?per_page=100"));
    let (value, response) = get_json(transport, token, &url)?;
    #[derive(Deserialize)]
    struct Commit {
        sha: String,
    }
    #[derive(Deserialize)]
    struct Branch {
        name: String,
        commit: Commit,
    }
    let parsed: Vec<Branch> = serde_json::from_value(value).map_err(|error| {
        RemoteProviderError::new(
            ErrorCode::ProviderProtocolError,
            format!("malformed branches response: {error}"),
        )
    })?;
    let items = parsed
        .into_iter()
        .map(|branch| GitHubBranch {
            name: branch.name,
            commit_sha: branch.commit.sha,
        })
        .collect();
    let next_cursor = parse_link_header_next(&response.link_header);
    Ok(Page {
        items,
        has_more: next_cursor.is_some(),
        next_cursor,
    })
}

pub fn list_tags(
    transport: &dyn RestTransport,
    token: Option<&Secret>,
    owner: &str,
    repo: &str,
    cursor: Option<&str>,
) -> RemoteProviderResult<Page<GitHubTag>> {
    let url = cursor
        .map(str::to_string)
        .unwrap_or_else(|| format!("{API_BASE}/repos/{owner}/{repo}/tags?per_page=100"));
    let (value, response) = get_json(transport, token, &url)?;
    #[derive(Deserialize)]
    struct Tag {
        name: String,
    }
    // Deliberately does not read `commit.sha` here (item 29): the tags
    // list endpoint's `commit` field shape is not documented as always
    // being the peeled underlying commit for an annotated tag. Names only;
    // `resolve_ref` is the sole authority for an immutable commit id.
    let parsed: Vec<Tag> = serde_json::from_value(value).map_err(|error| {
        RemoteProviderError::new(
            ErrorCode::ProviderProtocolError,
            format!("malformed tags response: {error}"),
        )
    })?;
    let items = parsed
        .into_iter()
        .map(|tag| GitHubTag { name: tag.name })
        .collect();
    let next_cursor = parse_link_header_next(&response.link_header);
    Ok(Page {
        items,
        has_more: next_cursor.is_some(),
        next_cursor,
    })
}

#[derive(Deserialize)]
struct GitObject {
    #[serde(rename = "type")]
    object_type: String,
    sha: String,
}

#[derive(Deserialize)]
struct GitRefResponse {
    object: GitObject,
}

#[derive(Deserialize)]
struct GitTagResponse {
    object: GitObject,
}

/// Resolves `refs/heads/{branch}` or `refs/tags/{tag}` to an exact,
/// immutable commit SHA (item 29/30/31), peeling an annotated tag object
/// through `GET /repos/{owner}/{repo}/git/tags/{sha}` until the
/// underlying object is a commit, bounded by `MAX_TAG_PEEL_DEPTH` against
/// a malformed or cyclic tag chain.
pub fn resolve_branch_or_tag(
    transport: &dyn RestTransport,
    token: Option<&Secret>,
    owner: &str,
    repo: &str,
    ref_path: &str, // "heads/main" or "tags/v1.0.0"
) -> RemoteProviderResult<String> {
    let url = format!("{API_BASE}/repos/{owner}/{repo}/git/ref/{ref_path}");
    let (value, _) = get_json(transport, token, &url)?;
    let parsed: GitRefResponse = serde_json::from_value(value).map_err(|error| {
        RemoteProviderError::new(
            ErrorCode::ProviderProtocolError,
            format!("malformed git ref response: {error}"),
        )
    })?;
    peel_to_commit(transport, token, owner, repo, parsed.object)
}

fn peel_to_commit(
    transport: &dyn RestTransport,
    token: Option<&Secret>,
    owner: &str,
    repo: &str,
    mut object: GitObject,
) -> RemoteProviderResult<String> {
    for _ in 0..MAX_TAG_PEEL_DEPTH {
        match object.object_type.as_str() {
            "commit" => return Ok(object.sha),
            "tag" => {
                let url = format!("{API_BASE}/repos/{owner}/{repo}/git/tags/{}", object.sha);
                let (value, _) = get_json(transport, token, &url)?;
                let parsed: GitTagResponse = serde_json::from_value(value).map_err(|error| {
                    RemoteProviderError::new(
                        ErrorCode::ProviderProtocolError,
                        format!("malformed git tag object response: {error}"),
                    )
                })?;
                object = parsed.object;
            }
            other => {
                return Err(RemoteProviderError::new(
                    ErrorCode::ProviderProtocolError,
                    format!("unexpected git object type while resolving a ref: {other}"),
                ))
            }
        }
    }
    Err(RemoteProviderError::new(
        ErrorCode::ProviderProtocolError,
        format!("annotated tag chain exceeded the {MAX_TAG_PEEL_DEPTH}-object peel bound"),
    ))
}

/// Validates and canonicalizes a user-provided commit SHA (or short SHA)
/// against GitHub's own record of the commit, returning the full 40-
/// character SHA GitHub itself reports -- never trusting the caller's
/// input string alone as the resolved revision (item 30).
pub fn resolve_commit(
    transport: &dyn RestTransport,
    token: Option<&Secret>,
    owner: &str,
    repo: &str,
    sha_or_short_sha: &str,
) -> RemoteProviderResult<String> {
    let url = format!("{API_BASE}/repos/{owner}/{repo}/commits/{sha_or_short_sha}");
    let (value, _) = get_json(transport, token, &url)?;
    #[derive(Deserialize)]
    struct Response {
        sha: String,
    }
    let parsed: Response = serde_json::from_value(value).map_err(|error| {
        RemoteProviderError::new(
            ErrorCode::ProviderProtocolError,
            format!("malformed commit response: {error}"),
        )
    })?;
    Ok(parsed.sha)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{rest_json_response, RestResponse, ScriptedRestTransport};
    use serde_json::json;

    fn scripted() -> ScriptedRestTransport {
        ScriptedRestTransport::new()
    }

    #[test]
    fn get_current_user_parses_id_and_login() {
        let transport = scripted();
        transport.push_response(Ok(rest_json_response(
            200,
            json!({"id": 42, "login": "octocat"}),
        )));
        let user = get_current_user(&transport, &Secret::new("token")).unwrap();
        assert_eq!(user.id, 42);
        assert_eq!(user.login, "octocat");
        // No client_secret/token ever appears as a query param or field.
        let sent = transport.received_requests();
        assert!(sent[0].url.contains("/user"));
        assert!(sent[0]
            .headers
            .iter()
            .any(|(k, v)| k == "Authorization" && v.contains("token")));
    }

    #[test]
    fn list_installations_follows_the_link_header_for_pagination() {
        let transport = scripted();
        transport.push_response(Ok(RestResponse {
            status: 200,
            link_header: Some("<https://api.github.com/user/installations?page=2>; rel=\"next\"".into()),
            body: serde_json::to_vec(&json!({"installations": [
                {"id": 1, "account": {"login": "alice", "type": "User"}, "repository_selection": "selected"}
            ]}))
            .unwrap(),
            ..Default::default()
        }));
        let page1 = list_installations(&transport, &Secret::new("t"), None).unwrap();
        assert_eq!(page1.items.len(), 1);
        assert!(page1.has_more);
        assert_eq!(
            page1.next_cursor.as_deref(),
            Some("https://api.github.com/user/installations?page=2")
        );

        transport.push_response(Ok(rest_json_response(
            200,
            json!({"installations": [
                {"id": 2, "account": {"login": "acme-org", "type": "Organization"}, "repository_selection": "all"}
            ]}),
        )));
        let page2 = list_installations(&transport, &Secret::new("t"), page1.next_cursor.as_deref())
            .unwrap();
        assert_eq!(page2.items.len(), 1);
        assert!(!page2.has_more);
        assert_eq!(page2.items[0].account_type, "Organization");

        let sent = transport.received_requests();
        assert_eq!(
            sent[1].url,
            "https://api.github.com/user/installations?page=2"
        );
    }

    #[test]
    fn list_installation_repositories_parses_bounded_fields() {
        let transport = scripted();
        transport.push_response(Ok(rest_json_response(
            200,
            json!({"repositories": [{
                "id": 10, "name": "repo", "full_name": "octocat/repo", "owner": {"login": "octocat"},
                "private": false, "default_branch": "main", "archived": false
            }]}),
        )));
        let page =
            list_installation_repositories(&transport, &Secret::new("t"), 999, None).unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].full_name, "octocat/repo");
        assert!(!page.items[0].private);
    }

    #[test]
    fn list_branches_and_tags_parse_names() {
        let transport = scripted();
        transport.push_response(Ok(rest_json_response(
            200,
            json!([{"name": "main", "commit": {"sha": "a".repeat(40)}}]),
        )));
        let branches = list_branches(&transport, None, "o", "r", None).unwrap();
        assert_eq!(branches.items[0].name, "main");

        transport.push_response(Ok(rest_json_response(200, json!([{"name": "v1.0.0"}]))));
        let tags = list_tags(&transport, None, "o", "r", None).unwrap();
        assert_eq!(tags.items[0].name, "v1.0.0");
    }

    #[test]
    fn resolve_branch_or_tag_returns_the_commit_sha_directly_for_a_lightweight_tag() {
        let transport = scripted();
        transport.push_response(Ok(rest_json_response(
            200,
            json!({"object": {"type": "commit", "sha": "c".repeat(40)}}),
        )));
        let sha = resolve_branch_or_tag(&transport, None, "o", "r", "tags/lightweight").unwrap();
        assert_eq!(sha, "c".repeat(40));
    }

    #[test]
    fn resolve_branch_or_tag_peels_an_annotated_tag_to_its_commit() {
        let transport = scripted();
        // refs/tags/v1.0.0 points at a tag OBJECT, not a commit directly.
        transport.push_response(Ok(rest_json_response(
            200,
            json!({"object": {"type": "tag", "sha": "t".repeat(40)}}),
        )));
        // GET git/tags/{tag_sha} resolves the tag object to its commit.
        transport.push_response(Ok(rest_json_response(
            200,
            json!({"object": {"type": "commit", "sha": "d".repeat(40)}}),
        )));
        let sha = resolve_branch_or_tag(&transport, None, "o", "r", "tags/v1.0.0").unwrap();
        assert_eq!(sha, "d".repeat(40));
    }

    #[test]
    fn resolve_branch_or_tag_bounds_a_pathological_tag_chain() {
        let transport = scripted();
        // A ref pointing to a tag object.
        transport.push_response(Ok(rest_json_response(
            200,
            json!({"object": {"type": "tag", "sha": "0".repeat(40)}}),
        )));
        // ...that always points to another tag object, forever.
        for _ in 0..MAX_TAG_PEEL_DEPTH {
            transport.push_response(Ok(rest_json_response(
                200,
                json!({"object": {"type": "tag", "sha": "0".repeat(40)}}),
            )));
        }
        let result = resolve_branch_or_tag(&transport, None, "o", "r", "tags/cyclic");
        assert!(result.is_err());
    }

    #[test]
    fn resolve_commit_canonicalizes_a_short_sha() {
        let transport = scripted();
        transport.push_response(Ok(rest_json_response(200, json!({"sha": "e".repeat(40)}))));
        let sha = resolve_commit(&transport, None, "o", "r", "eeeeeee").unwrap();
        assert_eq!(sha, "e".repeat(40));
    }

    #[test]
    fn a_401_maps_to_credential_expired() {
        let transport = scripted();
        transport.push_response(Ok(RestResponse {
            status: 401,
            ..Default::default()
        }));
        let error = get_current_user(&transport, &Secret::new("t")).unwrap_err();
        assert_eq!(error.code, ErrorCode::CredentialExpired);
    }

    #[test]
    fn a_404_maps_to_provider_not_found() {
        let transport = scripted();
        transport.push_response(Ok(RestResponse {
            status: 404,
            ..Default::default()
        }));
        let error = get_current_user(&transport, &Secret::new("t")).unwrap_err();
        assert_eq!(error.code, ErrorCode::ProviderNotFound);
    }

    #[test]
    fn a_plain_403_maps_to_provider_forbidden() {
        let transport = scripted();
        transport.push_response(Ok(RestResponse {
            status: 403,
            ..Default::default()
        }));
        let error = get_current_user(&transport, &Secret::new("t")).unwrap_err();
        assert_eq!(error.code, ErrorCode::ProviderForbidden);
    }

    #[test]
    fn a_403_with_retry_after_maps_to_secondary_rate_limit() {
        let transport = scripted();
        transport.push_response(Ok(RestResponse {
            status: 403,
            retry_after_seconds: Some(30),
            ..Default::default()
        }));
        let error = get_current_user(&transport, &Secret::new("t")).unwrap_err();
        assert_eq!(error.code, ErrorCode::ProviderRateLimited);
    }

    #[test]
    fn a_429_maps_to_provider_rate_limited() {
        let transport = scripted();
        transport.push_response(Ok(RestResponse {
            status: 429,
            ..Default::default()
        }));
        let error = get_current_user(&transport, &Secret::new("t")).unwrap_err();
        assert_eq!(error.code, ErrorCode::ProviderRateLimited);
    }

    #[test]
    fn exhausted_primary_rate_limit_is_detected_even_on_a_200() {
        let transport = scripted();
        transport.push_response(Ok(RestResponse {
            status: 200,
            rate_limit_remaining: Some(0),
            rate_limit_reset_epoch_seconds: Some(1234567890),
            body: serde_json::to_vec(&json!({"id": 1, "login": "x"})).unwrap(),
            ..Default::default()
        }));
        let error = get_current_user(&transport, &Secret::new("t")).unwrap_err();
        assert_eq!(error.code, ErrorCode::ProviderRateLimited);
    }

    #[test]
    fn a_422_maps_to_provider_protocol_error() {
        let transport = scripted();
        transport.push_response(Ok(RestResponse {
            status: 422,
            ..Default::default()
        }));
        let error = get_current_user(&transport, &Secret::new("t")).unwrap_err();
        assert_eq!(error.code, ErrorCode::ProviderProtocolError);
    }

    #[test]
    fn malformed_json_is_a_protocol_error_not_a_panic() {
        let transport = scripted();
        transport.push_response(Ok(RestResponse {
            status: 200,
            body: b"not json".to_vec(),
            ..Default::default()
        }));
        let error = get_current_user(&transport, &Secret::new("t")).unwrap_err();
        assert_eq!(error.code, ErrorCode::ProviderProtocolError);
    }

    #[test]
    fn a_transport_level_failure_is_network_unavailable_not_a_panic() {
        let transport = scripted();
        transport.push_response(Err(crate::transport::TransportError {
            message: "connection refused".into(),
        }));
        let error = get_current_user(&transport, &Secret::new("t")).unwrap_err();
        assert_eq!(error.code, ErrorCode::NetworkUnavailable);
    }

    // --- Real, live GitHub network proof (item 39/45): unauthenticated
    // public-repository calls, requiring no client ID/device flow/App
    // registration at all. Not run by default -- `cargo test` skips
    // `#[ignore]` -- to avoid spending this machine's unauthenticated
    // GitHub rate-limit budget (60/hour) on every routine test run. Run
    // explicitly with `--ignored` for checkpoint evidence.

    #[test]
    #[ignore]
    fn live_resolves_a_real_public_branch_to_its_commit_sha() {
        let transport = crate::transport::ReqwestTransport::new().unwrap();
        let sha = resolve_branch_or_tag(&transport, None, "octocat", "Hello-World", "heads/master")
            .unwrap();
        assert_eq!(sha.len(), 40);
        assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    #[ignore]
    fn live_peels_a_real_annotated_tag_on_a_well_known_public_repository() {
        // torvalds/linux's kernel release tags are GPG-signed annotated
        // tags, not lightweight tags -- a real, stable proof that this
        // code path (object.type == "tag") is exercised against live
        // GitHub data, not only a scripted fixture.
        let transport = crate::transport::ReqwestTransport::new().unwrap();
        let sha =
            resolve_branch_or_tag(&transport, None, "torvalds", "linux", "tags/v6.1").unwrap();
        assert_eq!(sha.len(), 40);
        assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    #[ignore]
    fn live_canonicalizes_a_short_sha_on_a_real_public_repository() {
        let transport = crate::transport::ReqwestTransport::new().unwrap();
        // A short prefix of octocat/Hello-World's well-known first commit.
        let sha = resolve_commit(&transport, None, "octocat", "Hello-World", "7fd1a60").unwrap();
        assert_eq!(sha, "7fd1a60b01f91b314f59955a4e4d4e80d8edf11d");
    }

    #[test]
    #[ignore]
    fn live_lists_branches_on_a_real_public_repository() {
        let transport = crate::transport::ReqwestTransport::new().unwrap();
        let page = list_branches(&transport, None, "octocat", "Hello-World", None).unwrap();
        assert!(!page.items.is_empty());
        assert!(page.items.iter().any(|b| b.name == "master"));
    }
}
