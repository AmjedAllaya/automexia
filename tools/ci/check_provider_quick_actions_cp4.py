#!/usr/bin/env python3
"""Validate the bounded nonactivating M13/F13/CP4 provider-action contract."""

from __future__ import annotations

import json
from pathlib import Path
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "tests/fixtures/command-productivity/cp4-contract-v1.json"
MAX_EVIDENCE_BYTES = 1_048_576
EXPECTED_LIMITS = {
    "provider_actions": 256,
    "actions_per_provider": 16,
    "presentation_fields": 32,
    "published_routes": 32,
    "search_results": 128,
}
EXPECTED_PROVIDERS = [
    "ssh", "aws", "azure", "gcp", "kubernetes", "openshift", "teleport",
]
EXPECTED_PUBLICATION_OUTCOMES = ["published", "unchanged", "cleared"]
EXPECTED_DECISIONS = [
    "insert-without-enter", "broker-required", "refreshing", "stale",
    "expired", "offline", "unavailable", "error", "replaced",
]
EXPECTED_AUTHORITIES = {
    "provider_refresh": False,
    "process": False,
    "network": False,
    "credential": False,
    "filesystem": False,
    "pty": False,
    "renderer": False,
    "exact_execution": False,
    "startup": False,
    "keystroke_provider_work": False,
    "persistence": False,
}
PROVIDER_ADAPTER_SOURCES = {
    "extensions/devops-aws/src/lib.rs",
    "extensions/devops-azure/src/lib.rs",
    "extensions/devops-gcp/src/lib.rs",
    "extensions/devops-kubernetes/src/implementation.rs",
    "extensions/devops-openshift/src/implementation.rs",
    "extensions/devops-teleport/src/lib.rs",
}
PRODUCT_PUBLICATION_SOURCES = {
    "apps/automexia-terminal/src/automexia/connections/providers.rs",
    "apps/automexia-terminal/src/automexia/connections/controller.rs",
}
INTERACTIVE_SOURCES = {
    "apps/automexia-terminal/src/automexia/quick_actions/worker.rs",
    "apps/automexia-terminal/src/screen/action_surface.rs",
    "apps/automexia-terminal/src/renderer/command_palette.rs",
}
ADAPTER_CRATE_MARKERS = {
    "automexia_devops_aws::",
    "automexia_devops_azure::",
    "automexia_devops_gcp::",
    "automexia_devops_kubernetes::",
    "automexia_devops_openshift::",
    "automexia_devops_teleport::",
}
FORBIDDEN_AUTHORITY_PRIMITIVES = {
    "std::process",
    "command::new",
    ".spawn(",
    "std::net",
    "std::fs",
    "std::env",
    "tokio::",
    "async_std::",
    "reqwest",
    "ureq::",
    "hyper::",
    "unsafe {",
}
REQUIRED_SOURCE_TOKENS = {
    "automexia-devops/src/actions/provider.rs": {
        "MAX_PROVIDER_ACTIONS: usize = 256",
        "MAX_PROVIDER_ACTIONS_PER_PROVIDER: usize = 16",
        "MAX_PROVIDER_PRESENTATION_FIELDS: usize = 32",
        "pub fn build_provider_action_candidate",
        "pub fn build_provider_action_snapshot",
        "pub fn build_ssh_provider_action",
        "pub fn revalidate_provider_action",
        "ProviderActionDecision::InsertWithoutEnter",
        "ProviderActionDecision::BrokerRequired",
        ".field(\"field_count\"",
    },
    "automexia-devops/src/actions/activation.rs": {
        "pub fn merge_action_search_hits",
        "winner.shadowed_count",
        "sort_and_truncate_hits",
    },
    "apps/automexia-terminal/src/automexia/connections/providers.rs": {
        "pub struct ProviderProductPublication",
        "publication: None",
        "publication: Some",
        ".field(\"capsule\", &\"<redacted-public-provider-capsule>\")",
        "pub fn publication",
    },
    "apps/automexia-terminal/src/automexia/connections/controller.rs": {
        "pub fn provider_action_publication",
        "runtime_snapshot.providers.publication()",
    },
    "apps/automexia-terminal/src/automexia/quick_actions/providers.rs": {
        "pub fn compose_provider_action_snapshot",
        "ProviderKind::Ssh",
        "ProviderKind::Aws",
        "ProviderKind::Azure",
        "ProviderKind::Gcp",
        "ProviderKind::Kubernetes",
        "ProviderKind::OpenShift",
        "ProviderKind::Teleport",
        "ProviderKind::OpenBao",
        "UnsupportedProvider",
        "pub struct ProviderActionPublisher",
        "pub fn sync_route",
        "provider_snapshot_matches",
        "clear_provider_snapshot",
        "capsule.session_id != current_session_id",
    },
    "apps/automexia-terminal/src/automexia/quick_actions/worker.rs": {
        "const MAX_RESULT_ROUTES: usize = 32",
        "pub fn publish_provider_snapshot",
        "pub fn clear_provider_snapshot",
        "pub fn revalidate_provider_binding",
        "StaleProviderSnapshot",
        "merge_action_search_hits(provider_hits, base_hits.clone())",
        "current_provider_key",
        "forget_route",
        "provider_snapshot_matches",
    },
    "automexia-ui-model/src/quick_actions.rs": {
        "pub provider_context: bool",
        "pub fn with_provider_context",
        "requires_second_confirmation |= requires_production_confirmation",
    },
    "apps/automexia-terminal/src/screen/action_surface.rs": {
        "ensure_selected_provider_action_authorized",
        "revalidate_provider_binding",
        "provider_unavailable_reason",
        "ambient context is not allowed",
        "ProviderActionDecision::Expired",
        "ProviderActionDecision::Offline",
        "ProviderActionDecision::Replaced",
        "provider_action_publication",
        ".sync_route(",
        "provider_publication_notice",
    },
    "apps/automexia-terminal/src/renderer/command_palette.rs": {
        "item.provider_context",
        "CommandIcon::Connections",
        "BRAND_CYAN",
        "command_context_label",
    },
}
REQUIRED_TESTS = {
    "automexia-devops/tests/provider_quick_actions_cp4.rs": {
        "current_cached_context_is_searchable_and_revalidated_without_secret_debug",
        "noncurrent_expired_and_broker_required_actions_fail_closed_with_exact_states",
        "generations_hostile_targets_and_duplicate_contributions_are_rejected",
        "ssh_target_is_exact_insert_without_enter_and_requires_one_cached_context",
    },
    "automexia-devops/tests/quick_action_activation.rs": {
        "cached_provider_hits_take_precedence_without_duplicate_rows",
    },
    "extensions/devops-aws/src/lib.rs": {
        "provider_quick_action_reuses_exact_sts_grammar_and_public_account",
    },
    "extensions/devops-azure/src/lib.rs": {
        "provider_quick_action_reuses_exact_account_grammar_and_subscription",
    },
    "extensions/devops-gcp/src/lib.rs": {
        "provider_quick_action_reuses_exact_project_grammar_and_target",
    },
    "extensions/devops-kubernetes/tests/contracts.rs": {
        "capsule_pins_public_context_and_non_mutating_exact_cli_plans",
    },
    "extensions/devops-openshift/tests/contracts.rs": {
        "project_inspection_and_rsh_never_mutate_global_project",
    },
    "extensions/devops-teleport/tests/contracts.rs": {
        "version_and_status_plans_are_local_exact_and_agent_isolated",
    },
    "apps/automexia-terminal/src/automexia/connections/providers.rs": {
        "publication_is_public_bounded_and_stale_safe",
        "ssh_context_is_retained_without_claiming_provider_catalog",
        "revoke_is_session_bound_and_returns_to_nonconfigured_catalog",
    },
    "apps/automexia-terminal/tests/cp4_provider_product_publication.rs": {
        "cached_provider_product_reaches_route_scoped_quick_actions_without_execution",
        "route_mismatch_revocation_and_redacted_failures_are_fail_closed",
    },
    "apps/automexia-terminal/src/automexia/quick_actions/providers.rs": {
        "multi_provider_composition_is_complete_sorted_and_nonexecuting",
        "unsupported_provider_prevents_partial_snapshot_publication",
    },
    "apps/automexia-terminal/src/automexia/quick_actions/worker.rs": {
        "provider_snapshots_are_route_isolated_monotonic_and_revalidated",
        "provider_snapshot_capacity_recovers_after_route_cleanup",
    },
    "automexia-ui-model/src/quick_actions.rs": {
        "provider_context_is_concise_visible_accessible_and_production_confirmed",
    },
    "apps/automexia-terminal/src/screen/action_surface.rs": {
        "final_confirmation_keeps_exact_provider_context_accessible",
        "provider_failure_states_are_actionable_and_never_claim_execution",
        "provider_publication_notices_are_accessible_and_actionable",
    },
    "apps/automexia-terminal/src/renderer/command_palette.rs": {
        "provider_actions_use_connection_visuals_and_show_context_in_review",
    },
}


class CP4ContractError(ValueError):
    """The CP4 contract or implementation boundary drifted."""


def bounded_text(path: Path, maximum: int = MAX_EVIDENCE_BYTES) -> str:
    if path.is_symlink() or not path.is_file():
        raise CP4ContractError(f"required regular CP4 file is missing or linked: {path}")
    if path.stat().st_size > maximum:
        raise CP4ContractError(f"CP4 evidence exceeds {maximum} bytes: {path}")
    data = path.read_bytes()
    if len(data) > maximum:
        raise CP4ContractError(f"CP4 evidence grew while reading: {path}")
    return data.decode("utf-8")


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise CP4ContractError(f"duplicate CP4 contract key: {key}")
        result[key] = value
    return result


def load_contract(path: Path = CONTRACT) -> dict[str, Any]:
    document = json.loads(bounded_text(path), object_pairs_hook=reject_duplicate_keys)
    expected_keys = {
        "schema", "phase", "status", "publication_outcomes", "limits", "providers", "decisions",
        "authorities", "source_files", "test_files", "fuzz_target",
        "benchmark", "documents",
    }
    if not isinstance(document, dict) or set(document) != expected_keys:
        raise CP4ContractError("CP4 contract keys changed")
    if (document["schema"], document["phase"], document["status"]) != (
        1, "M13/F13/CP4", "product-integrated-nonactivated",
    ):
        raise CP4ContractError("CP4 contract identity changed")
    expected = {
        "limits": EXPECTED_LIMITS,
        "providers": EXPECTED_PROVIDERS,
        "publication_outcomes": EXPECTED_PUBLICATION_OUTCOMES,
        "decisions": EXPECTED_DECISIONS,
        "authorities": EXPECTED_AUTHORITIES,
    }
    for key, value in expected.items():
        if document[key] != value:
            raise CP4ContractError(f"CP4 {key} changed")
    for key in ("source_files", "test_files", "documents"):
        values = document[key]
        if not isinstance(values, list) or not values or len(values) != len(set(values)):
            raise CP4ContractError(f"CP4 {key} must be a non-empty unique list")
    return document


def validate_sources(document: dict[str, Any]) -> None:
    declared = set(document["source_files"])
    if (
        not PROVIDER_ADAPTER_SOURCES.issubset(declared)
        or not PRODUCT_PUBLICATION_SOURCES.issubset(declared)
    ):
        raise CP4ContractError("CP4 provider/product source set is incomplete")
    for relative in document["source_files"]:
        bounded_text(ROOT / relative)
    for relative, tokens in REQUIRED_SOURCE_TOKENS.items():
        source = bounded_text(ROOT / relative)
        missing = sorted(token for token in tokens if token not in source)
        if missing:
            raise CP4ContractError(f"missing CP4 source evidence in {relative}: {missing}")

    action_surface = bounded_text(
        ROOT / "apps/automexia-terminal/src/screen/action_surface.rs"
    )
    if action_surface.count("self.sync_provider_actions_for_current_route()") != 2:
        raise CP4ContractError(
            "CP4 product synchronization must occur at open and final authorization"
        )

    for relative in (
        "automexia-devops/src/actions/provider.rs",
        "apps/automexia-terminal/src/automexia/connections/providers.rs",
        "apps/automexia-terminal/src/automexia/quick_actions/providers.rs",
    ):
        source = bounded_text(ROOT / relative).lower()
        widened = sorted(
            primitive for primitive in FORBIDDEN_AUTHORITY_PRIMITIVES
            if primitive in source
        )
        if widened:
            raise CP4ContractError(f"{relative} gained CP4 runtime authority: {widened}")

    for relative in sorted(INTERACTIVE_SOURCES):
        source = bounded_text(ROOT / relative).lower()
        imported = sorted(marker for marker in ADAPTER_CRATE_MARKERS if marker in source)
        if imported or "compose_provider_action_snapshot(" in source:
            raise CP4ContractError(
                f"{relative} performs provider composition on an interactive path: {imported}"
            )


def validate_evidence(document: dict[str, Any]) -> None:
    for relative, tests in REQUIRED_TESTS.items():
        source = bounded_text(ROOT / relative)
        missing = sorted(name for name in tests if f"fn {name}(" not in source)
        if missing:
            raise CP4ContractError(f"missing CP4 tests in {relative}: {missing}")
    for key in ("test_files", "documents"):
        for relative in document[key]:
            bounded_text(ROOT / relative)
    fuzz = bounded_text(ROOT / document["fuzz_target"])
    for token in (
        "parse_provider_capsule_json", "build_provider_action_candidate",
        "build_provider_action_snapshot", "revalidate_provider_action",
        "MAX_FUZZ_INPUT_BYTES",
    ):
        if token not in fuzz:
            raise CP4ContractError(f"CP4 fuzz target is missing {token}")
    benchmark = bounded_text(ROOT / document["benchmark"])
    for token in (
        "provider_quick_action_snapshot_build_16",
        "provider_quick_action_cached_search_16",
    ):
        if token not in benchmark:
            raise CP4ContractError(f"CP4 benchmark is missing {token}")


def validate_repository() -> dict[str, int]:
    document = load_contract()
    validate_sources(document)
    validate_evidence(document)
    return {
        "providers": len(document["providers"]),
        "publication_outcomes": len(document["publication_outcomes"]),
        "decisions": len(document["decisions"]),
        "authorities_denied": sum(not value for value in document["authorities"].values()),
        "tests": sum(len(tests) for tests in REQUIRED_TESTS.values()),
    }


if __name__ == "__main__":
    try:
        counts = validate_repository()
    except (CP4ContractError, OSError, UnicodeError, json.JSONDecodeError) as exc:
        print(f"CP4 provider-action validation failed: {exc}", file=sys.stderr)
        raise SystemExit(1) from exc
    print(
        "PASS: M13/F13/CP4 provider actions are bounded and nonactivating "
        f"(providers={counts['providers']}, publication_outcomes={counts['publication_outcomes']}, decisions={counts['decisions']}, "
        f"authorities_denied={counts['authorities_denied']}, tests={counts['tests']})"
    )