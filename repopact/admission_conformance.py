"""Adopter-neutral WI050 semantic corpus.

These vectors exercise the policy and adapter contract without depending on a
vendor SDK or a particular host.  The normal conformance runner reports this
corpus separately from the legacy repository-shape fixture count.
"""
from __future__ import annotations

from .admission import (
    _NO_LEASE_KINDS, AdmissionDecision, Ed25519Signer, delegation_subset,
    issue_receipt, verify_receipt,
)
from .adapters import AdapterCapabilities


def run_admission_corpus() -> list[tuple[str, bool, str]]:
    results: list[tuple[str, bool, str]] = []

    def record(name: str, passed: bool, detail: str) -> None:
        results.append((name, passed, detail))

    record("WI050-no-lease-taxonomy", _NO_LEASE_KINDS == {"read/orient", "bootstrap-propose", "bootstrap-amend", "approval-request"}, "mutation is not a no-lease operation")
    signer = Ed25519Signer.generate("corpus-key", "corpus-operator")
    request = {"approval_class": "activate", "expires_at": "2099-01-01T00:00:00Z", "revocation_epoch": 0, "nonce": "corpus-nonce", "repository_identity": "r"}
    authority = {"authority_version": "1", "approval_classes": ["activate"], "operators": [{"operator_id": signer.operator_id, "key_id": signer.key_id, "public_key": signer.public_key}]}
    receipt = issue_receipt(request, signer)
    record("WI050-receipt-binding", verify_receipt(request, receipt, authority).allowed, "signed receipt verifies")
    tampered = dict(receipt); tampered["approval_class"] = "frozen"
    record("WI050-receipt-class-tamper", not verify_receipt(request, tampered, authority).allowed, "receipt class mutation is rejected")
    parent = {"lease_id": "p", "repository_identity": "r", "work_item": "050", "principal": "operator", "approval_class": "activate", "profile": "bounded", "mode": "normal", "delegation_ceiling": 2, "scopes": ["src"], "paths": ["src/a.py"], "capabilities": [], "delegation_lineage": [], "expires_at": "2099-01-01T00:00:00Z"}
    child = {**parent, "lease_id": "c", "principal": "child", "parent_lease_id": "p", "delegation_lineage": ["p"], "delegation_ceiling": 1, "expires_at": "2098-01-01T00:00:00Z"}
    record("WI050-delegation-subset", delegation_subset(parent, child).allowed, "strict child authority accepted")
    record("WI050-delegation-widening", not delegation_subset(parent, {**child, "paths": ["src/b.py"]}).allowed, "path widening rejected")
    pre = AdapterCapabilities("corpus-pre", pre_action_interception=True)
    launcher = AdapterCapabilities("corpus-launcher", pre_action_interception=False, session_start_gate=True)
    record("WI050-adapter-pre-action", pre.enforcement_class() == "pre-action", "pre-action family advertised")
    record("WI050-adapter-launcher", launcher.enforcement_class() == "session-start", "independent lower-assurance family reported honestly")
    record("WI050-fail-closed", AdmissionDecision.deny("NO_OPERATOR_PROOF").enforcement == "not-covered", "denial cannot downgrade to advisory")
    return results
