#!/usr/bin/env python3
"""Validate the fail-closed F1/D0-D3 session-launch review contract."""

from __future__ import annotations

import json
from pathlib import Path
import sys
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "tests/fixtures/session-launch/d0-d3-contract-v1.json"
MAX_EVIDENCE_BYTES = 262_144
EXPECTED_KEYS = {
    "schema", "phase", "status", "activation", "package_identity", "grant",
    "audit", "executable_resolution", "strict_defaults", "authority_ceiling",
    "native_scenarios", "evidence", "external_prerequisites",
}
EXPECTED_NATIVE_PLATFORMS = ["windows", "macos", "linux", "wsl"]
EXPECTED_SCENARIOS = [
    {"id": "direct-alias", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "one-literal-destination-argument"},
    {"id": "explicit-destination", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "explicit-host-connect-without-shell"},
    {"id": "user-and-port", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "typed-user-host-port-without-option-confusion"},
    {"id": "encrypted-key", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "prompt-remains-in-pty-no-secret-capture"},
    {"id": "agent", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "openssh-agent-authoritative-forwarding-off"},
    {"id": "certificate", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "openssh-auth-no-private-material-in-core"},
    {"id": "host-key-new", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "deny-until-explicit-trust"},
    {"id": "host-key-known", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "strict-known-host-connect"},
    {"id": "host-key-changed", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "hard-deny-with-safe-explanation"},
    {"id": "proxy-jump", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "openssh-config-authoritative-no-discovery-exec"},
    {"id": "local-forward", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "disabled-until-confirmed-loopback-default"},
    {"id": "remote-forward", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "disabled-until-separately-confirmed"},
    {"id": "dynamic-forward", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "disabled-until-confirmed-loopback-default"},
    {"id": "cancellation", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "graceful-then-bounded-force-close"},
    {"id": "exit-code", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "exact-result-class-and-redacted-audit"},
    {"id": "hostile-output", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "bounded-parser-no-control-or-secret-leak"},
    {"id": "offline", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "bounded-failure-without-hidden-retry-loop"},
    {"id": "shutdown-cleanup", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "zero-owned-child-pty-listener-after-close"},
    {"id": "one-ten-fifty-sessions", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "zero-leak-and-recorded-performance-budget"},
]
EXPECTED_SOURCES = [
    "apps/automexia-terminal/src/context/launch_broker.rs",
    "apps/automexia-terminal/src/context/mod.rs",
    "automexia-extension-api/src/lib.rs",
]
EXPECTED_CHECKS = [
    "tools/ci/check_session_launch_d0.py",
    "tools/ci/test_session_launch_d0.py",
]
EXPECTED_DOCUMENTS = [
    "docs/adr/0012-first-party-ssh-and-session-launch-boundary.md",
    "docs/SESSION-LAUNCH-BROKER.md",
    "docs/CONNECTIVITY-COMMAND-PRODUCTIVITY-ROADMAP.md",
    "docs/ARCHITECTURE.md",
    "docs/SSH-DEVOPS-MULTICLOUD-ARCHITECTURE.md",
    "docs/TESTING.md",
    "docs/PHASE-IMPLEMENTATION-AUDIT.md",
    "docs/ROADMAP.md",
    "docs/DECISIONS.md",
    "docs/READINESS-AUDIT.md",
    "docs/STABILIZATION-ROADMAP.md",
]


class SessionLaunchD0Error(ValueError):
    pass


def bounded_text(path: Path, maximum: int = MAX_EVIDENCE_BYTES) -> str:
    if path.is_symlink() or not path.is_file():
        raise SessionLaunchD0Error(f"required regular file is missing or linked: {path}")
    if path.stat().st_size > maximum:
        raise SessionLaunchD0Error(f"session-launch evidence exceeds {maximum} bytes: {path}")
    data = path.read_bytes()
    if len(data) > maximum:
        raise SessionLaunchD0Error(f"session-launch evidence grew while reading: {path}")
    return data.decode("utf-8")


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise SessionLaunchD0Error(f"duplicate D0/D3 contract key: {key}")
        result[key] = value
    return result


def load_contract(path: Path = CONTRACT) -> dict[str, Any]:
    document = json.loads(bounded_text(path), object_pairs_hook=reject_duplicate_keys)
    if not isinstance(document, dict) or set(document) != EXPECTED_KEYS:
        raise SessionLaunchD0Error("D0/D3 contract keys changed")
    if (document["schema"], document["phase"], document["status"]) != (
        1, "F1/D0-D3", "local-contract-complete-protected-approval-pending",
    ):
        raise SessionLaunchD0Error("D0/D3 contract identity changed")
    expected_sections = {
        "activation": {"production_enabled": False, "required_adr": "0012", "adr_status": "Proposed", "required_protected_approvals": 2},
        "package_identity": {"extension_id": "automexia.devops-ssh", "publisher": "io.github.AmjedAllaya", "version_source": "workspace-package-version", "digest": "sha256-32-bytes-nonzero", "contract_version": 1, "compatibility": "exact", "allowed_verification": ["repository-reviewed", "first-party-signed"], "unverified_denied": True, "revocation_fail_closed": True},
        "grant": {"capability": "session.launch", "required_bindings": ["extension_id", "operation_id", "session_id", "capsule_revision", "resource", "decision", "decided_at_ms", "expires_at_ms"], "decisions": ["allow-once", "allow-session", "deny"], "persistent_grants": False, "replay_denied": True, "scope_rebind_denied": True, "future_decisions_denied": True, "expired_decisions_denied": True},
        "audit": {"allowed": ["extension_id", "extension_version", "publisher", "decision", "operation_kind", "public_connection_id", "operation_id", "session_id", "timestamp_ms", "duration_ms", "result"], "forbidden": ["argv", "environment_values", "cwd", "terminal_content", "username", "secret_reference", "process_id", "executable_path", "package_digest"]},
        "executable_resolution": {"windows": {"mode": "system-directory", "ssh": "OpenSSH/ssh.exe", "ssh_add": "OpenSSH/ssh-add.exe", "ssh_keygen": "OpenSSH/ssh-keygen.exe"}, "macos": {"roots": ["/usr/bin", "/usr/local/bin", "/opt/homebrew/bin"]}, "linux": {"roots": ["/usr/bin", "/bin", "/usr/local/bin"]}, "wsl": {"production_enabled": False, "future_windows_launcher": "System32/wsl.exe", "future_linux_ssh": "/usr/bin/ssh", "shell": False}, "path_lookup": False, "cwd_lookup": False, "relative_paths": False, "fallback_after_configured_override_failure": False, "identity_revalidated_before_spawn": True},
        "strict_defaults": {"host_key_checking": "strict", "unknown_host": "deny-until-explicit-trust", "changed_host": "deny", "agent_forwarding": False, "tcp_forwarding": False, "forward_listener_scope": "loopback-unless-explicitly-confirmed", "x11_forwarding": False, "remote_command": False, "config_command_execution_during_discovery": False, "environment_inheritance": False, "secret_resolution": False, "shell_interpretation": False},
        "authority_ceiling": {"process": False, "pty": False, "network": False, "provider": False, "authentication": False, "key_custody": False, "renderer": False},
        "external_prerequisites": ["protected security review", "two protected-path approvals", "native process and PTY evidence in later phases"],
    }
    for key, expected in expected_sections.items():
        if document[key] != expected:
            raise SessionLaunchD0Error(f"D0/D3 {key.replace('_', ' ')} changed")
    scenarios = document["native_scenarios"]
    if scenarios != EXPECTED_SCENARIOS:
        raise SessionLaunchD0Error("D0/D3 native scenario matrix changed")
    if document["evidence"] != {"source": EXPECTED_SOURCES, "checks": EXPECTED_CHECKS, "documents": EXPECTED_DOCUMENTS}:
        raise SessionLaunchD0Error("D0/D3 evidence inventory changed")
    return document


def require_tokens(relative: str, tokens: set[str], root: Path = ROOT) -> str:
    source = bounded_text(root / relative)
    missing = sorted(token for token in tokens if token not in source)
    if missing:
        raise SessionLaunchD0Error(f"{relative} is missing D0/D3 evidence: {missing}")
    return source


def validate_sources(document: dict[str, Any], root: Path = ROOT) -> dict[str, int]:
    broker = require_tokens(document["evidence"]["source"][0], {
        "pub const MANAGED_SESSION_LAUNCH_ENABLED: bool = false",
        "const _: () = assert!(!MANAGED_SESSION_LAUNCH_ENABLED)",
        "pub struct PackageDigest", "pub enum PackageVerification",
        "pub struct ReviewedPackagePolicy", "package_policy:",
        "contract_version", "PackageVerification::Unverified",
        "version != env!(\"CARGO_PKG_VERSION\")",
        "contract_version != REVIEWED_CONTRACT_VERSION",
        "ExecutableIdentityChanged", "falls back", "/opt/homebrew/bin",
        "target_os = \"macos\"", "target_os = \"linux\"",
        "one_ten_and_fifty_session_cycles_release_all_bounded_state",
        "package_identity_digest_compatibility_and_verification_fail_closed",
        "platform_resolution_contract_is_fixed_and_wsl_remains_disabled",
    }, root)
    require_tokens(document["evidence"]["source"][1], {"#[cfg(test)]", "pub mod launch_broker;"}, root)
    require_tokens(document["evidence"]["source"][2], {
        "pub struct CapabilityDecision", "pub operation_id: OperationId",
        "pub session_id: SessionId", "pub capsule_revision: u64",
        "pub expires_at_ms: u64",
    }, root)
    audit_start = broker.index("pub struct LaunchAuditRecord")
    audit_end = broker.index("pub struct DeniedLaunch", audit_start)
    audit_block = broker[audit_start:audit_end]
    leaked = sorted(field for field in document["audit"]["forbidden"] if field in audit_block)
    if leaked:
        raise SessionLaunchD0Error(f"audit record exposes forbidden fields: {leaked}")
    for relative in document["evidence"]["documents"]:
        require_tokens(relative, {"D0", "ADR 0012"}, root)
    return {"scenarios": len(document["native_scenarios"]), "sources": len(document["evidence"]["source"]), "documents": len(document["evidence"]["documents"]), "production_enabled": int(document["activation"]["production_enabled"])}


def validate_repository(root: Path = ROOT) -> dict[str, int]:
    document = load_contract(root / CONTRACT.relative_to(ROOT))
    return validate_sources(document, root)


def main() -> int:
    try:
        counts = validate_repository()
    except (SessionLaunchD0Error, OSError, UnicodeError, json.JSONDecodeError) as error:
        print(f"D0/D3 session-launch validation failed: {error}", file=sys.stderr)
        return 1
    print(f"PASS: D0/D3 session-launch contract is exact, fail-closed, and review-gated ({counts})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
