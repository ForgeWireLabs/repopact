# 068 — Embedded Mobile Git Backend and Remote Repository Synchronization

> **Status**: 📋 Proposed (not activated)
> **Owners**: TBD (lead) at activation time.
> **Depends on**: [065](../../active/065-mobile-repository-acquisition-app-private-workspace-and-explicit-import-export-implementation/README.md) (Mobile Repository Acquisition, Stage 1 — SAF/archive import-export; expected to move to `work/completed/` at WI065's own closeout in this session).
> **Cross-references**: [Decision 0056](../../../decisions/0056-mobile-repositories-use-app-private-working-copies-with-explicit-acquisition-and-synchronization-boundaries.md), [Decision 0057](../../../decisions/0057-mobile-acquisition-runtime-workspace-registry-and-bounded-import-export-contract.md), [067](../067-github-repository-provider-secure-user-authorization-and-remote-repository-import/README.md) (GitHub Repository Provider — related but distinct, see below). Not a hard dependency on 067.

## Intent

WI065 (Stage 1) delivered mobile repository acquisition via Android SAF directory/archive
import and export, with no embedded Git dependency. Decision 0056 explicitly staged real
Git integration as a Stage 2 follow-up, and reserved the seam for it: the workspace
registry already carries `AcquisitionKind::RemoteGit` and `GitState::EmbeddedGitManaged`
as declared-but-unused variants, and the typed `mobile_git_capabilities` command already
returns a `Stage2Status::UnsupportedStage2` descriptor rather than pretending clone/pull/push
exist. WI065's AC-9 required that this follow-up be created (not activated) at WI065's own
closeout, which is what this item is.

This item owns the actual embedded Git backend: a `GitBackend` abstraction distinct from
the existing shell-command-shaped `GitRunner` trait (Decision 0056 was explicit that
`GitRunner` is not itself the final embedded-Git abstraction), and real clone/fetch/pull/push
semantics against a real `.git` working tree, on both mobile and (where the backend is
shared) desktop.

**In scope**: embedded Git backend design and implementation; real clone/fetch/pull/push;
Android (and shared desktop) runtime proof; credential storage; conflict/divergence
handling for pull/update; typed command surface for Git operations.

**Out of scope**: GitHub-specific API/OAuth/App integration (that is WI067); any change to
`DesktopService`, `RepositorySession`, `RepositoryTopology`'s core repository model, the
`MutationRequest -> MutationPlan -> MutationResult` pipeline, or ROG authority — Git
synchronization must sit behind acquisition/sync boundaries, not inside the repository
mutation kernel.

## Relationship to WI067

WI067 (GitHub Repository Provider, Secure User Authorization, and Remote Repository
Import) and this item are related but distinct, and must not be combined:

- **WI067 v1**: GitHub REST API -> resolve a ref to an exact commit SHA -> bounded
  snapshot/archive materialization -> ordinary workspace. No `.git` directory. No live
  remote relationship after import. Provider-specific (GitHub first).
- **WI068 (this item)**: Git remote (any provider or bare URL) -> real `clone`/`fetch`/
  `pull`/`push` -> a real `.git` working tree -> ordinary workspace. Provider-neutral;
  not GitHub-specific.

GitHub API archive import must not pretend to be Git synchronization, and this embedded
Git backend must not be built GitHub-specific. The two may later share provider/account
UX (e.g. a single "connect a remote" surface) once both exist, but that sharing is a
future decision, not a dependency assumed here. This item depends only on WI065.

## Decisions

None yet — this item is proposed, not activated. Architecture decisions made once active
(backend choice, credential storage mechanism, conflict-resolution policy) should be
promoted to `decisions/` records rather than left implicit in this README.

## Scope

Not yet started. At activation, expected surfaces include (subject to design at
activation time): a `GitBackend` trait and at least one implementation in a Rust crate
alongside `repopact-mobile-acquisition`/`repopact-mobile-saf`; extension of the mobile
Tauri command surface beyond the current `mobile_git_capabilities` stub; Android Gradle/
NDK build-gating for the chosen native Git library; protected credential storage
integration per platform; frontend UX for connect/clone/sync distinct from the existing
SAF import/export UX.

## Closeout

Each acceptance criterion (GIT-001..GIT-008 in `work-item.json`) must be satisfied by
linked evidence before this item can move to `work/completed/`. This item is not
activated by WI065's closeout; sequencing against WI066 (3.1.0 release work), WI050, and
WI067 will be decided separately.
