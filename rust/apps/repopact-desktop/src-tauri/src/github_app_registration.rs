//! Decision 0062: typed native application registration configuration for
//! the official RepoPact GitHub App. Registering a GitHub App is a
//! RepoPact/ForgeWire Labs *product* concern, not an end-user concern --
//! an installed RepoPact user never sees, supplies, or is asked to
//! configure a client ID, a client secret, or an app slug.
//!
//! `client_id` and `public_client_secret` are both public application
//! metadata under GitHub's own native/public-client model (see
//! `repopact-provider-github::browser_flow`'s module doc and Decision
//! 0062): GitHub calls one of them a "client secret", but a value baked
//! into a distributed native binary cannot be kept confidential, so this
//! type never stores it in `CredentialStore`, never labels it as securely
//! stored, and this module never claims otherwise.

/// Official values, baked in at *build* time (not read from the running
/// user's environment) via `option_env!`. RepoPact's release pipeline sets
/// `REPOPACT_GITHUB_APP_CLIENT_ID`/`REPOPACT_GITHUB_APP_CLIENT_SECRET`/
/// `REPOPACT_GITHUB_APP_SLUG` as build-time environment variables before
/// invoking `cargo build`/`tauri build`; a source checkout with nothing set
/// compiles a binary that is honestly "not configured" rather than
/// fabricating a value.
pub struct GitHubAppRegistration {
    pub client_id: String,
    pub public_client_secret: String,
    pub app_slug: String,
}

fn non_empty(value: Option<&'static str>) -> Option<&'static str> {
    value.filter(|v| !v.is_empty())
}

impl GitHubAppRegistration {
    fn from_build_time_values() -> Option<Self> {
        let client_id = non_empty(option_env!("REPOPACT_GITHUB_APP_CLIENT_ID"))?;
        let public_client_secret = non_empty(option_env!("REPOPACT_GITHUB_APP_CLIENT_SECRET"))?;
        let app_slug = non_empty(option_env!("REPOPACT_GITHUB_APP_SLUG"))?;
        Some(Self {
            client_id: client_id.to_string(),
            public_client_secret: public_client_secret.to_string(),
            app_slug: app_slug.to_string(),
        })
    }

    /// A debug/development-only escape hatch so a developer can point a
    /// local debug build at a real registered GitHub App (e.g. a personal
    /// test app) without a full official build pipeline. Compiled out
    /// entirely in a release build (`cfg(debug_assertions)`) -- there is no
    /// runtime flag that re-enables it in a release binary -- and this is
    /// never documented as normal product setup; see
    /// `docs/guides/github-app-setup.md`.
    #[cfg(debug_assertions)]
    fn from_debug_only_environment() -> Option<Self> {
        let read = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
        Some(Self {
            client_id: read("REPOPACT_DEV_GITHUB_APP_CLIENT_ID")?,
            public_client_secret: read("REPOPACT_DEV_GITHUB_APP_CLIENT_SECRET")?,
            app_slug: read("REPOPACT_DEV_GITHUB_APP_SLUG")?,
        })
    }

    #[cfg(not(debug_assertions))]
    fn from_debug_only_environment() -> Option<Self> {
        None
    }

    /// `None` means an official build's registration metadata was not
    /// baked in and no developer override is present -- the honest,
    /// user-facing outcome is "GitHub integration is not configured in
    /// this development build," never a prompt to open a terminal and set
    /// an environment variable.
    pub fn load() -> Option<Self> {
        Self::from_build_time_values().or_else(Self::from_debug_only_environment)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_empty_filters_out_the_empty_string() {
        assert_eq!(non_empty(Some("")), None);
        assert_eq!(non_empty(Some("x")), Some("x"));
        assert_eq!(non_empty(None), None);
    }
}
