"""Run the WI050 Linux-native Landlock proof against disposable fixtures.

This is an operator harness: it must run as root so it can mint protected
registrations and exercise revocation/service-loss transitions, while every
launcher target runs as the ordinary fixture principal uid 1000.
"""
from __future__ import annotations

import datetime as dt
import hashlib
import json
import os
import subprocess
import sys
import time
import uuid
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from repopact.admission import Ed25519Signer, issue_receipt, make_request
from repopact.guard_ipc import NativeGuardClient

FIX = Path("/home/Jeremy-L/wi050-native-fixtures-20260915")
ROOT = FIX / "repo-c"
ALLOWED = ROOT / "src/allowed"
DENIED = ROOT / "denied"
OTHER = FIX / "repo-b"
STATE = Path("/var/lib/repopact/registrations")
HELPER = Path("/usr/local/lib/repopact/guard/repopact-sandbox")
SIGNER = Ed25519Signer.load(Path("/home/Jeremy-L/.wi050-operator.key"), "wi050-native-proof-passphrase-c")


def drop_to_fixture_user() -> None:
    os.setgroups([1000, 100])
    os.setgid(1000)
    os.setuid(1000)


def make_authorization(*, paths=("src/allowed/**",), scopes=("src",), capabilities=None,
                       approval="activate", expires_at=None):
    req = make_request(
        ROOT, "050", f"native-matrix-{uuid.uuid4()}", principal="agent", profile="bounded",
        scopes=list(scopes), paths=list(paths), capabilities=capabilities or {"process": True, "shell": True},
        approval_class=approval, protected_dir=STATE, expires_at=expires_at,
    )
    return req, issue_receipt(req, SIGNER)


def launch(req, receipt, command, *, cwd=ALLOWED, pass_fds=()):
    args = [str(HELPER), "--root", str(ROOT), "--request-json", json.dumps(req, separators=(",", ":")),
            "--receipt-json", json.dumps(receipt, separators=(",", ":")), "--cwd", str(cwd), "--", *command]
    process = subprocess.Popen(args, cwd=str(cwd), stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                               text=True, preexec_fn=drop_to_fixture_user, pass_fds=pass_fds)
    stdout, stderr = process.communicate(timeout=20)
    return {"returncode": process.returncode, "stdout": stdout.strip(), "stderr": stderr.strip()}


def hash_file(path: Path):
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.is_file() else None


def case(name, expected, result, **checks):
    return {"case": name, "expected": expected, "result": result, "checks": checks,
            "pass": all(checks.values())}


def main() -> None:
    results = []
    positives = {
        "direct_python": (["/usr/bin/python3.13", "-c", "from pathlib import Path; Path('positive-python.txt').write_text('ok')"], ALLOWED / "positive-python.txt"),
        "posix_shell": (["/bin/sh", "-c", "printf ok > positive-shell.txt"], ALLOWED / "positive-shell.txt"),
        "child_grandchild": (["/usr/bin/python3.13", "-c", "import subprocess; from pathlib import Path; Path('positive-child.txt').write_text('child'); subprocess.run(['/usr/bin/python3.13','-c','from pathlib import Path; Path(\\\"positive-grandchild.txt\\\").write_text(\\\"grandchild\\\")'], check=True)"], ALLOWED / "positive-grandchild.txt"),
        "touch": (["/usr/bin/touch", "positive-touch.txt"], ALLOWED / "positive-touch.txt"),
    }
    for name, (command, target) in positives.items():
        target.unlink(missing_ok=True)
        result = launch(*make_authorization(), command)
        results.append(case(name, "success", result, launcher_success=result["returncode"] == 0, target_created=target.is_file()))

    sentinel = DENIED / "sentinel.txt"
    sentinel_hash = hash_file(sentinel)
    negatives = {
        "python_write": ["/usr/bin/python3.13", "-c", f"from pathlib import Path; Path('{sentinel}').write_text('bad')"],
        "shell_redirect": ["/bin/sh", "-c", f"printf bad > '{DENIED}/shell.txt'"],
        "touch": ["/usr/bin/touch", str(DENIED / "touch.txt")],
        "cp": ["/bin/cp", str(ALLOWED / "positive-python.txt"), str(DENIED / "cp.txt")],
        "mv": ["/bin/mv", str(ALLOWED / "positive-python.txt"), str(DENIED / "mv.txt")],
        "unlink": ["/usr/bin/python3.13", "-c", f"from pathlib import Path; Path('{sentinel}').unlink()"],
        "rename": ["/usr/bin/python3.13", "-c", f"import os; os.rename('{ALLOWED}/positive-shell.txt','{DENIED}/rename.txt')"],
        "hard_link": ["/usr/bin/python3.13", "-c", f"import os; os.link('{ALLOWED}/positive-shell.txt','{DENIED}/link.txt')"],
        "symlink_escape": ["/usr/bin/python3.13", "-c", f"from pathlib import Path; Path('{ALLOWED}/escape/via-symlink.txt').write_text('bad')"],
        "second_repository": ["/usr/bin/python3.13", "-c", f"from pathlib import Path; Path('{OTHER}/outside.txt').write_text('bad')"],
        "guard_state": ["/usr/bin/python3.13", "-c", "from pathlib import Path; Path('/var/lib/repopact/registrations/escape.txt').write_text('bad')"],
    }
    for name, command in negatives.items():
        before = hash_file(sentinel)
        result = launch(*make_authorization(), command)
        results.append(case(name, "OS denial and unchanged sentinel", result,
                            child_failed=result["returncode"] != 0, sentinel_unchanged=hash_file(sentinel) == before == sentinel_hash,
                            named_target_absent=not (DENIED / f"{name}.txt").exists()))

    nested = ALLOWED / "nested"
    result = launch(*make_authorization(), ["/usr/bin/python3.13", "-c", f"from pathlib import Path; Path('{DENIED}/nested-cwd.txt').write_text('bad')"], cwd=nested)
    results.append(case("nested_working_directory_escape", "OS denial", result,
                        child_failed=result["returncode"] != 0, target_absent=not (DENIED / "nested-cwd.txt").exists()))

    link = FIX / "repo-c-linked"
    git = ["git", f"-c", f"safe.directory={ROOT}", "-C", str(ROOT)]
    subprocess.run(git + ["worktree", "add", "--detach", str(link)], check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    try:
        result = launch(*make_authorization(), ["/usr/bin/python3.13", "-c", f"from pathlib import Path; Path('{link}/linked-escape.txt').write_text('bad')"], cwd=link)
        results.append(case("linked_worktree_escape", "OS denial", result,
                            child_failed=result["returncode"] != 0, target_absent=not (link / "linked-escape.txt").exists()))
    finally:
        subprocess.run(git + ["worktree", "remove", "--force", str(link)], check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)

    req, rec = make_authorization()
    result = launch({**req, "paths": ["src/allowed/**", "denied/**"]}, rec, ["/usr/bin/touch", "positive-widened.txt"])
    results.append(case("caller_widening_signed_digest", "guard denial", result, not_started=result["returncode"] == 125))
    req, rec = make_authorization()
    result = launch({**req, "repopact_root": str(OTHER)}, rec, ["/usr/bin/touch", "positive-identity.txt"])
    results.append(case("caller_repository_substitution", "guard denial", result, not_started=result["returncode"] == 125))
    req, rec = make_authorization(paths=("governance/invariants.json",), scopes=("governance",),
                                  capabilities={"process": True, "shell": True, "frozen_surface": True})
    result = launch(req, rec, ["/usr/bin/touch", "positive-frozen.txt"])
    results.append(case("frozen_without_approval", "guard denial", result, not_started=result["returncode"] == 125))

    fd = os.open(sentinel, os.O_WRONLY | os.O_APPEND)
    try:
        req, rec = make_authorization()
        result = launch(req, rec, ["/usr/bin/python3.13", "-c", "import os; os.write(3, b'bad')"], pass_fds=(fd,))
    finally:
        os.close(fd)
    results.append(case("inherited_writable_fd", "descriptor closed before target", result,
                        target_cannot_use_fd=result["returncode"] != 0, sentinel_unchanged=hash_file(sentinel) == sentinel_hash))

    start, finish = ALLOWED / "expiry-started.txt", ALLOWED / "expiry-finished.txt"
    start.unlink(missing_ok=True); finish.unlink(missing_ok=True)
    req, rec = make_authorization(expires_at=dt.datetime.now(dt.timezone.utc) + dt.timedelta(seconds=2))
    result = launch(req, rec, ["/usr/bin/python3.13", "-c", "import time; from pathlib import Path; Path('expiry-started.txt').write_text('started'); time.sleep(5); Path('expiry-finished.txt').write_text('finished')"])
    results.append(case("lease_expiry", "supervised termination", result, started=start.is_file(), finished_absent=not finish.exists(), launcher_failed=result["returncode"] == 125))

    start, finish = ALLOWED / "revoke-started.txt", ALLOWED / "revoke-finished.txt"
    start.unlink(missing_ok=True); finish.unlink(missing_ok=True)
    req, rec = make_authorization()
    args = [str(HELPER), "--root", str(ROOT), "--request-json", json.dumps(req, separators=(",", ":")), "--receipt-json", json.dumps(rec, separators=(",", ":")), "--cwd", str(ALLOWED), "--", "/usr/bin/python3.13", "-c", "import time; from pathlib import Path; Path('revoke-started.txt').write_text('started'); time.sleep(5); Path('revoke-finished.txt').write_text('finished')"]
    process = subprocess.Popen(args, cwd=str(ALLOWED), stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, preexec_fn=drop_to_fixture_user)
    deadline = time.time() + 5
    while not start.exists() and time.time() < deadline:
        time.sleep(0.05)
    revoke_req = make_request(ROOT, "000", "operator-revocation", principal=SIGNER.operator_id, profile="observe", scopes=(), paths=(), capabilities={}, approval_class="revoke", mode="normal", protected_dir=STATE)
    revoke_rec = issue_receipt(revoke_req, SIGNER, "revoke")
    revoke_response = NativeGuardClient()._call("revoke", {"root": str(ROOT), "request": revoke_req, "receipt": revoke_rec})
    stdout, stderr = process.communicate(timeout=20)
    result = {"returncode": process.returncode, "stdout": stdout.strip(), "stderr": stderr.strip(), "revoke_response": revoke_response}
    results.append(case("operator_revocation", "supervised termination", result, started=start.is_file(), finished_absent=not finish.exists(), revocation_applied=isinstance(revoke_response.get("revocation_epoch"), int), launcher_failed=process.returncode == 125))

    start, finish = ALLOWED / "loss-started.txt", ALLOWED / "loss-finished.txt"
    start.unlink(missing_ok=True); finish.unlink(missing_ok=True)
    req, rec = make_authorization()
    args = [str(HELPER), "--root", str(ROOT), "--request-json", json.dumps(req, separators=(",", ":")), "--receipt-json", json.dumps(rec, separators=(",", ":")), "--cwd", str(ALLOWED), "--", "/usr/bin/python3.13", "-c", "import time; from pathlib import Path; Path('loss-started.txt').write_text('started'); time.sleep(5); Path('loss-finished.txt').write_text('finished')"]
    process = subprocess.Popen(args, cwd=str(ALLOWED), stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, preexec_fn=drop_to_fixture_user)
    deadline = time.time() + 5
    while not start.exists() and time.time() < deadline:
        time.sleep(0.05)
    subprocess.run(["systemctl", "stop", "repopact-guard.service"], check=True)
    stdout, stderr = process.communicate(timeout=20)
    subprocess.run(["systemctl", "start", "repopact-guard.service"], check=True)
    result = {"returncode": process.returncode, "stdout": stdout.strip(), "stderr": stderr.strip()}
    results.append(case("guard_loss", "supervised termination", result, started=start.is_file(), finished_absent=not finish.exists(), launcher_failed=process.returncode == 125))

    print(json.dumps({"helper": str(HELPER), "passed": sum(item["pass"] for item in results), "total": len(results), "results": results}, sort_keys=True, indent=2))


if __name__ == "__main__":
    if os.geteuid() != 0:
        raise SystemExit("run this operator harness as root")
    main()
