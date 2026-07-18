# Tooling Audit Inventory

| path | owner | last_reviewed | alignment | next_review | notes |
| --- | --- | --- | --- | --- | --- |
| scripts/validate_repo.py | tooling-owner | 2026-07-18 | current | 2026-10-16 | Schema (jsonschema) + semantic validation: contracts, invariants, frozen surface, roles, findings, work lifecycle including proposed-authority dependencies, cycles, evidence, decisions, policies, version, and canonical dashboard parity. |
| scripts/generate_dashboard.py | tooling-owner | 2026-07-18 | current | 2026-10-16 | Deterministically derives status, record counts, spec version, and audit freshness under audits/reports; omits run-date churn and exposes the canonical writer used by mutation and repair commands. |
| scripts/repo_model.py | tooling-owner | 2026-06-29 | current | 2026-09-29 | Shared record discovery, lifecycle status list, and JSON loading. |
| scripts/frontmatter.py | tooling-owner | 2026-06-15 | current | 2026-09-15 | Minimal front-matter parser for decision and policy records. |
| scripts/check_frozen_surface.py | tooling-owner | 2026-06-15 | current | 2026-09-15 | Diff-time backstop for INV-6: path and symbol matching; best-effort outside git. |
| scripts/init_repo.py | tooling-owner | 2026-06-29 | current | 2026-09-29 | Bootstraps a valid RepoPact into a target directory, including all lifecycle directories (003 B1). |
| scripts/new.py | tooling-owner | 2026-06-29 | current | 2026-09-29 | Stamps work item / decision / policy records from templates; work items accept a lifecycle status (003 B2, 0023). |
| scripts/generate_spec.py | tooling-owner | 2026-06-15 | current | 2026-09-15 | Generates SPEC.md catalog + invariant blocks from schemas and invariants (004). |
| scripts/demo.sh / scripts/demo.ps1 | tooling-owner | 2026-06-15 | current | 2026-09-15 | Scripted demo of the init -> work -> evidence loop (004). |
| scripts/repopact_cli.py | tooling-owner | 2026-07-18 | current | 2026-10-16 | `repopact` console entry point dispatching init/validate/new/dashboard/spec/check-frozen; mutations refresh canonical dashboard output. |
| scripts/fleet_verify.py | tooling-owner | 2026-07-18 | current | 2026-10-16 | Deterministically verifies declared public adopter heads and version contracts, reconstructs reviewable vendored overlays, collapses duplicate local checkouts by remote identity, and gates release closeout on separate publication and ecosystem phases. |
