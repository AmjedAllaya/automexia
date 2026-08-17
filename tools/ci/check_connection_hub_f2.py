#!/usr/bin/env python3
"""Validate the capability-free F2/D5.0 connection and Hub model contract."""

from __future__ import annotations

import json
from pathlib import Path
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "tests/fixtures/connection-hub/f2-contract-v1.json"
MAX_EVIDENCE_BYTES = 524_288
EXPECTED_LIMITS = {
    "profiles": 10_000,
    "recipes": 2_000,
    "steps_per_recipe": 64,
    "variables_per_recipe": 64,
    "tunnels_per_profile": 32,
    "tags_per_profile": 32,
    "jump_profiles": 8,
    "display_value_bytes": 4_096,
    "document_bytes": 16_777_216,
    "automatic_attempts": 3,
    "step_timeout_ms": 30_000,
}
EXPECTED_PROVIDERS = [
    "none", "ssh", "aws", "azure", "gcp", "kubernetes", "open-shift",
    "teleport", "open-bao", "local-container",
]
EXPECTED_AUTH_STATES = [
    "unknown", "checking", "ready", "locked", "missing", "expired",
    "mfa-required", "authenticating", "cancelled", "offline", "denied",
    "unsupported", "stale", "error",
]
EXPECTED_RESULT_STATES = [
    "pending", "running", "waiting-for-user", "succeeded", "warning",
    "failed", "cancelled", "skipped-by-user", "offline", "denied",
    "unsupported", "stale", "error",
]
EXPECTED_CONTENT_STATES = [
    "initial-setup", "loading", "empty", "filtered-empty", "ready",
    "partial-failure", "stale", "offline", "denied", "unsupported",
    "extension-crashed", "revoked-capability", "error",
]
EXPECTED_LAYOUTS = [
    "wide-1440x900-100", "medium-1024x768-100",
    "narrow-700x800-200", "extreme-360x640-400",
]
EXPECTED_AUTHORITIES = {
    "process": False,
    "network": False,
    "provider": False,
    "credential": False,
    "pty": False,
    "listener": False,
    "renderer": False,
    "gpu": False,
}
FORBIDDEN_PRIMITIVES = {
    "std::process",
    "command::new",
    "std::net",
    "tcpstream",
    "tcplistener",
    "udpsocket",
    "std::fs",
    "tokio::process",
    "tokio::net",
    "reqwest",
    "hyper::",
    "unsafe {",
}
REQUIRED_SOURCE_TOKENS = {
    "automexia-devops/src/connections/model.rs": {
        "ConnectionDefinition", "ConnectionObservation", "ConnectionIntent",
        "ConnectionReview", "ConnectionReceipt", "ConnectionProfileV1",
        "AutomationRecipeV1", "AutomationStepV1", "TunnelDefinitionV1",
        "ResolvedConnectionPlan", "AuthState", "OperationResultState",
        "MAX_STEPS_PER_RECIPE: usize = 64", "deny_unknown_fields",
    },
    "automexia-devops/src/connections/documents.rs": {
        "validate_profile_document", "validate_recipe_document",
        "DependencyCycle", "MAX_PROFILES", "MAX_RECIPES",
    },
    "automexia-devops/src/connections/validation.rs": {
        "contains_hostile_format", "looks_secret_bearing_name",
        "validate_retry", "dependency_cycle", "MAX_AUTOMATIC_ATTEMPTS",
        "option-like targets are forbidden",
    },
    "automexia-devops/src/connections/planner.rs": {
        "fingerprint_profile", "fingerprint_recipe", "resolve_connection_plan",
        "requested_capabilities.sort", "execution_enabled: false",
        "AuthorityKind::Process", "AuthorityKind::Listener",
    },
    "automexia-devops/src/connections/state.rs": {
        "apply_auth_event", "apply_result_event", "InvalidTransition",
        "AuthState::Denied", "AuthState::Stale", "OperationResultState::Offline",
    },
    "automexia-ui-model/src/connection_hub.rs": {
        "HubLayout", "HubContentState", "background_inert: true",
        "focus_trapped: true", "pty_resize_requested: false",
        "execution_enabled: false", "project_connection_review",
        "project_recipe_planner", "AccessibilityRole::Dialog",
    },
}
REQUIRED_TESTS = {
    "automexia-devops/tests/connection_planning.rs": {
        "strict_profiles_and_recipes_compile_to_a_non_executing_plan",
        "hostile_unknown_secret_command_and_bidi_fields_fail_closed",
        "duplicates_cycles_limits_and_policy_mismatches_are_rejected",
        "every_material_plan_change_invalidates_the_fingerprint",
        "authentication_and_result_reducers_cover_truthful_terminal_states",
        "the_64_step_architecture_limit_is_accepted_but_limit_plus_one_is_not",
        "profile_documents_reject_duplicate_ids_missing_jumps_and_jump_cycles",
        "resolved_plans_reject_cross_recipe_variable_collisions_and_preallocate_step_overflow",
    },
    "automexia-devops/tests/connection_properties.rs": {
        "bounded_printable_unicode_labels_and_targets_validate",
        "every_ascii_control_character_is_rejected",
        "executable_and_capability_input_order_does_not_change_the_plan_fingerprint",
    },
    "automexia-devops/tests/connection_records.rs": {
        "all_top_level_connection_records_are_strict_versioned_and_bounded",
    },
    "automexia-devops/tests/connection_state_matrix.rs": {
        "authentication_reducer_reaches_every_truthful_public_state",
        "result_reducer_reaches_every_truthful_public_state_and_keeps_terminals_terminal",
    },
    "automexia-ui-model/tests/connection_hub.rs": {
        "responsive_projection_is_modal_inert_and_never_requests_execution_or_pty_resize",
        "accessibility_tree_names_provider_target_identity_state_and_risk_without_color",
        "every_empty_failure_and_authentication_state_has_stable_text_and_actions",
        "keyboard_navigation_never_connects_from_the_results_and_restores_focus",
        "recipe_planner_is_bounded_accessible_and_explicitly_dry_run_only",
        "synthetic_provider_and_auth_fixtures_cover_the_frozen_matrices",
        "structured_layout_and_accessibility_goldens_match_the_projection",
    },
    "automexia-ui-model/tests/connection_review.rs": {
        "connection_review_exposes_every_decision_section_and_remains_non_executing",
    },
}


class F2ContractError(ValueError):
    pass


def bounded_text(path: Path, maximum: int = MAX_EVIDENCE_BYTES) -> str:
    if path.is_symlink() or not path.is_file():
        raise F2ContractError(f"required regular file is missing or linked: {path}")
    if path.stat().st_size > maximum:
        raise F2ContractError(f"F2 evidence exceeds {maximum} bytes: {path}")
    data = path.read_bytes()
    if len(data) > maximum:
        raise F2ContractError(f"F2 evidence grew while reading: {path}")
    return data.decode("utf-8")


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise F2ContractError(f"duplicate F2 contract key: {key}")
        result[key] = value
    return result


def load_contract(path: Path = CONTRACT) -> dict[str, Any]:
    document = json.loads(bounded_text(path), object_pairs_hook=reject_duplicate_keys)
    expected_keys = {
        "schema", "phase", "status", "limits", "providers",
        "authentication_states", "result_states", "content_states",
        "layout_fixtures", "authorities", "model_files", "test_files",
        "fixture_files", "fuzz_target", "benchmark", "documents",
    }
    if not isinstance(document, dict) or set(document) != expected_keys:
        raise F2ContractError("F2 contract keys changed")
    if (document["schema"], document["phase"], document["status"]) != (
        1, "F2/D5.0", "implemented-non-executing-external-adr-pending"
    ):
        raise F2ContractError("F2 contract identity changed")
    expected = {
        "limits": EXPECTED_LIMITS,
        "providers": EXPECTED_PROVIDERS,
        "authentication_states": EXPECTED_AUTH_STATES,
        "result_states": EXPECTED_RESULT_STATES,
        "content_states": EXPECTED_CONTENT_STATES,
        "layout_fixtures": EXPECTED_LAYOUTS,
        "authorities": EXPECTED_AUTHORITIES,
    }
    for key, value in expected.items():
        if document[key] != value:
            raise F2ContractError(f"F2 {key} changed")
    for key in ("model_files", "test_files", "fixture_files", "documents"):
        values = document[key]
        if not isinstance(values, list) or not values or len(values) != len(set(values)):
            raise F2ContractError(f"F2 {key} must contain unique entries")
    return document


def require_tokens(relative: str, tokens: set[str], root: Path = ROOT) -> str:
    source = bounded_text(root / relative)
    missing = sorted(token for token in tokens if token not in source)
    if missing:
        raise F2ContractError(f"{relative} is missing F2 evidence: {missing}")
    return source


def validate_sources(document: dict[str, Any], root: Path = ROOT) -> dict[str, int]:
    combined = []
    for relative, tokens in REQUIRED_SOURCE_TOKENS.items():
        combined.append(require_tokens(relative, tokens, root))
    lowered = "\n".join(combined).casefold()
    forbidden = next(
        (token for token in sorted(FORBIDDEN_PRIMITIVES) if token in lowered), None
    )
    if forbidden:
        raise F2ContractError(f"F2 crossed the capability-free boundary: {forbidden}")
    for relative, tests in REQUIRED_TESTS.items():
        source = bounded_text(root / relative)
        missing = sorted(name for name in tests if f"fn {name}(" not in source)
        if missing:
            raise F2ContractError(f"{relative} is missing F2 tests: {missing}")
    for relative in document["fixture_files"]:
        bounded_text(root / relative)
    require_tokens(
        document["fuzz_target"],
        {"fuzz_target!", "parse_profile_json", "parse_recipe_document_json"},
        root,
    )
    require_tokens(
        document["benchmark"],
        {"connection_plan_validate_64_steps", "connection_plan_resolve_64_steps"},
        root,
    )
    for relative in document["documents"]:
        source = bounded_text(root / relative)
        if "F2" not in source and "D5.0" not in source:
            raise F2ContractError(f"{relative} does not identify F2/D5.0")
    return {
        "models": len(document["model_files"]),
        "tests": sum(len(tests) for tests in REQUIRED_TESTS.values()),
        "providers": len(document["providers"]),
        "auth_states": len(document["authentication_states"]),
        "layouts": len(document["layout_fixtures"]),
        "fixtures": len(document["fixture_files"]),
    }


def validate_repository(root: Path = ROOT) -> dict[str, int]:
    document = load_contract(root / CONTRACT.relative_to(ROOT))
    return validate_sources(document, root)


def main() -> int:
    try:
        counts = validate_repository()
    except (F2ContractError, OSError, UnicodeError, json.JSONDecodeError) as error:
        print(f"F2 Connection Hub validation failed: {error}", file=sys.stderr)
        return 1
    print(
        "PASS: F2 models, dry-run planner, state reducers, responsive Hub, "
        f"fixtures, and disabled authority are frozen ({counts})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
