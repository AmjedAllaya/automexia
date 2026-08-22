#!/usr/bin/env python3
"""Validate the fail-closed D0-D3/M3-M4 session-launch review contract."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import sys
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "tests/fixtures/session-launch/d0-d3-contract-v4.json"
HISTORICAL_CONTRACTS = [
    (ROOT / "tests/fixtures/session-launch/d0-d3-contract-v1.json", "0d2120bd9aef13b3d75053d595356c9b844107bc9ddc98db60f01f0c779bd9d8", "schema-1"),
    (ROOT / "tests/fixtures/session-launch/d0-d3-contract-v2.json", "eb561716075bd546268b1a3c4fdb2d3c3dceaafd5ca2f9fd7113f69c29d9ba4e", "schema-2"),
    (ROOT / "tests/fixtures/session-launch/d0-d3-contract-v3.json", "112209b674263a6a996a119b9e9120175f0d41aed082463a3e17d1bf806c3ed1", "schema-3"),
]
MAX_EVIDENCE_BYTES = 262_144
EXPECTED_KEYS = {
    "schema", "phase", "status", "activation", "package_identity", "grant",
    "audit", "executable_resolution", "strict_defaults", "authority_ceiling",
    "manual_baseline", "trust_boundaries", "native_fixture_protocol",
    "native_scenarios", "evidence", "external_prerequisites", "managed_session",
    "ssh_routes_trust",
}
EXPECTED_NATIVE_PLATFORMS = ["windows", "macos", "linux", "wsl"]
EXPECTED_MANAGED_OPTIONS = [
    "-oAddKeysToAgent=no", "-oClearAllForwardings=yes", "-oControlMaster=no",
    "-oControlPath=none", "-oControlPersist=no", "-oEnableEscapeCommandline=no",
    "-oForkAfterAuthentication=no", "-oForwardAgent=no", "-oForwardX11=no",
    "-oGSSAPIDelegateCredentials=no", "-oPermitLocalCommand=no",
    "-oProxyCommand=none", "-oProxyJump=none", "-oRemoteCommand=none",
    "-oStdinNull=no", "-oStrictHostKeyChecking=ask", "-oTunnel=no",
]
EXPECTED_ROUTED_OPTIONS = [
    option for option in EXPECTED_MANAGED_OPTIONS
    if option not in {"-oProxyCommand=none", "-oProxyJump=none"}
]
EXPECTED_SSH_ROUTES_TRUST = {
    "route_grammar": {
        "typed_fields": ["host", "user", "port"],
        "direct_shape": "managed-options-then-optional-l-user-optional-p-port-one-literal-host",
        "routed_shape": "routed-managed-options-then-J-one-canonical-chain-one-config-alias",
        "proxy_jump_source": "openssh-config-static-public-index",
        "proxy_jump_hop": "[user@]host[:port]",
        "max_proxy_jumps": 8,
        "max_proxy_jump_bytes": 2048,
        "proxy_command": False,
        "freeform_options": False,
        "remote_command": False,
        "shell_interpretation": False,
    },
    "direct_managed_options": EXPECTED_MANAGED_OPTIONS,
    "routed_managed_options": EXPECTED_ROUTED_OPTIONS,
    "host_trust": {
        "states": ["unknown", "first-use", "known", "changed"],
        "fingerprint_format": "full-OpenSSH-SHA256-32-byte-base64",
        "algorithm_required": True,
        "truncation": False,
        "first_use": "explicit-openssh-confirmation",
        "known": "reviewed-evidence-match",
        "changed": "hard-deny-no-launch-binding",
        "known_hosts_mutation": False,
        "known_hosts_write_owner": "system-openssh-or-explicit-user-recovery",
    },
    "user_owned_trust_handoff": {
        "clipboard_copy": True,
        "executes": False,
        "implicit_enter": False,
        "newline": False,
    },
    "public_identity_status": {
        "executable_id": "ssh-add",
        "arguments": ["-l", "-E", "sha256"],
        "supported_kinds": ["agent", "certificate", "hardware"],
        "private_material": False,
        "timeout_ms": 2000,
        "max_output_bytes": 65536,
        "max_identities": 64,
        "execution_enabled": False,
    },
    "agent_forwarding": False,
    "production_activation": False,
}
EXPECTED_SCENARIOS = [
    {"id": "direct-alias", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "exact-managed-options-then-one-literal-destination-argument"},
    {"id": "explicit-destination", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "explicit-host-connect-without-shell"},
    {"id": "user-and-port", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "typed-user-host-port-without-option-confusion"},
    {"id": "encrypted-key", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "prompt-remains-in-pty-no-secret-capture"},
    {"id": "agent", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "agent-success-and-failure-openssh-authoritative-forwarding-off"},
    {"id": "certificate", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "openssh-auth-no-private-material-in-core"},
    {"id": "host-key-new", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "deny-until-explicit-trust"},
    {"id": "host-key-known", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "strict-known-host-connect"},
    {"id": "host-key-changed", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "hard-deny-with-safe-explanation"},
    {"id": "proxy-jump", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "openssh-config-authoritative-no-discovery-exec"},
    {"id": "local-forward", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "disabled-until-confirmed-loopback-default"},
    {"id": "remote-forward", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "disabled-until-separately-confirmed"},
    {"id": "dynamic-forward", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "disabled-until-confirmed-loopback-default"},
    {"id": "cancellation", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "dns-connect-auth-graceful-then-bounded-force-close"},
    {"id": "exit-code", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "exact-result-class-and-redacted-audit"},
    {"id": "hostile-output", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "bounded-parser-no-control-or-secret-leak"},
    {"id": "offline", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "bounded-failure-without-hidden-retry-loop"},
    {"id": "shutdown-cleanup", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "zero-owned-child-pty-listener-route-tunnel-temp-secret-after-close"},
    {"id": "one-ten-fifty-sessions", "required": True, "platforms": EXPECTED_NATIVE_PLATFORMS, "expected": "zero-leak-and-recorded-same-host-performance-budget"},
]
EXPECTED_TRUST_BOUNDARIES = [
    {"id": "extension-model", "accepts": "bounded-public-models", "returns": "typed-launch-intent-without-secrets", "limits": "extension-api-checked-constructors", "cancellation_owner": "extension-runtime-generation", "log_policy": "public-metadata-only", "failure": "deny-or-last-known-good-without-core-impact"},
    {"id": "application-capability-broker", "accepts": "verified-principal-capability-decision-and-exact-scope", "returns": "prepared-exact-argv-or-redacted-denial", "limits": "32-kib-argv-256-environment-entries-256-kib-environment-expiring-decision", "cancellation_owner": "application-session-owner", "log_policy": "allowlisted-launch-audit-fields", "failure": "fail-closed-before-authority"},
    {"id": "pty-process-owner", "accepts": "prepared-launch-and-exact-lease", "returns": "route-bound-pty-and-result-class", "limits": "native-fixture-lifecycle-budgets", "cancellation_owner": "application-route-session-owner", "log_policy": "result-class-and-duration-no-pid-or-content", "failure": "close-child-pty-listeners-before-route-release"},
    {"id": "renderer-vt-parser", "accepts": "bounded-untrusted-terminal-bytes-and-public-state", "returns": "rendered-grid-and-sanitized-actions", "limits": "existing-s0-control-string-and-renderer-ratchets", "cancellation_owner": "renderer-generation-owner", "log_policy": "no-terminal-content", "failure": "drop-or-bound-hostile-sequence-and-preserve-input"},
    {"id": "openssh-child", "accepts": "literal-argv-core-environment-and-pty", "returns": "untrusted-pty-bytes-and-exit-status", "limits": "native-fixture-connect-auth-output-and-lifecycle-budgets", "cancellation_owner": "pty-process-owner", "log_policy": "no-child-output-or-prompts", "failure": "bounded-cancel-then-force-close"},
    {"id": "openssh-configuration", "accepts": "user-owned-files-and-static-public-index-subset", "returns": "openssh-runtime-semantics-and-bounded-public-inventory", "limits": "d4-file-count-size-depth-and-alias-ceilings", "cancellation_owner": "openssh-child-for-runtime-and-d4-refresh-owner-for-index", "log_policy": "source-label-and-line-without-content", "failure": "openssh-authoritative-and-d4-last-known-good"},
    {"id": "agent-keychain-hardware", "accepts": "openssh-owned-protocol-and-user-presence", "returns": "signature-or-auth-result-without-private-material", "limits": "external-owner-timeouts-and-policy", "cancellation_owner": "external-owner-and-openssh", "log_policy": "result-class-only-no-key-or-agent-data", "failure": "authentication-fails-with-external-recovery"},
    {"id": "remote-host", "accepts": "openssh-protocol-and-user-terminal-input", "returns": "untrusted-remote-terminal-output-and-exit", "limits": "native-fixture-network-and-pty-budgets", "cancellation_owner": "openssh-child-process-owner", "log_policy": "no-host-output-or-credentials", "failure": "isolated-session-failure-without-hidden-retry"},
    {"id": "future-provider-helper", "accepts": "future-reviewed-public-capsule-and-exact-argv", "returns": "future-public-context-or-session-result", "limits": "separate-provider-capability-contract", "cancellation_owner": "future-provider-operation-owner", "log_policy": "future-redacted-provider-audit", "failure": "disabled-until-separate-review"},
]
EXPECTED_NATIVE_FIXTURE_PROTOCOL = {
    "definition_status": "defined-not-executed",
    "execution_owner": "M3/F5.1+M4/F5.2",
    "server": "ephemeral-loopback-openssh",
    "network": "loopback-only-no-internet-or-cloud-account",
    "workspace": "per-case-private-temporary-directory",
    "known_hosts": "isolated-per-case",
    "credentials": "disposable-per-run-outside-repository",
    "agent": "isolated-per-run",
    "random_seed": "fixed-and-recorded",
    "readiness": "bounded-probe",
    "arbitrary_sleeps": False,
    "timeouts_ms": {"readiness": 10000, "connect": 15000, "cancellation_grace": 2000, "force_close": 3000, "shutdown": 10000, "case": 60000},
    "cancellation_checkpoints": ["dns", "connect", "authentication"],
    "argument_corpus": ["ascii", "unicode", "whitespace", "leading-dash", "metacharacters", "maximum-length", "over-limit"],
    "conditional_cases": ["hardware-key-when-controlled-hardware-is-available"],
    "platform_activation": {"windows": "scenario-outcome-required-before-managed-launch", "macos": "scenario-outcome-required-before-managed-launch", "linux": "scenario-outcome-required-before-managed-launch", "wsl": "managed-launch-denied-until-wsl-scenario-outcome-passes"},
    "cleanup_invariants": ["zero-owned-children", "zero-owned-ptys", "zero-owned-listeners", "zero-owned-tunnels", "zero-stale-routes", "zero-temporary-secret-files"],
    "redaction_surfaces": ["configuration", "extension-state", "logs", "renderer-snapshots", "diagnostics", "crash-and-qa-bundles", "telemetry", "clipboard-history", "ai-surfaces"],
    "required_evidence": ["native-platform-and-architecture", "openssh-client-and-server-versions", "fixture-and-config-hashes", "case-result-and-duration", "peak-memory-and-handle-counts", "child-pty-listener-route-counts", "redaction-canary-results"],
    "artifact_policy": "private-bounded-redacted-hash-manifest",
}
EXPECTED_SOURCES = [
    "apps/automexia-terminal/src/context/launch_broker.rs",
    "apps/automexia-terminal/src/context/external_tool_runner.rs",
    "apps/automexia-terminal/src/context/mod.rs",
    "apps/automexia-terminal/src/router/mod.rs",
    "apps/automexia-terminal/src/screen/connection_hub.rs",
    "automexia-ui-model/src/connection_hub.rs",
    "automexia-extension-api/src/lib.rs",
    "automexia-devops/src/connections/direct_openssh.rs",
    "apps/automexia-terminal/src/automexia/connections/receipts.rs",
    "apps/automexia-terminal/src/automexia/connections/runtime.rs",
    "apps/automexia-terminal/src/application.rs",
    "apps/automexia-terminal/src/automexia/connections/private_fs.rs",
    "extensions/devops-ssh/src/model.rs",
    "extensions/devops-ssh/src/inventory.rs",
    "apps/automexia-terminal/src/automexia/connections/direct_openssh.rs",
    "apps/automexia-terminal/src/renderer/connection_hub.rs",
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
            raise SessionLaunchD0Error(f"duplicate D0/D3/M4 contract key: {key}")
        result[key] = value
    return result


def load_contract(path: Path = CONTRACT) -> dict[str, Any]:
    document = json.loads(bounded_text(path), object_pairs_hook=reject_duplicate_keys)
    if not isinstance(document, dict) or set(document) != EXPECTED_KEYS:
        raise SessionLaunchD0Error("D0/D3/M4 contract keys changed")
    if (document["schema"], document["phase"], document["status"]) != (
        4, "F1/D0-D3+M3/F5.1/D5.2+M4/F5.2", "local-managed-ssh-routes-trust-source-complete-protected-activation-pending",
    ):
        raise SessionLaunchD0Error("D0/D3/M4 contract identity changed")
    expected_sections = {
        "activation": {"production_enabled": False, "required_adr": "0012", "adr_status": "Accepted", "required_protected_approvals": 2},
        "package_identity": {"extension_id": "automexia.devops-ssh", "publisher": "io.github.AmjedAllaya", "version_source": "workspace-package-version", "digest_algorithm": "sha256", "digest_size_bytes": 32, "digest_zero_denied": True, "digest_source": "trusted-package-loader", "contract_version": 1, "compatibility": "exact", "allowed_verification": ["repository-reviewed", "first-party-signed"], "unverified_denied": True, "revocation_fail_closed": True},
        "manual_baseline": {"preserved": True, "managed_launch_additive": True, "shells": ["powershell", "cmd", "bash", "zsh", "wsl"], "lookup_owner": "interactive-shell", "openssh_behavior_owner": "system-openssh", "download_or_install_during_startup_or_launch": False, "missing_client": "redacted-platform-installation-guidance-no-substitution"},
        "grant": {"capability": "session.launch", "required_bindings": ["extension_id", "operation_id", "session_id", "capsule_revision", "resource", "decision", "decided_at_ms", "expires_at_ms"], "decisions": ["allow-once", "allow-session", "deny"], "persistent_grants": False, "replay_denied": True, "scope_rebind_denied": True, "future_decisions_denied": True, "expired_decisions_denied": True},
        "audit": {"allowed": ["extension_id", "extension_version", "publisher", "decision", "operation_kind", "public_connection_id", "operation_id", "session_id", "timestamp_ms", "duration_ms", "result"], "forbidden": ["argv", "environment_values", "cwd", "terminal_content", "username", "secret_reference", "process_id", "executable_path", "package_digest"]},
        "executable_resolution": {"windows": {"mode": "system-directory", "ssh": "OpenSSH/ssh.exe", "ssh_add": "OpenSSH/ssh-add.exe", "ssh_keygen": "OpenSSH/ssh-keygen.exe"}, "macos": {"roots": ["/usr/bin", "/usr/local/bin", "/opt/homebrew/bin"]}, "linux": {"roots": ["/usr/bin", "/bin", "/usr/local/bin"]}, "wsl": {"production_enabled": False, "future_windows_launcher": "System32/wsl.exe", "future_linux_ssh": "/usr/bin/ssh", "shell": False}, "path_lookup": False, "cwd_lookup": False, "relative_paths": False, "fallback_after_configured_override_failure": False, "identity_revalidated_before_spawn": True},
        "strict_defaults": {"host_key_checking": "ask-on-first-use-reject-changed", "unknown_host": "interactive-pty-confirmation", "changed_host": "deny", "agent_forwarding": False, "tcp_forwarding": False, "forward_listener_scope": "loopback-unless-explicitly-confirmed", "x11_forwarding": False, "remote_command": False, "config_command_execution_during_discovery": False, "environment_inheritance": False, "secret_resolution": False, "shell_interpretation": False, "managed_options": EXPECTED_MANAGED_OPTIONS, "openssh_kex_override": False, "openssh_weak_crypto_warning_override": False},
        "authority_ceiling": {"process": False, "pty": False, "network": False, "provider": False, "authentication": False, "key_custody": False, "renderer": False},
        "ssh_routes_trust": EXPECTED_SSH_ROUTES_TRUST,
        "trust_boundaries": EXPECTED_TRUST_BOUNDARIES,
        "native_fixture_protocol": EXPECTED_NATIVE_FIXTURE_PROTOCOL,
        "managed_session": {"launch_binding": "fresh-full-review-equality", "argument_shape": "exact-route-specific-options-then-typed-user-port-jump-destination", "executable_identity": "reviewed-native-file-identity-revalidated-before-authorization", "user_configuration": "system-openssh-authoritative-launch-helper-risk-reviewed", "terminal_outcome": "child-exit-derived-no-close-assumption", "receipt_store": {"schema": 1, "max_records": 256, "max_bytes": 2097152, "write_owner": "bounded-connection-worker", "persistence": "private-atomic-primary-previous-recovery", "content": "provider-neutral-redacted-no-destination-or-terminal-text"}, "reconnect": "opaque-inventory-identity-current-source-only-fresh-review-and-approval", "notifications": "fixed-redacted-success-failure-status-unavailable-cancelled-and-storage-unavailable", "production_enabled": False},
        "external_prerequisites": ["protected security review", "two protected exact-head approvals", "real loader attestation and revocation", "native process-tree PTY OpenSSH accessibility and resource evidence"],
    }
    for key, expected in expected_sections.items():
        if document[key] != expected:
            raise SessionLaunchD0Error(f"D0/D3/M4 {key.replace('_', ' ')} changed")
    scenarios = document["native_scenarios"]
    if scenarios != EXPECTED_SCENARIOS:
        raise SessionLaunchD0Error("D0/D3/M4 native scenario matrix changed")
    if document["evidence"] != {"source": EXPECTED_SOURCES, "checks": EXPECTED_CHECKS, "documents": EXPECTED_DOCUMENTS}:
        raise SessionLaunchD0Error("D0/D3/M4 evidence inventory changed")
    return document


def require_tokens(relative: str, tokens: set[str], root: Path = ROOT) -> str:
    source = bounded_text(root / relative)
    missing = sorted(token for token in tokens if token not in source)
    if missing:
        raise SessionLaunchD0Error(f"{relative} is missing D0/D3/M4 evidence: {missing}")
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
        "linked_candidate_path_stays_fail_closed_and_redacted_without_attestation",
        "reviewed_executable_identity_digest", "DIRECT_OPENSSH_MANAGED_OPTIONS",
        "DIRECT_OPENSSH_ROUTED_OPTIONS", "validate_direct_openssh_arguments",
    }, root)
    broker_lower = broker.lower()
    broker_authority_markers = {
        ".spawn(", ".output(", ".status(", ".kill(", ".wait(",
        ".wait_with_output(", "std::net::", "tcplistener", "create_pty",
        "create_exact_pty", "forkpty", "conpty", "commandext::exec",
        "createprocess", "execve", "posix_spawn",
    }
    widened = sorted(
        marker for marker in broker_authority_markers if marker in broker_lower
    )
    if widened:
        raise SessionLaunchD0Error(
            f"capability broker gained runtime process/network/PTY authority: {widened}"
        )

    runner = require_tokens(document["evidence"]["source"][1], {
        "pub const MAX_CONCURRENT_EXTERNAL_TOOLS: usize = 50",
        "pub const MAX_RUNNER_AUDIT_RECORDS: usize = 256",
        "pub struct OpenSshLaunchIntent",
        "pub struct ExternalToolRunner",
        "pub fn pending_security_review",
        "VerifiedExtension::linked_unverified_candidate()",
        "RunnerErrorCode::SafeDefaultUnavailable",
        "u64::try_from(route_id).ok() != Some(lease.session_id().get())",
        "pub fn mark_published",
        "pub fn complete",
        "pub fn cancel",
        "pub fn shutdown_now",
        "VecDeque::with_capacity(MAX_RUNNER_AUDIT_RECORDS)",
        "DirectOpenSshLaunchBinding", "record_terminal_outcome",
        "ManagedProcessOutcome::StatusUnavailable", "ManagedReceiptRecord::new",
    }, root)
    runner_lower = runner.lower()
    runner_authority_markers = {
        "std::process::command", ".spawn(", "create_pty", "create_exact_pty",
        "cmd /c", "powershell -command", "sh -c", "std::net::", "tcplistener",
    }
    widened = sorted(
        marker for marker in runner_authority_markers if marker in runner_lower
    )
    if widened:
        raise SessionLaunchD0Error(
            f"application runner bypasses the exact ContextManager launch seam: {widened}"
        )

    context = require_tokens(document["evidence"]["source"][2], {
        "pub mod external_tool_runner;",
        "pub mod launch_broker;",
        "pub struct ManagedRouteReservation",
        "pub fn reserve_managed_route",
        "pub fn publish_managed_context",
        "teletypewriter::create_exact_pty(",
        "runner.mark_published(lease, route_id)",
        "managed_session: Option<ManagedSessionGuard>",
        "reconcile_managed_child_exit", "ManagedProcessOutcome::Failed",
    }, root).replace("\r\n", "\n")
    for declaration in (
        "pub mod external_tool_runner;",
        "pub mod launch_broker;",
    ):
        if context.count(declaration) != 1 or f"#[cfg(test)]\n{declaration}" in context:
            raise SessionLaunchD0Error(
                f"{declaration} is not a single production-compiled module declaration"
            )
    if context.count("teletypewriter::create_exact_pty(") != 2:
        raise SessionLaunchD0Error(
            "ContextManager lost its exact platform PTY adapters or gained a duplicate seam"
        )

    require_tokens(document["evidence"]["source"][3], {
        "external_tool_runner: crate::context::external_tool_runner::ExternalToolRunner",
        "ExternalToolRunner::pending_security_review()",
        "self.external_tool_runner.shutdown_now()",
        "self.external_tool_runner.clone()",
        "attach_receipt_sink", "Arc::new(connection_hub.clone())",
    }, root)
    require_tokens(document["evidence"]["source"][4], {
        "fn attempt_managed_openssh",
        "Decision::AllowOnce",
        "Decision::AllowSession",
        "ConnectionHubHit::DenyManagedLaunch",
        "managed_openssh_activation_error",
        "DirectOpenSshLaunchBinding",
        "Never fall back",
        "connection-launch-protected-review-pending",
    }, root)
    require_tokens(document["evidence"]["source"][5], {
        "pub approval_action_enabled: bool",
        "append_direct_decision_accessibility",
        'format!("direct-openssh-decision-{id}")',
        '"allow-once"',
        '"allow-session"',
        '"deny"',
        "execution_enabled: false",
        "direct-openssh-trust-copy",
    }, root)
    require_tokens(document["evidence"]["source"][6], {
        "pub struct CapabilityDecision", "pub operation_id: OperationId",
        "pub session_id: SessionId", "pub capsule_revision: u64",
        "pub expires_at_ms: u64",
    }, root)
    require_tokens(document["evidence"]["source"][7], {
        "DIRECT_OPENSSH_MANAGED_OPTIONS", "DIRECT_OPENSSH_ROUTED_OPTIONS",
        "pub struct DirectOpenSshLaunchBinding", "pub fn bind_launch",
        "pub fn bind_launch_m4", "prepare_direct_openssh_identity_status",
        "parse_direct_openssh_agent_identities", "pub fn user_owned_command",
    }, root)
    require_tokens("automexia-devops/tests/direct_openssh_review.rs", {
        "managed_options_preserve_openssh_post_quantum_defaults_and_warnings",
        'starts_with("-oKexAlgorithms=")', 'starts_with("-oWarnWeakCrypto=")',
    }, root)
    require_tokens(document["evidence"]["source"][8], {
        "MAX_MANAGED_RECEIPTS: usize = 256", "MAX_MANAGED_RECEIPT_BYTES",
        "pub trait ManagedReceiptSink", "append_batch", "PreviousRecovery",
    }, root)
    require_tokens(document["evidence"]["source"][9], {
        "Work::FlushReceipts", "impl ManagedReceiptSink for ConnectionHubRuntime",
        "prepare_managed_reconnect", "StaleReconnect",
        "managed_receipts_survive_restart_with_only_fresh_review_identity",
    }, root)
    require_tokens(document["evidence"]["source"][10], {
        "RioEvent::ChildExited", "reconcile_managed_child_exit",
        "handle_desktop_notification",
    }, root)
    require_tokens(document["evidence"]["source"][11], {
        "pub(super) enum PrivateFsErrorCode", "FILE_FLAG_OPEN_REPARSE_POINT",
        "PROTECTED_DACL_SECURITY_INFORMATION", "libc::O_NOFOLLOW",
        "same_snapshot", "private_permissions_are_safe",
        "GetFileInformationByHandle", "nFileIndexHigh",
    }, root)
    require_tokens(document["evidence"]["source"][12], {
        "parse_proxy_jump_chain", "MAX_PROXY_JUMPS", "MAX_PROXY_JUMP_BYTES",
        "canonical_proxy_jump_hop",
    }, root)
    require_tokens(document["evidence"]["source"][13], {
        "proxy_jump_is_a_bounded_canonical_first_value_chain",
        "proxy_jump_rejects_executable_ambiguous_and_excessive_routes",
    }, root)
    require_tokens(document["evidence"]["source"][14], {
        "prepare_literal_direct_openssh_typed", "record.proxy_jump.clone()",
        "config_route_is_preserved_while_invalid_generation_and_hostile_aliases_fail_closed",
    }, root)
    require_tokens(document["evidence"]["source"][15], {
        "wrap_without_truncation", "Protected checks required",
        "host-trust",
    }, root)

    audit_start = broker.index("pub struct LaunchAuditRecord")
    audit_end = broker.index("pub struct DeniedLaunch", audit_start)
    audit_block = broker[audit_start:audit_end]
    leaked = sorted(field for field in document["audit"]["forbidden"] if field in audit_block)
    if leaked:
        raise SessionLaunchD0Error(f"audit record exposes forbidden fields: {leaked}")
    for relative in document["evidence"]["documents"]:
        require_tokens(relative, {"D0", "ADR 0012"}, root)
    return {
        "schema": document["schema"],
        "scenarios": len(document["native_scenarios"]),
        "boundaries": len(document["trust_boundaries"]),
        "sources": len(document["evidence"]["source"]),
        "documents": len(document["evidence"]["documents"]),
        "production_enabled": int(document["activation"]["production_enabled"]),
    }

def validate_repository(root: Path = ROOT) -> dict[str, int]:
    for historical_path, expected_digest, label in HISTORICAL_CONTRACTS:
        historical = bounded_text(root / historical_path.relative_to(ROOT)).encode("utf-8")
        if hashlib.sha256(historical).hexdigest() != expected_digest:
            raise SessionLaunchD0Error(f"historical D0/D3/M4 {label} contract changed")
    document = load_contract(root / CONTRACT.relative_to(ROOT))
    return validate_sources(document, root)


def main() -> int:
    try:
        counts = validate_repository()
    except (SessionLaunchD0Error, OSError, UnicodeError, json.JSONDecodeError) as error:
        print(f"D0/D3/M4 session-launch validation failed: {error}", file=sys.stderr)
        return 1
    print(f"PASS: D0/D3/M4 session-launch contract is exact, fail-closed, and review-gated ({counts})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
