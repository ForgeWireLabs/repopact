# Docs Agent Contract

## Scope

This subtree owns the Diataxis documentation set: tutorial (`adopt-repopact.md`),
how-to guides (`guides/`), and explanation (`concepts.md`). The normative reference
is `SPEC.md` at the repository root (governance scope), not here.

## Constraints

- Docs explain and instruct; they do not restate source records. Link to the
  charter, schemas, `SPEC.md`, and decisions rather than copying them (policy 001).
- Each document declares its Diataxis mode in a one-line note at the top.
- No per-page audit inventories. Coverage is declared once in the audit registry.

## Required checks

```powershell
python scripts/validate_repo.py
```

## Traceability

This scope is registered in `audits/registry.json`. Update that row's review dates
when the documentation set changes materially.
