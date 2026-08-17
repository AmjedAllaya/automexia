#!/usr/bin/env python3
"""Validate implemented CP2-CP3.3 foundations and planned CP4+ work."""

from __future__ import annotations

import json
from pathlib import Path
import re
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
CONTRACT_PATH = ROOT / "tests/fixtures/command-productivity/cp2-cp3-alias-spec-v1.json"
MAX_POLICY_BYTES = 65_536
MAX_APPLICATION_SOURCE_BYTES = 131_072
MAX_DOCUMENT_BYTES = 131_072

SHELLS = {
    "powershell": ["windows", "linux", "macos"],
    "bash": ["linux", "macos", "wsl"],
    "zsh": ["linux", "macos", "wsl"],
    "fish": ["linux", "macos", "wsl"],
    "cmd": ["windows"],
}
PROVIDERS = [
    "git",
    "docker",
    "kubernetes",
    "openshift",
    "helm",
    "terraform",
    "opentofu",
    "aws",
    "azure",
    "gcp",
    "openssh",
]
SCOPES = [
    "session",
    "capsule",
    "trusted-workspace",
    "shell-user",
    "global-user",
    "builtin-disabled",
]
LIMITS = {
    "source_file_bytes": 1_048_576,
    "active_actions": 1_024,
    "enabled_aliases": 256,
    "arguments_per_action": 64,
    "placeholders_per_action": 32,
    "tags_per_action": 64,
    "string_bytes": 4_096,
    "generated_file_bytes": 1_048_576,
    "in_process_cache_bytes": 8_388_608,
}
DOCUMENT_FIELDS = ["schema_version", "revision", "actions"]
ACTION_FIELDS = [
    "id",
    "display_name",
    "description",
    "tags",
    "scope",
    "shells",
    "template",
    "placeholders",
    "working_directory_policy",
    "risk",
    "execution",
    "provenance",
    "enabled",
    "alias_projection",
]
MODEL_ENUMS = {
    "action_scopes": ["Session", "Capsule", "TrustedWorkspace", "ShellUser", "GlobalUser", "BuiltinDisabled"],
    "shells": ["Powershell", "Bash", "Zsh", "Fish", "Cmd"],
    "templates": ["TypedArgv", "RawInsertOnly"],
    "argument_tokens": ["Literal", "Placeholder"],
    "working_directory_policies": ["Inherit", "WorkspaceRoot", "Fixed"],
    "risk_classes": ["ReadOnly", "Mutating", "Destructive", "Privileged"],
    "provenance": ["User", "BuiltIn", "Imported", "WorkspaceTask"],
    "alias_projection_modes": ["Auto", "CommandAlias", "WrapperFunction", "FishAbbreviation", "DoskeyMacro"],
    "argument_policies": ["None", "ForwardAll", "TypedBindings"],
    "execution_modes": ["Insert", "Copy", "ExactLaunch"],
    "completion_modes": ["Required", "BestEffort", "Disabled"],
    "override_policies": ["NativeWins", "ExplicitExactOverride"],
}
ALIAS_POLICY = {
    "portable_pattern": "^[a-z][a-z0-9-]{1,31}$",
    "minimum_length": 2,
    "maximum_length": 32,
    "builtin_one_letter_names": False,
    "secret_placeholders": False,
    "raw_insert_projection": False,
    "mutating_alias_requires_acknowledgement": True,
}
MODEL_FILES = [
    "automexia-devops/src/actions/mod.rs",
    "automexia-devops/src/actions/model.rs",
    "automexia-devops/src/actions/projection.rs",
    "automexia-devops/src/actions/validation.rs",
    "automexia-devops/src/actions/activation.rs",
    "automexia-devops/src/actions/packs.rs",
    "automexia-devops/src/actions/imports.rs",
]
PERSISTENCE_FILES = [
    "apps/automexia-terminal/src/automexia/quick_actions/mod.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/refresh.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/secure_fs.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/service.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/store.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/cli.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/transfer.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/worker.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/aliases.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/aliases_cli.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/packs_cli.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/native_import.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/workspace.rs",
]
HOSTILE_FIXTURE = "tests/fixtures/command-productivity/cp2-hostile-actions-v1.json"
HOSTILE_CASES = {
    "unknown-field": ("unknown-field.toml", "decode-error"),
    "duplicate-id": ("duplicate-id.toml", "duplicate-action-id"),
    "destructive-alias": ("destructive-alias.toml", "alias-risk-denied"),
    "secret-alias": ("secret-alias.toml", "alias-secret-denied"),
    "raw-insert-alias": ("raw-insert-alias.toml", "raw-insert-alias-denied"),
    "missing-placeholder": ("missing-placeholder.toml", "missing-placeholder"),
    "command-alias-arguments": ("command-alias-arguments.toml", "command-alias-has-arguments"),
    "shell-mismatch": ("shell-mismatch.toml", "alias-shell-not-allowed"),
    "mutating-unreviewed": ("mutating-unreviewed.toml", "mutating-alias-not-acknowledged"),
    "secret-default": ("secret-default.toml", "secret-default-denied"),
    "unsafe-bidi": ("unsafe-bidi.toml", "unsafe-text"),
}
PROJECTION_MODES = {
    "powershell": ["CommandAlias", "WrapperFunction"],
    "bash": ["CommandAlias", "WrapperFunction"],
    "zsh": ["CommandAlias", "WrapperFunction"],
    "fish": ["FishAbbreviation", "WrapperFunction"],
    "cmd": ["DoskeyMacro"],
}
HEALTH_STATES = [
    "Ready",
    "Disabled",
    "Missing tool",
    "Unsupported tool",
    "Collision",
    "Completion unavailable",
    "Stale source",
    "Reload required",
    "Tampered artifact",
    "Unsafe permissions",
    "Malformed source",
    "Generation failed",
]
DEFAULTS = {
    "builtin_aliases_enabled": False,
    "action_execution": "insert-without-enter",
    "workspace_trusted": False,
    "collision_precedence": "native-wins",
    "provider_refresh": "explicit-only",
}
RISK_POLICY = {
    "ReadOnly": "eligible-after-review",
    "Mutating": "eligible-only-when-bounded-reversible-and-descriptive",
    "Destructive": "denied",
    "Privileged": "denied",
}
PERSISTENCE = {
    "source": "actions/actions.toml",
    "previous_revision": "actions/actions.previous.toml",
    "generated_root": "generated/aliases",
    "publication": "atomic-all-or-old",
    "concurrency": "exact-lock-and-compare-and-swap",
    "active_session_reload": "explicit",
    "workspace_alias_projection": False,
    "remote_installation": "explicit",
}
CAPABILITIES = {
    "network": False,
    "secret_read": False,
    "provider_on_startup": False,
    "provider_on_keystroke": False,
    "terminal_grid_inference": False,
    "shell_evaluation": False,
    "implicit_execution": False,
    "hidden_global_context_mutation": False,
}
PERFORMANCE_TARGETS = {
    "baseline_days": 30,
    "warm_load_p95_ms": 25,
    "search_p95_ms": 16,
    "collision_p95_ms": 25,
    "compile_shell_p95_ms": 50,
    "shell_startup_verify_p95_ms": 50,
}
UX_INVARIANTS = [
    "keyboard-complete",
    "focus-trap-and-exact-return",
    "underlay-accessibility-inert",
    "no-pty-geometry-mutation",
    "text-and-icon-not-color-alone",
    "responsive-at-400-percent",
    "high-contrast-and-reduced-motion",
    "narrator-nvda-voiceover-orca",
]
VERIFICATION_DOMAINS = [
    "model-and-persistence",
    "projection-compilers",
    "provider-packs",
    "completion-linkage",
    "native-shells-and-platforms",
    "ui-and-accessibility",
    "security-and-privacy",
    "fuzz-and-mutation",
    "performance-and-resources",
    "rollback-and-uninstall",
]
AUTHORITIES = {
    "product": "docs/DEVOPS-ALIASES.md",
    "architecture": "docs/ARCHITECTURE.md",
    "decision": "docs/adr/0015-shell-native-completion-and-typed-quick-actions.md",
    "threats": "docs/COMMAND-PRODUCTIVITY-THREAT-MODEL.md",
}
REQUIRED_HEADINGS = {
    "Product outcome",
    "Non-negotiable principles",
    "Architecture and ownership",
    "Canonical data model",
    "Persistence and cross-session behavior",
    "Scope, precedence, and collision handling",
    "Shell-specific projections",
    "Completion linkage",
    "User experience",
    "Built-in first-party DevOps packs",
    "User-created aliases and import/export",
    "Existing tools and open-source projects",
    "Security and privacy",
    "Performance, resilience, and resource budgets",
    "Accessibility and responsive UX",
    "Verification plan",
    "Delivery phases",
    "Acceptance criteria",
    "Primary references",
}
REQUIRED_SPEC_SNIPPETS = {
    "Status: CP2.0-CP3.3",
    "No first-party pack alias is activated by default or shipped implicitly by CP3.2.",
    "actions/actions.toml",
    "actions/actions.previous.toml",
    "generated/aliases",
    "[a-z][a-z0-9-]{1,31}",
    "Nothing short is enabled by default.",
    "No secret aliases",
    "No hidden context mutation",
    "pointer-last commit",
    "compare-and-swap",
    "template: TypedArgv(executable_id, ArgumentToken[]) | RawInsertOnly(shell, text)",
    "mode: Auto | CommandAlias | WrapperFunction | FishAbbreviation | DoskeyMacro",
    "argument_policy: None | ForwardAll | TypedBindings",
    "Health states include `Ready`",
    "<= 25 ms p95 off renderer/input/PTY paths",
    "<= 16 ms p95; deterministic and allocation-bounded",
    "<= 50 ms p95 post-warmup; no provider, network, action execution, or per-alias subprocess",
    "200%/400% text scale",
    "PowerShell 5.1 and PowerShell 7+",
    "Bash",
    "Zsh",
    "Fish",
    "CMD",
    "WSL",
    "Narrator/",
    "VoiceOver",
    "AT-SPI/Orca",
    "1,000 save/regenerate/reload cycles",
    "no handle/task/file/storage growth",
    "CP2.0 - contract and fixtures (implemented)",
    "CP2.1 - user-private action store (implemented foundation)",
    "CP3.1 - persistent opt-in user aliases",
    "CP3.2 - reviewed first-party DevOps packs",
    "CP3.3 - native imports and trusted workspace task bridges",
    "No native inventory, task discovery, recipe parsing, provider process, network, credential read, or task execution",
}
WIRING = {
    "docs/ROADMAP.md": "DEVOPS-ALIASES.md",
    "docs/STABILIZATION-ROADMAP.md": "DEVOPS-ALIASES.md",
    "docs/ARCHITECTURE.md": "DEVOPS-ALIASES.md",
    "docs/TESTING.md": "DEVOPS-ALIASES.md#verification-plan",
    "docs/FEATURES.md": "DEVOPS-ALIASES.md",
    "docs/SHELL-INTEGRATION.md": "DEVOPS-ALIASES.md",
    "docs/COMMAND-PRODUCTIVITY.md": "DEVOPS-ALIASES.md",
    ".github/workflows/ci.yml": "test_devops_alias_spec.py",
    "docs/index.md": "DEVOPS-ALIASES.md",
    "docs/adr/0015-shell-native-completion-and-typed-quick-actions.md": (
        "DEVOPS-ALIASES.md"
    ),
}


class AliasSpecError(ValueError):
    """The alias specification is incomplete, unsafe, or exceeds its activated stage."""


def bounded_text(path: Path, maximum: int, label: str) -> str:
    if path.is_symlink():
        raise AliasSpecError(f"{label} must not be a symbolic link: {path}")
    if not path.is_file():
        raise AliasSpecError(f"{label} is missing or not a regular file: {path}")
    size = path.stat().st_size
    if size > maximum:
        raise AliasSpecError(f"{label} exceeds {maximum} bytes: {path}")
    return path.read_text(encoding="utf-8")


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise AliasSpecError(f"duplicate JSON key is forbidden: {key}")
        result[key] = value
    return result


def reject_nonstandard_constant(value: str) -> None:
    raise AliasSpecError(f"non-standard JSON constant is forbidden: {value}")


def load_contract(text: str) -> Any:
    return json.loads(
        text,
        object_pairs_hook=reject_duplicate_keys,
        parse_constant=reject_nonstandard_constant,
    )


def validate_contract(document: Any) -> dict[str, int]:
    if not isinstance(document, dict):
        raise AliasSpecError("alias specification contract must be an object")
    expected_keys = {
        "schema",
        "phase",
        "status",
        "implemented_stage",
        "authorities",
        "shells",
        "providers",
        "scopes",
        "limits",
        "required_document_fields",
        "required_action_fields",
        "model_enums",
        "alias_policy",
        "projection_modes",
        "health_states",
        "defaults",
        "risk_alias_eligibility",
        "persistence",
        "capabilities",
        "performance_targets",
        "ux_invariants",
        "verification_domains",
        "model_files",
        "hostile_fixture",
        "activation_files",
    }
    if set(document) != expected_keys:
        raise AliasSpecError("alias specification contract keys changed")
    if (
        document["schema"],
        document["phase"],
        document["status"],
        document["implemented_stage"],
    ) != (
        1,
        "CP2-CP3-SPEC",
        "planned",
        "CP3.3-native-imports-trusted-workspace-tasks",
    ):
        raise AliasSpecError(
            "CP2/CP3 must remain planned schema 1 with implementation through reviewed CP3.3 imports and task bridges"
        )
    checks = (
        ("authorities", AUTHORITIES, "authority map"),
        ("shells", SHELLS, "shell/platform matrix"),
        ("providers", PROVIDERS, "provider catalog"),
        ("scopes", SCOPES, "scope precedence"),
        ("limits", LIMITS, "resource ceilings"),
        ("required_document_fields", DOCUMENT_FIELDS, "typed document fields"),
        ("required_action_fields", ACTION_FIELDS, "typed action fields"),
        ("model_enums", MODEL_ENUMS, "typed model enums"),
        ("alias_policy", ALIAS_POLICY, "portable alias policy"),
        ("projection_modes", PROJECTION_MODES, "shell projection modes"),
        ("health_states", HEALTH_STATES, "health-state model"),
        ("defaults", DEFAULTS, "safe defaults"),
        ("risk_alias_eligibility", RISK_POLICY, "risk eligibility"),
        ("persistence", PERSISTENCE, "persistence contract"),
        ("capabilities", CAPABILITIES, "capability boundary"),
        ("performance_targets", PERFORMANCE_TARGETS, "performance ratchets"),
        ("ux_invariants", UX_INVARIANTS, "UX/accessibility invariants"),
        ("verification_domains", VERIFICATION_DOMAINS, "verification matrix"),
        ("model_files", MODEL_FILES, "pure model source boundary"),
        ("hostile_fixture", HOSTILE_FIXTURE, "hostile fixture authority"),
        ("activation_files", PERSISTENCE_FILES, "CP2-CP3.3 application source boundary"),
    )
    for key, expected, label in checks:
        if document[key] != expected:
            raise AliasSpecError(f"CP2/CP3 {label} changed")
    return {
        "shells": len(SHELLS),
        "providers": len(PROVIDERS),
        "scopes": len(SCOPES),
        "verification_domains": len(VERIFICATION_DOMAINS),
        "ux_invariants": len(UX_INVARIANTS),
        "model_files": len(MODEL_FILES),
        "persistence_files": len(PERSISTENCE_FILES),
    }


def validate_model_evidence(root: Path) -> dict[str, int]:
    required_tokens = {
        MODEL_FILES[0]: {"parse_quick_actions", "ValidatedQuickActions", "MAX_SOURCE_BYTES"},
        MODEL_FILES[1]: {"QuickActionDocument", "ActionTemplate", "AliasProjection"},
        MODEL_FILES[2]: {"compile_shell_projection", "render_powershell", "activation_enabled: false"},
        MODEL_FILES[3]: {"validate_document", "MAX_ACTIONS", "MutatingAliasNotAcknowledged"},
        MODEL_FILES[4]: {"ActionIndex", "expand_for_shell", "LayerIdentity"},
        MODEL_FILES[5]: {"builtin_packs", "validate_pack_registry", "plan_pack_update"},
        MODEL_FILES[6]: {"preview_native_alias_import", "build_trusted_task_bridge", "trusted_workspace_layer"},
    }
    for relative, tokens in required_tokens.items():
        text = bounded_text(root / relative, MAX_POLICY_BYTES, "CP2.0 pure model")
        missing = sorted(token for token in tokens if token not in text)
        if missing:
            raise AliasSpecError(f"{relative} is missing CP2.0 model tokens: {missing}")

    manifest = load_contract(
        bounded_text(root / HOSTILE_FIXTURE, MAX_POLICY_BYTES, "CP2.0 hostile fixture")
    )
    if not isinstance(manifest, dict) or set(manifest) != {"schema", "phase", "cases"}:
        raise AliasSpecError("CP2.0 hostile fixture keys changed")
    if manifest["schema"] != 1 or manifest["phase"] != "CP2.0":
        raise AliasSpecError("CP2.0 hostile fixture identity changed")
    cases = manifest["cases"]
    if not isinstance(cases, list):
        raise AliasSpecError("CP2.0 hostile cases must be a list")
    actual: dict[str, tuple[str, str]] = {}
    fixture_root = root / "tests/fixtures/command-productivity/cp2-quick-actions"
    for case in cases:
        if not isinstance(case, dict) or set(case) != {"id", "fixture", "expected"}:
            raise AliasSpecError("CP2.0 hostile case fields changed")
        identifier, fixture, expected = case["id"], case["fixture"], case["expected"]
        if not all(isinstance(value, str) and value for value in (identifier, fixture, expected)):
            raise AliasSpecError("CP2.0 hostile case values must be non-empty strings")
        if identifier in actual:
            raise AliasSpecError(f"duplicate CP2.0 hostile case: {identifier}")
        fixture_path = Path(fixture)
        if fixture_path.is_absolute() or ".." in fixture_path.parts or fixture_path.suffix != ".toml":
            raise AliasSpecError(f"unsafe CP2.0 hostile fixture path: {fixture}")
        bounded_text(fixture_root / fixture_path, MAX_POLICY_BYTES, "CP2.0 hostile TOML")
        actual[identifier] = (fixture, expected)
    if actual != HOSTILE_CASES:
        raise AliasSpecError("CP2.0 hostile fixture catalog changed")
    required_persistence_tokens = {
        PERSISTENCE_FILES[0]: {"QuickActionStore", "QuickActionService", "QuickActionMonitor"},
        PERSISTENCE_FILES[1]: {"ExactActionWatchPlan", "WATCH_EVENT_CAPACITY", "RecursiveMode::NonRecursive"},
        PERSISTENCE_FILES[2]: {"read_bounded_regular", "O_NOFOLLOW", "PROTECTED_DACL_SECURITY_INFORMATION"},
        PERSISTENCE_FILES[3]: {"RetainedLastKnownGood", "StaleRevision", "Arc<QuickActionSnapshot>"},
        PERSISTENCE_FILES[4]: {"actions.previous.toml", "try_lock", "MAX_CACHED_ACTION_BYTES", "recover_previous"},
        PERSISTENCE_FILES[5]: {"ActionsAction", "expected_revision", "read_single_action"},
        PERSISTENCE_FILES[6]: {"source_digest", "apply_import", "preview_import"},
        PERSISTENCE_FILES[7]: {"SEARCH_COALESCE_INTERVAL", "forget_route", "handle.join()"},
        PERSISTENCE_FILES[8]: {"AliasProjectionStore", "prepare_transition", "recover_pending", "current_exact_overrides"},
        PERSISTENCE_FILES[9]: {"AliasesAction::Preview", "AliasesAction::Test", "prepare_transition", "activate_prepared"},
        PERSISTENCE_FILES[10]: {"PacksAction::List", "PacksAction::Doctor", "materialize_pack_action", "expected_revision"},
        PERSISTENCE_FILES[11]: {"preview_native_alias_import_file", "apply_native_alias_import", "replace_conflicts"},
        PERSISTENCE_FILES[12]: {"open_existing_read_only", "put_task_bridge", "remove_task_bridge", "trusted_layer"},
    }
    for relative, tokens in required_persistence_tokens.items():
        text = bounded_text(
            root / relative,
            MAX_APPLICATION_SOURCE_BYTES,
            "CP2-CP3.3 application source",
        )
        missing = sorted(token for token in tokens if token not in text)
        if missing:
            raise AliasSpecError(
                f"{relative} is missing CP2-CP3.3 application tokens: {missing}"
            )
    return {"hostile_cases": len(actual)}


def validate_spec_text(text: str) -> None:
    headings = {
        match.group(1).strip()
        for match in re.finditer(r"^##\s+(.+?)\s*$", text, re.MULTILINE)
    }
    missing_headings = sorted(REQUIRED_HEADINGS - headings)
    if missing_headings:
        raise AliasSpecError(
            f"alias specification headings missing: {missing_headings}"
        )
    if re.search(
        r"^Status:\s+(?:active|shipped|complete)",
        text,
        re.MULTILINE | re.IGNORECASE,
    ):
        raise AliasSpecError(
            "CP2-CP3.3 specification must not overclaim a shipped product"
        )
    missing_snippets = sorted(
        token for token in REQUIRED_SPEC_SNIPPETS if token not in text
    )
    if missing_snippets:
        raise AliasSpecError(
            f"alias specification controls missing: {missing_snippets}"
        )


def validate_wiring(root: Path) -> None:
    for relative, token in WIRING.items():
        text = bounded_text(
            root / relative,
            MAX_DOCUMENT_BYTES,
            "alias specification wiring",
        )
        if token not in text:
            raise AliasSpecError(
                f"{relative} is missing alias specification link {token}"
            )


def validate_repository(root: Path = ROOT) -> dict[str, int]:
    contract = load_contract(
        bounded_text(
            root / CONTRACT_PATH.relative_to(ROOT),
            MAX_POLICY_BYTES,
            "CP2/CP3 alias specification contract",
        )
    )
    counts = validate_contract(contract)
    spec = bounded_text(
        root / "docs/DEVOPS-ALIASES.md",
        MAX_DOCUMENT_BYTES,
        "alias specification",
    )
    validate_spec_text(spec)
    counts.update(validate_model_evidence(root))
    validate_wiring(root)
    counts["wiring"] = len(WIRING)
    return counts


def main() -> int:
    try:
        counts = validate_repository()
    except (AliasSpecError, OSError, UnicodeError, json.JSONDecodeError) as error:
        print(
            f"CP2/CP3 alias specification validation failed: {error}",
            file=sys.stderr,
        )
        return 1
    print(
        "PASS: CP2.0-CP3.3 foundations and planned CP4+ aliases are contract-complete "
        f"(shells={counts['shells']}, providers={counts['providers']}, "
        f"scopes={counts['scopes']}, assurance={counts['verification_domains']}, "
        f"ux={counts['ux_invariants']}, model_files={counts['model_files']}, "
        f"persistence_files={counts['persistence_files']}, "
        f"hostile_cases={counts['hostile_cases']}, wiring={counts['wiring']})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
