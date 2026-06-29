---
id: 0023
title: Add proposed work lifecycle state
status: accepted
date: 2026-06-29
supersedes: []
---

# 0023: Add proposed work lifecycle state

## Context

RepoPact's lifecycle model had `active`, `blocked`, `deferred`, and `completed`.
A downstream public adopter, `ForgeWireLabs/moto-one-hyper-forgewire-rom-lab`,
exposed a missing state: candidate work that should be captured durably but has
not yet been accepted or authorized for implementation.

Mapping that work to `active` overstates authority. Mapping it to `blocked`
misstates the reason it is not moving. Mapping it to `deferred` implies the work
has already been accepted and intentionally postponed.

## Decision

Add `proposed` as a first-class work lifecycle state and directory:

    work/proposed/

A proposed work item records possible intent; it does not grant implementation
authority.

Definitions:

- `proposed`: captured but unauthorized candidate work.
- `active`: accepted work authorized for design or implementation.
- `blocked`: accepted/current work that cannot proceed until a named condition changes.
- `deferred`: accepted work intentionally postponed.
- `completed`: delivered work whose acceptance criteria are evidence-closed.

Proposed work items still validate structurally as work items. Active and
completed work items may not depend on proposed work items, because that would
treat unaccepted candidate work as accepted implementation authority.

## Consequences

- Bootstrap creates `work/proposed/`.
- `repopact new work-item "Title" --status proposed` records a candidate without
  promoting it to active work.
- Adopters can keep candidate work in RepoPact without conflating it with
  authorized implementation.
- Lifecycle semantics are part of conformance, so the conformance suite includes
  a valid proposed item and a rejection case for active work depending on
  proposed work.
