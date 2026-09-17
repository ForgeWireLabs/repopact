//! Decision 0062: RFC 7636 PKCE (`S256`) and OAuth `state` generation for
//! the browser-redirect authorization-code flow that replaces the GitHub
//! App device flow. Provider-neutral -- any future OAuth2+PKCE provider
//! reuses this module rather than each adapter hand-rolling its own
//! randomness/encoding.
//!
//! Both [`CodeVerifier`] and [`AuthorizationState`] are CSPRNG-generated
//! (`rand::rng()`, the crate's own cryptographically secure thread-local
//! generator, never a non-cryptographic PRNG) and are ephemeral
//! by construction: neither type implements `Serialize`/`Deserialize`, so
//! nothing here can be accidentally persisted to the workspace registry,
//! frontend storage, logs, or evidence.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::Rng;
use sha2::{Digest, Sha256};

use crate::redact::Secret;

/// RFC 7636 recommends 43-128 characters of base64url output; 32 random
/// bytes base64url-encodes to 43 characters (the minimum), which is also
/// GitHub's own documented example length.
const CODE_VERIFIER_RANDOM_BYTES: usize = 32;
/// 256 bits of entropy for the anti-CSRF `state` parameter -- large enough
/// that guessing or brute-forcing a live value is infeasible.
const STATE_RANDOM_BYTES: usize = 32;

/// The PKCE `code_verifier`. Wraps [`Secret`] even though it is not GitHub
/// account credential material: it authorizes a single in-flight token
/// exchange, so it gets the same "never `Serialize`/`Debug`-leak, never
/// crosses the Tauri command boundary" treatment as an access token.
#[derive(Clone)]
pub struct CodeVerifier(Secret);

impl CodeVerifier {
    pub fn generate() -> Self {
        let mut bytes = [0u8; CODE_VERIFIER_RANDOM_BYTES];
        rand::rng().fill_bytes(&mut bytes);
        Self(Secret::new(URL_SAFE_NO_PAD.encode(bytes)))
    }

    pub fn expose(&self) -> &str {
        self.0.expose()
    }

    /// The RFC 7636 `S256` `code_challenge`: `BASE64URL(SHA256(verifier))`.
    /// Safe to place in the authorization URL -- unlike the verifier
    /// itself, the challenge is not sufficient to complete the exchange.
    pub fn s256_challenge(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.expose().as_bytes());
        URL_SAFE_NO_PAD.encode(hasher.finalize())
    }
}

impl std::fmt::Debug for CodeVerifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CodeVerifier(REDACTED)")
    }
}

/// The anti-CSRF/anti-callback-confusion `state` value for one
/// authorization attempt. Not a credential in the confidentiality sense
/// (GitHub echoes it back over an unauthenticated redirect), but it must
/// be unguessable and single-use, so it is generated with the same CSPRNG
/// as the PKCE verifier and compared in constant time.
#[derive(Clone, PartialEq, Eq)]
pub struct AuthorizationState(String);

impl AuthorizationState {
    pub fn generate() -> Self {
        let mut bytes = [0u8; STATE_RANDOM_BYTES];
        rand::rng().fill_bytes(&mut bytes);
        Self(URL_SAFE_NO_PAD.encode(bytes))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Constant-time comparison so a callback's `state` cannot be matched
    /// via a timing side channel.
    pub fn matches(&self, candidate: &str) -> bool {
        let expected = self.0.as_bytes();
        let actual = candidate.as_bytes();
        if expected.len() != actual.len() {
            return false;
        }
        let mut diff = 0u8;
        for (a, b) in expected.iter().zip(actual.iter()) {
            diff |= a ^ b;
        }
        diff == 0
    }
}

impl std::fmt::Debug for AuthorizationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AuthorizationState({})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_verifier_is_url_safe_and_within_rfc_7636_length_bounds() {
        let verifier = CodeVerifier::generate();
        let value = verifier.expose();
        assert!(value.len() >= 43 && value.len() <= 128);
        assert!(value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn two_generated_verifiers_are_never_equal() {
        let a = CodeVerifier::generate();
        let b = CodeVerifier::generate();
        assert_ne!(a.expose(), b.expose());
    }

    #[test]
    fn s256_challenge_is_deterministic_for_a_fixed_verifier_and_matches_rfc_7636_vector() {
        // RFC 7636 Appendix B's own worked example.
        let bytes: Vec<u8> = vec![
            116, 24, 223, 180, 151, 153, 224, 37, 79, 250, 96, 125, 216, 173, 187, 186, 22, 212,
            37, 77, 105, 214, 191, 240, 91, 88, 5, 88, 83, 132, 141, 121,
        ];
        let verifier = CodeVerifier(Secret::new(URL_SAFE_NO_PAD.encode(&bytes)));
        assert_eq!(
            verifier.expose(),
            "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
        );
        assert_eq!(
            verifier.s256_challenge(),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn state_is_url_safe_and_high_entropy() {
        let state = AuthorizationState::generate();
        assert!(state.as_str().len() >= 40);
        assert!(state
            .as_str()
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn two_generated_states_are_never_equal() {
        let a = AuthorizationState::generate();
        let b = AuthorizationState::generate();
        assert_ne!(a.as_str(), b.as_str());
    }

    #[test]
    fn state_matches_itself_and_rejects_a_wrong_value() {
        let state = AuthorizationState::generate();
        assert!(state.matches(state.as_str()));
        assert!(!state.matches("not-the-real-state"));
    }

    #[test]
    fn state_debug_does_not_hide_the_value_since_it_is_not_a_credential() {
        // Unlike Secret/CodeVerifier, `state` is echoed back over an
        // unauthenticated redirect by design, so it is not redacted --
        // this test documents that distinction rather than asserting
        // redaction that would be misleading about its actual sensitivity.
        let state = AuthorizationState::generate();
        assert!(format!("{state:?}").contains(state.as_str()));
    }
}
