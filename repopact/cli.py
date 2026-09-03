"""The `repopact` console entry point (work item 005, issue #2).

A thin dispatcher over the existing tooling so adopters can run `repopact init`
rather than invoking modules directly. Every subcommand except `init` operates
on the current working directory (or `--root`), so the installed command works
against the user's repository rather than the install location.
"""

from __future__ import annotations

import argparse
import json
import sys
from datetime import date
from pathlib import Path
from typing import Any

import jsonschema

from .repo_model import STATUSES


def _operator_signer(admission: Any, key_file: Path | None, root: Path):
    """Load or create an external encrypted signer only with user presence."""
    if key_file is None:
        raise admission.SignerError("--key-file is required to establish operator trust")
    key_file = key_file.resolve()
    if key_file.is_relative_to(root):
        raise admission.SignerError("private operator keys must be stored outside the repository")
    if not sys.stdin.isatty():
        raise admission.SignerError("operator key setup requires an interactive terminal")
    from getpass import getpass
    phrase = getpass("Operator key passphrase: ")
    if key_file.exists():
        return admission.Ed25519Signer.load(key_file, phrase)
    signer = admission.Ed25519Signer.generate()
    signer.save(key_file, phrase)
    return signer


def _write_canonical_record(root: Path, directory: str, name: str, payload: bytes) -> Path:
    """Write an admission record without following a repository symlink."""
    target = root / directory / name
    current = root
    for component in Path(directory).parts:
        current = current / component
        if current.is_symlink():
            raise RuntimeError("canonical admission directory contains a symlink")
    target.parent.mkdir(parents=True, exist_ok=True)
    if target.is_symlink() or target.exists():
        raise RuntimeError("canonical admission record already exists")
    with target.open("xb") as handle:
        handle.write(payload)
    return target


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="repopact", description="RepoPact: durable agent work, governed in the repo.")
    sub = parser.add_subparsers(dest="command", required=True)

    p_init = sub.add_parser("init", help="Bootstrap a new RepoPact repository")
    p_init.add_argument("--target", type=Path, required=True)
    p_init.add_argument("--admission", action="store_true", help="Opt in to protected pre-execution admission")
    p_init.add_argument("--key-file", type=Path, help="External encrypted operator key (required with --admission)")

    p_adopt = sub.add_parser("adopt", help="Adopt RepoPact into an existing repository")
    p_adopt.add_argument("--target", type=Path, required=True)
    p_adopt.add_argument("--dry-run", action="store_true", help="Report the plan without writing files")
    p_adopt.add_argument("--admission", action="store_true", help="Opt in to protected pre-execution admission")
    p_adopt.add_argument("--key-file", type=Path, help="External encrypted operator key (required with --admission)")

    p_imp = sub.add_parser("import-plan", help="Import existing plan items (todos, checklists, issues) into work/")
    p_imp.add_argument("--root", type=Path, default=Path.cwd())
    p_imp.add_argument("--dry-run", action="store_true", help="Report the plan without writing files")
    p_imp.add_argument("--issues", action="store_true", help="Also import GitHub issues (needs gh + a GitHub remote)")

    p_doc = sub.add_parser("doctor", help="Diagnose (and optionally --fix) RepoPact drift")
    p_doc.add_argument("--root", type=Path, default=Path.cwd())
    p_doc.add_argument("--fix", action="store_true", help="Apply safe, non-destructive repairs")

    p_take = sub.add_parser("takeover", help="Retire legacy planning sources RepoPact has fully imported")
    p_take.add_argument("--root", type=Path, default=Path.cwd())
    p_take.add_argument("--delete", action="store_true",
                        help="Delete instead of archiving (git-guarded; writes a decisions/ ADR; "
                             "downgrades to archive if not recoverable from git)")
    p_take.add_argument("--dry-run", action="store_true", help="Report the plan without changing files")

    p_val = sub.add_parser("validate", help="Validate the repository")
    p_val.add_argument("--root", type=Path, default=Path.cwd())

    p_dash = sub.add_parser("dashboard", help="Regenerate the dashboard")
    p_dash.add_argument("--root", type=Path, default=Path.cwd())

    p_spec = sub.add_parser("spec", help="Regenerate SPEC.md derived blocks")
    p_spec.add_argument("--root", type=Path, default=Path.cwd())

    p_new = sub.add_parser("new", help="Stamp a new record from a template")
    p_new.add_argument("kind", choices=["work-item", "decision", "policy"])
    p_new.add_argument("title")
    p_new.add_argument("--root", type=Path, default=Path.cwd())
    p_new.add_argument("--status", choices=STATUSES, default="active",
                       help="Lifecycle status for a new work item")

    p_frz = sub.add_parser("check-frozen", help="Report frozen-surface changes in a diff range")
    p_frz.add_argument("--root", type=Path, default=Path.cwd())
    p_frz.add_argument("--base", default="origin/main")
    p_frz.add_argument("--ack", action="store_true")

    p_fleet = sub.add_parser("fleet-verify", help="Verify every declared public adopter")
    p_fleet.add_argument("--root", type=Path, default=Path.cwd())
    p_fleet.add_argument("--manifest", type=Path)
    p_fleet.add_argument("--discover-root", type=Path, action="append")
    p_fleet.add_argument("--json", action="store_true", help="Emit deterministic JSON")

    p_close = sub.add_parser("release-closeout", help="Gate release closeout on publication and adopter rollout")
    p_close.add_argument("--root", type=Path, default=Path.cwd())
    p_close.add_argument("--manifest", type=Path)
    p_close.add_argument("--discover-root", type=Path, action="append")
    p_close.add_argument("--package-evidence", type=Path)
    p_close.add_argument("--json", action="store_true", help="Emit deterministic JSON")

    p_release = sub.add_parser(
        "release-build",
        help="Build reproducible, structurally verified artifacts from a clean committed tree",
    )
    p_release.add_argument("--root", type=Path, default=Path.cwd())
    p_release.add_argument("--outdir", type=Path, required=True)
    p_release.add_argument("--revision", default="HEAD")

    p_adm = sub.add_parser("admission", help="Manage opt-in pre-execution admission")
    adm_sub = p_adm.add_subparsers(dest="admission_command", required=True)
    p_setup = adm_sub.add_parser("setup", help="Explicitly register this repository")
    p_setup.add_argument("--root", type=Path, default=Path.cwd())
    p_setup.add_argument("--protected-dir", type=Path)
    p_setup.add_argument("--key-file", type=Path, help="External encrypted operator key file")
    p_status = adm_sub.add_parser("status", help="Show safe admission health")
    p_status.add_argument("--root", type=Path, default=Path.cwd())
    p_status.add_argument("--protected-dir", type=Path)
    p_begin = adm_sub.add_parser("begin", help="Create a canonical authorization request")
    p_begin.add_argument("--root", type=Path, default=Path.cwd()); p_begin.add_argument("--work-item", required=True); p_begin.add_argument("--session", required=True); p_begin.add_argument("--profile", default="bounded"); p_begin.add_argument("--protected-dir", type=Path)
    p_revoke = adm_sub.add_parser("revoke", help="Operator-controlled revocation transition")
    p_revoke.add_argument("--root", type=Path, default=Path.cwd()); p_revoke.add_argument("--protected-dir", type=Path); p_revoke.add_argument("--key-file", type=Path)

    p_guard = sub.add_parser("guard", help="Install and inspect the host-protected RepoPact guard")
    guard_sub = p_guard.add_subparsers(dest="guard_command", required=True)
    p_guard_install = guard_sub.add_parser("install", help="Install the protected guard (operator elevation required)")
    p_guard_install.add_argument("--root", type=Path, default=Path.cwd())
    p_guard_install.add_argument("--preflight", action="store_true", help="Run non-mutating installation diagnostics only")
    p_guard_status = guard_sub.add_parser("status", help="Report backend-owned guard attestation")
    p_guard_status.add_argument("--root", type=Path, default=Path.cwd())
    p_guard_status.add_argument("--json", action="store_true")
    p_guard_register = guard_sub.add_parser("register", help="Bind an adopted repository to the installed guard")
    p_guard_register.add_argument("--root", type=Path, default=Path.cwd())
    p_guard_register.add_argument("--key-file", type=Path, required=True)
    p_guard_register.add_argument("--protected-dir", type=Path)
    p_guard_uninstall = guard_sub.add_parser("uninstall", help="Remove the installed guard service (operator elevation required)")
    p_guard_uninstall.add_argument("--root", type=Path, default=Path.cwd())

    p_work = sub.add_parser("work", help="Bounded bootstrap work operations")
    work_sub = p_work.add_subparsers(dest="work_command", required=True)
    p_prop = work_sub.add_parser("propose", help="Create proposed work without implementation authority")
    p_prop.add_argument("title"); p_prop.add_argument("--root", type=Path, default=Path.cwd())
    p_amend = work_sub.add_parser("amend-proposal", help="Change only the title of proposed work")
    p_amend.add_argument("work_item"); p_amend.add_argument("title"); p_amend.add_argument("--root", type=Path, default=Path.cwd())

    p_appr = sub.add_parser("approval", help="Operator approval receipt operations")
    appr_sub = p_appr.add_subparsers(dest="approval_command", required=True)
    p_req = appr_sub.add_parser("request", help="Create and save an authorization request")
    p_req.add_argument("--root", type=Path, default=Path.cwd()); p_req.add_argument("--work-item", required=True); p_req.add_argument("--session", required=True); p_req.add_argument("--output", type=Path); p_req.add_argument("--protected-dir", type=Path)
    p_approve = appr_sub.add_parser("approve", help="Interactively sign a request")
    p_approve.add_argument("--request", type=Path, required=True); p_approve.add_argument("--key-file", type=Path, required=True); p_approve.add_argument("--output", type=Path); p_approve.add_argument("--protected-dir", type=Path)
    p_pending = appr_sub.add_parser("pending", help="List safe pending request metadata")
    p_pending.add_argument("--root", type=Path, default=Path.cwd())
    p_show = appr_sub.add_parser("show", help="Show a request or receipt record")
    p_show.add_argument("record", type=Path)
    p_deny = appr_sub.add_parser("deny", help="Decline a request without minting a receipt")
    p_deny.add_argument("record", type=Path)

    args = parser.parse_args(argv)

    if args.command == "init":
        from . import init_repo
        target = args.target.resolve()
        if args.admission and args.key_file is None:
            print("init --admission requires an external --key-file and interactive operator presence", file=sys.stderr)
            return 1
        init_repo.bootstrap(target)
        if args.admission:
            from . import admission
            try: admission.setup_admission(target, signer=_operator_signer(admission, args.key_file, target))
            except Exception as exc: print(f"Admission setup failed: {exc}", file=sys.stderr); return 1
        from . import validate_repo
        problems = validate_repo.validate(target)
        if problems:
            for p in problems:
                print(f"ERROR {p.path.relative_to(target)}: {p.message}")
            print(f"\nBootstrap produced an invalid repository: {len(problems)} error(s).")
            return 1
        print(f"Bootstrapped a valid RepoPact at {target}")
        return 0

    if args.command == "adopt":
        from . import adopt_repo
        target = args.target.resolve()
        if args.admission and args.key_file is None:
            print("adopt --admission requires an external --key-file and interactive operator presence", file=sys.stderr)
            return 1
        rep = adopt_repo.adopt(target, dry_run=args.dry_run)
        adopt_repo._print_report(rep)
        if args.dry_run:
            print("\nDry run: nothing written. Re-run without --dry-run to apply.")
            return 0
        if args.admission:
            from . import admission
            try: admission.setup_admission(target, signer=_operator_signer(admission, args.key_file, target))
            except Exception as exc: print(f"Admission setup failed: {exc}", file=sys.stderr); return 1
        from . import validate_repo
        problems = validate_repo.validate(target)
        if problems:
            for p in problems:
                print(f"ERROR {p.path.relative_to(target)}: {p.message}")
            print(f"\nAdoption produced {len(problems)} validation error(s) to resolve.")
            return 1
        print("\nAdopted repository validates as a conformant RepoPact.")
        return 0

    if args.command == "guard":
        from .platform_backends import current_backend
        root = args.root.resolve()
        backend = current_backend(root)
        try:
            if args.guard_command == "status":
                payload = backend.health()
                print(json.dumps(payload, sort_keys=True) if args.json else json.dumps(payload, indent=2, sort_keys=True))
                return 0 if payload.get("healthy") else 1
            if args.guard_command == "install":
                result = backend.install(root, preflight=args.preflight)
                print(json.dumps(result, sort_keys=True)); return 0
            if args.guard_command == "uninstall":
                result = backend.uninstall(root=root)
                print(json.dumps(result, sort_keys=True)); return 0
            if args.guard_command == "register":
                from . import admission
                signer = _operator_signer(admission, args.key_file, root)
                if args.protected_dir is not None:
                    # A production backend owns its protected state location;
                    # accepting an arbitrary caller path would reintroduce the
                    # trust-boundary bug this command exists to prevent.
                    print("guard register ignores caller-selected protected state; use the installed backend location", file=sys.stderr)
                    return 1
                result = backend.register(root, signer=signer)
                print(json.dumps(result, sort_keys=True)); return 0
        except Exception as exc:
            print(f"Guard operation failed: {exc}", file=sys.stderr); return 1

    if args.command in {"admission", "work", "approval"}:
        from . import admission
        if args.command == "admission":
            root = args.root.resolve(); protected = args.protected_dir.resolve() if args.protected_dir else None
            if args.admission_command == "setup":
                try: signer = _operator_signer(admission, args.key_file, root); result = admission.setup_admission(root, protected, signer)
                except Exception as exc: print(f"Admission setup failed: {exc}", file=sys.stderr); return 1
                print(json.dumps({k: str(v) for k, v in result.items() if k != "signer"}, sort_keys=True)); return 0
            if args.admission_command == "status":
                print(json.dumps(admission.diagnose(root, protected), sort_keys=True)); return 0 if admission.verify_registration(root, protected).allowed else 1
            if args.admission_command == "begin":
                try: print(json.dumps(admission.make_request(root, args.work_item, args.session, profile=args.profile, protected_dir=protected), sort_keys=True)); return 0
                except Exception as exc: print(f"Admission request denied: {exc}", file=sys.stderr); return 1
            if args.admission_command == "revoke":
                try: print(json.dumps({"revocation_epoch": admission.operator_revoke(root, _operator_signer(admission, args.key_file, root), protected)})); return 0
                except Exception as exc: print(f"Revocation failed: {exc}", file=sys.stderr); return 1
        if args.command == "work":
            root = args.root.resolve()
            from . import new
            if args.work_command == "propose":
                path = new.new_work_item(args.title, date.today(), root, status="proposed"); print(f"Created proposed {path.relative_to(root)}"); return 0
            path = next(root.glob(f"work/*/{args.work_item}*/work-item.json"), None)
            if path is None: print("Unknown work item", file=sys.stderr); return 1
            if path.is_symlink() or not path.resolve().is_relative_to(root):
                print("Proposal path is outside the repository", file=sys.stderr); return 1
            data = json.loads(path.read_text(encoding="utf-8"));
            if data.get("status") != "proposed": print("Only proposed work may be amended", file=sys.stderr); return 1
            data["title"] = args.title; data["updated"] = date.today().isoformat(); path.write_bytes(admission.canonical_json(data) + b"\n"); print(f"Updated {path.relative_to(root)}"); return 0
        if args.command == "approval":
            if args.approval_command == "pending":
                root = args.root.resolve(); req_dir = root / "evidence" / "admission" / "requests"
                rows = []
                for path in sorted(req_dir.glob("*.json")) if req_dir.is_dir() else []:
                    try:
                        rec = json.loads(path.read_text(encoding="utf-8")); rows.append({"request_id": rec.get("request_id"), "work_item": rec.get("work_item"), "profile": rec.get("profile"), "path": str(path.relative_to(root))})
                    except Exception: continue
                print(json.dumps(rows, sort_keys=True)); return 0
            if args.approval_command == "show":
                try: print(json.dumps(json.loads(args.record.read_text(encoding="utf-8")), sort_keys=True)); return 0
                except Exception as exc: print(f"Cannot read record: {exc}", file=sys.stderr); return 1
            if args.approval_command == "deny":
                if not args.record.is_file(): print("Unknown request", file=sys.stderr); return 1
                print(json.dumps({"allowed": False, "code": "NO_OPERATOR_PROOF", "request": str(args.record)})); return 0
            if args.approval_command == "request":
                root = args.root.resolve(); req = admission.make_request(root, args.work_item, args.session, protected_dir=args.protected_dir.resolve() if args.protected_dir else None)
                canonical = root / "evidence" / "admission" / "requests" / f"{req['request_id']}.json"
                if args.output is not None and args.output.resolve() != canonical.resolve():
                    print("approval requests may only be written to the canonical evidence/admission/requests directory", file=sys.stderr); return 1
                try: canonical = _write_canonical_record(root, "evidence/admission/requests", f"{req['request_id']}.json", admission.canonical_json(req) + b"\n")
                except Exception as exc: print(f"Cannot write canonical request: {exc}", file=sys.stderr); return 1
                print(f"Wrote {canonical}"); return 0
            if not sys.stdin.isatty(): print("Operator approval requires an interactive terminal", file=sys.stderr); return 1
            from getpass import getpass
            try:
                req = json.loads(args.request.read_text(encoding="utf-8")); root = Path(req["repopact_root"]).resolve()
                expected_request = root / "evidence" / "admission" / "requests" / f"{req['request_id']}.json"
                if args.request.resolve() != expected_request or not admission.verify_registration(root, args.protected_dir.resolve() if args.protected_dir else None).allowed:
                    raise RuntimeError("operator may only approve a registered repository's canonical request")
                if args.key_file.resolve().is_relative_to(root):
                    raise RuntimeError("private operator keys must be stored outside the repository")
                signer = admission.Ed25519Signer.load(args.key_file, getpass("Operator key passphrase: ")); receipt = admission.issue_receipt(req, signer)
                canonical = root / "evidence" / "admission" / "receipts" / f"{receipt['request_digest']}.json"
                if args.output is not None and args.output.resolve() != canonical.resolve():
                    print("approval receipts may only be written to the canonical evidence/admission/receipts directory", file=sys.stderr); return 1
                try: canonical = _write_canonical_record(root, "evidence/admission/receipts", f"{receipt['request_digest']}.json", admission.canonical_json(receipt) + b"\n")
                except Exception as exc: print(f"Cannot write canonical receipt: {exc}", file=sys.stderr); return 1
                print(f"Wrote {canonical}"); return 0
            except Exception as exc: print(f"Approval failed: {exc}", file=sys.stderr); return 1

    root = args.root.resolve()

    if args.command == "release-build":
        from . import release_build
        try:
            report = release_build.build_release(
                root,
                args.outdir,
                revision=args.revision,
            )
        except release_build.ReleaseBuildError as exc:
            print(f"Release build failed: {exc}", file=sys.stderr)
            return 1
        print(release_build.render_json(report), end="")
        return 0

    if args.command in {"fleet-verify", "release-closeout"}:
        from . import fleet_verify
        try:
            report = fleet_verify.verify_fleet(
                root,
                manifest_path=args.manifest,
                discovery_roots=args.discover_root,
            )
            if args.command == "fleet-verify":
                print(
                    fleet_verify.render_json(report) if args.json else fleet_verify.render_fleet(report),
                    end="" if args.json else "\n",
                )
                return 0 if report.ok else 1
            closeout = fleet_verify.release_closeout(root, report, args.package_evidence)
            print(
                fleet_verify.render_json(closeout) if args.json else fleet_verify.render_closeout(closeout),
                end="" if args.json else "\n",
            )
            return 0 if closeout["status"] == "pass" else 1
        except (OSError, ValueError, jsonschema.ValidationError) as exc:
            print(f"Fleet verification configuration error: {exc}", file=sys.stderr)
            return 2

    if args.command == "doctor":
        from . import doctor
        from . import validate_repo
        if args.fix:
            for a in (doctor.fix(root) or ["nothing to fix"]):
                print(f"  ~ {a}")
        findings = doctor.diagnose(root)
        problems = validate_repo.validate(root)
        errs = [f for f in findings if f.severity == "error"]
        warns = [f for f in findings if f.severity == "warn"]
        for f in errs + warns:
            print(f"{f.severity.upper():5} [{f.code}] {f.message}")
        for p in problems:
            print(f"INVALID {p.path.relative_to(root)}: {p.message}")
        if not errs and not warns and not problems:
            print("repopact doctor: healthy.")
            return 0
        return 1 if errs or problems else 0

    if args.command == "takeover":
        from . import takeover
        report = takeover.takeover(root, delete=args.delete, dry_run=args.dry_run)
        rc = takeover._print(report, args.dry_run)
        if args.dry_run:
            print("\nDry run: nothing changed.")
        return rc

    if args.command == "import-plan":
        from . import plan_import
        rep = plan_import.import_plan(root, dry_run=args.dry_run, import_issues=args.issues)
        plan_import._print(rep)
        if args.dry_run:
            print("\nDry run: nothing written.")
            return 0
        from . import validate_repo
        problems = validate_repo.validate(root)
        for p in problems:
            print(f"ERROR {p.path.relative_to(root)}: {p.message}")
        print("\nwork/ ledger imported; repository validates." if not problems
              else f"\nImport produced {len(problems)} validation error(s).")
        return 1 if problems else 0

    if args.command == "validate":
        from . import validate_repo
        problems = validate_repo.validate(root)
        for p in problems:
            print(f"ERROR {p.path.relative_to(root)}: {p.message}")
        print("Repository governance validation passed." if not problems
              else f"\nValidation failed with {len(problems)} error(s).")
        return 1 if problems else 0

    if args.command == "dashboard":
        from . import generate_dashboard
        out = generate_dashboard.write_dashboard(root)
        print(f"Generated {out.relative_to(root)}")
        return 0

    if args.command == "spec":
        from . import generate_spec
        spec = root / "SPEC.md"
        if not spec.is_file():
            print(
                f"No SPEC.md at {root}. `spec` regenerates the derived blocks of an "
                "existing SPEC.md (it is a maintainer command for repositories that "
                "publish a RepoPact specification); an adopter repository does not "
                "need one.",
                file=sys.stderr,
            )
            return 1
        spec.write_text(generate_spec.render(spec.read_text(encoding="utf-8"), root), encoding="utf-8")
        print("Generated SPEC.md derived blocks")
        return 0

    if args.command == "new":
        from . import new
        if args.kind == "work-item":
            if args.status != "proposed" and (root / "governance" / "admission-policy.json").is_file():
                print("WI050 admission requires operator-controlled activation; create proposed work first.", file=sys.stderr)
                return 1
            path = new.new_work_item(args.title, date.today(), root, status=args.status)
        else:
            path = new.new_markdown(args.kind, args.title, date.today(), root)
        print(f"Created {path.relative_to(root)}")
        return 0

    if args.command == "check-frozen":
        from . import check_frozen_surface
        hits = check_frozen_surface.violations(root, args.base)
        if not hits:
            print("No frozen-surface changes detected.")
            return 0
        print("Frozen-surface changes detected (INV-6 requires operator approval):")
        for name, reason in hits:
            print(f"  {name}: {reason}")
        if args.ack and (args.root.resolve() / "governance" / "admission-policy.json").is_file():
            print("--ack is advisory only while WI050 admission is enabled; provide a protected operator receipt.", file=sys.stderr)
            return 1
        return 0 if args.ack else 1

    return 2


if __name__ == "__main__":
    sys.exit(main())
