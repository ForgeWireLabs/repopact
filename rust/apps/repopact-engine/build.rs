use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest.join("../../..");
    let version_path = root.join("VERSION");
    let label_path = root.join("RELEASE_LABEL");
    println!("cargo:rerun-if-changed={}", version_path.display());
    println!("cargo:rerun-if-changed={}", label_path.display());

    let version = fs::read_to_string(&version_path)
        .expect("RepoPact VERSION must be present when building repopact-engine")
        .trim()
        .to_owned();
    let label = fs::read_to_string(&label_path)
        .ok()
        .map(|value| value.trim().to_owned());
    let product_version = pep440_version(&version, label.as_deref());
    println!("cargo:rustc-env=REPOPACT_ENGINE_VERSION={product_version}");
}

fn pep440_version(version: &str, label: Option<&str>) -> String {
    let Some(label) = label else {
        return version.to_owned();
    };
    let prefix = format!("{version}-");
    let Some(prerelease) = label.strip_prefix(&prefix) else {
        panic!("RELEASE_LABEL must be a SemVer pre-release of VERSION");
    };
    let parts = prerelease.split('.').collect::<Vec<_>>();
    if parts.len() == 2 && parts[1].chars().all(|character| character.is_ascii_digit()) {
        let kind = match parts[0] {
            "alpha" | "a" => "a",
            "beta" | "b" => "b",
            "rc" => "rc",
            "dev" => "dev",
            _ => "",
        };
        if !kind.is_empty() {
            return format!("{version}.{kind}{}", parts[1]);
        }
    }
    let encoded = label
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{version}.dev0+semver.{encoded}")
}
