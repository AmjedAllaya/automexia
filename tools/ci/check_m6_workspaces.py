#!/usr/bin/env python3
"""Validate the semantic M6/F6 declarative-workspace release contract."""

from __future__ import annotations

import json
from pathlib import Path
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "tests/fixtures/connection-hub/m6-workspaces-contract-v1.json"
MAX_EVIDENCE_BYTES = 1_048_576
EXPECTED_LIMITS = {
    "workspaces": 256,
    "windows_per_workspace": 16,
    "panes_per_workspace": 64,
    "connections_per_workspace": 128,
    "recipe_bindings_per_connection": 32,
    "broadcast_targets": 50,
    "broadcast_command_bytes": 8_192,
    "broadcast_arm_ms": 60_000,
    "library_profiles": 10_000,
    "library_document_bytes": 16_777_216,
}
EXPECTED_AUTHORITIES = {
    "process": False,
    "network": False,
    "provider": False,
    "credential": False,
    "pty": False,
    "listener": False,
    "shell_evaluation": False,
    "implicit_enter": False,
}
EXPECTED_BLOCKERS = ["D3ProtectedActivation", "M5NativeLifecycleEvidence"]
REQUIRED_SOURCE_TOKENS = {
    "automexia-connectivity/src/connections/strict_json.rs": {
        "from_json_slice_without_duplicate_keys",
        "duplicate JSON object member name",
        "HashSet::with_capacity",
        "deserializer.end()?",
        "serde_json::from_value",
    },
    "automexia-connectivity/src/connections/automation.rs": {
        "validate_reviewed_recipe_run",
        "disabled_authority_ceiling_is_valid",
        "omitted_hook_count",
        "started_at_ms",
        ".checked_add(",
        "recipe-run fingerprint does not match its contents",
        "timestamp is outside the active step",
        "execution_enabled: false",
    },
    "automexia-connectivity/src/connections/workspace.rs": {
        "validate_broadcast_review",
        "reviewed_at_ms",
        "expire_pending_targets",
        "BroadcastTargetOutcome::Expired",
        "BroadcastAuditOutcome::Expired",
        ".checked_add(",
        ".checked_sub(",
        "broadcast target result predates arming",
        "automatic_reconnect: false",
        "resume_interrupted_actions: false",
        "execution_enabled: false",
    },
    "apps/automexia-terminal/src/automexia/connections/library.rs": {
        "from_json_slice_without_duplicate_keys",
        "compare_and_swap",
        "preview_import_redacted",
        "validate_workspace_document",
    },
    "apps/automexia-terminal/src/automexia/connections/workspaces.rs": {
        "required_profiles",
        "required_profiles.remove",
        "WorkspaceProductErrorCode::ProfileNotFound",
        "resolve_workspace_restore",
        "m6_activation_readiness",
        "D3ProtectedActivation",
        "M5NativeLifecycleEvidence",
        "pub const fn execution_enabled(&self) -> bool {\n        false",
    },
    "apps/automexia-terminal/src/automexia/connections/workspaces_cli.rs": {
        "from_json_slice_without_duplicate_keys",
        "parse_workspace_json",
        '"reviewed_at_ms": review.reviewed_at_ms',
        "No Enter key was sent and no session was opened.",
    },
    "automexia-ui-model/src/connection_hub.rs": {
        "project_workspace_catalog",
        "project_workspace_restore",
        "project_broadcast_review",
        "BroadcastTargetOutcome::Expired",
        '("Expired", "◷", SemanticTone::Warning)',
        "pty_resize_requested: false",
    },
}
REQUIRED_TESTS = {
    "automexia-connectivity/tests/connection_automation_m6.rs": {
        "reviewed_runs_preserve_exact_stage_order_and_no_hooks_keeps_only_planner_steps",
        "lifecycle_enforces_deadlines_bounded_retry_cancellation_and_generation_isolation",
        "reviewed_run_integrity_rejects_forgery_clock_reversal_and_deadline_overflow",
        "arbitrary_remote_code_and_implicit_enter_have_no_m6_contract",
    },
    "automexia-connectivity/tests/workspace_automation_m6.rs": {
        "declarative_workspace_restore_is_review_only_and_never_resumes_live_state",
        "broadcast_results_are_isolated_cancellable_generation_bound_and_redacted",
        "broadcast_clock_integrity_and_expiry_terminalize_every_pending_target",
        "strict_workspace_parsers_reject_unknown_fields_and_oversized_documents",
        "repeated_maximum_broadcast_generations_remain_bounded_and_isolated",
    },
    "apps/automexia-terminal/tests/connection_library.rs": {
        "duplicate_json_keys_fail_closed_for_persistence_and_redacted_import",
        "concurrent_writers_never_both_publish_the_same_reviewed_revision",
        "schema_one_library_loads_as_an_explicit_migration_preview_before_cas",
        "import_and_export_previews_are_redacted_nonexecuting_and_commit_with_cas",
    },
    "apps/automexia-terminal/tests/m6_workspace_product.rs": {
        "product_restore_uses_current_exact_profile_bindings_and_never_resumes_state",
        "product_restore_selects_only_profiles_referenced_by_the_workspace",
        "managed_activation_is_fail_closed_until_every_protected_native_gate_exists",
        "workspace_cli_preview_is_non_mutating_and_apply_is_atomic_and_stale_safe",
        "workspace_cli_previous_schema_migration_requires_explicit_recovery",
        "connection_hub_publishes_saved_workspaces_and_reviews_restore_without_pty_input",
    },
}
REQUIRED_TEST_TOKENS = {
    "apps/automexia-terminal/tests/m6_workspace_product.rs": {
        "0..(MAX_PROFILES - 1)",
        "assert_eq!(document.profiles.profiles.len(), MAX_PROFILES)",
        "document.profiles.profiles.rotate_left(1)",
        "Some(\"production-api\")",
    },
}
FORBIDDEN_MODEL_PRIMITIVES = {
    "std::process",
    "command::new",
    "std::net",
    "tcpstream",
    "tcplistener",
    "std::fs",
    "tokio::process",
    "tokio::net",
    "reqwest",
    "unsafe {",
}
REQUIRED_S1 = {
    "native": "connection-hub-workspaces-review",
    "resource": "connection-hub-workspaces-replacement",
    "accessibility": "connection-hub-workspaces-review",
}
REQUIRED_VISUAL_SURFACES = {
    "connection-hub-workspaces-catalog",
    "connection-hub-workspace-restore",
    "connection-hub-workspace-broadcast",
}
REQUIRED_VISUAL_CAPTURE_COUNT = 9_216


class M6ContractError(ValueError):
    """The M6/F6 contract or one of its evidence owners drifted."""


def bounded_text(path: Path, maximum: int = MAX_EVIDENCE_BYTES) -> str:
    if path.is_symlink() or not path.is_file():
        raise M6ContractError(f"required regular file is missing or linked: {path}")
    if path.stat().st_size > maximum:
        raise M6ContractError(f"M6 evidence exceeds {maximum} bytes: {path}")
    data = path.read_bytes()
    if len(data) > maximum:
        raise M6ContractError(f"M6 evidence grew while reading: {path}")
    return data.decode("utf-8")


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise M6ContractError(f"duplicate M6 contract key: {key}")
        result[key] = value
    return result


def load_json(path: Path) -> Any:
    return json.loads(bounded_text(path), object_pairs_hook=reject_duplicate_keys)


def load_contract(path: Path = CONTRACT) -> dict[str, Any]:
    document = load_json(path)
    expected_keys = {
        "schema", "phase", "status", "limits", "authorities",
        "activation_blockers", "model_files", "application_files", "ui_files",
        "test_files", "fuzz_target", "benchmark", "s1_policy", "documents",
    }
    if not isinstance(document, dict) or set(document) != expected_keys:
        raise M6ContractError("M6 contract keys changed")
    if (document["schema"], document["phase"], document["status"]) != (
        1,
        "M6/F6",
        "implemented-review-only-external-activation-and-native-evidence-pending",
    ):
        raise M6ContractError("M6 contract identity changed")
    if document["limits"] != EXPECTED_LIMITS:
        raise M6ContractError("M6 limits changed")
    if document["authorities"] != EXPECTED_AUTHORITIES:
        raise M6ContractError("M6 disabled authorities changed")
    if document["activation_blockers"] != EXPECTED_BLOCKERS:
        raise M6ContractError("M6 activation blockers changed")
    for key in ("model_files", "application_files", "ui_files", "test_files", "documents"):
        values = document[key]
        if not isinstance(values, list) or not values or len(values) != len(set(values)):
            raise M6ContractError(f"M6 {key} must contain unique entries")
    expected_files = {
        *REQUIRED_SOURCE_TOKENS,
        *REQUIRED_TESTS,
    }
    declared_files = {
        *document["model_files"],
        *document["application_files"],
        *document["ui_files"],
        *document["test_files"],
    }
    if declared_files != expected_files:
        raise M6ContractError("M6 declared source or test ownership changed")
    return document


def require_tokens(relative: str, tokens: set[str], root: Path) -> str:
    source = bounded_text(root / relative)
    missing = sorted(token for token in tokens if token not in source)
    if missing:
        raise M6ContractError(f"{relative} is missing M6 evidence: {missing}")
    return source


def validate_s1(relative: str, root: Path) -> None:
    policy = load_json(root / relative)
    suites = policy.get("required_suites") if isinstance(policy, dict) else None
    if not isinstance(suites, list):
        raise M6ContractError("S1 policy has no bounded required_suites")
    found = {
        "native": False,
        "resource": False,
        "visual": False,
        "accessibility": False,
    }
    for suite in suites:
        domain = suite.get("domain")
        coverage = suite.get("coverage", {})
        if domain == "native":
            found["native"] = True
            if REQUIRED_S1["native"] not in coverage.get("scenarios", []):
                raise M6ContractError("native S1 suite is missing the M6 workspace scenario")
        if suite.get("tool") == "native-resource":
            found["resource"] = True
            if REQUIRED_S1["resource"] not in coverage.get("scenarios", []):
                raise M6ContractError("resource S1 suite is missing M6 replacement coverage")
        if domain == "visual":
            found["visual"] = True
            if not REQUIRED_VISUAL_SURFACES.issubset(coverage.get("surfaces", [])):
                raise M6ContractError("visual S1 suite is missing M6 workspace surfaces")
            if coverage.get("capture_count") != REQUIRED_VISUAL_CAPTURE_COUNT:
                raise M6ContractError("visual S1 suite has the wrong M6 capture count")
        if domain == "accessibility":
            found["accessibility"] = True
            if REQUIRED_S1["accessibility"] not in coverage.get("tasks", []):
                raise M6ContractError("accessibility S1 suite is missing M6 workspace review")
    missing = sorted(name for name, present in found.items() if not present)
    if missing:
        raise M6ContractError(f"S1 policy is missing required M6 suites: {missing}")


def validate_repository(root: Path = ROOT) -> dict[str, int]:
    contract = load_contract(root / CONTRACT.relative_to(ROOT))
    sources = {
        relative: require_tokens(relative, tokens, root)
        for relative, tokens in REQUIRED_SOURCE_TOKENS.items()
    }
    model_text = "\n".join(
        sources[relative].split("#[cfg(test)]", 1)[0]
        for relative in contract["model_files"]
    ).casefold()
    forbidden = next(
        (token for token in sorted(FORBIDDEN_MODEL_PRIMITIVES) if token in model_text),
        None,
    )
    if forbidden:
        raise M6ContractError(f"M6 crossed its capability-free model boundary: {forbidden}")
    for relative, names in REQUIRED_TESTS.items():
        source = bounded_text(root / relative)
        missing = sorted(name for name in names if f"fn {name}(" not in source)
        if missing:
            raise M6ContractError(f"{relative} is missing M6 tests: {missing}")
    for relative, tokens in REQUIRED_TEST_TOKENS.items():
        require_tokens(relative, tokens, root)
    require_tokens(
        contract["fuzz_target"],
        {"fuzz_target!", "parse_workspace_json", "review_broadcast"},
        root,
    )
    require_tokens(
        contract["benchmark"],
        {
            "m6_workspace_and_broadcast_planning",
            "connection_plan_workspace_restore_128_connections",
            "workspace_parse_strict_json_16_windows_64_panes_128_connections",
            "MAX_WORKSPACE_CONNECTIONS",
            "resolve_workspace_restore",
            "review_broadcast",
        },
        root,
    )
    validate_s1(contract["s1_policy"], root)
    for relative in contract["documents"]:
        source = bounded_text(root / relative)
        if "M6" not in source and "F6" not in source:
            raise M6ContractError(f"{relative} does not identify M6/F6")
    require_tokens(
        ".github/workflows/ci.yml",
        {"python tools/ci/check_m6_workspaces.py", "python tools/ci/test_m6_workspaces.py"},
        root,
    )
    require_tokens(
        "tools/ci/qa.py",
        {"tools/ci/check_m6_workspaces.py", "tools/ci/test_m6_workspaces.py"},
        root,
    )
    return {
        "sources": len(REQUIRED_SOURCE_TOKENS),
        "tests": sum(len(names) for names in REQUIRED_TESTS.values()),
        "limits": len(EXPECTED_LIMITS),
        "s1_surfaces": len(REQUIRED_VISUAL_SURFACES),
    }


def main() -> int:
    try:
        counts = validate_repository()
    except (M6ContractError, OSError, UnicodeError, json.JSONDecodeError) as error:
        print(f"M6/F6 workspace validation failed: {error}", file=sys.stderr)
        return 1
    print(f"PASS: M6/F6 review-only workspace contract is frozen ({counts})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
