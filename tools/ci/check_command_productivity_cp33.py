#!/usr/bin/env python3
"""Validate CP3.3 native imports and trusted workspace task bridges."""

from __future__ import annotations

import json
from pathlib import Path
import sys
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "tests/fixtures/command-productivity/cp33-contract-v1.json"
MAX_POLICY_BYTES = 262_144
EXPECTED_NATIVE_SOURCES = ["powershell", "bash", "zsh", "fish", "cmd", "git"]
EXPECTED_TASK_RUNNERS = ["just", "task", "mise"]
EXPECTED_COMMANDS = [
    "import-aliases",
    "task-put",
    "task-remove",
    "workspace-trust",
    "workspace-revoke",
    "workspace-doctor",
]
EXPECTED_SECURITY = {
    "explicit_selection": True,
    "dry_run_default": True,
    "bounded_no_follow_reads": True,
    "no_native_listing_or_discovery": True,
    "no_recipe_parsing": True,
    "no_task_discovery": True,
    "insert_only": True,
    "aliases_disabled": True,
    "imported_risk_floor_mutating": True,
    "exact_digest_revision_trust": True,
    "private_path_free_receipts": True,
    "compare_and_swap_writes": True,
    "revocation_fail_closed": True,
    "stale_route_authorization_expiry": True,
    "task_execution": False,
    "provider_processes": False,
    "network_access": False,
    "credential_reads": False,
}
EXPECTED_LIFECYCLE = {
    "rename_supported": True,
    "conflict_requires_replace": True,
    "removal_supported": True,
    "source_authority_unchanged": True,
    "portable_export_supported": True,
    "source_change_revokes_trust": True,
    "trust_revision_conflicts_rejected": True,
    "ancestor_workspace_lookup_bounded": True,
    "cache_entries_bounded": True,
    "background_reconciliation": True,
    "insertion_rechecks_authorization": True,
    "read_only_lookup_has_no_side_effects": True,
}
EXPECTED_SOURCE_FILES = [
    "automexia-devops/src/actions/imports.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/native_import.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/workspace.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/worker.rs",
    "apps/automexia-terminal/src/screen/action_surface.rs",
    "apps/automexia-terminal/src/cli.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/cli.rs",
    "automexia-devops/tests/quick_action_imports.rs",
    "apps/automexia-terminal/tests/quick_action_native_import.rs",
    "apps/automexia-terminal/tests/quick_action_workspace_trust.rs",
]
EXPECTED_TESTS = [
    "bash_and_zsh_import_only_simple_fixed_token_aliases",
    "powershell_fish_cmd_and_git_use_their_native_inventory_contracts",
    "duplicate_secret_control_and_oversized_inventories_fail_closed",
    "trusted_task_bridges_store_exact_named_argv_without_recipe_discovery",
    "task_bridge_names_risk_and_exact_trust_are_fail_closed",
    "selected_native_alias_import_is_dry_run_cas_conflict_and_rename_safe",
    "native_import_requires_explicit_supported_unique_names_and_portably_exports",
    "linked_native_inventory_is_rejected_without_following_it",
    "workspace_task_source_is_dry_run_cas_trusted_and_revocable",
    "conflicts_rename_removal_and_private_receipts_fail_closed",
    "read_only_trust_lookup_never_creates_or_mutates_state",
    "linked_workspace_sources_and_trust_files_are_rejected",
    "trusted_workspace_tasks_are_cached_off_thread_and_revocation_fails_closed",
    "cp33_native_import_and_workspace_mutations_are_explicit_and_cas_guarded",
]
EXPECTED_BENCHMARK = "automexia-devops/benches/quick_actions.rs"
EXPECTED_FUZZ_TARGET = "fuzz/fuzz_targets/quick_action_imports.rs"
EXPECTED_DOCUMENTS = [
    "docs/adr/0021-trusted-workspace-task-bridges.md",
    "docs/ARCHITECTURE.md",
    "docs/CLI-REFERENCE.md",
    "docs/COMMAND-PRODUCTIVITY-THREAT-MODEL.md",
    "docs/COMMAND-PRODUCTIVITY.md",
    "docs/DEVOPS-ALIASES.md",
    "docs/FEATURES.md",
    "docs/PHASE-IMPLEMENTATION-AUDIT.md",
    "docs/ROADMAP.md",
    "docs/STABILIZATION-ROADMAP.md",
    "docs/TESTING.md",
    "docs/index.md",
]


class Cp33Error(ValueError):
    pass


def bounded_text(path: Path, maximum: int = MAX_POLICY_BYTES) -> str:
    if path.is_symlink() or not path.is_file():
        raise Cp33Error(f"required regular file is missing or linked: {path}")
    if path.stat().st_size > maximum:
        raise Cp33Error(f"CP3.3 evidence exceeds {maximum} bytes: {path}")
    data = path.read_bytes()
    if len(data) > maximum:
        raise Cp33Error(f"CP3.3 evidence grew while reading: {path}")
    return data.decode("utf-8")


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise Cp33Error(f"duplicate CP3.3 contract key: {key}")
        result[key] = value
    return result


def load_contract(path: Path = CONTRACT) -> dict[str, Any]:
    document = json.loads(bounded_text(path), object_pairs_hook=reject_duplicate_keys)
    expected_keys = {
        "schema",
        "phase",
        "status",
        "native_sources",
        "task_runners",
        "management_commands",
        "security",
        "lifecycle",
        "source_files",
        "required_tests",
        "benchmark",
        "fuzz_target",
        "documents",
    }
    if not isinstance(document, dict) or set(document) != expected_keys:
        raise Cp33Error("CP3.3 contract keys changed")
    if (document["schema"], document["phase"], document["status"]) != (
        1,
        "CP3.3",
        "fully-done-trusted-imports-task-bridges",
    ):
        raise Cp33Error("CP3.3 contract identity changed")
    expected = (
        ("native_sources", EXPECTED_NATIVE_SOURCES),
        ("task_runners", EXPECTED_TASK_RUNNERS),
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
            raise Cp33Error(f"CP3.3 {key.replace('_', ' ')} changed")
    for key in (
        "native_sources",
        "task_runners",
        "management_commands",
        "source_files",
        "required_tests",
        "documents",
    ):
        values = document[key]
        if not values or len(values) != len(set(values)):
            raise Cp33Error(f"CP3.3 {key} must contain unique entries")
    return document


def require_tokens(relative: str, tokens: set[str], root: Path = ROOT) -> str:
    source = bounded_text(root / relative)
    missing = sorted(token for token in tokens if token not in source)
    if missing:
        raise Cp33Error(f"{relative} is missing CP3.3 evidence: {missing}")
    return source


def validate_sources(document: dict[str, Any], root: Path = ROOT) -> dict[str, int]:
    token_sets = {
        document["source_files"][0]: {
            "parse_powershell_csv",
            "parse_posix_inventory",
            "parse_fish_inventory",
            "parse_cmd_inventory",
            "parse_git_inventory",
            "PossibleSecret",
            "UnsafeShellConstruct",
            "MachineSpecificPath",
            "build_trusted_task_bridge",
            "trusted_workspace_layer",
            "pub struct WorkspaceTrustReceipt",
            "RiskClass::Mutating",
            "ExecutionMode::Insert",
            "alias_projection: None",
            "ActionScope::TrustedWorkspace",
            "WorkingDirectoryPolicy::WorkspaceRoot",
        },
        document["source_files"][1]: {
            "read_bounded_regular",
            "SelectionRequired",
            "DuplicateSelection",
            "SelectedAliasRejected",
            "StaleRevision",
            "replace_conflicts",
            "service",
            ".replace(",
        },
        document["source_files"][2]: {
            "WORKSPACE_ACTION_DIRECTORY_NAME",
            "WORKSPACE_ACTION_FILE_NAME",
            "WORKSPACE_TRUST_FILE_NAME",
            "MAX_WORKSPACE_TRUST_RECEIPTS",
            "open_existing_read_only",
            "inspect_private_child_directory",
            "read_bounded_regular",
            "put_task_bridge",
            "remove_task_bridge",
            "StaleRevision",
            "Conflict",
            "read_only",
            "try_lock",
            "persist(",
        },
        document["source_files"][3]: {
            "WORKSPACE_RECONCILE",
            "WORKSPACE_AUTHORIZATION_TTL",
            "MAX_WORKSPACE_CACHE_ENTRIES",
            "submit_for_workspace",
            "workspace_action_is_authorized",
            "resolve_trusted_workspace",
            "open_existing_read_only",
        },
        document["source_files"][4]: {
            "ensure_selected_workspace_action_authorized",
            "workspace_action_is_authorized",
            "Workspace trust changed or expired",
        },
        document["source_files"][5]: {
            "ImportAliases",
            "TaskPut",
            "TaskRemove",
            "WorkspaceTrust",
            "WorkspaceRevoke",
            "WorkspaceDoctor",
            "requires = \"expected_revision\"",
            "requires = \"expected_trust_revision\"",
        },
        document["source_files"][6]: {
            "ActionsAction::ImportAliases",
            "preview_native_alias_import_file",
            "apply_native_alias_import",
            "ActionsAction::TaskPut",
            "ActionsAction::TaskRemove",
            "ActionsAction::WorkspaceTrust",
            "ActionsAction::WorkspaceRevoke",
            "ActionsAction::WorkspaceDoctor",
            "required_revision",
        },
    }
    for relative, tokens in token_sets.items():
        require_tokens(relative, tokens, root)

    evidence = "\n".join(bounded_text(root / relative) for relative in document["source_files"])
    missing_tests = sorted(
        name for name in document["required_tests"] if f"fn {name}" not in evidence
    )
    if missing_tests:
        raise Cp33Error(f"CP3.3 required tests are missing: {missing_tests}")

    require_tokens(
        document["benchmark"],
        {
            "quick_action_native_import_bash_1024",
            "quick_action_workspace_trust_verify",
            "preview_native_alias_import",
            "trusted_workspace_layer",
        },
        root,
    )
    fuzz = require_tokens(
        document["fuzz_target"],
        {"preview_native_alias_import"}
        | {f"NativeAliasSource::{name.title()}" for name in ("powershell", "bash", "zsh", "fish", "cmd", "git")},
        root,
    )
    if "NativeAliasSource::Powershell" not in fuzz:
        raise Cp33Error("CP3.3 PowerShell fuzz coverage changed")
    require_tokens("fuzz/Cargo.toml", {'name = "quick_action_imports"'}, root)
    require_tokens(
        ".github/workflows/nightly.yml",
        {"quick_action_imports"},
        root,
    )
    for relative in document["documents"]:
        require_tokens(relative, {"CP3.3"}, root)

    return {
        "native_sources": len(document["native_sources"]),
        "task_runners": len(document["task_runners"]),
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
    except (Cp33Error, OSError, UnicodeError, json.JSONDecodeError) as error:
        print(f"CP3.3 import/task-bridge validation failed: {error}", file=sys.stderr)
        return 1
    print(
        "PASS: CP3.3 native imports and trusted workspace task bridges are "
        f"bounded, dry-run/CAS managed, revocable, benchmarked, and fuzzed ({counts})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
