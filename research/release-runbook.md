# Release runbook

## Current billing-locked Actions fallback (used for 2.2.0)

When GitHub Actions cannot execute, the OIDC trusted-publishing path is unavailable.
An operator-authorized direct upload may publish the exact locally validated tag
artifacts without weakening the release gates:

1. Validate the release tree, run the full unit and conformance suites, regenerate
   derived artifacts, and require deterministic output.
2. Build the sdist and wheel from the release commit, run `twine check`, and record
   SHA-256 hashes.
3. Merge and push the release commit, create and push the annotated version tag, and
   confirm the remote tag resolves to that commit.
4. Run `python -m twine upload <exact-wheel> <exact-sdist>` using an operator-held
   PyPI token. Never write the token, `.pypirc`, or secret-bearing output to evidence.
5. Verify the public PyPI JSON/index metadata and install `repopact==<version>` with
   `--no-cache-dir` in a clean virtual environment. Record installed metadata and a
   validator smoke test.

This proves package publication and package identity. It does not prove the unavailable
GitHub workflow or restore CI coverage; that limitation remains explicit in the gap
audit.

## Historical v1.0.0 handoff

The build and verification are done and recorded ([run 003](run-log.md)). What
remains are the outward-facing, credential-bound steps. They are listed here so the
operator can execute them and the paper can cite the exact procedure.

## State at handoff

- Branch `007-proving-ground-hardening`, commit `7ee40d1`: fixes, tests (30/30),
  research record, `VERSION=1.0.0`, decisions `0006`/`0007`.
- Built artifacts: `dist/repopact-1.0.0-py3-none-any.whl` (and rebuildable sdist).
- Proving ground: `C:/Projects/repopact-proving-ground` (local git repo, not pushed).

## 1. Merge to main

```
git checkout main
git merge --no-ff 007-proving-ground-hardening
git push origin main
```

(Or open a PR from the branch and merge via GitHub. The branch is not yet pushed.)

## 2. Tag and GitHub release

```
git tag -a v1.0.0 -m "RepoPact 1.0.0"
git push origin v1.0.0
gh release create v1.0.0 dist/repopact-1.0.0-py3-none-any.whl dist/repopact-1.0.0.tar.gz \
  --title "RepoPact 1.0.0" \
  --notes "First stable release. See decision 0007 and research/ for the adopter evidence."
```

## 3. PyPI — recommended: Trusted Publishing (no stored token)

**DONE:** the workflow `.github/workflows/release.yml` exists (build + Trusted
Publishing publish job, OIDC, no stored token); it was approved through the frozen
surface (`check-frozen --ack`) and decided in `0009`. Work item `009`.

**Operator action remaining (cannot be automated from this repo):**

1. On PyPI, register a **Trusted Publisher** for the project: owner `ForgeWireLabs`,
   repo `repopact`, workflow `release.yml`, environment `pypi` (a *pending* publisher
   is fine before the first upload).
2. Then publish the matching GitHub release (or run the workflow via
   `workflow_dispatch`) — the workflow uploads to PyPI automatically. The current
   `VERSION` is `1.0.1`, so cut `v1.0.1`.

   Manual fallback (token-based, no workflow):
   ```
   python -m pip install twine
   python -m twine upload dist/repopact-1.0.1*
   ```
   using a PyPI API token. Verify with `pip install repopact` in a clean venv.

## 4. Publish the proving ground (optional but recommended)

The proving ground is the evidence behind 1.0. Pushing it public makes the citations
in `research/` and decision `0007` verifiable:

```
gh repo create ForgeWireLabs/repopact-proving-ground --public --source C:/Projects/repopact-proving-ground --push
```

## Credentials / decisions needed from the operator

- PyPI account + Trusted Publishing config (or an API token).
- Approval to merge to `main` and to add a release workflow to the frozen surface.
- Whether to publish the proving-ground repository publicly.
