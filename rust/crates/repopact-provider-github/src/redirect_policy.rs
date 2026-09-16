//! WI067 item 19, item 25: allowlisted origins. Never open an arbitrary
//! provider-supplied URL in the system browser, and never let an
//! `Authorization` header follow a redirect to a host outside this
//! allowlist.

/// The only verification origin the device-flow UI may display/open in
/// the system browser. GitHub's device-flow documentation fixes this
/// exact origin; RepoPact never opens a `verification_uri` from response
/// data without checking it against this constant first.
pub const TRUSTED_DEVICE_VERIFICATION_ORIGIN: &str = "https://github.com/login/device";

pub fn is_trusted_verification_uri(uri: &str) -> bool {
    uri == TRUSTED_DEVICE_VERIFICATION_ORIGIN
        || uri
            .strip_prefix(TRUSTED_DEVICE_VERIFICATION_ORIGIN)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('?') || rest.starts_with('/'))
}

/// Hosts that may legitimately receive this app's GitHub `Authorization`
/// header. `codeload.github.com` is GitHub's own archive-download host;
/// `api.github.com` is the REST API host. A redirect to any other host
/// must have the header stripped before the request is followed -- this
/// is a design record for the archive-download implementation a later
/// checkpoint lands, not yet exercised against live traffic since no
/// live download exists in Checkpoint A.
pub const AUTHORIZED_HEADER_HOSTS: &[&str] = &["api.github.com", "codeload.github.com"];

pub fn may_receive_authorization_header(host: &str) -> bool {
    AUTHORIZED_HEADER_HOSTS.contains(&host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trusted_origin_matches_itself_and_subpaths() {
        assert!(is_trusted_verification_uri(
            TRUSTED_DEVICE_VERIFICATION_ORIGIN
        ));
        assert!(is_trusted_verification_uri(
            "https://github.com/login/device?user_code=ABCD-EFGH"
        ));
    }

    #[test]
    fn a_lookalike_host_is_rejected() {
        assert!(!is_trusted_verification_uri(
            "https://github.com.evil.example/login/device"
        ));
        assert!(!is_trusted_verification_uri(
            "https://githu6.com/login/device"
        ));
    }

    #[test]
    fn only_the_two_documented_hosts_may_receive_the_authorization_header() {
        assert!(may_receive_authorization_header("api.github.com"));
        assert!(may_receive_authorization_header("codeload.github.com"));
        assert!(!may_receive_authorization_header("evil.example"));
        assert!(!may_receive_authorization_header("github.com"));
    }
}
