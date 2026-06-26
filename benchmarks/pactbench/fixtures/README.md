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
| `api-orders` | ✅ built (auth guard, routes, tests) | 0002, 0003, 0014–0017, 0019, 0021, 0023, 0024 |
| `importer` | ✅ built (importer.py) | 0004 |
| `checkout` | ✅ built (checkout.py) | 0005, 0022 |
| `counter-service` | ✅ built (counter.py) | 0006 |
| `auth-service` | ✅ built (auth.py) | 0007 |
| `billing-service` | ✅ built (billing.py) | 0008 |
| `file-service` | ✅ built (files.py) | 0009 |
| `metrics-service` | ✅ built (metrics.py) | 0010 |
| `vendor-client` | ✅ built (client.py) | 0011 |

All ten fixtures are real source. `calc-rounding` and `api-orders` carry full test suites
(the largest clusters); the single-task fixtures carry the one guarded behaviour their task
exercises.
