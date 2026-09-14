# RepoPact public copy

Status: launch copy prepared; not yet posted.

This file is the durable source for RepoPact's initial public-facing copy. It intentionally separates the stable package from newer development work on `main` and avoids claiming comparative results that do not yet exist.

## One-line description

Repository-native governance for durable human-agent software engineering. Shared intent, authority, work state, evidence, and history in Git.

## Problem statement

RepoPact preserves the engineering state that code alone does not: what work is authorized, what must remain true, what decisions were made, and what evidence proves completion.

## LinkedIn launch draft

AI coding has made implementation dramatically faster. What I kept running into was a different problem: the state around the code did not survive nearly as well.

Why was this change authorized? What must not be weakened? Which decisions are already settled? What counts as done? What evidence actually proves it?

Those answers often live in a chat session, an agent's memory, an issue tracker, or somebody's head.

That is why I built **RepoPact**.

RepoPact keeps durable engineering state in the repository itself so humans and coding agents can work from the same intent, authority, work state, decisions, invariants, and evidence.

The goal is not to slow AI-assisted development down. It is to make that speed sustainable across agents, models, machines, and sessions.

RepoPact is open source under Apache 2.0:
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

RepoPact uses typed, version-controlled records for work lifecycle, authority, invariants, decisions, provenance, and evidence. Humans and coding agents operate against the same repository state instead of maintaining separate hidden control planes.

It isn't an agent, an agent-memory service, or a hosted CI product. It is intended to be a durable governance layer that those systems can share.

The project is Apache-2.0:
https://github.com/ForgeWireLabs/repopact

The stable package is currently 3.0.2. Development `main` has moved considerably farther, including the Rust semantic engine, Tauri Workbench, local-first verification/release architecture, and active repository-orientation work. I am preparing the accompanying research paper for arXiv.

I would particularly appreciate criticism of the model, the amount of governance ceremony, places where the repository-native approach breaks down, and comparisons with systems solving the same problem differently.

## GitHub repository metadata handoff

The repository description currently uses the older "repository-native operating system" language. Update it in GitHub repository settings to:

> Repository-native governance for durable human-agent software engineering. Shared intent, authority, work state, evidence, and history in Git.

Retain useful discovery topics such as `agents-md`, `coding-agents`, `developer-tools`, `ai-governance`, `policy-as-code`, and `repository-governance`. Add, where GitHub topic naming permits:

- `software-engineering`
- `human-agent-collaboration`
- `agentic-software-engineering`
- `governance-continuity`

The GitHub connector used for this preparation pass does not expose repository-description or topic mutation, so these remain explicit operator actions rather than silently unperformed work.

## Social preview handoff

Use `docs/assets/readme/repopact-hero-social.png` as the GitHub social preview. It is a 1280 x 640 composition with a solid background, readable RepoPact branding, and the repository/rendezvous concept rather than generic AI imagery. It does not encode release numbers or mutable benchmark claims.

## Posting boundaries

- Do not claim the current development tree is the stable PyPI 3.0.2 package.
- Do not claim comparative model, token-efficiency, security, or coordination improvements before reportable live runs exist.
- Do not describe macOS or iOS as natively validated yet.
- Do not describe WI050 protected pre-execution enforcement as complete.
- Do not describe the Repository Orientation Graph as authoritative repository truth or as a proven orientation-performance win.
- Prefer "repository-native governance", "durable engineering state", and "governance continuity" over "AI operating system" as the lead framing.
