# Guide: CI integration

*Diataxis mode: how-to (task-oriented).*

RepoPact's gates are plain commands, so any CI runner works. The reference setup is
[`.github/workflows/governance.yml`](../../.github/workflows/governance.yml).

## Minimum gate

```yaml
- run: pip install -r requirements.txt
- run: repopact validate
- run: python -m unittest discover -s tests -v
```

## Keep derived artifacts honest

Derived files (dashboard, SPEC catalog) must match a fresh regeneration, or the
"derive over declare" guarantee (INV-7) is hollow:

```yaml
- run: repopact dashboard
- run: repopact spec
- run: git diff --exit-code -- audits/reports/dashboard.md SPEC.md
```

## Surface frozen-surface changes

On pull requests, report (do not hard-block) changes to the frozen surface, because
the binding gate is human review (INV-6):

```yaml
- if: github.event_name == 'pull_request'
  continue-on-error: true
  run: repopact check-frozen --base "origin/${{ github.base_ref }}"
```

Use `fetch-depth: 0` on checkout so the base ref is available for the diff.
