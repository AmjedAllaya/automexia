#!/usr/bin/env python3
"""Validate the provider-neutral M7/D6.0 authentication contract."""

from __future__ import annotations

import json
from pathlib import Path
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "tests/fixtures/provider-auth/m7-contract-v1.json"
MAX_EVIDENCE_BYTES = 524_288
EXPECTED_LIMITS = {
    "capsules": 64,
    "provider_contexts_per_capsule": 16,
    "scope_bindings_per_context": 32,
    "browser_origins": 16,
    "capability_requests": 8,
    "arguments": 128,
    "public_environment_names": 16,
    "operation_timeout_ms": 300_000,
    "stale_after_ms": 604_800_000,
    "document_bytes": 16_777_216,
    "public_text_bytes": 4_096,
}
EXPECTED_STATES = [
    "unknown", "checking", "available", "refreshing", "authenticating",
    "mfa-required", "mfa-pending", "browser-pending", "device-code-pending",
    "ready", "expired", "locked", "missing", "offline", "denied",
    "unsupported", "cancelled", "stale", "error",
]
EXPECTED_ISOLATION = [
    "exact-arguments", "scoped-environment", "private-transient-config",
]
EXPECTED_BROWSER_FLOWS = [
    "none", "external-browser", "device-code", "system-broker",
]
EXPECTED_MUTATIONS = [
    "az account set",
    "gcloud config set",
    "gcloud config configurations activate",
    "gcloud init",
    "kubectl config use-context",
    "oc config use-context",
    "aws configure set",
]
EXPECTED_AUTHORITIES = {
    "process": False,
    "network": False,
    "browser": False,
    "credential": False,
    "token_cache": False,
    "certificate": False,
    "provider_config_write": False,
    "pty": False,
    "renderer": False,
}
EXPECTED_SECRET_SURFACES = [
    "persistence", "logs", "diagnostics", "snapshots", "qa-bundles",
    "clipboard", "telemetry", "ai",
]
FORBIDDEN_AUTHORITY_PRIMITIVES = {
    "std::process",
    "command::new",
    ".spawn(",
    "std::net",
    "std::fs",
    "tokio::process",
    "tokio::net",
    "reqwest",
    "hyper::",
    "unsafe {",
}
REQUIRED_SOURCE_TOKENS = {
    "automexia-devops/src/connections/provider_auth.rs": {
        "ProviderAuthCapsuleStore",
        "ProviderAuthObservation",
        "ProviderAuthOperation",
        "ProviderAuthReceipt",
        "ProviderAuthAudit",
        "core::net::Ipv6Addr",
        "capability_scope_is_exact",
        "pub capability_requests: Vec<CapabilityRequest>",
        "pub browser_flow: ProviderBrowserFlow",
        "pub callback_uri: Option<String>",
        "provider operation isolation must match the capsule's pinned context",
        "last-known-good context does not match the pinned provider scope",
        "validate_provider_auth_observation(&observation)?",
        "initial_auth_state",
        "review.risk != context.risk",
        "MAX_PROVIDER_CAPSULES",
        "MAX_PROVIDER_STALE_AFTER_MS",
        "parse_provider_auth_observation_json",
        "parse_provider_auth_operation_json",
        "parse_provider_auth_receipt_json",
        "parse_provider_auth_audit_json",
        "next_generation",
        "HTTPS authority port must be nonzero",
        "session_id: operation.session_id.get()",
        "pub fn from_template",
        "Decision::AllowOnce",
        "ProviderIsolationStrategy",
        "ProviderBrowserFlow",
        "validate_callback_uri",
        "stale or cross-session provider result was rejected",
        "managed provider operations cannot mutate global CLI context",
        "secret-bearing command-line options are forbidden",
        "pub fn begin_refresh",
        "pub fn begin_authentication",
        "pub fn cancel",
        "pub fn expire",
        "pub fn revoke",
        "pub fn disable_provider",
        "pub fn uninstall_provider",
        "pub fn shutdown",
    },
    "automexia-devops/src/connections/model.rs": {
        "provider_contexts: Vec<ProviderContextTemplate>",
        "Available",
        "Refreshing",
        "MfaPending",
        "BrowserPending",
        "DeviceCodePending",
    },
    "automexia-devops/src/connections/state.rs": {
        "AuthEvent::BeginRefresh",
        "AuthEvent::ObservedBrowserPending",
        "AuthEvent::ObservedDeviceCodePending",
        "AuthEvent::ObservedMfaPending",
        "require_current_operation",
    },
    "automexia-extension-runtime/src/lib.rs": {
        "current.providers != next.providers",
        "provider_context_rebind_requires_a_fresh_session",
    },
    "automexia-devops/src/context.rs": {
        "Passive status discovery reads only bounded public local files",
        "passive_status_discovery_has_no_process_shell_or_provider_launch_authority",
        "docker: docker.map(|value| sanitize_label(&value))",
        "environment: environment.map(|value| sanitize_label(&value))",
    },
    "automexia-ui-model/src/connection_hub.rs": {
        'AuthState::Available { .. } => ("Available", "Refresh or sign in")',
        'AuthState::Refreshing { .. } => ("Refreshing", "Cancel refresh")',
        "AuthState::BrowserPending { .. }",
        '"Waiting for browser"',
        'AuthState::DeviceCodePending { .. }',
        'StaleAuthState::Available => "Stale (was available)"',
    },
}
REQUIRED_TESTS = {
    "automexia-devops/tests/provider_auth_m7.rs": {
        "strict_provider_capsule_parser_rejects_unknown_fields_and_oversized_documents",
        "strict_capsules_are_bounded_unique_and_reject_cross_session_reads",
        "rebind_cancels_old_work_and_never_carries_context_across_capsules",
        "cached_observation_preserves_last_known_good_across_offline_and_expiry",
        "cancellation_expiry_and_shutdown_are_explicit_and_generation_safe",
        "provider_auth_state_machine_covers_refresh_browser_device_and_mfa_waits",
        "official_cli_launch_requires_exact_visible_allow_once_decisions",
        "global_context_mutations_secret_flags_and_unapproved_origins_fail_closed",
        "receipt_audit_debug_and_serialized_public_observation_exclude_canaries",
        "existing_connection_capsule_template_instantiates_exact_pinned_provider_context",
        "every_public_provider_auth_record_has_strict_bounded_semantic_ingress",
        "maximum_capsule_lifecycle_is_bounded_and_releases_all_session_state",
    },
}


class M7ContractError(ValueError):
    pass


def bounded_text(path: Path, maximum: int = MAX_EVIDENCE_BYTES) -> str:
    if path.is_symlink() or not path.is_file():
        raise M7ContractError(f"required regular M7 file is missing or linked: {path}")
    if path.stat().st_size > maximum:
        raise M7ContractError(f"M7 evidence exceeds {maximum} bytes: {path}")
    data = path.read_bytes()
    if len(data) > maximum:
        raise M7ContractError(f"M7 evidence grew while reading: {path}")
    return data.decode("utf-8")


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise M7ContractError(f"duplicate M7 contract key: {key}")
        result[key] = value
    return result


def load_contract(path: Path = CONTRACT) -> dict[str, Any]:
    document = json.loads(bounded_text(path), object_pairs_hook=reject_duplicate_keys)
    expected_keys = {
        "schema", "phase", "status", "limits", "authentication_states",
        "isolation_strategies", "browser_flows", "forbidden_managed_mutations",
        "authorities", "secret_surfaces", "source_files", "test_files",
        "fixture_files", "fuzz_target", "benchmark", "documents",
    }
    if not isinstance(document, dict) or set(document) != expected_keys:
        raise M7ContractError("M7 contract keys changed")
    if (document["schema"], document["phase"], document["status"]) != (
        1,
        "M7/D6.0",
        "implemented-framework-provider-adapters-separate",
    ):
        raise M7ContractError("M7 contract identity changed")
    expected = {
        "limits": EXPECTED_LIMITS,
        "authentication_states": EXPECTED_STATES,
        "isolation_strategies": EXPECTED_ISOLATION,
        "browser_flows": EXPECTED_BROWSER_FLOWS,
        "forbidden_managed_mutations": EXPECTED_MUTATIONS,
        "authorities": EXPECTED_AUTHORITIES,
        "secret_surfaces": EXPECTED_SECRET_SURFACES,
    }
    for key, value in expected.items():
        if document[key] != value:
            raise M7ContractError(f"M7 {key} changed")
    for key in (
        "source_files", "test_files", "fixture_files", "documents",
    ):
        values = document[key]
        if not isinstance(values, list) or not values or len(values) != len(set(values)):
            raise M7ContractError(f"M7 {key} must be a non-empty unique list")
    return document


def validate_sources(document: dict[str, Any]) -> None:
    for relative in document["source_files"]:
        bounded_text(ROOT / relative)
    for relative, tokens in REQUIRED_SOURCE_TOKENS.items():
        source = bounded_text(ROOT / relative)
        missing = sorted(token for token in tokens if token not in source)
        if missing:
            raise M7ContractError(f"missing M7 source evidence in {relative}: {missing}")

    auth_source = bounded_text(
        ROOT / "automexia-devops/src/connections/provider_auth.rs"
    ).lower()
    widened = sorted(
        primitive for primitive in FORBIDDEN_AUTHORITY_PRIMITIVES
        if primitive in auth_source
    )
    if widened:
        raise M7ContractError(f"M7 model gained runtime authority: {widened}")

    legacy_source = bounded_text(ROOT / "automexia-devops/src/context.rs")
    production = legacy_source.split("#[cfg(all(test", 1)[0].lower()
    legacy_launch = [
        primitive for primitive in (
            "std::process", "command::new", ".spawn(", "wsl_context_probe_script",
            "wsl_live_contexts", '"--exec", "sh", "-c"',
        )
        if primitive in production
    ]
    if legacy_launch:
        raise M7ContractError(
            f"passive legacy status regained launch authority: {legacy_launch}"
        )


def validate_evidence(document: dict[str, Any]) -> None:
    for relative, tokens in REQUIRED_TESTS.items():
        source = bounded_text(ROOT / relative)
        missing = sorted(token for token in tokens if f"fn {token}(" not in source)
        if missing:
            raise M7ContractError(f"missing M7 tests in {relative}: {missing}")
    for key in ("test_files", "fixture_files", "documents"):
        for relative in document[key]:
            bounded_text(ROOT / relative)
    bounded_text(ROOT / document["fuzz_target"])
    bounded_text(ROOT / document["benchmark"])


def validate_repository() -> dict[str, int]:
    document = load_contract()
    validate_sources(document)
    validate_evidence(document)
    return {
        "states": len(document["authentication_states"]),
        "mutations": len(document["forbidden_managed_mutations"]),
        "surfaces": len(document["secret_surfaces"]),
        "tests": sum(len(tokens) for tokens in REQUIRED_TESTS.values()),
    }


if __name__ == "__main__":
    try:
        counts = validate_repository()
    except (M7ContractError, OSError, UnicodeError, json.JSONDecodeError) as exc:
        print(f"M7 provider-auth validation failed: {exc}", file=sys.stderr)
        raise SystemExit(1) from exc
    print(
        "PASS: M7 provider-auth contract is bounded and authority-free "
        f"(states={counts['states']}, mutations={counts['mutations']}, "
        f"surfaces={counts['surfaces']}, tests={counts['tests']})"
    )
