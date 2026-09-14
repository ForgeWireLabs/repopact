//! ROG-036 executable/static dependency audit (Decision 0053 section 7):
//! the deterministic ROG core requires no LLM, embedding/vector
//! database, cloud indexing service, or external network provider.
//!
//! This runs `cargo tree` against the exact three crates named by the
//! AC's own audit list (`repopact-graph`, `repopact-repository`,
//! `repopact-core`) and asserts the resolved dependency graph contains
//! none of a documented blocklist of network/LLM/cloud-shaped crate
//! names. It is a dependency-level seam (Decision 0053 section 7 /
//! ROG-036 step 29): "a process-level or dependency-level seam is
//! acceptable if defensible." A brittle DNS/firewall hack is not used.
//!
//! Skipped (not failed) when `cargo` itself is unavailable, matching
//! this workspace's existing best-effort precedent for tool-dependent
//! proofs.

use std::process::Command;

/// Crate name substrings that would indicate a network client, an LLM/
/// embedding provider SDK, a vector database client, or a cloud
/// indexing/search service. Matched against the full `cargo tree`
/// output, so a crate merely *named* similarly in an unrelated context
/// would still need to not appear at all -- there is no legitimate
/// reason any of these should resolve into the deterministic core.
const FORBIDDEN_SUBSTRINGS: &[&str] = &[
    // HTTP/network clients and async network runtimes.
    "reqwest",
    "hyper",
    "tokio",
    "ureq",
    "curl-sys",
    "isahc",
    "surf",
    "async-std",
    "tonic",
    "grpc",
    "websocket",
    "tungstenite",
    // LLM/embedding/provider SDKs.
    "openai",
    "anthropic",
    "async-openai",
    "tiktoken",
    "llm",
    "llama",
    "candle",
    "onnxruntime",
    "ort-",
    "fastembed",
    // Vector/embedding databases and cloud search/index services.
    "pinecone",
    "qdrant",
    "milvus",
    "weaviate",
    "elasticsearch",
    "meilisearch",
    "typesense",
    "algolia",
    // Generic cloud provider SDKs.
    "aws-sdk",
    "aws-config",
    "azure_",
    "google-cloud",
    "gcp-",
];

const AUDITED_CRATES: &[&str] = &["repopact-graph", "repopact-repository", "repopact-core"];

#[test]
fn deterministic_core_dependency_graph_contains_no_network_llm_or_cloud_crate() {
    if Command::new("cargo").arg("--version").output().is_err() {
        eprintln!("skipping: cargo unavailable");
        return;
    }

    for crate_name in AUDITED_CRATES {
        let output = Command::new("cargo")
            .args(["tree", "-p", crate_name, "-e", "normal,build"])
            .current_dir(env!("CARGO_MANIFEST_DIR").to_owned() + "/../..")
            .output()
            .expect("cargo tree must run");
        assert!(
            output.status.success(),
            "cargo tree -p {crate_name} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let tree = String::from_utf8_lossy(&output.stdout).to_lowercase();
        for forbidden in FORBIDDEN_SUBSTRINGS {
            assert!(
                !tree.contains(forbidden),
                "the deterministic core crate `{crate_name}` resolved a forbidden \
                 network/LLM/cloud-shaped dependency matching `{forbidden}` -- ROG-036 \
                 requires this core to need none of those. Full tree:\n{tree}"
            );
        }
    }
}

/// A narrower, purely textual check on each crate's own `Cargo.toml`
/// (not the resolved tree), so a future direct dependency addition is
/// caught even before `cargo tree` would reflect it in a stale lock.
#[test]
fn deterministic_core_manifests_declare_no_network_llm_or_cloud_dependency() {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for crate_name in AUDITED_CRATES {
        let manifest_dir = crate_name.strip_prefix("repopact-").unwrap();
        let manifest_path = workspace_root
            .join("crates")
            .join(format!("repopact-{manifest_dir}"))
            .join("Cargo.toml");
        let Ok(content) = std::fs::read_to_string(&manifest_path) else {
            continue;
        };
        let lower = content.to_lowercase();
        for forbidden in FORBIDDEN_SUBSTRINGS {
            assert!(
                !lower.contains(forbidden),
                "{}'s own Cargo.toml names a forbidden dependency matching `{forbidden}`",
                manifest_path.display()
            );
        }
    }
}
