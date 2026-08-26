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
    "unknown", "checking", "available", "refreshing", "ready", "locked",
    "missing", "expired", "mfa-required", "mfa-pending", "browser-pending",
    "device-code-pending", "authenticating", "cancelled", "offline", "denied",
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
FORBIDDEN_VALIDATION_BYPASSES = {
    "automexia-connectivity/src/connections/model.rs": {
        "impl From<ConnectionProfileV1> for ValidatedConnectionProfile",
        "impl From<AutomationRecipeV1> for ValidatedAutomationRecipe",
    },
}
FORBIDDEN_PANIC_PRIMITIVES = {".expect(", ".unwrap("}
REQUIRED_SOURCE_TOKENS = {
    "automexia-connectivity/src/connections/model.rs": {
        "ConnectionDefinition", "ConnectionObservation", "ConnectionIntent",
        "ConnectionReview", "ConnectionReceipt", "ConnectionProfileV1",
        "AutomationRecipeV1", "AutomationStepV1", "TunnelDefinitionV1",
        "ResolvedConnectionPlan", "AuthState", "OperationResultState",
        "MAX_STEPS_PER_RECIPE: usize = 64", "deny_unknown_fields",
        "from_validated", "operation_id: String",
    },
    "automexia-connectivity/src/connections/automation.rs": {
        "RecipeRunMode", "NoHooks", "review_recipe_run",
        "review_remote_initialization", "RemoteOperation",
        "RecipeRunLifecycle", "apply_recipe_run_event",
        "execution_enabled: false", "stale recipe-run generation was rejected",
    },
    "automexia-connectivity/src/connections/workspace.rs": {
        "WorkspaceIntentV1", "WorkspaceRestorePlan", "validate_workspace",
        "resolve_workspace_restore", "automatic_reconnect: false",
        "resume_interrupted_actions: false", "BroadcastReview",
        "review_broadcast", "BroadcastLifecycle", "apply_broadcast_event",
        "MAX_BROADCAST_TARGETS: usize = 50", "execution_enabled: false",
        "lifecycle.approval_fingerprint != review.approval_fingerprint",
    },
    "automexia-connectivity/src/connections/documents.rs": {
        "validate_profile_document", "validate_recipe_document",
        "DependencyCycle", "MAX_PROFILES", "MAX_RECIPES",
    },
    "automexia-connectivity/src/connections/validation.rs": {
        "contains_hostile_format", "looks_secret_bearing_name",
        "validate_retry", "dependency_cycle", "MAX_AUTOMATIC_ATTEMPTS",
        "option-like targets are forbidden", "decision_codes", "executable_ids",
        "validate_resolved_step_policy",
    },
    "automexia-connectivity/src/connections/planner.rs": {
        "fingerprint_profile", "fingerprint_recipe", "resolve_connection_plan",
        "requested_capabilities.sort", "execution_enabled: false", "plan_sequence",
        "AuthorityKind::Process", "AuthorityKind::Listener",
    },
    "automexia-connectivity/src/connections/state.rs": {
        "apply_auth_event", "apply_result_event", "InvalidTransition",
        "AuthState::Denied", "AuthState::Stale", "OperationResultState::Offline",
        "require_current_operation",
    },
    "automexia-ui-model/src/connection_hub.rs": {
        "HubLayout", "HubContentState", "background_inert: true",
        "focus_trapped: true", "pty_resize_requested: false",
        "execution_enabled: false", "project_connection_review",
        "project_recipe_planner", "AccessibilityRole::Dialog",
        "AccessibilityRole::Progress", "action_label", "is_selected",
    },
}
REQUIRED_APPLICATION_TOKENS = {
    "apps/automexia-terminal/src/automexia/connections/library.rs": {
        "CONNECTION_LIBRARY_SCHEMA: u16 = 2", "PrimaryMigrationPreview",
        "LibraryEditPreview", "preview_library_edit", "commit_edit",
        "preview_export_redacted", "preview_import_redacted", "commit_import",
        "PreviewMismatch", "validate_workspace_document",
        "MAX_FRESH_ID_ATTEMPTS", "next_entity_revision",
    },
}
REQUIRED_TESTS = {
    "automexia-connectivity/tests/connection_automation_m6.rs": {
        "reviewed_runs_preserve_exact_stage_order_and_no_hooks_keeps_only_planner_steps",
        "remote_initialization_is_typed_reviewed_and_never_contains_a_command_string",
        "lifecycle_enforces_deadlines_bounded_retry_cancellation_and_generation_isolation",
        "arbitrary_remote_code_and_implicit_enter_have_no_m6_contract",
        "remote_initialization_revalidates_privileged_steps_at_the_review_boundary",
        "lifecycle_rejects_cross_review_substitution_and_unbounded_diagnostics",
    },
    "automexia-connectivity/tests/workspace_automation_m6.rs": {
        "declarative_workspace_restore_is_review_only_and_never_resumes_live_state",
        "clone_and_rebind_create_isolated_ids_and_invalidate_prior_approval",
        "clone_scopes_reused_pane_ids_to_their_own_windows",
        "pane_cycles_cross_window_references_hostile_text_and_stale_profiles_fail_closed",
        "broadcast_requires_exact_preview_production_confirmation_and_explicit_arming",
        "broadcast_results_are_isolated_cancellable_generation_bound_and_redacted",
        "broadcast_rejects_newlines_controls_bidi_duplicates_and_unbounded_targets",
        "strict_workspace_parsers_reject_unknown_fields_and_oversized_documents",
        "repeated_maximum_broadcast_generations_remain_bounded_and_isolated",
        "broadcast_rejects_cross_review_substitution_and_unsafe_diagnostics",
    },
    "apps/automexia-terminal/tests/connection_library.rs": {
        "redacted_workspace_export_scopes_reused_pane_ids_per_window",
        "schema_one_library_loads_as_an_explicit_migration_preview_before_cas",
        "editor_preview_invalidates_recipe_profile_and_workspace_approvals_and_cas_conflicts",
        "import_and_export_previews_are_redacted_nonexecuting_and_commit_with_cas",
        "mismatched_recipe_reference_fingerprint_is_rejected_by_the_library",
    },
    "automexia-connectivity/tests/connection_planning.rs": {
        "strict_profiles_and_recipes_compile_to_a_non_executing_plan",
        "hostile_unknown_secret_command_and_bidi_fields_fail_closed",
        "duplicates_cycles_limits_and_policy_mismatches_are_rejected",
        "every_material_plan_change_invalidates_the_fingerprint",
        "authentication_and_result_reducers_cover_truthful_terminal_states",
        "the_64_step_architecture_limit_is_accepted_but_limit_plus_one_is_not",
        "profile_documents_reject_duplicate_ids_missing_jumps_and_jump_cycles",
        "resolved_plans_reject_cross_recipe_variable_collisions_and_preallocate_step_overflow",
        "plan_context_rejects_hostile_bidi_variable_overrides",
    },
    "automexia-connectivity/tests/connection_properties.rs": {
        "bounded_printable_unicode_labels_and_targets_validate",
        "every_ascii_control_character_is_rejected",
        "executable_and_capability_input_order_does_not_change_the_plan_fingerprint",
    },
    "automexia-connectivity/tests/connection_records.rs": {
        "all_top_level_connection_records_are_strict_versioned_and_bounded",
        "review_records_reject_duplicate_policy_and_executable_entries",
    },
    "automexia-connectivity/tests/connection_state_matrix.rs": {
        "authentication_reducer_reaches_every_truthful_public_state",
        "result_reducer_reaches_every_truthful_public_state_and_keeps_terminals_terminal",
        "late_authentication_results_cannot_cross_operation_generations",
        "authentication_event_ids_use_the_canonical_identifier_contract",
    },
    "automexia-ui-model/tests/connection_hub.rs": {
        "responsive_projection_is_modal_inert_and_never_requests_execution_or_pty_resize",
        "accessibility_tree_names_provider_target_identity_state_and_risk_without_color",
        "every_empty_failure_and_authentication_state_has_stable_text_and_actions",
        "keyboard_navigation_never_connects_from_the_results_and_restores_focus",
        "recipe_planner_is_bounded_accessible_and_explicitly_dry_run_only",
        "synthetic_provider_and_auth_fixtures_cover_the_frozen_matrices",
        "structured_layout_and_accessibility_goldens_match_the_projection",
        "missing_selection_still_exposes_one_managed_grid_focus_target",
        "loading_state_exposes_live_progress_semantics",
        "modal_tab_cycle_stays_on_controls_for_the_active_route",
        "planner_accessibility_summary_does_not_expose_public_value_contents",
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
    sources = {
        relative: require_tokens(relative, tokens, root)
        for relative, tokens in REQUIRED_SOURCE_TOKENS.items()
    }
    combined = "\n".join(sources.values())
    lowered = combined.casefold()
    forbidden = next(
        (token for token in sorted(FORBIDDEN_PRIMITIVES) if token in lowered), None
    )
    if forbidden:
        raise F2ContractError(f"F2 crossed the capability-free boundary: {forbidden}")
    panic_primitive = next(
        (token for token in sorted(FORBIDDEN_PANIC_PRIMITIVES) if token in combined), None
    )
    if panic_primitive:
        raise F2ContractError(f"F2 production model contains a panic primitive: {panic_primitive}")
    for relative, bypasses in FORBIDDEN_VALIDATION_BYPASSES.items():
        bypass = next((token for token in sorted(bypasses) if token in sources[relative]), None)
        if bypass:
            raise F2ContractError(f"F2 validated wrapper can be forged: {bypass}")
    for relative, tokens in REQUIRED_APPLICATION_TOKENS.items():
        production = require_tokens(relative, tokens, root).split("#[cfg(test)]", 1)[0]
        forbidden_application = next(
            (
                token
                for token in ("std::process", "std::net", "unsafe {", ".unwrap(", ".expect(")
                if token in production.casefold()
            ),
            None,
        )
        if forbidden_application:
            raise F2ContractError(
                f"M6 library crossed its reviewed storage boundary: {forbidden_application}"
            )
    for relative, tests in REQUIRED_TESTS.items():
        source = bounded_text(root / relative)
        missing = sorted(name for name in tests if f"fn {name}(" not in source)
        if missing:
            raise F2ContractError(f"{relative} is missing F2 tests: {missing}")
    for relative in document["fixture_files"]:
        bounded_text(root / relative)
    require_tokens(
        document["fuzz_target"],
        {"fuzz_target!", "parse_profile_json", "parse_recipe_document_json", "parse_workspace_json"},
        root,
    )
    require_tokens(
        document["benchmark"],
        {"connection_plan_validate_64_steps", "connection_plan_resolve_64_steps", "m6_workspace_and_broadcast_planning"},
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
