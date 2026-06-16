# Tooling Audit Inventory

| path | owner | last_reviewed | alignment | next_review | notes |
| --- | --- | --- | --- | --- | --- |
| scripts/validate_repo.py | tooling-owner | 2026-06-15 | current | 2026-09-15 | Schema (jsonschema) + semantic validation: contracts, invariants, frozen surface, roles, findings, work lifecycle, cycles, evidence, decisions, policies, version. |
| scripts/generate_dashboard.py | tooling-owner | 2026-06-15 | current | 2026-09-15 | Derives status, record counts, spec version, and audit freshness under audits/reports. |
| scripts/repo_model.py | tooling-owner | 2026-06-15 | current | 2026-09-15 | Shared record discovery and JSON loading. |
| scripts/frontmatter.py | tooling-owner | 2026-06-15 | current | 2026-09-15 | Minimal front-matter parser for decision and policy records. |
| scripts/check_frozen_surface.py | tooling-owner | 2026-06-15 | current | 2026-09-15 | Diff-time backstop for INV-6: path and symbol matching; best-effort outside git. |
| scripts/init_repo.py | tooling-owner | 2026-06-15 | current | 2026-09-15 | Bootstraps a valid RepoPact into a target directory (003 B1). |
| scripts/new.py | tooling-owner | 2026-06-15 | current | 2026-09-15 | Stamps work item / decision / policy records from templates (003 B2). |
| scripts/generate_spec.py | tooling-owner | 2026-06-15 | current | 2026-09-15 | Generates SPEC.md catalog + invariant blocks from schemas and invariants (004). |
| scripts/demo.sh / scripts/demo.ps1 | tooling-owner | 2026-06-15 | current | 2026-09-15 | Scripted demo of the init -> work -> evidence loop (004). |
| scripts/repopact_cli.py | tooling-owner | 2026-06-15 | current | 2026-09-15 | `repopact` console entry point dispatching init/validate/new/dashboard/spec/check-frozen (005, issue #2). |
