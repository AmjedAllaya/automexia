#!/usr/bin/env python3
"""Validate the accepted, source-complete, non-activating D7/CP6 boundary."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
CONTRACT_PATH = Path("tests/fixtures/ecosystem/d7-cp6-ecosystem-contract-v1.json")
ACCEPTANCE_PATH = Path("tests/fixtures/ecosystem/d7-cp6-acceptance-v1.json")
ADR_PATH = Path("docs/adr/0029-sandboxed-signed-ecosystem-boundary.md")
PLAN_PATH = Path("docs/research/D7-CP6-IMPLEMENTATION-AUDIT.md")
REFERENCE_PATH = Path("docs/ECOSYSTEM-PLATFORM.md")
MAX_CONTRACT_BYTES = 262_144
MAX_DOCUMENT_BYTES = 393_216
EXPECTED_CANONICAL_SHA256 = (
    "fdd765ec52cf043ebbf2132177a94ae8be5bcb8175d4f1b566bcac4b96b346b4"
)

EXPECTED_TOP_LEVEL_KEYS = {
    "schema",
    "phase",
    "status",
    "authority",
    "ownership",
    "package",
    "sandbox",
    "capabilities",
    "provenance",
    "distribution",
    "lifecycle",
    "ai",
    "security_threats",
    "limits",
    "verification",
    "external_gates",
}

EXPECTED_AUTHORITY = {
    "adr": "0029",
    "accepted": False,
    "runtime_activation": False,
    "public_downloads": False,
    "public_sdk": False,
    "component_execution": False,
    "ai_provider_calls": False,
    "ai_tool_calls": False,
    "automatic_execution": False,
    "new_dependencies": "none",
    "fallback": "private-first-party-extensions-and-cp1-cp3",
}

EXPECTED_THREATS = {
    "D7-T01-supply-chain-rollback-freeze",
    "D7-T02-hostile-bundle-parser",
    "D7-T03-sandbox-and-confused-deputy",
    "D7-T04-resource-and-lifecycle-amplification",
    "D7-T05-capability-upgrade-and-isolation",
    "D7-T06-untrusted-output-and-ui-spoofing",
    "D7-T07-compatibility-and-migration-downgrade",
    "CP6-T08-ai-privacy-prompt-injection-and-execution",
    "D7-T09-cache-uninstall-and-cross-scope-leakage",
}

EXPECTED_LIMITS = {
    "bundle_bytes": 16_777_216,
    "expanded_bytes": 33_554_432,
    "package_files": 32,
    "path_bytes": 512,
    "manifest_bytes": 65_536,
    "manifest_depth": 16,
    "manifest_string_bytes": 4_096,
    "component_bytes": 8_388_608,
    "wit_imports": 64,
    "capability_requests": 32,
    "linear_memory_bytes": 67_108_864,
    "table_elements": 100_000,
    "instances_per_extension": 2,
    "fuel_per_call": 10_000_000,
    "host_transfer_bytes_per_call": 1_048_576,
    "interactive_deadline_ms": 250,
    "explicit_deadline_ms": 2_000,
    "output_bytes_per_call": 1_048_576,
    "log_bytes_per_call": 65_536,
    "queued_calls_per_extension": 16,
    "concurrent_calls_per_extension": 2,
    "concurrent_calls_global": 8,
    "crashes_per_five_minutes": 3,
    "selected_ai_input_bytes": 16_384,
    "ai_response_bytes": 65_536,
    "cache_bytes": 268_435_456,
    "retained_versions_per_extension": 2,
    "installed_extensions": 128,
}

EXPECTED_VERIFICATION_DOMAINS = {
    "contract",
    "package",
    "provenance",
    "sandbox",
    "concurrency",
    "privacy",
    "ui",
    "accessibility",
    "performance",
    "resources",
    "platforms",
    "rollback",
}

REQUIRED_PACKAGE_CONTROLS = {
    "verify-before-extract",
    "private-no-follow-staging",
    "disk-space-preflight",
    "bounded-expansion",
    "fsync-and-atomic-publish",
    "last-known-good-generation",
    "crash-recovery",
}

REQUIRED_FORBIDDEN_IMPORTS = {
    "wasi:cli",
    "wasi:filesystem",
    "wasi:sockets",
    "wasi:http",
    "wasi:random",
    "process",
    "environment",
    "clipboard",
    "pty",
    "terminal-history",
    "terminal-output",
    "ssh-agent",
    "credentials",
    "provider-cache",
    "capsule-secret",
    "connection",
}

REQUIRED_UNAVAILABLE_CAPABILITIES = {
    "filesystem.read",
    "filesystem.write",
    "network.connect",
    "process.request",
    "clipboard.read",
    "clipboard.write",
    "pty.read",
    "pty.write",
    "terminal-history.read",
    "terminal-output.read",
    "environment.read",
    "credential-reference.use",
    "ssh-agent.use",
    "provider-cache.read",
    "capsule-secret.read",
    "connection.use",
    "ai.tool-call",
    "command.execute",
}

REQUIRED_GRANT_BINDING = {
    "publisher",
    "extension-id",
    "version",
    "exact-package-digest",
    "capability",
    "exact-scope",
    "profile",
    "expiry",
}

REQUIRED_PROVENANCE_BINDING = {
    "package-digest",
    "publisher-identity",
    "signature",
    "trusted-root",
    "timestamp",
    "provenance",
    "sbom-and-licenses",
    "compatibility",
    "current-revocation-state",
}

REQUIRED_DISTRIBUTION_CONTROLS = {
    "threshold-and-role-separation",
    "trusted-root-rotation",
    "monotonic-versions",
    "expiry",
    "rollback-freeze-and-mix-and-match-resistance",
    "consistent-snapshots",
    "bounded-cache-retries-and-refresh",
    "publisher-delegation-and-emergency-revocation",
}

REQUIRED_AI_FORBIDDEN_INPUT = {
    "ambient-terminal-output",
    "terminal-history",
    "clipboard",
    "filesystem",
    "environment",
    "credentials",
    "ssh-agent",
    "provider-cache",
    "capsule-secrets",
    "connections",
    "other-panes",
    "logs",
    "telemetry",
    "support-bundles",
}


class EcosystemContractError(ValueError):
    """The D7/CP6 proposal contract is invalid or gained authority."""


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    document: dict[str, Any] = {}
    for key, value in pairs:
        if key in document:
            raise EcosystemContractError(
                f"D7/CP6 contract contains duplicate key {key!r}"
            )
        document[key] = value
    return document


def parse_contract(text: str) -> Any:
    try:
        return json.loads(text, object_pairs_hook=_unique_object)
    except json.JSONDecodeError as error:
        raise EcosystemContractError(f"D7/CP6 contract is invalid JSON: {error}") from error


def _bounded_text(path: Path, maximum: int, label: str) -> str:
    if path.is_symlink():
        raise EcosystemContractError(f"{label} must not be a symbolic link: {path}")
    if not path.is_file():
        raise EcosystemContractError(f"{label} is missing: {path}")
    if path.stat().st_size > maximum:
        raise EcosystemContractError(f"{label} exceeds {maximum} bytes: {path}")
    return path.read_text(encoding="utf-8")


def _require_exact_keys(value: Any, keys: set[str], label: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        raise EcosystemContractError(f"{label} keys changed")
    return value


def _require_exact_set(value: Any, expected: set[str], label: str) -> None:
    if not isinstance(value, list) or len(value) != len(set(value)) or set(value) != expected:
        raise EcosystemContractError(f"{label} changed")


def _require_nonempty_strings(value: Any, label: str) -> None:
    if not isinstance(value, list) or not value:
        raise EcosystemContractError(f"{label} must be a non-empty list")
    if any(not isinstance(item, str) or not item.strip() for item in value):
        raise EcosystemContractError(f"{label} contains an invalid item")
    if len(value) != len(set(value)):
        raise EcosystemContractError(f"{label} contains duplicates")


def validate_contract(document: Any) -> dict[str, int]:
    document = _require_exact_keys(document, EXPECTED_TOP_LEVEL_KEYS, "contract")
    if (document["schema"], document["phase"], document["status"]) != (
        1,
        "D7/CP6",
        "proposed-not-authorized",
    ):
        raise EcosystemContractError("proposal schema, phase, or status changed")

    authority = _require_exact_keys(
        document["authority"], set(EXPECTED_AUTHORITY), "authority"
    )
    if authority != EXPECTED_AUTHORITY:
        raise EcosystemContractError("proposal gained authority or changed fallback")

    ownership = _require_exact_keys(
        document["ownership"],
        {
            "terminal_core",
            "ecosystem_domain",
            "sandbox_host",
            "external_authorities",
            "forbidden_duplicate_owners",
        },
        "ownership",
    )
    for key, value in ownership.items():
        _require_nonempty_strings(value, f"ownership.{key}")
    _require_exact_set(
        ownership["forbidden_duplicate_owners"],
        {
            "pty",
            "terminal-grid",
            "session",
            "renderer",
            "process-runner",
            "credential-store",
            "shell-editor",
        },
        "forbidden duplicate owners",
    )

    package = _require_exact_keys(
        document["package"],
        {
            "initial_ingress",
            "immutable",
            "required_entries",
            "forbidden_payloads",
            "path_policy",
            "extraction",
        },
        "package",
    )
    if package["initial_ingress"] != "explicit-local-file-only" or package["immutable"] is not True:
        raise EcosystemContractError("package ingress or immutability changed")
    _require_exact_set(package["extraction"], REQUIRED_PACKAGE_CONTROLS, "package extraction")
    for key in ("required_entries", "forbidden_payloads", "path_policy"):
        _require_nonempty_strings(package[key], f"package.{key}")
    if not {"native-library", "native-executable", "shell-script", "symlink", "hardlink"}.issubset(
        set(package["forbidden_payloads"])
    ):
        raise EcosystemContractError("forbidden package payloads changed")

    sandbox = _require_exact_keys(
        document["sandbox"],
        {
            "candidate",
            "adopted_dependency",
            "world",
            "default_wasi",
            "allowed_imports",
            "forbidden_import_families",
            "cpu",
            "memory",
            "worker",
            "publication",
            "execution_output",
        },
        "sandbox",
    )
    if (
        sandbox["candidate"] != "wasmtime-component-model"
        or sandbox["adopted_dependency"] is not False
        or sandbox["default_wasi"] != "none"
        or sandbox["cpu"] != "deterministic-fuel-plus-emergency-epoch-deadline"
        or sandbox["execution_output"] != "never-pty-input-never-enter-never-process"
    ):
        raise EcosystemContractError("sandbox authority or limits changed")
    _require_exact_set(
        sandbox["forbidden_import_families"],
        REQUIRED_FORBIDDEN_IMPORTS,
        "forbidden imports",
    )
    _require_nonempty_strings(sandbox["allowed_imports"], "allowed imports")
    if any(item.startswith("wasi:") for item in sandbox["allowed_imports"]):
        raise EcosystemContractError("allowed imports gained WASI authority")

    capabilities = _require_exact_keys(
        document["capabilities"],
        {
            "default",
            "initial_available",
            "initial_unavailable",
            "grant_binding",
            "fresh_review_on_change",
            "persistent_ambient_grants",
            "read_implies_command",
            "guest_may_mint_grant",
        },
        "capabilities",
    )
    if capabilities["default"] != "deny" or any(
        capabilities[key] is not False
        for key in (
            "persistent_ambient_grants",
            "read_implies_command",
            "guest_may_mint_grant",
        )
    ):
        raise EcosystemContractError("capability default-deny policy changed")
    _require_exact_set(
        capabilities["initial_unavailable"],
        REQUIRED_UNAVAILABLE_CAPABILITIES,
        "initial unavailable capabilities",
    )
    _require_exact_set(
        capabilities["grant_binding"], REQUIRED_GRANT_BINDING, "grant binding"
    )
    _require_nonempty_strings(capabilities["initial_available"], "initial available capabilities")
    _require_nonempty_strings(capabilities["fresh_review_on_change"], "fresh review fields")

    provenance = _require_exact_keys(
        document["provenance"],
        {
            "verification_before_review",
            "binds",
            "signature_is_safety_claim",
            "offline_bundle_required",
            "verification_receipt",
            "revocation_check",
            "older-cache_bypasses_revocation",
        },
        "provenance",
    )
    if (
        provenance["verification_before_review"] is not True
        or provenance["signature_is_safety_claim"] is not False
        or provenance["offline_bundle_required"] is not True
        or provenance["older-cache_bypasses_revocation"] is not False
        or provenance["revocation_check"] != "before-every-invocation"
    ):
        raise EcosystemContractError("provenance or revocation policy changed")
    _require_exact_set(provenance["binds"], REQUIRED_PROVENANCE_BINDING, "provenance binding")

    distribution = _require_exact_keys(
        document["distribution"],
        {
            "enabled",
            "transport",
            "https_is_authenticity",
            "metadata_roles",
            "required_controls",
            "refresh",
            "startup_network",
            "typing_network",
            "offline_behavior",
        },
        "distribution",
    )
    if (
        distribution["enabled"] is not False
        or distribution["https_is_authenticity"] is not False
        or distribution["startup_network"] is not False
        or distribution["typing_network"] is not False
        or distribution["refresh"] != "explicit-only"
    ):
        raise EcosystemContractError("distribution gained authority")
    _require_exact_set(
        distribution["metadata_roles"],
        {"root", "targets", "snapshot", "timestamp"},
        "metadata roles",
    )
    _require_exact_set(
        distribution["required_controls"],
        REQUIRED_DISTRIBUTION_CONTROLS,
        "distribution controls",
    )

    lifecycle = _require_exact_keys(
        document["lifecycle"],
        {
            "states",
            "default_state",
            "activation_now",
            "queue",
            "stale_results",
            "crash_policy",
            "kill_switch",
            "disable",
            "uninstall",
            "preserve",
        },
        "lifecycle",
    )
    if (
        lifecycle["default_state"] != "unavailable"
        or lifecycle["activation_now"] != "always-denied"
        or lifecycle["stale_results"] != "reject"
        or lifecycle["kill_switch"]
        != "deny-cancel-join-close-clear-rotate-fallback-without-restart"
    ):
        raise EcosystemContractError("lifecycle activation, staleness, or kill policy changed")
    _require_nonempty_strings(lifecycle["states"], "lifecycle states")
    _require_exact_set(
        lifecycle["preserve"],
        {
            "native-shell-state",
            "provider-state",
            "credentials",
            "user-files",
            "first-party-extensions",
            "cp1-cp3-fallback",
        },
        "preserved state",
    )

    ai = _require_exact_keys(
        document["ai"],
        {
            "enabled",
            "separate_opt_in",
            "trigger",
            "consent_per_request",
            "review_discloses",
            "allowed_input",
            "forbidden_input",
            "response",
            "risk",
            "delivery",
            "tool_calls",
            "mcp_passthrough",
            "background_requests",
            "typing_requests",
            "automatic_execution",
            "logging",
        },
        "AI",
    )
    if (
        ai["enabled"] is not False
        or ai["separate_opt_in"] is not True
        or ai["consent_per_request"] is not True
        or ai["allowed_input"] != ["explicit-selected-text"]
        or ai["delivery"] != "copy-or-insert-without-enter"
        or any(
            ai[key] is not False
            for key in (
                "tool_calls",
                "mcp_passthrough",
                "background_requests",
                "typing_requests",
                "automatic_execution",
            )
        )
    ):
        raise EcosystemContractError("AI consent, input, or execution policy changed")
    _require_exact_set(ai["forbidden_input"], REQUIRED_AI_FORBIDDEN_INPUT, "AI forbidden input")
    _require_nonempty_strings(ai["review_discloses"], "AI review disclosures")

    threats = document["security_threats"]
    if not isinstance(threats, list) or len(threats) != len(EXPECTED_THREATS):
        raise EcosystemContractError("security threat inventory changed")
    threat_ids: set[str] = set()
    for threat in threats:
        threat = _require_exact_keys(
            threat,
            {
                "id",
                "asset",
                "controls",
                "hostile_mutations",
                "verification_owner",
                "residual_risk",
            },
            "security threat",
        )
        threat_id = threat["id"]
        if not isinstance(threat_id, str) or threat_id in threat_ids:
            raise EcosystemContractError("security threat IDs are invalid or duplicated")
        threat_ids.add(threat_id)
        for key in ("controls", "hostile_mutations", "verification_owner"):
            _require_nonempty_strings(threat[key], f"{threat_id}.{key}")
        if (
            threat_id == "CP6-T08-ai-privacy-prompt-injection-and-execution"
            and "suggestion auto-runs" not in threat["hostile_mutations"]
        ):
            raise EcosystemContractError("AI auto-execution mutation was removed")
        if not isinstance(threat["asset"], str) or not threat["asset"].strip():
            raise EcosystemContractError(f"{threat_id} has no asset")
        if not isinstance(threat["residual_risk"], str) or not threat["residual_risk"].strip():
            raise EcosystemContractError(f"{threat_id} has no residual risk")
    if threat_ids != EXPECTED_THREATS:
        raise EcosystemContractError("security threat IDs changed")

    limits = _require_exact_keys(document["limits"], set(EXPECTED_LIMITS), "limits")
    if limits != EXPECTED_LIMITS:
        raise EcosystemContractError("resource limits changed")

    verification = _require_exact_keys(
        document["verification"], EXPECTED_VERIFICATION_DOMAINS, "verification"
    )
    for key, value in verification.items():
        _require_nonempty_strings(value, f"verification.{key}")

    gates = document["external_gates"]
    _require_nonempty_strings(gates, "external gates")
    if len(gates) != 10:
        raise EcosystemContractError("external gate inventory changed")
    gate_text = " ".join(gates).lower()
    for marker in (
        "adr 0029",
        "two independent",
        "dependency",
        "malicious-package",
        "windows linux and macos",
        "30-day",
        "privacy and legal",
        "fallback",
    ):
        if marker not in gate_text:
            raise EcosystemContractError(f"external gates lost {marker!r}")

    return {
        "threats": len(threats),
        "limits": len(limits),
        "verification_domains": len(verification),
        "external_gates": len(gates),
    }


def canonical_digest(document: Any) -> str:
    encoded = json.dumps(
        document, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


EXPECTED_ACCEPTANCE_DEPENDENCIES = {
    "ed25519-dalek",
    "unicode-normalization",
    "wasmtime",
    "wat",
    "wit-parser",
    "zip",
}

EXPECTED_SOURCE_AUTHORITY = {
    "private_domain_crate": True,
    "local_signed_bundle_review": True,
    "disabled_atomic_install_store": True,
    "component_host_conformance": True,
    "renderer_neutral_review_models": True,
    "signed_action_pack_mapping": True,
    "selected_input_consent_models": True,
    "new_dependencies": sorted(EXPECTED_ACCEPTANCE_DEPENDENCIES),
}

EXPECTED_RELEASE_DENIALS = {
    "component_execution",
    "public_downloads",
    "public_sdk_publish",
    "model_provider_calls",
    "model_tool_calls",
    "automatic_execution",
    "process_launch",
    "network_access",
    "credential_access",
    "pty_input",
}


def validate_acceptance(document: Any) -> None:
    document = _require_exact_keys(
        document,
        {
            "schema",
            "phase",
            "decision",
            "accepted_on",
            "accepted_contract_path",
            "accepted_contract_canonical_sha256",
            "wit_mapping",
            "authorized_source",
            "release_authority",
        },
        "acceptance receipt",
    )
    if (
        document["schema"] != 1
        or document["phase"] != "D7/CP6"
        or document["decision"] != "accepted-for-source-implementation"
        or document["accepted_on"] != "2026-08-25"
        or document["accepted_contract_path"] != CONTRACT_PATH.as_posix()
        or document["accepted_contract_canonical_sha256"]
        != EXPECTED_CANONICAL_SHA256
    ):
        raise EcosystemContractError("acceptance receipt identity changed")

    mapping = _require_exact_keys(
        document["wit_mapping"],
        {"logical_world", "source_world", "reason"},
        "WIT mapping",
    )
    if (
        mapping["logical_world"] != "automexia:ecosystem/suggestion@1"
        or mapping["source_world"] != "extension"
        or "share a namespace" not in mapping["reason"]
    ):
        raise EcosystemContractError("accepted WIT syntax mapping changed")

    source = _require_exact_keys(
        document["authorized_source"], set(EXPECTED_SOURCE_AUTHORITY), "source authority"
    )
    if source != EXPECTED_SOURCE_AUTHORITY:
        raise EcosystemContractError("source implementation authority changed")

    release = _require_exact_keys(
        document["release_authority"], EXPECTED_RELEASE_DENIALS, "release authority"
    )
    if any(value is not False for value in release.values()):
        raise EcosystemContractError("acceptance receipt gained release authority")


def _require_source(
    path: Path,
    markers: tuple[str, ...],
    *,
    forbidden: tuple[str, ...] = (),
    maximum: int = MAX_DOCUMENT_BYTES,
) -> str:
    source = _bounded_text(ROOT / path, maximum, path.as_posix())
    for marker in markers:
        if marker not in source:
            raise EcosystemContractError(f"{path.as_posix()} lost {marker!r}")
    for marker in forbidden:
        if marker in source:
            raise EcosystemContractError(f"{path.as_posix()} gained forbidden {marker!r}")
    return source


def _validate_documents() -> None:
    required = {
        ADR_PATH: (
            "Status: Accepted",
            "accepted for source implementation",
            "activation remains denied",
            EXPECTED_CANONICAL_SHA256,
        ),
        PLAN_PATH: (
            "Status: Partially done overall; fully done locally at the accepted source boundary",
            "## Evidence ledger",
            "### D7.1",
            "### D7.5/CP6.3",
            "## External prerequisites",
        ),
        REFERENCE_PATH: (
            "Status: accepted source implementation; release activation disabled",
            "## Current behavior",
            "## Fixed safety boundary",
            "## Recovery and fallback",
        ),
        Path("docs/ECOSYSTEM-PLATFORM-TESTING.md"): (
            "Windows x86_64",
            "cargo test -p automexia-ecosystem",
            "component-host",
            "External evidence still required",
        ),
    }
    for path, markers in required.items():
        _require_source(path, markers)


def _validate_implementation() -> int:
    workspace = _require_source(
        Path("Cargo.toml"),
        (
            '"automexia-ecosystem"',
            '"automexia-ecosystem-runtime"',
            'zip = { version = "=8.6.0", default-features = false',
            'ed25519-dalek = { version = "3.0.0", default-features = false',
            'wasmtime = { version = "48.0.1", default-features = false',
            'wit-parser = "=0.254.0"',
            'wat = "=1.254.0"',
        ),
    )
    if '"default"' in workspace.split('wasmtime =', 1)[1].splitlines()[0]:
        raise EcosystemContractError("Wasmtime unexpectedly enabled default features")

    lock = _require_source(
        Path("Cargo.lock"),
        (
            'name = "automexia-ecosystem"',
            'name = "automexia-ecosystem-runtime"',
            'name = "ed25519-dalek"',
            'name = "wasmtime"',
            'name = "zip"',
        ),
        maximum=8 * 1024 * 1024,
    )
    del lock

    domain_manifest = _require_source(
        Path("automexia-ecosystem/Cargo.toml"),
        ("publish = false", "proptest = { workspace = true }", "criterion = { workspace = true }"),
    )
    del domain_manifest
    domain_sources = "\n".join(
        _bounded_text(path, MAX_DOCUMENT_BYTES, path.relative_to(ROOT).as_posix())
        for path in sorted((ROOT / "automexia-ecosystem/src").glob("*.rs"))
    )
    for marker in (
        "std::fs",
        "std::process",
        "TcpStream",
        "UdpSocket",
        "Command::new",
        "wasmtime",
        "ZipArchive",
    ):
        if marker in domain_sources:
            raise EcosystemContractError(
                f"pure ecosystem domain gained IO/runtime authority {marker!r}"
            )
    for marker in (
        EXPECTED_CANONICAL_SHA256,
        "decode_strict_json",
        "GrantBinding",
        "FairCallQueue",
        "ModelReview",
        "EcosystemReviewSurface",
    ):
        if marker not in domain_sources:
            raise EcosystemContractError(f"pure ecosystem domain lost {marker!r}")

    _require_source(
        Path("automexia-ecosystem/tests/properties.rs"),
        ("ProptestConfig::with_cases(512)", "GrantError::Revoked", "safe_relative_path"),
    )
    _require_source(
        Path("automexia-ecosystem-runtime/Cargo.toml"),
        (
            'default = []',
            'component-host = ["dep:wasmtime"]',
            'wit-parser = { workspace = true }',
        ),
    )
    _require_source(
        Path("automexia-ecosystem-runtime/src/package.rs"),
        (
            "read_and_verify_local_bundle",
            "verify_signature",
            "ZipArchive",
            "Limits::EXPANDED_BYTES",
            "VerificationReceipt",
        ),
        forbidden=(".extract(", "extract_unwrapped_root_dir"),
    )
    _require_source(
        Path("automexia-ecosystem-runtime/src/store.rs"),
        (
            "MOVEFILE_WRITE_THROUGH",
            "PROTECTED_DACL_SECURITY_INFORMATION",
            "private_permissions_are_safe",
            "remove_owned_tree",
            "Limits::RETAINED_VERSIONS",
            "recovery_prunes_one_interrupted_publish",
        ),
    )
    _require_source(
        Path("automexia-ecosystem-runtime/src/sandbox.rs"),
        (
            "wasm_component_model(true)",
            "consume_fuel(true)",
            "epoch_interruption(true)",
            "StoreLimitsBuilder",
            "ActivationDenied",
            "ReleasePermit",
            "TypedHostGate",
            "epoch_deadline_callback",
            "CompleteCallOnDrop",
            "self.deadline.saturating_duration_since",
            "interrupt_before_store_arming_never_enters_guest",
            "shared_engine_ticks_interrupt_only_the_cancelled_store",
            "completed_call_does_not_disarm_a_reused_cancellation_token",
            "worker_unwind_wakes_and_joins_its_watchdog",
            "epoch_deadline_covers_component_instantiation_start_functions",
        ),
        forbidden=("wasmtime_wasi", "WasiCtx", "Command::new", "std::net"),
    )
    _require_source(
        Path("automexia-ecosystem-runtime/tests/wit_contract.rs"),
        ("Resolve::default()", 'Some("extension")', "ALLOWED_IMPORTS"),
    )
    _require_source(
        Path("wit/automexia-ecosystem-1.0.0/ecosystem.wit"),
        (
            "package automexia:ecosystem@1.0.0;",
            "world extension",
            "import public-context;",
            "import selected-input;",
            "import suggestion;",
            "import diagnostic;",
        ),
        forbidden=("wasi:",),
    )
    _require_source(
        Path("sdk/automexia-ecosystem/README.md"),
        ("private, unpublished", "not a public SDK", "runtime activation remains unavailable"),
    )
    _require_source(
        Path("sdk/automexia-ecosystem/conformance/compatibility-v1.json"),
        ('"runtime_activation": false', '"public_distribution": false', '"default_wasi": "none"'),
    )
    _require_source(
        Path("fuzz/fuzz_targets/ecosystem_bundle.rs"),
        ("fuzz_target!", "verify_bundle_bytes", "RevocationSnapshot"),
    )
    _require_source(
        Path("apps/automexia-terminal/src/automexia/ecosystem.rs"),
        (
            "inspect_local_bundle",
            "install_disabled",
            "model_consent_surface",
            "ActivationDenied",
            "DownloadsDisabled",
        ),
        forbidden=("Command::new", "std::net", "process::"),
    )
    _require_source(
        Path("apps/automexia-terminal/src/automexia/marketplace.rs"),
        (
            "AcceptedSourceDisabled",
            "component_execution: false",
            "downloads: false",
            "model_provider_calls: false",
        ),
    )
    _require_source(
        Path("automexia-ecosystem/src/model_suggestion.rs"),
        (
            "SelectedInput",
            "ProviderCallsDisabled",
            "ConsentAlreadyUsed",
            "selected_text",
        ),
        forbidden=("reqwest", "TcpStream", "Command::new"),
    )
    return 18


def validate_repository(root: Path = ROOT) -> dict[str, int]:
    if root != ROOT:
        raise EcosystemContractError("alternate repository roots are not supported")
    contract_text = _bounded_text(
        ROOT / CONTRACT_PATH, MAX_CONTRACT_BYTES, "D7/CP6 contract"
    )
    document = parse_contract(contract_text)
    counts = validate_contract(document)
    digest = canonical_digest(document)
    if digest != EXPECTED_CANONICAL_SHA256:
        raise EcosystemContractError(
            f"reviewed D7/CP6 contract digest changed: {digest}"
        )

    acceptance_text = _bounded_text(
        ROOT / ACCEPTANCE_PATH, MAX_CONTRACT_BYTES, "D7/CP6 acceptance receipt"
    )
    validate_acceptance(parse_contract(acceptance_text))
    _validate_documents()
    counts["source_files"] = _validate_implementation()
    return counts


def main() -> int:
    try:
        counts = validate_repository()
    except (OSError, UnicodeError, EcosystemContractError) as error:
        print(f"D7/CP6 ecosystem validation failed: {error}", file=sys.stderr)
        return 1
    print(
        "PASS: D7/CP6 accepted source boundary is strict, bounded, and non-activating "
        f"(threats={counts['threats']}, limits={counts['limits']}, "
        f"verification={counts['verification_domains']}, source={counts['source_files']}, "
        f"gates={counts['external_gates']})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
