# Demo

*Diataxis mode: tutorial-adjacent (a scripted walkthrough you can watch or record).*

This is the 60-second story of RepoPact: bootstrap a governed repo, capture work,
and watch the validator enforce the pact. The same sequence is automated in
[`scripts/demo.sh`](../scripts/demo.sh) and
[`scripts/demo.ps1`](../scripts/demo.ps1) so it can be recorded.

## Watch it run

```
bash scripts/demo.sh          # macOS/Linux
pwsh -File scripts/demo.ps1   # Windows
```

The script, in a throwaway directory:

1. `init_repo.py` → a valid RepoPact, then `validate_repo.py` passes.
2. `new.py work-item "Demo work"` → a work item appears under `work/active/`.
3. Validation still passes (an active item with a pending criterion is legitimate).
4. It marks the criterion `satisfied` **without** evidence and runs the validator
   again — which now **fails** with `satisfied without evidence`. That failure is
   the point: completion requires proof.

## Record the animation

The animated GIF is a manual pass (this script is the storyboard):

```
asciinema rec demo.cast -c "bash scripts/demo.sh"
agg demo.cast docs/assets/demo.gif      # or svg-term / vhs
```

Then embed `docs/assets/demo.gif` in the README. Keeping the cast generated from
the script means the demo never drifts from the real tooling.
