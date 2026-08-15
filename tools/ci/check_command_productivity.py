#!/usr/bin/env python3
"""Validate the accepted CP0 completion/Quick Action architecture contract."""

from __future__ import annotations

import json
from pathlib import Path
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
CONTRACT_PATH = ROOT / "tests/fixtures/command-productivity/cp0-contract-v1.json"
THREATS_PATH = ROOT / "tests/fixtures/command-productivity/cp0-threats-v1.json"

REQUIRED_SHELLS = {
    "powershell": {"windows", "linux", "macos"},
    "bash": {"linux", "macos", "wsl"},
    "zsh": {"linux", "macos", "wsl"},
    "fish": {"linux", "macos", "wsl"},
    "cmd": {"windows"},
}
REQUIRED_PROVIDERS = {
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
}
REQUIRED_PRECEDENCE = [
    "session",
    "capsule",
    "trusted-workspace",
    "shell-user",
    "global-user",
    "builtin-disabled",
]
REQUIRED_LIMITS = {
    "source_file_bytes": 1_048_576,
    "active_actions": 1_024,
    "enabled_aliases": 256,
    "arguments_per_action": 64,
    "placeholders_per_action": 32,
    "string_bytes": 4_096,
    "candidates_per_request": 512,
    "candidate_bytes": 1_024,
    "candidate_response_bytes": 524_288,
    "generated_file_bytes": 1_048_576,
    "in_process_cache_bytes": 8_388_608,
}
REQUIRED_CASES = {
    "native-definition-collision",
    "duplicate-id-same-layer",
    "same-id-different-layer",
    "unsupported-shell-version",
    "integration-disabled",
    "provider-missing",
    "provider-timeout",
    "generated-file-tamper",
    "workspace-trust-revoked",
    "session-capsule-rebind",
    "uninstall",
}
REQUIRED_THREATS = {f"CP-T{index:02d}" for index in range(1, 17)}
REQUIRED_BOUNDARIES = {
    "action-input-to-typed-model",
    "provider-generator-to-managed-artifact",
    "typed-action-to-shell-editor",
    "typed-exact-action-to-D3-broker",
    "scoped-layer-to-session-index",
    "generated-file-to-shell-startup",
    "private-state-to-diagnostics",
}
SHELL_PROVIDER_HOOKS = {
    "register-argumentcompleter",
    "predictionsource",
    "docker completion",
    "kubectl completion",
    "helm completion",
    "terraform -install-autocomplete",
    "aws_completer",
    "alias k=kubectl",
    "abbr --add k ",
}
PRODUCTIVITY_MARKERS = {
    "quickaction",
    "quick_action",
    "completionadapter",
    "completion_adapter",
    "actions.toml",
}
GRID_INFERENCE_MARKERS = {
    "terminal.grid",
    "raw_cursor_line_text",
    "visible_text",
}


class CommandProductivityError(ValueError):
    """The CP0 contract, evidence, or pre-activation boundary is invalid."""


def require_object(value: Any, owner: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise CommandProductivityError(f"{owner} must be an object")
    return value


def unique_records(value: Any, owner: str) -> dict[str, dict[str, Any]]:
    if not isinstance(value, list) or not value:
        raise CommandProductivityError(f"{owner} must be a non-empty list")
    records: dict[str, dict[str, Any]] = {}
    for index, raw in enumerate(value):
        record = require_object(raw, f"{owner}[{index}]")
        identifier = record.get("id")
        if not isinstance(identifier, str) or not identifier:
            raise CommandProductivityError(f"{owner}[{index}].id must be non-empty")
        if identifier in records:
            raise CommandProductivityError(f"{owner} has duplicate id {identifier}")
        records[identifier] = record
    return records


def validate_contract(document: Any) -> dict[str, int]:
    contract = require_object(document, "CP0 contract")
    required_keys = {
        "schema",
        "phase",
        "status",
        "activation",
        "native_definition_policy",
        "shells",
        "providers",
        "precedence",
        "execution_modes",
        "limits",
        "forbidden_runtime_paths",
        "compatibility_cases",
    }
    if set(contract) != required_keys:
        raise CommandProductivityError(
            f"CP0 contract keys must be exactly {sorted(required_keys)}"
        )
    if (
        contract["schema"] != 1
        or contract["phase"] != "CP0"
        or contract["status"] != "accepted"
        or contract["activation"] != "non-runtime"
    ):
        raise CommandProductivityError(
            "CP0 contract must remain accepted schema 1 with non-runtime activation"
        )
    if (
        contract["native_definition_policy"]
        != "native-wins-unless-explicit-reversible-override"
    ):
        raise CommandProductivityError("native shell definitions must remain authoritative")

    shells = unique_records(contract["shells"], "shells")
    if set(shells) != set(REQUIRED_SHELLS):
        raise CommandProductivityError(
            f"shell matrix must be exactly {sorted(REQUIRED_SHELLS)}"
        )
    allowed_platforms = {"windows", "linux", "macos", "wsl"}
    for shell, expected_platforms in REQUIRED_SHELLS.items():
        record = shells[shell]
        platforms = record.get("platforms")
        if (
            not isinstance(platforms, list)
            or set(platforms) != expected_platforms
            or any(platform not in allowed_platforms for platform in platforms)
        ):
            raise CommandProductivityError(
                f"{shell} platforms must be exactly {sorted(expected_platforms)}"
            )
        if (
            record.get("implementation_stage") != "CP1"
            or record.get("completion_owner") != "shell-editor"
            or not str(record.get("alias_projection", "")).strip()
            or not str(record.get("native_fallback", "")).startswith("preserved")
        ):
            raise CommandProductivityError(
                f"{shell} must retain shell-editor ownership, CP1 staging, alias policy, and fallback"
            )

    providers = unique_records(contract["providers"], "providers")
    if set(providers) != REQUIRED_PROVIDERS:
        raise CommandProductivityError(
            f"provider matrix must be exactly {sorted(REQUIRED_PROVIDERS)}"
        )
    for provider, record in providers.items():
        if record.get("startup") is not False or record.get("keystroke") is not False:
            raise CommandProductivityError(
                f"{provider} completion must not run at startup or per keystroke"
            )
        if record.get("trigger") not in {
            "explicit-refresh-only",
            "explicit-consent-only",
        }:
            raise CommandProductivityError(
                f"{provider} completion trigger must be explicit"
            )
        provider_shells = record.get("shells")
        if (
            not isinstance(provider_shells, list)
            or not provider_shells
            or not set(provider_shells).issubset(REQUIRED_SHELLS)
        ):
            raise CommandProductivityError(
                f"{provider} must declare supported known shells"
            )

    if contract["precedence"] != REQUIRED_PRECEDENCE:
        raise CommandProductivityError(
            f"precedence must remain {REQUIRED_PRECEDENCE}"
        )
    modes = require_object(contract["execution_modes"], "execution_modes")
    expected_modes = {
        "default": "insert-without-enter",
        "copy": "explicit-only",
        "raw": "shell-scoped-insert-only",
        "exact_launch": "D3-broker-only",
    }
    if modes != expected_modes:
        raise CommandProductivityError(
            "execution modes must preserve review-before-insert and D3-only exact launch"
        )
    limits = require_object(contract["limits"], "limits")
    if limits != REQUIRED_LIMITS:
        raise CommandProductivityError(
            f"CP0 resource ceilings must remain exactly {REQUIRED_LIMITS}"
        )
    runtime_paths = contract["forbidden_runtime_paths"]
    if not isinstance(runtime_paths, list) or set(runtime_paths) != {
        "renderer",
        "screen-input",
        "vt-parser",
        "pty-reader",
        "startup",
        "keystroke",
    }:
        raise CommandProductivityError("interactive forbidden-path matrix is incomplete")
    cases = unique_records(contract["compatibility_cases"], "compatibility_cases")
    if set(cases) != REQUIRED_CASES:
        raise CommandProductivityError(
            f"compatibility cases must be exactly {sorted(REQUIRED_CASES)}"
        )
    if any(not str(record.get("expected", "")).strip() for record in cases.values()):
        raise CommandProductivityError("every compatibility case needs an expected result")
    return {
        "shells": len(shells),
        "providers": len(providers),
        "cases": len(cases),
        "limits": len(limits),
    }


def validate_threats(document: Any) -> dict[str, int]:
    threat_model = require_object(document, "CP0 threat model")
    required_keys = {
        "schema",
        "phase",
        "status",
        "assets",
        "boundaries",
        "threats",
        "review_triggers",
    }
    if set(threat_model) != required_keys:
        raise CommandProductivityError(
            f"CP0 threat keys must be exactly {sorted(required_keys)}"
        )
    if (
        threat_model["schema"] != 1
        or threat_model["phase"] != "CP0"
        or threat_model["status"] != "accepted"
    ):
        raise CommandProductivityError("CP0 threat model must remain accepted schema 1")
    for field in ("assets", "boundaries", "review_triggers"):
        values = threat_model[field]
        if (
            not isinstance(values, list)
            or not values
            or len(values) != len(set(values))
            or any(not isinstance(value, str) or not value for value in values)
        ):
            raise CommandProductivityError(f"{field} must be unique non-empty strings")
    if set(threat_model["boundaries"]) != REQUIRED_BOUNDARIES:
        raise CommandProductivityError(
            f"trust boundaries must be exactly {sorted(REQUIRED_BOUNDARIES)}"
        )
    threats = unique_records(threat_model["threats"], "threats")
    if set(threats) != REQUIRED_THREATS:
        raise CommandProductivityError(
            f"threat catalog must be exactly {sorted(REQUIRED_THREATS)}"
        )
    for identifier, threat in threats.items():
        for field in ("category", "title"):
            if not isinstance(threat.get(field), str) or not threat[field]:
                raise CommandProductivityError(f"{identifier}.{field} must be non-empty")
        for field in ("controls", "verification"):
            values = threat.get(field)
            if (
                not isinstance(values, list)
                or not values
                or any(not isinstance(value, str) or not value for value in values)
            ):
                raise CommandProductivityError(
                    f"{identifier}.{field} must be non-empty strings"
                )
    return {
        "assets": len(threat_model["assets"]),
        "boundaries": len(threat_model["boundaries"]),
        "threats": len(threats),
        "review_triggers": len(threat_model["review_triggers"]),
    }


def read_lower(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="strict").casefold()


def source_files(root: Path, relative: str) -> list[Path]:
    directory = root / relative
    if not directory.is_dir():
        raise CommandProductivityError(f"required source directory is missing: {relative}")
    return sorted(path for path in directory.rglob("*") if path.is_file())


def validate_pre_activation(root: Path = ROOT) -> dict[str, int]:
    shell_files = source_files(root, "shell-integration")
    shell_text = "\n".join(read_lower(path) for path in shell_files)
    for hook in sorted(SHELL_PROVIDER_HOOKS):
        if hook in shell_text:
            raise CommandProductivityError(
                f"CP0 shell startup unexpectedly activates provider/completion hook {hook!r}"
            )

    interactive_roots = [
        "apps/automexia-terminal/src/renderer",
        "apps/automexia-terminal/src/screen",
        "rio-vt/src",
        "teletypewriter/src",
    ]
    interactive_files: list[Path] = []
    for relative in interactive_roots:
        interactive_files.extend(source_files(root, relative))
    for path in interactive_files:
        content = read_lower(path)
        for hook in sorted(SHELL_PROVIDER_HOOKS):
            if hook in content:
                raise CommandProductivityError(
                    f"{path.relative_to(root).as_posix()} invokes completion/provider work on an interactive path: {hook!r}"
                )
        if any(marker in content for marker in PRODUCTIVITY_MARKERS) and any(
            marker in content for marker in GRID_INFERENCE_MARKERS
        ):
            raise CommandProductivityError(
                f"{path.relative_to(root).as_posix()} couples command productivity to terminal-grid inference"
            )

    shell_test = read_lower(root / "tools/ci/test_shell_integration.ps1")
    for marker in ("alias docker", "alias kubectl", "function ax", "function kgp"):
        if marker not in shell_test:
            raise CommandProductivityError(
                f"PowerShell integration regression test no longer rejects {marker!r}"
            )
    return {
        "shell_files": len(shell_files),
        "interactive_files": len(interactive_files),
    }


def require_text(path: Path, tokens: set[str]) -> None:
    if not path.is_file():
        raise CommandProductivityError(f"required CP0 document is missing: {path}")
    content = path.read_text(encoding="utf-8")
    folded = content.casefold()
    missing = sorted(token for token in tokens if token.casefold() not in folded)
    if missing:
        try:
            display_path = path.relative_to(ROOT).as_posix()
        except ValueError:
            display_path = path.as_posix()
        raise CommandProductivityError(
            f"{display_path} is missing CP0 tokens: {missing}"
        )


def validate_documents(root: Path = ROOT) -> dict[str, int]:
    require_text(
        root / "docs/adr/0015-shell-native-completion-and-typed-quick-actions.md",
        {"- Status: Accepted for v0.5", "## Decision", "## Review and activation gate"},
    )
    require_text(
        root / "docs/COMMAND-PRODUCTIVITY.md",
        {
            "COMMAND-PRODUCTIVITY-COMPATIBILITY.md",
            "COMMAND-PRODUCTIVITY-THREAT-MODEL.md",
            "### CP0 — decisions, threats, and compatibility",
        },
    )
    require_text(
        root / "docs/COMMAND-PRODUCTIVITY-COMPATIBILITY.md",
        set(REQUIRED_SHELLS) | REQUIRED_PROVIDERS | {"## CP0 acceptance"},
    )
    require_text(
        root / "docs/COMMAND-PRODUCTIVITY-THREAT-MODEL.md",
        REQUIRED_THREATS | {"## Security invariants", "## Mandatory review triggers"},
    )
    require_text(
        root / "docs/STABILIZATION-ROADMAP.md",
        {"#### CP0 implementation ledger", "CP0 result: satisfied"},
    )
    return {"documents": 5}


def validate_wiring(root: Path = ROOT) -> dict[str, int]:
    contracts = {
        "tools/ci/validate_repository.py": {
            "validate_command_productivity()",
            '"command productivity CP0"',
        },
        "tools/xtask/src/main.rs": {
            'run_python("tools/ci/check_command_productivity.py")?',
        },
        ".github/workflows/ci.yml": {
            "python tools/ci/test_command_productivity.py",
        },
    }
    for relative, tokens in contracts.items():
        require_text(root / relative, tokens)
    return {"wiring": len(contracts)}


def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as source:
        return json.load(source)


def validate_repository(root: Path = ROOT) -> dict[str, int]:
    contract_path = root / CONTRACT_PATH.relative_to(ROOT)
    threats_path = root / THREATS_PATH.relative_to(ROOT)
    counts = {}
    counts.update(validate_contract(load_json(contract_path)))
    counts.update(validate_threats(load_json(threats_path)))
    counts.update(validate_pre_activation(root))
    counts.update(validate_documents(root))
    counts.update(validate_wiring(root))
    return counts


def main() -> int:
    try:
        counts = validate_repository()
    except (CommandProductivityError, OSError, json.JSONDecodeError) as error:
        print(f"command productivity CP0 validation failed: {error}", file=sys.stderr)
        return 1
    print(
        "PASS: command productivity CP0 contract is accepted and non-activated "
        f"(shells={counts['shells']}, providers={counts['providers']}, "
        f"cases={counts['cases']}, threats={counts['threats']}, "
        f"shell_files={counts['shell_files']}, interactive_files={counts['interactive_files']})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
