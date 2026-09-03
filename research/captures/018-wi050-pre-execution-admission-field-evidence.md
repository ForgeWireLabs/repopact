---
id: 018
title: WI050 pre-execution admission field evidence and bypass capture
date: 2026-09-03
status: recorded
source_of_truth: ../../work/active/050-pre-execution-agent-work-admission-and-preflight-enforcement/threat-model.md
---

# Capture 018 - WI050 pre-execution admission field evidence

## Question

Can the current repository-native workflow mechanically prevent an autonomous
agent from making its first runtime mutation before the work item, mandatory
preflight, owner scope, and frozen approval state are valid?

## Observation

No. The post-3.0.2 commit
f2c80b7dcdc54ff9f4753bc996ef0b6dfba539bf changed
repopact/validate_repo.py and tests to compare evidence timestamps with a
wall clock. Its author and commit timestamps are 2026-09-03 10:15:07 and
10:15:32 -0500. The repository had no pre-execution guard that required a
preflight-authorized active work item before the first source write. A later
local test/validate run, commit, and push could all succeed. WI049 was then
needed to document the governance breach, reconcile the rule, and restore
post-release development identity.

The sequence is materially different from a protected admission boundary:

1. task/session context supplied intent;
2. the first editor/patch or shell action changed runtime source;
3. local validation and tests ran after the change;
4. Git recorded and remote transport accepted the commit;
5. later review/verification exposed the missing first-write authority.

The existing mandatory preflight marker is durable and validator-enforced after
records exist. It is not an interception point for an arbitrary editor,
PowerShell, POSIX shell, Python process, child process, linked worktree, or
new adapter. check-frozen with ack similarly records a caller assertion; it is
not cryptographic operator proof.

## Bypass inventory

The threat model at
work/active/050-pre-execution-agent-work-admission-and-preflight-enforcement/threat-model.md
enumerates direct editor, patch, PowerShell, POSIX shell, Python, arbitrary
process, cwd, linked-worktree, session, subagent, alternate-host, hook/settings,
adapter/guard, self-activation, frozen-approval, replay, scope/profile,
delegation, stale-authority, and OS-difference bypasses. Each entry records
why repository records alone do not stop it and the future guard or sandbox
test that would falsify the proposed control.

## Disposition

This capture establishes the architecture requirement for WI050; it does not
claim that a guard exists. The accepted design must be vendor-neutral, must
fail closed when enforcement coverage is missing, and must distinguish
instruction, session-start, pre-action, sandbox/process, and Git/CI levels.
The implementation phase must prove denial before target bytes change on
Windows, Linux, and macOS.
