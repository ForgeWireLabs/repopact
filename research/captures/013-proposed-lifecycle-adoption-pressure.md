# Capture 013 — proposed lifecycle state under adopter pressure

Evidence trace repaired 2026-07-22 for an adoption episode that began on 2026-06-29.
This capture reconstructs only from versioned repository records and public commit
identity; it does not turn the retrospective trace repair into contemporaneous evidence.

## Pressure from a downstream adopter

The public Moto One Hyper ROM Lab had candidate work that needed durable recording
without implementation authority. The four existing states were dishonest fits:
`active` would authorize it, `blocked` would imply an external impediment, `deferred`
would imply prior acceptance, and omission would discard load-bearing intent.

Decision [`0023`](../../decisions/0023-add-proposed-lifecycle-state.md) records that
pressure and the affected adopter. The downstream adoption commit is public and exact:

<https://github.com/ForgeWireLabs/moto-one-hyper-forgewire-rom-lab/commit/0adb522ab4b3e4c33b0beb101ae637ad737c5e91>

That commit converted two planning notes into typed `work/proposed/` records while
preserving their lack of implementation authority.

## Resolution chain

Work item [`025`](../../work/completed/025-add-proposed-lifecycle-state/README.md)
implemented the accepted decision. The change added the state to the shared lifecycle
model and schema, created `work/proposed/` during bootstrap, exposed
`repopact new work-item ... --status proposed`, and rejected dependencies that would
treat proposed work as accepted authority. The conformance corpus added both an accepted
proposed repository and an invalid active-depends-on-proposed repository.

Concrete implementation evidence is
[`20260629-proposed-lifecycle-state`](../../evidence/runs/20260629-proposed-lifecycle-state.json).
It records governance validation, regression tests, and all eight then-current
conformance cases, including both proposed-state cases.

Decision [`0024`](../../decisions/0024-release-2.1.0-proposed-lifecycle.md) classified
the language change as a minor release. Annotated tag `v2.1.0` resolves to release commit
`f4a237d03105c991afcc4b371259f06200e7b226`.

## Rollout and later verification

The originating adopter consumed the new lifecycle at commit `0adb522` on 2026-06-29.
The later ecosystem evidence
[`20260718-repopact-2-2-0-adopter-rollout`](../../evidence/runs/20260718-repopact-2-2-0-adopter-rollout.json)
verified Moto's vendored successor plus every other known adopter on their public
default branches. That later rollout does not replace the originating datum; it proves
the lifecycle behavior survived subsequent release and ecosystem upgrades.

## Interpretation

This is finding F-014: an independent downstream adopter exposed an authority type the
standard lacked, and the resolution is recoverable as a decision → work → implementation
evidence → conformance → release → adopter chain. It supports H6 (recovery from the
repository) and H7 (real adoption pressure) for this episode. It is one traced case, not
a claim that every future standard change will follow the same path.
