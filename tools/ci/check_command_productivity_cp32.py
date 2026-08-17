#!/usr/bin/env python3
"""Validate reviewed, disabled-by-default CP3.2 DevOps Quick Action packs."""

from __future__ import annotations

import json
from pathlib import Path
import sys
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "tests/fixtures/command-productivity/cp32-contract-v1.json"
MAX_POLICY_BYTES = 262_144
EXPECTED_PROVIDERS = [
    "aws", "azure", "docker", "gcloud", "git", "helm", "kubernetes",
    "openshift", "openssh", "opentofu", "terraform",
]
EXPECTED_ACTION_IDS = [
    "aws.caller-identity",
    "aws.regions",
    "aws.sso-login",
    "azure.account-list",
    "azure.account-set",
    "azure.account-show",
    "docker.compose-ps",
    "docker.info",
    "docker.ps",
    "gcloud.config-list",
    "gcloud.project-set",
    "gcloud.projects-list",
    "git.log-recent",
    "git.status",
    "git.switch",
    "helm.get-values",
    "helm.list",
    "helm.status",
    "kubernetes.current-context",
    "kubernetes.get-pods",
    "kubernetes.use-context",
    "openshift.get-pods",
    "openshift.project",
    "openshift.status",
    "openssh.connect",
    "openssh.list-key-algorithms",
    "openssh.print-config",
    "opentofu.validate",
    "opentofu.workspace-select",
    "opentofu.workspace-show",
    "terraform.validate",
    "terraform.workspace-select",
    "terraform.workspace-show",
]
EXPECTED_INVENTORY = {
    "packs": 11,
    "actions_per_pack": 3,
    "actions": 33,
    "schema_version": 1,
    "generator": "automexia-devops-pack-registry-v1",
}
EXPECTED_EFFECTS = [
    "inspection", "bounded-mutation", "context-change", "authentication",
    "destructive", "privileged",
]
EXPECTED_COMMANDS = ["list", "show", "doctor", "enable"]
EXPECTED_SECURITY = {
    "builtins_disabled_by_default": True,
    "aliases_disabled_by_default": True,
    "builtin_manifest_identity": True,
    "custom_overlays_user_provenance": True,
    "context_aliases_denied": True,
    "authentication_aliases_denied": True,
    "destructive_aliases_denied": True,
    "privileged_aliases_denied": True,
    "doctor_strictly_read_only": True,
    "provider_execution": False,
    "credential_reads": False,
    "network_access": False,
    "compare_and_swap_writes": True,
    "enable_never_overwrites": True,
}
EXPECTED_LIFECYCLE = {
    "version_health": True,
    "provider_absence": True,
    "completion_health": True,
    "overlay_preservation": True,
    "stale_overlay_rejection": True,
    "deprecation_mapping": True,
    "version_regression_rejection": True,
}
EXPECTED_SOURCE_FILES = [
    "automexia-devops/src/actions/packs.rs",
    "automexia-devops/src/actions/validation.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/packs_cli.rs",
    "apps/automexia-terminal/src/cli.rs",
    "automexia-devops/tests/quick_action_packs.rs",
]
EXPECTED_TESTS = [
    "registry_contains_the_exact_reviewed_provider_set",
    "manifests_are_https_versioned_sorted_and_valid",
    "builtins_are_disabled_unaliased_and_insert_only",
    "every_action_materializes_only_after_explicit_selection",
    "effect_classification_denies_unsafe_aliases",
    "health_is_truthful_for_absence_versions_and_completion",
    "hostile_or_ambiguous_health_input_fails_closed",
    "changed_manifest_content_requires_a_new_version",
    "updates_preserve_overlays_and_explain_deprecations",
    "stale_overlays_are_rejected",
    "registry_effects_cover_read_and_mutating_risk_floors",
]
EXPECTED_BENCHMARK = "automexia-devops/benches/quick_actions.rs"
EXPECTED_FUZZ_TARGET = "fuzz/fuzz_targets/quick_action_packs.rs"
EXPECTED_DOCUMENTS = [
    "docs/ARCHITECTURE.md", "docs/COMMAND-PRODUCTIVITY.md",
    "docs/DEVOPS-ALIASES.md", "docs/PHASE-IMPLEMENTATION-AUDIT.md",
    "docs/ROADMAP.md", "docs/TESTING.md",
]


class Cp32Error(ValueError):
    pass


def bounded_text(path: Path, maximum: int = MAX_POLICY_BYTES) -> str:
    if path.is_symlink() or not path.is_file():
        raise Cp32Error(f"required regular file is missing or linked: {path}")
    if path.stat().st_size > maximum:
        raise Cp32Error(f"CP3.2 evidence exceeds {maximum} bytes: {path}")
    data = path.read_bytes()
    if len(data) > maximum:
        raise Cp32Error(f"CP3.2 evidence grew while reading: {path}")
    return data.decode("utf-8")


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise Cp32Error(f"duplicate CP3.2 contract key: {key}")
        result[key] = value
    return result


def load_contract(path: Path = CONTRACT) -> dict[str, Any]:
    document = json.loads(bounded_text(path), object_pairs_hook=reject_duplicate_keys)
    expected_keys = {
        "schema", "phase", "status", "providers", "action_ids", "inventory", "effects",
        "management_commands", "security", "lifecycle", "source_files",
        "required_tests", "benchmark", "fuzz_target", "documents",
    }
    if not isinstance(document, dict) or set(document) != expected_keys:
        raise Cp32Error("CP3.2 contract keys changed")
    if (document["schema"], document["phase"], document["status"]) != (
        1, "CP3.2", "fully-done-reviewed-devops-packs"
    ):
        raise Cp32Error("CP3.2 contract identity changed")
    expected = (
        ("providers", EXPECTED_PROVIDERS),
        ("action_ids", EXPECTED_ACTION_IDS),
        ("inventory", EXPECTED_INVENTORY),
        ("effects", EXPECTED_EFFECTS),
        ("management_commands", EXPECTED_COMMANDS),
        ("security", EXPECTED_SECURITY),
        ("lifecycle", EXPECTED_LIFECYCLE),
        ("source_files", EXPECTED_SOURCE_FILES),
        ("required_tests", EXPECTED_TESTS),
        ("benchmark", EXPECTED_BENCHMARK),
        ("fuzz_target", EXPECTED_FUZZ_TARGET),
        ("documents", EXPECTED_DOCUMENTS),
    )
    for key, value in expected:
        if document[key] != value:
            raise Cp32Error(f"CP3.2 {key.replace('_', ' ')} changed")
    for key in ("providers", "action_ids", "effects", "management_commands", "source_files", "required_tests", "documents"):
        values = document[key]
        if not values or len(values) != len(set(values)):
            raise Cp32Error(f"CP3.2 {key} must contain unique entries")
    return document


def require_tokens(relative: str, tokens: set[str], root: Path = ROOT) -> str:
    source = bounded_text(root / relative)
    missing = sorted(token for token in tokens if token not in source)
    if missing:
        raise Cp32Error(f"{relative} is missing CP3.2 evidence: {missing}")
    return source


def validate_sources(document: dict[str, Any], root: Path = ROOT) -> dict[str, int]:
    registry = require_tokens(document["source_files"][0], {
        "PACK_REGISTRY_GENERATOR", "BuiltinDisabled", "alias_projection.is_some()",
        "validate_pack_registry", "evaluate_pack_health", "plan_pack_update",
        "StaleOverlay", "VersionRegression", "ContextChange", "Authentication",
        "Destructive", "Privileged", "materialize_pack_action",
    } | {f'id: "{provider}"' for provider in document["providers"]}, root)
    require_tokens(document["source_files"][1], {
        "BuiltinAliasDenied", "builtin-alias-denied", "BuiltinManifestMismatch",
        "builtin-manifest-mismatch", "validate_builtin_alias",
    }, root)
    require_tokens(document["source_files"][2], {
        "PacksAction::List", "PacksAction::Show", "PacksAction::Doctor",
        "PacksAction::Enable", "open_existing_read_only", "if apply",
        "expected_revision", "alias=disabled", "provider_processes_started",
    }, root)
    require_tokens(document["source_files"][3], {
        "enum PacksAction", "conflicts_with = \"missing\"",
        "requires = \"expected_revision\"", "pack_enable_is_dry_run",
    }, root)
    tests = require_tokens(document["source_files"][4], {
        "BuiltinAliasDenied", "AliasRiskDenied", "PackHealthState::Missing",
        "PackHealthState::UnsupportedVersion", "PackUpdateState::PreservedOverlay",
        "PackUpdateState::Deprecated",
    }, root)
    missing_tests = sorted(name for name in document["required_tests"] if f"fn {name}(" not in tests)
    if missing_tests:
        raise Cp32Error(f"CP3.2 required tests are missing: {missing_tests}")
    require_tokens(document["benchmark"], {
        "quick_action_pack_registry_validate_11_33",
        "quick_action_pack_health_all_11", "validate_pack_registry",
    }, root)
    require_tokens(document["fuzz_target"], {
        "evaluate_pack_health", "plan_pack_update", "base_action_digest: digest",
    }, root)
    require_tokens("fuzz/Cargo.toml", {"name = \"quick_action_packs\""}, root)
    if registry.count("build_pack(PackSpec") != document["inventory"]["packs"]:
        raise Cp32Error("CP3.2 static pack count changed")
    if registry.count("spec(\"") != document["inventory"]["actions"]:
        raise Cp32Error("CP3.2 static action count changed")
    for relative in document["documents"]:
        require_tokens(relative, {"CP3.2"}, root)
    return {
        "providers": len(document["providers"]),
        "actions": document["inventory"]["actions"],
        "commands": len(document["management_commands"]),
        "tests": len(document["required_tests"]),
        "documents": len(document["documents"]),
    }


def validate_repository(root: Path = ROOT) -> dict[str, int]:
    document = load_contract(root / CONTRACT.relative_to(ROOT))
    return validate_sources(document, root)


def main() -> int:
    try:
        counts = validate_repository()
    except (Cp32Error, OSError, UnicodeError, json.JSONDecodeError) as error:
        print(f"CP3.2 pack validation failed: {error}", file=sys.stderr)
        return 1
    print(
        "PASS: CP3.2 reviewed DevOps packs are disabled by default, effect-classified, "
        f"version/completion aware, overlay safe, benchmarked, and fuzzed ({counts})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())