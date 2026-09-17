# Demo

*Diataxis mode: tutorial-adjacent (a short walkthrough you can run or record).*

This is the short RepoPact story using the installed product interface: create a governed repository, record work before implementation, validate the pact, and preserve a real verification run as evidence.

The installed `repopact` CLI is the canonical user-facing path. Historical source-module scripts are not the interface this demo teaches.

## 1. Create a throwaway governed repository

From an environment with the stable package installed:

```powershell
pip install repopact
repopact init --target ./repopact-demo
cd ./repopact-demo
repopact validate
```

The repository starts valid and already contains the governance structure and a minimal local-first verification contract.

## 2. Capture work before implementation

```powershell
repopact new work-item "Demo work"
repopact validate
```

An active work item with a pending acceptance criterion is valid. Pending means the work is not complete yet, not that the repository is broken.

## 3. Run the repository-defined check

```powershell
repopact verify quick
```

This proves only the verification profile that actually ran on this host. It does not automatically complete the work item.

## 4. Preserve a real run as evidence

Use the id of the generated work item:

```powershell
repopact verify quick --evidence-work-item NNN
```

RepoPact writes an immutable concrete evidence record under `evidence/runs/` and refreshes derived state as part of the write.

Open the generated evidence record, link its id from the acceptance criterion **only if it actually proves that criterion**, and then mark the criterion satisfied.

## 5. Show the evidence gate

The important behavior is not "validation always passes." The important behavior is that lifecycle and evidence claims are checked against the repository contract.

For a recording, show both sides:

1. an active item with a pending criterion validating successfully;
2. the criterion becoming satisfied only when linked evidence exists;
3. completion happening only after every criterion is evidence-closed;
4. `repopact dashboard` and `repopact validate` reconciling the final state.

A deliberately malformed fixture can also demonstrate a rejection such as a satisfied criterion with no evidence, but do not mutate a real work item merely to stage the failure. RepoPact's conformance fixtures already exist for repeatable negative cases.

## 6. Close the loop

After the criterion is truly satisfied, move the work item to `work/completed/`, set its machine-readable status to `completed`, and run:

```powershell
repopact dashboard
repopact validate
```

The point of the demo is the separation of concerns:

```text
recorded intent -> represented authority -> implementation -> evidence -> completion
```

A worker's confidence is not the completion boundary. Durable repository state is.

## Record it

Any terminal recorder can capture the walkthrough. Prefer recording the installed CLI sequence above so a public demo tracks the product interface rather than internal source modules.

For a deeper first-use walkthrough, see [Adopt RepoPact](adopt-repopact.md).
