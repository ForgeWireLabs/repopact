# WI056 command authority inventory

This inventory is the implementation boundary for WI056. “Rust canonical” means
the public compatibility command delegates semantic work to `repopact-engine`;
the Python module may remain as a named regression oracle. “Python retained”
means the command continues to own its unique workflow. “Deferred” means no
authority change is made by WI056.

| Command / subcommand | Current Python implementation | Rust-proven semantic overlap | WI056 authority | Final validation dependency | WI050/security |
| --- | --- | --- | --- | --- | --- |
| `init` | `repopact.init_repo` | Bootstrap is unique Python mutation; validation is proven | Python retained | Rust canonical after bootstrap | Optional admission setup remains Python/protected |
| `adopt` | `repopact.adopt_repo` | Adoption is unique Python mutation; validation is proven | Python retained | Rust canonical after adoption | Optional admission setup remains Python/protected |
| `import-plan` | `repopact.plan_import` | Import is unique Python mutation; validation is proven | Python retained | Rust canonical after import | No |
| `doctor` | `repopact.doctor` | Diagnosis/repair is unique Python workflow; validation is proven | Python retained | Rust canonical for final validity | No |
| `takeover` | `repopact.takeover` | Legacy-source retirement is unique Python workflow | Python retained | None | No |
| `validate` | `repopact.validate_repo` | Repository discovery, schema validation, diagnostics | Rust canonical | N/A | Rust rejects unsupported WI050 surfaces explicitly |
| `dashboard` | `repopact.generate_dashboard` | Canonical generated dashboard projection | Rust canonical | N/A | No |
| `spec` | `repopact.generate_spec` | SPEC generation is a separate documentation workflow | Python retained | None | No |
| `new work-item` | `repopact.new` | Typed work-item creation and dashboard impact | Rust canonical | Rust mutation post-validation | Admission policy still blocks unauthorized active creation in Python dispatch |
| `new decision` | `repopact.new` | Generic narrative mutation, not proven by WI054 | Python retained | None | No |
| `new policy` | `repopact.new` | Generic narrative mutation, not proven by WI054 | Python retained | None | No |
| `work propose` | `repopact.new` | Typed proposed work-item creation | Rust canonical | Rust mutation post-validation | No |
| `work amend-proposal` | `repopact.cli` + `repopact.new` data handling | Typed title edit with proposed-only precondition | Rust canonical | Rust mutation post-validation | No |
| `check-frozen` | `repopact.check_frozen_surface` | Git diff/frozen-surface enforcement is not migrated | Python retained | None | Admission acknowledgment rules remain Python |
| `fleet-verify` | `repopact.fleet_verify` | Adopter discovery and release checks are unique | Python retained | Uses its existing workflow | No |
| `release-closeout` | `repopact.fleet_verify` | Release orchestration is unique | Python retained | Uses its existing workflow | No |
| `release-build` | `repopact.release_build` | Packaging/reproducibility orchestration is unique | Python retained | Its artifact checks are Python-owned | No |
| `admission setup/status/begin/revoke` | `repopact.admission` | Protected authority and cryptographic records are outside WI056 | Python retained | None | WI050 authoritative |
| `guard install/status/register/uninstall` | `repopact.platform_backends`, `repopact.guard`, `repopact.windows_guard_service` | Protected service lifecycle is outside WI056 | Python retained | None | WI050 authoritative |
| `approval request/approve/pending/show/deny` | `repopact.admission` and CLI boundary | Protected approval receipts are outside WI056 | Python retained | None | WI050 authoritative |

The engine protocol also exposes read-only `graph` and `analyze` operations for
native/integration consumers. WI056 does not add a public Python human command
for either operation. No generic decision, policy, evidence, filesystem, raw
plan, or WI050 operation is exposed by the engine.
