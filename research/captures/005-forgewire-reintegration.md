# Capture 005 — forgewire kernel re-integration (real repo)

Raw transcript. 2026-06-15. Subject: the **real** `C:/Projects/forgewire` (not an
export). Branch `adopt-repopact`. Deliverable: structured, reviewable kernel
re-integration on the source repository.

## Safety check + branch

```
$ git -C C:/Projects/forgewire rev-parse --abbrev-ref HEAD
main
$ git -C C:/Projects/forgewire status --short | wc -l
0                                   # clean
$ git -C C:/Projects/forgewire checkout -b adopt-repopact
```

## Real adoption (non-destructive)

```
$ python C:/Projects/repopact/scripts/adopt_repo.py --target C:/Projects/forgewire
Created 26 record(s); skipped 52 existing file(s).
  ...
Adopted repository validates as a conformant RepoPact.

$ git status --short -uall | wc -l
26                                  # all additions, zero modifications to existing files
```

## F-008 — `.gitignore` swallows the evidence

```
$ git status --short -uall | grep evidence
                                    # (nothing — evidence/runs/*.json not shown)
$ ls evidence/runs/
20260615-213041-adopt.json          # exists on disk
$ git check-ignore -v evidence/runs/*.json
.gitignore:61:runs/   evidence/runs/20260615-213041-adopt.json   # IGNORED by `runs/`
```

Fix applied to the branch (preserves the original `runs/` intent):

```gitignore
# RepoPact evidence runs are durable governance records, not runtime data
!evidence/runs/
!evidence/runs/*.json
```

```
$ git check-ignore evidence/runs/*.json && echo IGNORED || echo "now tracked"
now tracked
```

## Validate + commit (unpushed, for review)

```
$ python C:/Projects/repopact/scripts/validate_repo.py --root .
Repository governance validation passed.
$ git commit -m "chore: adopt RepoPact governance (kernel re-integration)"
72c21f4da ...
# 29 files changed, 821 insertions(+)   (26 records + .gitignore + REPOPACT-ADOPTION.md + work README)
```

Deliverable for review: branch `adopt-repopact` + `REPOPACT-ADOPTION.md` (the
artifact→record mapping, the F-008 fix, verification, owner next-steps, and undo).
Nothing pushed or merged.
