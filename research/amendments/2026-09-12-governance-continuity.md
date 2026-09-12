# 2026-09-12 research amendment: governance continuity

This note records the provenance of the 2026-09-12 paper and protocol reconciliation.

## What changed

The research corpus now names **governance continuity** as a distinct property of
repository-native governance and registers **H15 / S8** before any S8 execution.

H15 asks whether represented repository-authoritative governance survives a clean handoff
across workers, machines, and supported clients without relying on predecessor-private
state. S8 measures both recovery correctness and orientation cost.

The paper, formal model, protocol, benchmark protocol, and working outline were reconciled
around that distinction on 2026-09-12.

## Pre-registration boundary

No S8 run had been performed when H15/S8 was added.

Prior evidence such as F-006 may motivate or bound the construct but cannot be counted as
prospective confirmation of H15. Any future graph-enabled S8 condition must be added by a
new dated amendment before graph-condition runs.

## Editorial reconciliation of earlier protocol text

The 2026-09-12 pass also normalized some wording and removed an unsupported quantitative
memory-efficiency comparison from the benchmark protocol while preserving the previously
registered H8-H14 constructs and outcomes. Git history preserves the exact earlier text.
No H8-H14 result had been reclassified by this editorial pass.

The adoption model was tightened at the same time. Provenance typing resolves the
**epistemic concrete-record problem** by allowing reconstructed claims to be `inferred` or
`provisional`. It does not make arbitrary structural contradictions conformant. Cycles,
invalid references, unsupported relationships, and similar structural residue remain
visible and must not be hidden behind provenance labels.

## Repository-orientation graph boundary

A durable repository-orientation graph is discussed in the paper only as future work at
this point. It is not current RepoPact functionality and is not evidence for H15.

S8 already records repository-wide search or grep operations, file reads, tool calls,
bytes read, time, and tokens as orientation-cost measures. Those measures were registered
before graph architecture or implementation so a later graph condition can be evaluated
against a frozen no-graph baseline.
