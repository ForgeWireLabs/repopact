# RepoPact public copy

*Diataxis mode: explanation (launch copy and posting boundary).*

Status: launch copy prepared; not yet fully posted.

This file is the durable source for RepoPact's initial public-facing copy. It separates the stable package from broader repository work and avoids claiming comparative results that do not yet exist.

## One-line description

Repository-native governance for durable human-agent software engineering. Shared intent, authority, work state, evidence, and history in Git.

## Problem statement

RepoPact preserves the engineering state that code alone does not: what work is authorized, what must remain true, what decisions were made, and what evidence proves completion.

## LinkedIn launch draft

AI coding has made implementation dramatically faster. What I kept running into was a different problem: the state around the code did not survive nearly as well.

Why was this change authorized? What must not be weakened? Which decisions are already settled? What counts as done? What evidence actually proves it?

Those answers often live in a chat session, an agent's memory, an issue tracker, or somebody's head.

That is why I built **RepoPact**.

RepoPact keeps durable engineering state in the repository itself so humans and coding agents can work from the same intent, authority, work state, decisions, invariants, provenance, and evidence.

The goal is not to slow AI-assisted development down. It is to make that speed sustainable across agents, models, machines, providers, and sessions.

RepoPact 3.1.2 is available on PyPI and the project is open source under Apache 2.0:
https://github.com/ForgeWireLabs/repopact

I am also preparing the paper, **"RepoPact: Repository-Native Governance for Durable Human-Agent Software Engineering,"** for arXiv.

Technical criticism is welcome. I would especially like feedback from people building or operating coding-agent systems.

## Show HN draft

### Title

Show HN: RepoPact - repository-native governance for humans and coding agents

### Body

AI coding tools made implementation much faster for me, but they exposed another problem: important engineering state kept getting trapped in individual sessions and tools.

Things like what work is actually authorized, what must not be weakened, which decisions are settled, what counts as completion, and what evidence supports a claim often disappear or have to be reconstructed when a new agent, model, machine, or human takes over.

I built RepoPact to move that state into the repository itself.

RepoPact uses typed, version-controlled records for work lifecycle, authority, invariants, decisions, provenance, and evidence. Humans and coding agents operate against the same durable project state instead of depending on one tool's hidden memory or control plane.

It is not an agent, an agent-memory service, a hosted CI product, or `AGENTS.md++`. It is a governance layer that those systems can share.

The stable package is RepoPact 3.1.2 and the project is Apache-2.0:
https://github.com/ForgeWireLabs/repopact

The repository also contains broader ongoing work such as the Tauri Workbench, Repository Orientation Graph evaluation/productization, provider integrations, optional admission/confinement work, and the research program. Those surfaces keep their own lifecycle, platform, packaging, and evidence boundaries rather than being implied by `pip install repopact`.

I am preparing the accompanying research paper for arXiv. Comparative benchmark results remain separate from the existence of the benchmark infrastructure and will be reported from the corresponding runs/evidence.

I would particularly appreciate criticism of the model, the amount of governance ceremony, places where the repository-native approach breaks down, and comparisons with systems solving the same problem differently.

## Recommended GitHub repository metadata

Recommended repository description:

> Repository-native governance for durable human-agent software engineering. Shared intent, authority, work state, evidence, and history in Git.

Useful discovery topics include `agents-md`, `coding-agents`, `developer-tools`, `ai-governance`, `policy-as-code`, `repository-governance`, `software-engineering`, `human-agent-collaboration`, `agentic-software-engineering`, and `governance-continuity` where GitHub topic naming permits them.

Repository settings remain an operator-managed surface unless the connected GitHub integration exposes the corresponding mutation directly.

## Social preview handoff

Use `docs/assets/readme/repopact-hero-social.png` as the GitHub social preview. It is a 1280 x 640 composition with a solid background, readable RepoPact branding, and the repository/rendezvous concept rather than generic AI imagery. It does not encode release numbers or mutable benchmark claims.

## Posting boundaries

- Stable package claims refer to **3.1.2** unless a later release is actually published.
- Do not imply that every application or active/deferred surface in the repository is part of the PyPI package contract.
- Do not claim comparative model, token-efficiency, security, or coordination improvements before reportable runs exist.
- Do not describe macOS or iOS Workbench validation as equivalent to Windows/Linux/Android without matching evidence.
- Do not describe WI050 as complete; it is deferred, and `pre-action` must not be described as arbitrary-process path confinement.
- Do not describe the Repository Orientation Graph as authoritative repository truth or as a proven orientation-performance win while its governing evaluation/closeout work remains open.
- Prefer "repository-native governance", "durable engineering state", and "governance continuity" over "AI operating system" as the lead framing.

## Short enforcement wording

When a compact explanation is needed:

> RepoPact separates governance authority from runtime enforcement. Its optional assurance model ranges from instruction-only operation through session-start and pre-action gates to OS-backed process confinement where a platform/provider actually proves that stronger boundary.

Do not collapse those assurance classes into a single unqualified "RepoPact enforces agent behavior" claim.
