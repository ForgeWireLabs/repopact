---
id: 0034
title: Resolve source_of_truth paths relative to declaring records
status: accepted
date: 2026-09-02
supersedes: []
---

# 0034: Resolve source_of_truth paths relative to declaring records

## Context

Finding F-017 and field capture 016 showed that `doctor` resolved
`source_of_truth:` tokens against the repository root even though the adopter
records were valid under the record-relative interpretation. Decision 0016 is
the existing durable precedent: `takeover` treats this field like a Markdown
relative link and preserves leading `../` runs when rewriting inbound
references. The two consumers must not disagree.

## Decision

Every `source_of_truth:` path value is resolved relative to the directory of the
record that declares it:

```text
resolved target = declaring_record.parent / source_of_truth_token
```

The rule is uniform. A bare token such as `sibling.md` is resolved beside its
declaring record just like a token containing `/` or `../`; RepoPact does not
silently reinterpret bare names as repository-root-relative merely because a
same-named file happens to exist at the root. Leading `../` traversal is
meaningful and preserved, as established by decision 0016 and its
`takeover.py` implementation.

`doctor` continues to diagnose stale or dead pointers without auto-repair.
`source-of-truth-stale` remains non-destructive because selecting a replacement
target requires operator judgment. Historical records, including F-017 and
capture 016, are not rewritten merely to make the corrected rule look cleaner;
the implementation and resolution note are recorded alongside that history.

## Consequences

`doctor` no longer false-positives valid record-relative pointers and no longer
accepts a root-level coincidence as proof that a bare pointer is valid. Existing
`takeover` behavior remains unchanged, and all source-of-truth consumers now
share one explicit semantic contract.
