#!/usr/bin/env python3
"""Validate the planned CP2/CP3 persistent DevOps alias specification."""

from __future__ import annotations

import json
from pathlib import Path
import re
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
CONTRACT_PATH = ROOT / "tests/fixtures/command-productivity/cp2-cp3-alias-spec-v1.json"
MAX_POLICY_BYTES = 65_536
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
    "string_bytes": 4_096,
    "generated_file_bytes": 1_048_576,
    "in_process_cache_bytes": 8_388_608,
}
ACTION_FIELDS = [
    "schema_version",
    "id",
    "display_name",
    "description",
    "tags",
    "scope",
    "shells",
    "template",
    "arguments",
    "placeholders",
    "working_directory_policy",
    "risk",
    "execution",
    "provenance",
    "enabled",
    "alias_projection",
]
MODEL_ENUMS = {
    "templates": ["TypedArgv", "RawInsertOnly"],
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
    "Status: planned for CP2",
    "not shipped in v0.4",
    "actions/actions.toml",
    "actions/actions.previous.toml",
    "generated/aliases",
    "[a-z][a-z0-9-]{1,31}",
    "Nothing short is enabled by default.",
    "No secret aliases",
    "No hidden context mutation",
    "atomic all-or-old",
    "compare-and-swap",
    "template: TypedArgv | RawInsertOnly",
    "mode: Auto | CommandAlias | WrapperFunction | FishAbbreviation | DoskeyMacro",
    "argument_policy: None | ForwardAll | TypedBindings",
    "Health states include `Ready`",
    "<= 25 ms p95 off renderer/input/PTY paths",
    "<= 16 ms p95; deterministic and allocation-bounded",
    "<= 50 ms p95 post-warmup, no subprocess/network",
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
    """The planned alias specification is incomplete, unsafe, or activated."""


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
        "authorities",
        "shells",
        "providers",
        "scopes",
        "limits",
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
        "activation_files",
    }
    if set(document) != expected_keys:
        raise AliasSpecError("alias specification contract keys changed")
    if (document["schema"], document["phase"], document["status"]) != (
        1,
        "CP2-CP3-SPEC",
        "planned",
    ):
        raise AliasSpecError("CP2/CP3 specification must remain planned schema 1")
    checks = (
        ("authorities", AUTHORITIES, "authority map"),
        ("shells", SHELLS, "shell/platform matrix"),
        ("providers", PROVIDERS, "provider catalog"),
        ("scopes", SCOPES, "scope precedence"),
        ("limits", LIMITS, "resource ceilings"),
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
        ("activation_files", [], "non-activation boundary"),
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
    }


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
    missing_snippets = sorted(
        token for token in REQUIRED_SPEC_SNIPPETS if token not in text
    )
    if missing_snippets:
        raise AliasSpecError(
            f"alias specification controls missing: {missing_snippets}"
        )
    if re.search(
        r"^Status:\s+(?:active|shipped|complete)",
        text,
        re.MULTILINE | re.IGNORECASE,
    ):
        raise AliasSpecError(
            "planned alias specification must not claim runtime activation"
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
        "PASS: planned CP2/CP3 alias specification is complete and non-activated "
        f"(shells={counts['shells']}, providers={counts['providers']}, "
        f"scopes={counts['scopes']}, assurance={counts['verification_domains']}, "
        f"ux={counts['ux_invariants']}, wiring={counts['wiring']})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())