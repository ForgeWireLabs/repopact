# Capture 012 — tracking import + takeover (SkillForge)

Raw transcript. 2026-06-16. Closes the gaps SkillForge surfaced: a `tracking/`
governance folder RepoPact didn't adapt to, and side-by-side old/new methods.

## tracking/ -> the right record types

```
$ import-plan --target <SkillForge>     # tracking/ now handled
  + decisions/0002-use-repo-native-markdown-foundation-as-the-initial.md   # decisions.md DEC-001
  + decisions/0003-... 0004-...                                            # DEC-002, DEC-003
  + audits/findings/001-...json ... 005-...json                            # risks.md RISK-001..005
  + work/completed/300-milestone-a-mvp                                     # milestones.md M0 (shipped)
  + work/completed/301-milestone-repo-native-work-foundation               # M1 (shipped)
  + work/active/302..305-milestone-*                                       # later milestones
work/ ledger imported; repository validates as a conformant RepoPact.
```

Mappings (decision 0012): `decisions.md`→`decisions/` (DEC-NNN reallocated to a free
4-digit id, original kept in body); `risks.md`→`audits/findings/` (numeric id per
schema, `RISK-NNN` preserved in `source` and prefixed into `observed`; P2→medium,
open→open, scope governance); `milestones.md`→`work/` (shipped→completed/waived).
`status.md`/`work-log.md`/`true-cert-factory.md` were **not** fabricated into records.

## takeover: one source of truth

```
$ takeover --root <SkillForge> --dry-run
  ~ Would archive todos/ -> archive/todos/ (7 migrated item(s))
  i review (not auto-retired; may hold un-migrated content): tracking, ROADMAP.md, CHANGELOG.md

$ takeover --root <SkillForge>
  ~ archive todos/ -> archive/todos/ (7 migrated item(s))
Retired 1 legacy plan dir(s); repository still validates.

$ validate -> passed ; doctor -> healthy
```

`todos/` is gone from the live tree (now `archive/todos/`); `work/` is the ledger.
`tracking/` is **kept and flagged for review** because `status.md`/`work-log.md` aren't
representable as records — takeover never deletes un-captured content. It only retired
`todos/` because every item there was provably migrated (matched via `source`).

## Honesty + safety rules exercised

- No fabricated records for snapshots/logs; risks→findings keep severity/state truthful.
- `takeover` is gated on validation + per-item provenance; archives (reversible) by
  default; refuses partially-migrated or mixed-content sources.

**F-013 (holds).** RepoPact now adapts to governance-folder planning and can fully take
over a migrated legacy method, leaving a single ledger without losing un-captured data.
