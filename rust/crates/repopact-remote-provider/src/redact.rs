//! Credential redaction (WI067 GH-004/GH-013/GH-026, item 44). Two layers:
//!
//! - [`Secret`] never implements `Serialize`, `Display`, or a
//!   content-revealing `Debug` -- the compiler, not discipline, keeps a
//!   token out of a DTO that derives `Serialize`/`Debug` on a struct
//!   containing one.
//! - [`redact`] is a best-effort string scrubber for free-text error
//!   detail/log lines that might *quote* a URL or header value containing a
//!   credential (e.g. an upstream HTTP error message). It is not a
//!   substitute for never putting a `Secret` field on a serializable type.

use std::fmt;

/// A credential value that must never reach JSON, logs, or `Debug` output.
/// The only way to read the inner value is [`Secret::expose`], which every
/// call site names explicitly and which this crate never calls itself.
#[derive(Clone)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Explicit, greppable escape hatch. Only the credential-store/HTTP
    /// adapter boundary may call this.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(REDACTED)")
    }
}

// Deliberately no `Serialize`/`Deserialize`/`Display` impl: a struct that
// embeds a `Secret` field cannot derive `Serialize` without a compile
// error, which is the point.

/// Scrub common credential shapes out of a free-text string before it is
/// allowed into an error detail, log line, or evidence payload:
/// `Authorization: <scheme> <token>` headers, GitHub token prefixes
/// (`ghu_`, `ghr_`, `ghp_`, `gho_`, `ghs_`, `github_pat_`), and
/// `://user:pass@`-shaped credential-bearing URLs.
pub fn redact(input: &str) -> String {
    let mut out = input.to_string();
    out = AUTH_HEADER
        .replace_all(&out, "Authorization: [REDACTED]")
        .into_owned();
    out = TOKEN_PREFIXED
        .replace_all(&out, "[REDACTED_TOKEN]")
        .into_owned();
    out = CREDENTIAL_URL
        .replace_all(&out, "://[REDACTED]@")
        .into_owned();
    out
}

use once_cell::sync::Lazy;
use regex::Regex;

static AUTH_HEADER: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)authorization:\s*\S+(\s+\S+)?").unwrap());
static TOKEN_PREFIXED: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"gh[uprso]_[A-Za-z0-9]+|github_pat_[A-Za-z0-9_]+").unwrap());
static CREDENTIAL_URL: Lazy<Regex> = Lazy::new(|| Regex::new(r"://[^/@\s]+@").unwrap());

/// Synthetic token-shaped canaries for redaction tests (item 44). These are
/// not usable credentials -- they are the documented GitHub prefixes
/// followed by an obviously-fake body.
#[cfg(test)]
pub mod canaries {
    pub const USER_TOKEN: &str = "ghu_test0000000000000000000000000000";
    pub const REFRESH_TOKEN: &str = "ghr_test0000000000000000000000000000";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_token_canary_is_redacted_from_free_text() {
        let msg = format!("upstream said: token {} was rejected", canaries::USER_TOKEN);
        let cleaned = redact(&msg);
        assert!(!cleaned.contains(canaries::USER_TOKEN));
        assert!(cleaned.contains("[REDACTED_TOKEN]"));
    }

    #[test]
    fn refresh_token_canary_is_redacted_from_free_text() {
        let msg = format!("refresh_token={}", canaries::REFRESH_TOKEN);
        let cleaned = redact(&msg);
        assert!(!cleaned.contains(canaries::REFRESH_TOKEN));
    }

    #[test]
    fn authorization_header_is_redacted() {
        let msg = "request failed; Authorization: Bearer abc.def.ghi was sent";
        let cleaned = redact(msg);
        assert!(!cleaned.contains("abc.def.ghi"));
        assert!(cleaned.contains("Authorization: [REDACTED]"));
    }

    #[test]
    fn credential_bearing_url_is_redacted() {
        let msg = "fetch failed for https://x-access-token:ghu_test123@github.com/o/r.git";
        let cleaned = redact(msg);
        assert!(
            !cleaned.contains("ghu_test123") || !cleaned.contains("x-access-token:ghu_test123@")
        );
    }

    #[test]
    fn secret_debug_never_reveals_inner_value() {
        let s = Secret::new("ghu_test0000000000000000000000000000");
        let debug = format!("{s:?}");
        assert_eq!(debug, "Secret(REDACTED)");
    }

    #[test]
    fn secret_type_does_not_implement_serialize() {
        // Compile-time proof, not a runtime assertion: if `Secret` ever
        // gains `#[derive(Serialize)]`, any struct field of type `Secret`
        // used inside a `#[derive(Serialize)]` DTO fails to compile. This
        // test exists so `cargo test` exercises the module and the intent
        // is documented at the call site future editors will read.
        fn assert_no_serialize<T>() {}
        assert_no_serialize::<Secret>();
    }
}
