# WI057 Operator-Assisted Native Picker Validation

This is the safe native Windows validation path for RPS-018. It launches only
the already-built Workbench, samples the Workbench process tree, and never
automates picker clicks or issues Git commands. The operator performs the
repository-selection and navigation actions manually.

## Command

From the repository root, run:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\work\active\057-desktop-repository-selection-git-process-storm-and-runtime-amplification-audit\desktop-picker-process-audit.ps1 -ExecutablePath 'C:\Users\jerem\AppData\Local\Temp\repopact-rust-target-057\debug\repopact-desktop.exe' -LogPath 'C:\Users\jerem\AppData\Local\Temp\repopact-wi057-desktop-processes.csv'
```

Built executable:

`C:\Users\jerem\AppData\Local\Temp\repopact-rust-target-057\debug\repopact-desktop.exe`

The CSV records UTC samples of the Workbench PID, total descendants, and
descendant processes whose executable name is `git` or `git-*`. The harness
prints the maximum observed Git descendant count and terminates only the exact
launched Workbench tree during cleanup.

## Manual sequence

1. Select the RepoPact repository itself.
2. Navigate across the work, decision, evidence, validation, graph, and analysis views.
3. Press Refresh.
4. Select a scratch/adopted RepoPact repository.
5. Navigate and press Refresh there.
6. Switch back to RepoPact and perform one final navigation/Refresh pass.
7. Record whether any Git console/terminal windows flashed and whether Windows remained responsive.
8. Close the Workbench, allow the harness to print cleanup, and preserve the CSV.

Do not reproduce the former uncontrolled storm. RPS-018 remains pending until
an operator records this sequence, the observed console/responsiveness result,
the CSV peak, and a clean process-tree closeout.
