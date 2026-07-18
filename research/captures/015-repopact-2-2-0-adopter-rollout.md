# Capture 015 — RepoPact 2.2.0 adopter rollout

Evidence capture, 2026-07-18. A local dependency scan was reconciled with a
ForgeWireLabs GitHub code search. Five adopter remotes were found; two local SkillForge
directories were duplicate checkouts of the same remote.

| Adopter | Consumption | Default-branch commit | Validation |
| --- | --- | --- | --- |
| ForgeLink | `repopact==2.2.0` | `86f8d67` | governance; 221 renderer; 202/203 Node, one opt-in live skip |
| SkillForge Academy | `repopact==2.2.0` | `71be8ed` | governance; complete content validator |
| ForgeWire | `repopact==2.2.0` | `d26be0fb` | governance; audit validator 0 errors, 8 age warnings |
| Moto One Hyper ROM Lab | vendored marker `2.2.0` | `4ad7400` | governance plus preserved ROM safety; 150 tests |
| RepoPact Proving Ground | `repopact==2.2.0` | `8f68bc4` | governance; 8 unit; 24 PactBench; drift selftest |

The Proving Ground upgrade exposed a measurement confound: RepoPact 2.2.0 correctly
reported the dashboard stale after the harness created a setup work item, masking the
mutation-specific signal. The harness now regenerates the declared projection after
setup mutations, so structural checks remain attributable and M7 remains an honest
semantic blind spot.

Moto retains its established vendored-tooling model. The 2.2.0 dashboard validator,
canonical generator, repair, bootstrap, import, new-record, takeover, and CLI behavior
were merged into the local copy while retaining the ROM-lab required-surface,
forbidden-artifact, destructive-command, and approval-language checks. No device,
firmware, source-sync, extraction, import, build, or flash operation occurred.

GitHub Contents API reads confirmed every default branch reports 2.2.0, and
`git ls-remote` confirmed each recorded local commit equals its public default-branch
head. ForgeWire's unrelated in-progress Fabric edits remained in its worktree and were
not staged or published by this rollout.
