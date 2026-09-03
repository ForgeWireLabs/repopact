# Pre-execution admission

RepoPact's WI050 admission plane is opt-in. It is useful when a host must ask
whether a session or action may mutate a repository before the mutation occurs.

Run `repopact admission setup --root PATH --key-file OUTSIDE_REPO` once during
operator-controlled setup. The command writes three public, schema-validated
records under `governance/` and a trust-pinned registration under an external
protected directory. Private Ed25519 key material is encrypted and never placed
in the repository. Existing protected registration is not silently replaced.

`repopact admission begin --work-item NNN --session ID` creates a canonical
request. An operator signs that request with the approval CLI in an interactive
terminal. The guard verifies the receipt and issues a short-lived lease. Every
pre-action adapter calls the guard before its callback; a denial means the
callback is not invoked. `repopact admission status` and `repopact doctor`
expose safe health diagnostics only.

The adapter SPI reports facts, not a marketing flag. The reference coding and
launcher adapters provide pre-action checks but do not claim arbitrary child
process or filesystem confinement. Host backends can report
`sandbox/process-enforced` only after a real OS boundary is installed. MCP,
hooks, repository settings, and shell wrappers are integration surfaces and
backstops, never the sole trust root.

Bootstrap commands `repopact work propose` and `repopact work amend-proposal`
cannot transition proposed work to active or fabricate an operator receipt.
Frozen-surface changes require a receipt bound to the exact request digest;
`check-frozen --ack` remains advisory.
