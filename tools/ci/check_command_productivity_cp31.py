#!/usr/bin/env python3
"""Validate persistent, explicitly opt-in CP3.1 alias publication."""

from __future__ import annotations

import json
from pathlib import Path
import sys
from typing import Any

from check_command_productivity import rust_code_without_comments_and_literals
from check_command_productivity_cp22 import Cp22Error, _rust_item


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "tests/fixtures/command-productivity/cp31-contract-v1.json"
MAX_POLICY_BYTES = 262_144
EXPECTED_LIMITS = {
    "enabled_aliases": 256,
    "generated_file_bytes": 1_048_576,
    "manifest_bytes": 65_536,
    "transaction_bytes": 1_024,
    "generation_directory_entries": 16,
    "retained_previous_generations": 1,
}
EXPECTED_PUBLICATION = {
    "canonical_source": "actions/actions.toml",
    "generated_root": "generated/aliases",
    "layout": "immutable-content-addressed-generations",
    "activation_pointer": "current",
    "rollback_pointer": "previous",
    "transaction_journal": "transaction.pending",
    "commit_order": "source-cas-then-pointer",
    "crash_outcomes": "all-old-or-all-new",
    "cross_process_lock": ".aliases.lock",
    "disable_preserves_actions": True,
}
EXPECTED_SECURITY = {
    "private_permissions": True,
    "nofollow_managed_state": True,
    "sha256_manifest_and_files": True,
    "compiler_identity_verified": True,
    "native_wins": True,
    "exact_override_same_owner_only": True,
    "override_revalidated_at_load": True,
    "doctor_strictly_read_only": True,
    "startup_provider_execution": False,
    "startup_network": False,
    "startup_canonical_rewrite": False,
    "implicit_action_execution": False,
}
EXPECTED_SHELL_FILES = {
    "powershell": "shell-integration/powershell/automexia.ps1",
    "bash": "shell-integration/bash/automexia.bash",
    "zsh": "shell-integration/zsh/automexia.zsh",
    "fish": "shell-integration/fish/automexia.fish",
    "cmd": "shell-integration/cmd/automexia.cmd",
}
EXPECTED_COMMANDS = [
    "list", "preview", "test", "enable", "disable", "rename", "regenerate",
    "disable-all", "rollback", "doctor", "reload",
]
EXPECTED_SOURCE_FILES = [
    "apps/automexia-terminal/src/automexia/quick_actions/aliases.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/aliases_cli.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/secure_fs.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/store.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/service.rs",
    "apps/automexia-terminal/src/cli.rs",
]
EXPECTED_REQUIRED_TESTS = [
    "publish_is_one_verified_generation_for_all_five_shells",
    "source_drift_is_truthful_and_regeneration_rotates_one_previous",
    "stale_writer_cannot_replace_a_newer_generation",
    "tampered_artifact_is_never_returned_for_activation",
    "doctor_rejects_tampered_retained_previous_generation",
    "doctor_rejects_unsafe_artifact_permissions_even_when_digest_matches",
    "doctor_rejects_unexpected_generation_entry",
    "compiler_identity_mismatch_is_rejected",
    "disabled_previous_pointer_fails_before_publication",
    "disable_changes_only_the_pointer_and_preserves_generation",
    "rollback_swaps_current_and_previous_generations",
    "prepared_transition_recovers_all_old_when_source_was_not_saved",
    "prepared_transition_recovers_all_new_when_source_was_saved",
    "prepared_transition_activates_under_the_held_lock",
    "powershell_loader_sources_one_verified_generation",
    "powershell_loader_keeps_a_late_native_collision",
    "powershell_loader_rejects_a_tampered_active_artifact",
    "authenticated_exact_override_survives_regenerate_and_disable_only_for_same_owner",
    "hostile_generation_manifest_bytes_never_panic",
    "cross_process_lock_fails_fast_without_partial_state",
    "linked_managed_parent_is_rejected",
]
EXPECTED_NATIVE_TEST_FILES = [
    "tools/ci/create_cp31_alias_fixture.py",
    "tools/ci/test_shell_integration.sh",
    "tools/ci/test_zsh_integration.zsh",
    "tools/ci/test_fish_integration.fish",
    "tools/ci/test_shell_integration.ps1",
    "tools/ci/test_shell_sources.sh",
]
EXPECTED_UNINSTALL_FILES = [
    "shell-integration/uninstall-unix.sh",
    "shell-integration/uninstall-windows.ps1",
]
EXPECTED_BENCHMARK = "apps/automexia-terminal/benches/quick_action_store.rs"
EXPECTED_DOCUMENTS = [
    "docs/ARCHITECTURE.md",
    "docs/COMMAND-PRODUCTIVITY.md",
    "docs/DEVOPS-ALIASES.md",
    "docs/SHELL-INTEGRATION.md",
    "docs/TESTING.md",
]


class Cp31Error(ValueError):
    pass


def bounded_text(path: Path, maximum: int = MAX_POLICY_BYTES) -> str:
    if path.is_symlink() or not path.is_file():
        raise Cp31Error(f"required regular file is missing or linked: {path}")
    if path.stat().st_size > maximum:
        raise Cp31Error(f"CP3.1 evidence exceeds {maximum} bytes: {path}")
    data = path.read_bytes()
    if len(data) > maximum:
        raise Cp31Error(f"CP3.1 evidence grew while reading: {path}")
    return data.decode("utf-8")


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise Cp31Error(f"duplicate CP3.1 contract key: {key}")
        result[key] = value
    return result


def load_contract(path: Path = CONTRACT) -> dict[str, Any]:
    document = json.loads(bounded_text(path), object_pairs_hook=reject_duplicate_keys)
    keys = {
        "schema", "phase", "status", "limits", "publication", "security",
        "shell_files", "management_commands", "source_files", "required_tests",
        "native_test_files", "uninstall_files", "benchmark", "documents",
    }
    if not isinstance(document, dict) or set(document) != keys:
        raise Cp31Error("CP3.1 contract keys changed")
    if (document["schema"], document["phase"], document["status"]) != (
        1, "CP3.1", "active-persistent-explicit-opt-in"
    ):
        raise Cp31Error("CP3.1 contract identity changed")
    expected = (
        ("limits", EXPECTED_LIMITS),
        ("publication", EXPECTED_PUBLICATION),
        ("security", EXPECTED_SECURITY),
        ("shell_files", EXPECTED_SHELL_FILES),
        ("management_commands", EXPECTED_COMMANDS),
        ("source_files", EXPECTED_SOURCE_FILES),
        ("required_tests", EXPECTED_REQUIRED_TESTS),
        ("native_test_files", EXPECTED_NATIVE_TEST_FILES),
        ("uninstall_files", EXPECTED_UNINSTALL_FILES),
        ("benchmark", EXPECTED_BENCHMARK),
        ("documents", EXPECTED_DOCUMENTS),
    )
    for key, value in expected:
        if document[key] != value:
            raise Cp31Error(f"CP3.1 {key.replace('_', ' ')} changed")
    for key in ("required_tests", "native_test_files", "uninstall_files", "documents"):
        values = document[key]
        if not isinstance(values, list) or not values or len(values) != len(set(values)):
            raise Cp31Error(f"CP3.1 {key} must contain unique entries")
    return document


def require_tokens(relative: str, tokens: set[str], root: Path = ROOT) -> str:
    source = bounded_text(root / relative)
    missing = sorted(token for token in tokens if token not in source)
    if missing:
        raise Cp31Error(f"{relative} is missing CP3.1 evidence: {missing}")
    return source


def validate_write_lock_lifecycle(root: Path = ROOT) -> None:
    """Connect persistence callers to the one explicit advisory-lock owner.

    Native tests prove the inherited-descriptor behavior; this check rejects
    disconnected acquisition, early release, and proof left only in comments.
    """
    owner_path = "apps/automexia-terminal/src/automexia/private_fs.rs"
    owner = rust_code_without_comments_and_literals(bounded_text(root / owner_path))

    def compact(source: str) -> str:
        return "".join(source.split())

    def item(source: str, declaration: str) -> str:
        try:
            return _rust_item(source, declaration)
        except Cp22Error as error:
            raise Cp31Error(f"advisory-lock ownership is incomplete: {declaration}") from error

    def require(body: str, fragment: str, label: str) -> None:
        if compact(fragment) not in compact(body):
            raise Cp31Error(f"advisory-lock ownership is disconnected: {label}")

    if compact(item(owner, "pub(crate) struct WriteLock")) != "file:File,":
        raise Cp31Error("advisory-lock ownership no longer retains its private file")
    acquire = item(item(owner, "impl WriteLock"), "pub(crate) fn try_acquire(")
    if compact(acquire) != "file.try_lock()?;Ok(Self{file})":
        raise Cp31Error("advisory-lock ownership must acquire the native lock before publication")
    release = item(item(owner, "impl Drop for WriteLock"), "fn drop(")
    if compact(release) != "let_=self.file.unlock();":
        raise Cp31Error("advisory-lock ownership must explicitly unlock before file close")
    for test in (
        "write_lock_excludes_a_concurrent_writer_until_the_transaction_ends",
        "an_inherited_description_does_not_extend_a_retired_transaction_lock",
    ):
        item(owner, f"fn {test}(")

    prefix = "apps/automexia-terminal/src/automexia/"
    for relative in (
        "quick_actions/store.rs", "quick_actions/aliases.rs",
        "connections/library.rs", "connections/receipts.rs",
    ):
        source = rust_code_without_comments_and_literals(bounded_text(root / prefix / relative))
        body = item(source, "fn try_write_lock(")
        require(source, "fn try_write_lock(&self) -> Result<WriteLock,", relative)
        require(body, "match WriteLock::try_acquire(lock) { Ok(lock) => Ok(lock),", relative)
        if relative.endswith("aliases.rs"):
            require(item(source, "pub struct PreparedAliasPublication"),
                    "_lock: WriteLock,", "prepared alias transaction")

    workspace = rust_code_without_comments_and_literals(
        bounded_text(root / prefix / "quick_actions/workspace.rs"))
    require(workspace, "fn try_lock(file: File) -> Result<WriteLock,", "workspace transaction")
    require(item(workspace, "fn try_lock("),
            "match WriteLock::try_acquire(file) { Ok(lock) => Ok(lock),", "workspace transaction")
    require(item(workspace, "fn try_write_lock("),
            "try_lock(open_lock(&self.lock_path(), false)?)", "workspace source acquisition")
    preferences = rust_code_without_comments_and_literals(
        bounded_text(root / prefix / "preferences.rs"))
    require(item(preferences, "fn write_to_root_with_package("),
            "let _lock = match WriteLock::try_acquire(lock) { Ok(lock) => lock,",
            "preference transaction")


def validate_sources(document: dict[str, Any], root: Path = ROOT) -> dict[str, int]:
    aliases = require_tokens(document["source_files"][0], {
        "AliasProjectionStore", "prepare_transition", "activate_prepared",
        "recover_pending", "current_exact_overrides", "GenerationManifest",
        "transaction.pending", "WriteLock::try_acquire", "sha256_hex", "open_existing",
        "pub fn doctor", "activation_path", "cleanup_old_generations",
        "verify_exact_directory_entries", "verify_artifact_identity", "PROJECTION_GENERATOR",
    }, root)
    validate_write_lock_lifecycle(root)
    missing_tests = sorted(
        name for name in document["required_tests"] if f"fn {name}(" not in aliases
    )
    if missing_tests:
        raise Cp31Error(f"CP3.1 required tests are missing: {missing_tests}")
    require_tokens(document["source_files"][1], {
        "AliasesAction::Preview", "AliasesAction::Test", "AliasesAction::Enable",
        "AliasesAction::Disable", "AliasesAction::Rename", "AliasesAction::Regenerate",
        "AliasesAction::DisableAll", "AliasesAction::Rollback", "AliasesAction::Doctor",
        "AliasesAction::Reload", "reuse_authenticated_exact_overrides",
        "if apply", "prepare_transition", "service.replace", "activate_prepared",
        "expected_generation", "owner_fingerprint", "completion_json", "tool_json",
    }, root)
    require_tokens(document["source_files"][2], {
        "ensure_private_aliases_directory", "inspect_private_aliases_directory",
        "O_NOFOLLOW", "PROTECTED_DACL_SECURITY_INFORMATION",
    }, root)
    require_tokens(document["source_files"][3], {
        "open_existing_read_only", "load_read_only", "read_private_optional",
    }, root)
    require_tokens(document["source_files"][4], {"open_read_only"}, root)
    require_tokens(document["source_files"][5], {
        "enum AliasesAction", "Preview", "Test", "Enable", "DisableAll", "Rollback",
    }, root)

    loader_tokens = {
        "powershell": {"Read-AutomexiaAliasCandidate", "Reload-AutomexiaAliases", "Get-AutomexiaAliasHealth", "AutomexiaAliasCandidateOverrides", "generator=automexia-devops/0.4.0"},
        "bash": {"__automexia_alias_prepare", "automexia_aliases_reload", "automexia_aliases_health", "__automexia_alias_runtime_fingerprint", "generator=automexia-devops/0.4.0"},
        "zsh": {"__automexia_alias_prepare", "automexia_aliases_reload", "automexia_aliases_health", "__automexia_alias_runtime_fingerprint", "generator=automexia-devops/0.4.0"},
        "fish": {"__automexia_alias_prepare", "automexia_aliases_reload", "automexia_aliases_health", "__automexia_alias_runtime_fingerprint", "generator=automexia-devops/0.4.0"},
        "cmd": {"automexia-alias-loader.ps1", "doskey /macrofile", "automexia_aliases_health", "Open a new CMD session"},
    }
    for shell, relative in document["shell_files"].items():
        require_tokens(relative, loader_tokens[shell], root)
    require_tokens("shell-integration/cmd/automexia-alias-loader.ps1", {
        "Test-PrivateItem", "generation.manifest", "Write-Result 'READY'",
        "Write-Result 'COLLISION'", "Get-Sha256",
        "generator=automexia-devops/0.4.0",
    }, root)

    for relative in document["uninstall_files"]:
        require_tokens(relative, {
            "generated", "aliases", "generation.manifest", "transaction.pending",
            "automexia-aliases", "Quick Action",
        }, root)
    native_tokens = {
        "tools/ci/create_cp31_alias_fixture.py": {"automexia-alias-generation-v1", "automexia-devops/0.4.0", "os.chmod", "os.replace"},
        "tools/ci/test_shell_integration.sh": {"alias-reload-p95", "state=collision", "state=tampered", "state=disabled"},
        "tools/ci/test_zsh_integration.zsh": {"alias-reload-p95", "state=collision", "state=tampered", "state=disabled"},
        "tools/ci/test_fish_integration.fish": {"alias-reload-p95", "state=collision", "state=tampered", "state=disabled"},
        "tools/ci/test_shell_integration.ps1": {"AUTOMEXIA_ALIAS_STATE=UNSAFE_PERMISSIONS", "canonical saved actions"},
        "tools/ci/test_shell_sources.sh": {"generated/aliases", "revision = 7", "unexpected generated-alias entry"},
    }
    for relative in document["native_test_files"]:
        require_tokens(relative, native_tokens[relative], root)
    require_tokens(document["benchmark"], {
        "cp31_alias_projection_256", "compile_all_five_shells",
        "durable_publish_and_verify", "read_only_doctor",
    }, root)
    for workflow in (".github/workflows/nightly.yml", ".github/workflows/release.yml"):
        require_tokens(workflow, {
            "Verify CP3.1 native shell lifecycle in the configured WSL distro",
            "bash tools/ci/test_shell_sources.sh",
            "AUTOMEXIA_TEST_WSL_DISTRO",
        }, root)
    for relative in document["documents"]:
        require_tokens(relative, {"CP3.1"}, root)
    return {
        "shells": len(document["shell_files"]),
        "commands": len(document["management_commands"]),
        "tests": len(document["required_tests"]),
        "native_files": len(document["native_test_files"]),
        "documents": len(document["documents"]),
    }


def validate_repository(root: Path = ROOT) -> dict[str, int]:
    document = load_contract(root / CONTRACT.relative_to(ROOT))
    return validate_sources(document, root)


def main() -> int:
    try:
        counts = validate_repository()
    except (Cp31Error, OSError, UnicodeError, json.JSONDecodeError) as error:
        print(f"CP3.1 persistence validation failed: {error}", file=sys.stderr)
        return 1
    print(
        "PASS: CP3.1 persistent aliases are explicit, private, transactional, "
        f"native-first, reloadable, diagnosable, benchmarked, and uninstallable ({counts})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
