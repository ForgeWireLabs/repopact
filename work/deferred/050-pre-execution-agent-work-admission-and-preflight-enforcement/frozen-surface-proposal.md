# WI050 exact frozen-surface proposal

> No frozen path is changed in this architecture pass. This is the list an
> implementation pass must take to the operator for WI050-specific INV-6
> approval. Approval from WI044, another work item, or a self-supplied
> check-frozen acknowledgement does not carry over.

## Required if the selected architecture is implemented

| Path | Proposed change | Why it is required |
| --- | --- | --- |
| repopact/schemas/admission-policy.schema.json | Add the canonical adopter policy/profile and enforcement-coverage record. | The policy is a load-bearing, versioned contract and must be schema-validated. |
| repopact/schemas/operator-authority.schema.json | Add the public non-secret authority declaration. | Operator ids, public keys/fingerprints, roles, classes, quorum, rotation, and delegation ceilings need a durable structural contract. |
| repopact/schemas/authorization-request.schema.json | Add the canonical request/digest input. | UI-neutral requests need deterministic structure and exact binding fields. |
| repopact/schemas/authorization-receipt.schema.json | Add signed receipt and revocation/invalidation references. | The guard must verify replay-resistant operator proof without private material. |
| repopact/schemas/repository-registration.schema.json | Add the recoverable public registration shape. | Canonical repository/root identity and worktree binding need a stable record. |
| repopact/schemas/adapter-capabilities.schema.json | Add the adapter capability vector and truthful enforcement declaration. | Adapters must declare facts rather than assert a boolean enforced flag. |

Adding any of these files matches frozen glob repopact/schemas/** and therefore
requires an explicit operator approval before implementation. Existing
work-item and evidence schemas should be extended only if implementation
proves the new records cannot remain separate; such an extension would be an
additional exact path in the approval request, not an implied permission.

## Optional later changes

| Path | Status | Rationale |
| --- | --- | --- |
| .github/workflows/** | Optional, not part of first-write enforcement. | A CI job may verify receipts, conformance, or coverage as a Git/CI backstop, but CI cannot prevent the first local write. |
| governance/invariants.json | Optional only for a future universal invariant. | Opt-in enforced admission can be implemented as policy without changing the existing pact. Adding a new universal invariant would be a separate operator decision. |
| governance/charter.md | Optional documentation clarification. | The current charter already states systems-before-sessions and completion-with-proof; no semantic change is needed. |

## Not needed for the architecture selected

* No change to the existing frozen-surface declaration is needed to authorize
  this design. The declaration is consulted by the future guard.
* No WI044 typed-completion schema, WI047 documentation-closure schema, or
  WI046 promotion schema belongs in WI050.
* No vendor configuration path, ForgeWire Fabric integration, chat transcript,
  private key, hook script, or host-specific setting is a RepoPact authority
  schema.
* No version bump, release label change, stable tag move, or release workflow
  mutation is required.

## Approval packet required before implementation

The implementation request must include the exact list of changed files,
symbols if any, schema compatibility/version impact, operator approval class,
base HEAD/tree, authority-state digest, and whether the change affects
ordinary, frozen, trust, or repair operations. The protected operator protocol
must bind that packet; an agent cannot create its own approval by running
check-frozen with an acknowledgement flag.
