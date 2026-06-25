# PactBench fixtures

Each fixture is a small, **real** source tree plus a `fixture.json` that declares the
guarantees and frozen surface in an **arm-neutral** way. The harness materializes the two
arms from it:

- **`repopact` arm** — `fixture.json` invariants/frozen-surface become RepoPact records
  (`governance/invariants.json`, `governance/frozen-surface.json`), enforced by the
  validator and `check-frozen`.
- **`baseline` arm** — the same guarantees are rendered as *prose guidance* in a root
  `AGENTS.md` (instructions only, no enforcement).

Keeping the guarantee content identical across arms is what makes the comparison fair
(threat T6): only the *delivery mechanism* differs, not the information.

## Layout

```
fixtures/<name>/
  <source tree>          # real, runnable code
  tests/                 # tests that encode the guarantees
  fixture.json           # arm-neutral governance spec + used_by_tasks
```

## Status

| Fixture | Real? | Tasks |
|---|---|---|
| `calc-rounding` | ✅ built (calc.py + tests) | 0001, 0012, 0013, 0018, 0020 |
| `api-orders` | ✅ built (auth guard, routes, tests) | 0002, 0003, 0014, 0015, 0016, 0017, 0019, 0021 |
| `importer` | ⏳ referenced by 0004; to build | 0004 |
| `checkout` | ⏳ referenced by 0005; to build | 0005 |
| `counter-service` | ⏳ referenced by 0006; to build | 0006 |
| `auth-service` | ⏳ referenced by 0007; to build | 0007 |
| `billing-service` | ⏳ referenced by 0008; to build | 0008 |
| `file-service` | ⏳ referenced by 0009; to build | 0009 |
| `metrics-service` | ⏳ referenced by 0010; to build | 0010 |
| `vendor-client` | ⏳ referenced by 0011; to build | 0011 |

The two built fixtures cover the largest task clusters (correctness + authz/input-validation
+ the decoys). The remaining single-task fixtures are stubs to build as the suite scales;
tasks already reference them by name so they slot in without task edits.
