# Release runbook

## Current 3.1.3 downloadable-installers release

The 3.1.3 release is a backwards-compatible packaging/distribution release
(decision 0066, work item 070). WI062 proved that a real Windows NSIS
installer, Linux .deb, and Android APK build, install, launch, and uninstall
correctly, but that build predates all 3.1.x release work and none of those
artifacts were ever attached to a public GitHub Release -- only the Python
wheel/sdist has ever been publicly downloadable. This release:

- rebuilds all three installer artifacts from the current tree;
- moves Android from WI062's debug build to a signed release build (fresh
  keystore generated 2026-09-17, private key material never committed);
- ships Windows/macOS artifacts unsigned (operator's explicit cost/time
  decision -- no code-signing certificate acquired for this release);
- attaches all installer artifacts to the v3.1.3 GitHub Release alongside the
  normal PyPI wheel/sdist publication.

No schema, protocol, CLI subcommand semantics, lifecycle, or provenance
behavior changed. The `v3.1.0`, `v3.1.1`, and `v3.1.2` tags and their
already-published artifacts are left untouched; `3.1.3` is the current stable
identity going forward.

Build the Python package only from the clean committed release tree with
`repopact release-build --root . --outdir dist`, inspect the wheel and sdist,
and run `python -m twine check dist\\repopact-3.1.3*`. Build the installer
artifacts with `npx tauri build` (Windows, from `rust/apps/repopact-desktop`),
`npx tauri build --bundles deb` (Linux, from a native Linux/WSL checkout), and
`npx tauri android build --apk` followed by `apksigner sign` with the
operator-held release keystore (Android). Publish the Python wheel/sdist
through the existing secure credential path; attach the installer artifacts
to the GitHub Release via `gh release create`/`gh release upload`. Verify
public metadata and hashes, then install `repopact==3.1.3` in a clean
environment outside the checkout and run the package/resource/conformance
smoke checks. Create and push the annotated `v3.1.3` tag and the GitHub
release.

After the stable tag and publication, continuing `main` development must
restore `VERSION=3.1.3`, `RELEASE_LABEL=3.1.3-dev.1`, and package metadata
`3.1.3.dev1`. Do not move `v3.1.3` or treat that development identity as a
stable publication.

## Historical 3.1.2 corrective release

The 3.1.2 release is a backwards-compatible corrective patch (decision 0064)
addressing two avoidable local/tooling failure modes exposed by an independent
validation pass against public `repopact==3.1.1` (that package itself was
confirmed sound): there was no conventional top-level `repopact --version`
observability surface, and the Workbench type generator
(`rust/apps/repopact-desktop/scripts/generate-types.mjs`) unconditionally
defaulted `CARGO_TARGET_DIR` to a single machine-wide shared temp directory,
implicated in a `tree-sitter` build corruption under concurrent unrelated Rust
builds. `--version` is strictly additive; the `CARGO_TARGET_DIR` default is
removed so Cargo uses the normal, already-`.gitignore`d workspace target
(`rust/target/`) while an explicit override still works. No schema, protocol,
CLI subcommand semantics, lifecycle, or provenance behavior changed. The
`v3.1.0` and `v3.1.1` tags and their already-published wheels/sdists are left
untouched; `3.1.2` is the current stable identity going forward.

Build only from the clean committed release tree with
`repopact release-build --root . --outdir dist`, inspect the wheel and sdist,
and run `python -m twine check dist\\repopact-3.1.2*`. Publish both the wheel
and sdist through the existing secure credential path. Verify public metadata
and hashes, then install `repopact==3.1.2` in a clean environment outside the
checkout and run the package/resource/conformance smoke checks. Create and push
the annotated `v3.1.2` tag and the GitHub release.

After the stable tag and publication, continuing `main` development must
restore `VERSION=3.1.2`, `RELEASE_LABEL=3.1.2-dev.1`, and package metadata
`3.1.2.dev1`. Do not move `v3.1.2` or treat that development identity as a
stable publication.

## Historical 3.1.1 corrective release

The 3.1.1 release is a backwards-compatible corrective patch (decision 0063)
fixing a packaging defect discovered while publishing `v3.1.0`:
`pyproject.toml`'s `[tool.maturin] include` list never added `LICENSE` to the
sdist format, so `repopact-3.1.0.tar.gz` declared a `License-File` in
`PKG-INFO` that the archive did not contain. PyPI's upload-time check rejected
it (HTTP 400); it was never stored. `repopact-3.1.0-py3-none-win_amd64.whl`
uploaded successfully and is live on PyPI — Maturin bundles the license into a
wheel's `.dist-info/licenses/` independently of the `include` list, so wheels
were never affected. The `v3.1.0` tag and its published wheel are left
untouched; `3.1.1` is the current stable identity going forward. No schema,
protocol, CLI, lifecycle, or provenance behavior changed.

Build only from the clean committed release tree with
`repopact release-build --root . --outdir dist`, inspect the wheel and sdist,
and run `python -m twine check dist\\repopact-3.1.1*`. Publish both the wheel
and sdist through the existing secure credential path. Verify public metadata
and hashes, then install `repopact==3.1.1` in a clean environment outside the
checkout and run the package/resource/conformance smoke checks. Create and push
the annotated `v3.1.1` tag and the GitHub release.

## Historical 3.1.0 minor release

The 3.1.0 release is the accepted compatible-minor milestone from decision
0058 and WI066. The live 3.0.2 compatibility audit found no mandatory behavior
break: the adopter schema change is optional, new record families are
additive, existing conformance IDs/rules remain valid, the engine protocol
major is unchanged, and new CLI operations are additive. Keep `VERSION=3.1.0`
and omit `RELEASE_LABEL` on the exact stable release commit. The stable package
includes the canonical Rust/Python engine, local-first verification and release
surfaces, and the additive graph and assurance capabilities. WI063 and WI065
remain active implementation boundaries; the arXiv package remains preparation
only and **ARXIV NOT SUBMITTED**.

Build only from the clean committed release tree with
`repopact release build --root . --outdir dist`, inspect the wheel and sdist,
and run `python -m twine check dist\\repopact-3.1.0*`. Publish only those exact
files through the existing secure credential path. Verify public metadata and
hashes, then install `repopact==3.1.0` in a clean environment outside the
checkout and run the package/resource/conformance smoke checks. Create and push
the annotated `v3.1.0` tag and the GitHub release because that process is
established by prior releases; release notes must preserve the active WI063 and
WI065 boundaries and the **ARXIV NOT SUBMITTED** status.

After the stable tag and publication, continuing `main` development must restore
`VERSION=3.1.0`, `RELEASE_LABEL=3.1.0-dev.1`, and package metadata
`3.1.0.dev1`. Do not move `v3.1.0` or treat that development identity as a
stable publication.

## Historical 3.0.2 corrective release

The historical 3.0.2 release contains the record-relative `source_of_truth:` correction
and structural same-repository linked-worktree contract-discovery correction
from WI043 and WI042. Keep `VERSION=3.0.2` and omit `RELEASE_LABEL` on the
exact stable release commit. Build only from that clean committed tree with
`repopact release-build --root . --outdir dist`, inspect the wheel/sdist, and
run `python -m twine check dist\\repopact-3.0.2*`. Publish only those exact
files, then verify public hashes and a fresh `site-packages` installation.
This local procedure does not restore GitHub-hosted enforcement; WI032 remains
blocked under its temporary local-only directive.

## Post-3.0.2 development baseline (WI049)

The exact stable release tree described above remains unlabeled at `v3.0.2`.
Current `main` is intentionally later than that tag because WI049 reconciles a
post-release validator correction. It therefore keeps `VERSION=3.0.2` and
requires `RELEASE_LABEL=3.0.2-dev.1`; package metadata derives `3.0.2dev1`.
This is a development identity, not a stable release and not a package
publication instruction. Do not move `v3.0.2`, cut 3.0.3, or publish artifacts
from this baseline. WI032 remains local-only/blocked.

## Historical 3.0.1 corrective release

The 3.0.1 release contains the post-3.0.0 `worktrees/` contract-discovery
exclusion and its regression coverage. Keep `VERSION=3.0.1` and omit
`RELEASE_LABEL` on the exact stable release commit. After the `v3.0.1` tag is
published, any later package/runtime source at that same `VERSION` must add a
VERSION-pinned label; package metadata maps that label deterministically to PEP
440 so development artifacts cannot be mistaken for the stable wheel.

From the exact clean release commit, run `repopact release-build --root .
--outdir dist`, inspect the wheel/sdist, and run `python -m twine check
dist\\repopact-3.0.1*`. Publish only those exact files. Verify the public hashes
and an installation from `site-packages` separately. This local procedure does
not restore GitHub-hosted enforcement; WI032 remains blocked under its
temporary local-only directive.

## Current billing-locked Actions fallback

When GitHub Actions cannot execute, the OIDC trusted-publishing path is unavailable.
An operator-authorized direct upload may publish the exact locally validated tag
artifacts without weakening the release gates:

1. Prepare the version, decision, conformance identity, and release narrative;
   regenerate derived artifacts; run governance, unit, conformance, and frozen
   checks; then commit the exact release tree.
2. From that clean commit run
   `repopact release-build --root . --outdir dist`. The release builder exports
   the commit twice, fixes `SOURCE_DATE_EPOCH`, requires byte-identical artifacts,
   and rejects flat root modules, missing package resources, or data-files. Do
   not publish an unchecked `python -m build --wheel` result from a checkout:
   setuptools may retain obsolete files in ignored `build/lib` state.
3. Run `python -m twine check dist/repopact-<version>*` and record exact SHA-256
   hashes.
4. Push the release commit, create and push the annotated version tag, and
   confirm both remote refs resolve to that commit.
5. Run `python -m twine upload <exact-wheel> <exact-sdist>` using an operator-held
   PyPI token. Never write the token, `.pypirc`, or secret-bearing output to evidence.
6. Verify the public PyPI JSON/index metadata and download the public wheel with
   `--no-cache-dir`. Its SHA-256 must equal the locally validated wheel. Install
   it in a clean virtual environment outside the checkout; verify generic flat
   imports are absent, package resources resolve from site-packages, and an
   initialized repository validates.

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
