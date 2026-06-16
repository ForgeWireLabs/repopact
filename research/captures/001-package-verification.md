# Capture 001 — package verification

Raw transcript. 2026-06-15. Host: Windows 10, Python 3.12.4, pip 24.0.
Wheel built from repopact @ commit `ac7848e` (v0.1.0).

## Build (isolated build venv)

```
$ python -m venv .venv
$ .venv/Scripts/python.exe -m pip install --upgrade pip build
$ .venv/Scripts/python.exe -m build
...
adding 'repopact-0.1.0.data/data/share/repopact/schemas/audit-finding.schema.json'
adding 'repopact-0.1.0.data/data/share/repopact/schemas/evidence-run.schema.json'
adding 'repopact-0.1.0.data/data/share/repopact/schemas/frozen-surface.schema.json'
adding 'repopact-0.1.0.data/data/share/repopact/schemas/invariants.schema.json'
adding 'repopact-0.1.0.data/data/share/repopact/schemas/record-frontmatter.schema.json'
adding 'repopact-0.1.0.data/data/share/repopact/schemas/work-item.schema.json'
adding 'repopact-0.1.0.data/data/share/repopact/templates/...'
Successfully built repopact-0.1.0.tar.gz and repopact-0.1.0-py3-none-any.whl
```

## Install into a clean, separate venv

```
$ python -m venv $TEMP/rp_pkgtest/venv
$ $TEMP/rp_pkgtest/venv/Scripts/python.exe -m pip install dist/repopact-0.1.0-py3-none-any.whl

$ repopact --help
usage: repopact [-h] {init,validate,dashboard,spec,new,check-frozen} ...
RepoPact: durable agent work, governed in the repo.

$ ls $TEMP/rp_pkgtest/venv/share/repopact/
schemas
templates
```

Seed data resolved from `<venv>/share/repopact/` — `_seed_dir()`'s `sys.prefix`
branch works under a real venv install. **H1 holds.**

## CLI round-trip against the bootstrapped sandbox

```
$ repopact init --target $TEMP/rp_pkgtest/sandbox
Bootstrapped a valid RepoPact at .../sandbox

$ repopact validate --root .../sandbox
Repository governance validation passed.

$ repopact new work-item "Prove the architecture" --root .../sandbox
Created work\active\001-prove-the-architecture\work-item.json

$ repopact validate --root .../sandbox
Repository governance validation passed.

$ repopact dashboard --root .../sandbox
Generated audits\reports\dashboard.md

$ repopact spec --root .../sandbox
...
FileNotFoundError: [Errno 2] No such file or directory: '.../sandbox/SPEC.md'      # <-- F-001

$ repopact new decision "Adopt RepoPact" --root .../sandbox
Created decisions\0001-adopt-repopact.md
```

`init`, `validate`, `new` (work-item + decision), and `dashboard` all succeeded and
re-validated clean. `spec` crashed — see **F-001**. This is the only break in the
advertised surface on an `init`-fresh repository.
