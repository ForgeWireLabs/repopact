use std::fs;

use repopact_desktop_api::DesktopService;
use tempfile::tempdir;

#[test]
fn workbench_validation_includes_shared_verification_contract_diagnostics() {
    let temp = tempdir().expect("temp repository");
    let root = temp.path();
    fs::create_dir_all(root.join("governance")).expect("governance directory");
    fs::write(
        root.join("governance/verification.json"),
        r#"{
          "$schema":"../schemas/verification-profile.schema.json",
          "version":1,
          "default_profile":"missing",
          "execution_policy":{"local_primary":true,"hosted_ci_default":false,"hosted_cd_default":false},
          "profiles":{"quick":{"description":"quick","coverage":"host","steps":[{"id":"validate","argv":["{repopact}","validate"]}]}}
        }"#,
    )
    .expect("verification contract");

    let service = DesktopService::new();
    let overview = service.open_repository(root).expect("repository opens");

    assert!(overview.validation.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "verification.default-profile-missing"
            && diagnostic.path.as_deref() == Some("governance/verification.json")
    }));
}
