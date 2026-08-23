#!/usr/bin/env python3
"""Validate the accepted CP0 completion/Quick Action architecture contract."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re
import sys
import tomllib
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
CONTRACT_PATH = ROOT / "tests/fixtures/command-productivity/cp0-contract-v1.json"
THREATS_PATH = ROOT / "tests/fixtures/command-productivity/cp0-threats-v1.json"
POLICY_DOCUMENT_MAX_BYTES = 262_144
SCANNED_SOURCE_MAX_BYTES = 2_097_152
SCANNED_SOURCE_MAX_FILES = 10_000
EXPECTED_CONTRACT_SHA256 = (
    "5f094279a89c94d34647a8bc1901c179aa7491c940461acff075e22aaabe3413"
)
EXPECTED_THREATS_SHA256 = (
    "e39bd60511092abcc028226b0fd4c457f4ad6ab1101cf0e55e32dcec275e2a59"
)

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
    "discovery_deadline_ms": 500,
    "discovery_output_bytes": 262_144,
    "discovery_entries": 4_096,
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
REQUIRED_ASSETS = {
    "shell-editor-state",
    "user-profile-and-native-definitions",
    "action-source-and-generated-artifacts",
    "credentials-and-secret-references",
    "capability-and-audit-integrity",
    "latency-memory-storage-and-lifecycle",
    "session-capsule-workspace-isolation",
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
REQUIRED_REVIEW_TRIGGERS = {
    "schema-limit-or-precedence-change",
    "new-persistence-profile-or-generated-root",
    "new-shell-provider-plugin-or-generator",
    "new-process-network-secret-clipboard-history-capability",
    "rich-completion-editor-bridge",
    "remote-install-sync-or-pack-distribution",
    "telemetry-or-crash-payload-change",
    "exact-launch-activation",
    "security-incident",
}
SHELL_PROVIDER_HOOKS = {
    "register-argumentcompleter",
    "predictionsource",
    "bash_completion",
    "complete -c ",
    "complete -f ",
    "compdef ",
    "fish_complete_path",
    "fpath=",
    "docker completion",
    "kubectl completion",
    "helm completion",
    "terraform -install-autocomplete",
    "aws_completer",
    "alias k=kubectl",
    "abbr --add k ",
}
PRODUCTIVITY_MARKERS = {
    "action_store",
    "actionstore",
    "alias_projection",
    "autocomplete",
    "completion_provider",
    "completionprovider",
    "quickaction",
    "quick_action",
    "quick action",
    "shell_completion",
    "completionadapter",
    "completion_adapter",
    "actions.toml",
    "commandproductivity",
    "command_productivity",
}
GRID_INFERENCE_MARKERS = {
    "terminal.grid",
    "raw_cursor_line_text",
    "visible_text",
}
# CP0 remains the immutable architecture baseline. CP1 may activate completion
# only through this reviewed, machine-checked file set; every other shell-startup
# path remains subject to the original nonactivation ratchet.
CP1_ALLOWED_SHELL_FILES = {
    "shell-integration/bash/automexia.bash",
    "shell-integration/zsh/automexia.zsh",
    "shell-integration/fish/automexia.fish",
    "shell-integration/powershell/automexia.ps1",
    "shell-integration/completion/bash/automexia-completion.bash",
    "shell-integration/completion/zsh/automexia-completion.zsh",
    "shell-integration/completion/fish/automexia-completion.fish",
    "shell-integration/completion/powershell/automexia-completion.ps1",
    "shell-integration/install-unix.sh",
    "shell-integration/install-windows.ps1",
    "shell-integration/uninstall-unix.sh",
    "shell-integration/uninstall-windows.ps1",
}
CP2_PURE_ACTION_FILES = {
    "automexia-devops/src/actions/activation.rs",
    "automexia-devops/src/actions/imports.rs",
    "automexia-devops/src/actions/mod.rs",
    "automexia-devops/src/actions/model.rs",
    "automexia-devops/src/actions/packs.rs",
    "automexia-devops/src/actions/projection.rs",
    "automexia-devops/src/actions/validation.rs",
}
CP4_PURE_ACTION_FILES = {
    "automexia-devops/src/actions/provider.rs",
}
PURE_ACTION_FILES = CP2_PURE_ACTION_FILES | CP4_PURE_ACTION_FILES
CP2_PERSISTENCE_FILES = {
    "apps/automexia-terminal/src/automexia/quick_actions/cli.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/mod.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/refresh.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/secure_fs.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/service.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/store.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/transfer.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/worker.rs",
}
CP31_PUBLICATION_FILES = {
    "apps/automexia-terminal/src/automexia/quick_actions/aliases.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/aliases_cli.rs",
}
CP32_PACK_FILES = {
    "apps/automexia-terminal/src/automexia/quick_actions/packs_cli.rs",
}
CP33_WORKSPACE_FILES = {
    "apps/automexia-terminal/src/automexia/quick_actions/native_import.rs",
    "apps/automexia-terminal/src/automexia/quick_actions/workspace.rs",
}
CP2_PERSISTENCE_WIRING_FILES = {
    "apps/automexia-terminal/src/automexia/mod.rs",
}
CP2_ACTIVATION_WIRING_FILES = {
    "apps/automexia-terminal/src/automexia/connections/library.rs",
    "apps/automexia-terminal/src/cli.rs",
    "apps/automexia-terminal/src/lib.rs",
    "apps/automexia-terminal/src/main.rs",
    "apps/automexia-terminal/src/renderer/command_palette.rs",
    "apps/automexia-terminal/src/router/mod.rs",
    "apps/automexia-terminal/src/screen/mod.rs",
    "apps/automexia-terminal/src/screen/action_surface.rs",
    "automexia-ui-model/src/lib.rs",
    "automexia-ui-model/src/quick_actions.rs",
}
CP2_PERSISTENCE_FORBIDDEN_MARKERS = {
    "std::net",
    "std::process",
    "command::new",
    "std::env",
    "clipboard",
    "launch_broker",
    "shell_integration",
    "rio_vt::",
    "teletypewriter::",
    "reqwest",
    "ureq::",
    "hyper::",
    "tokio::",
    "async_std::",
    "terminal.grid",
    "visible_text",
    "raw_cursor_line_text",
}
CP31_PUBLICATION_FORBIDDEN_MARKERS = {
    "std::net",
    "clipboard",
    "launch_broker",
    "rio_vt::",
    "teletypewriter::",
    "reqwest",
    "ureq::",
    "hyper::",
    "tokio::",
    "async_std::",
    "terminal.grid",
    "visible_text",
    "raw_cursor_line_text",
}
CP2_PURE_FORBIDDEN_MARKERS = {
    "std::env",
    "std::fs",
    "std::net",
    "std::process",
    "command::new",
    "dirs::",
    "filesystemread",
    "environmentread",
    "terminaloutputread",
    "network",
    "clipboard",
    "launch_broker",
    "shell_integration",
    "tokio::",
    "async_std::",
    "ureq::",
    "hyper::",
    "notify::",
    "automexia_ui_model::",
    "rio_vt::",
    "teletypewriter::",
    "reqwest",
    "unsafe {",
}
SHELL_RECORD_KEYS = {
    "id",
    "editor",
    "platforms",
    "implementation_stage",
    "completion_owner",
    "alias_projection",
    "native_fallback",
}
PROVIDER_RECORD_KEYS = {
    "id",
    "contract",
    "shells",
    "trigger",
    "startup",
    "keystroke",
}
DISCOVERY_RECORD_KEYS = {
    "id",
    "sources",
    "profile_candidates",
    "mutates",
    "invokes_definitions",
}
CASE_RECORD_KEYS = {"id", "expected"}
THREAT_RECORD_KEYS = {
    "id",
    "category",
    "title",
    "controls",
    "verification",
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


def require_exact_keys(
    record: dict[str, Any], expected: set[str], owner: str
) -> None:
    if set(record) != expected:
        raise CommandProductivityError(
            f"{owner} keys must be exactly {sorted(expected)}"
        )


def require_unique_strings(value: Any, owner: str) -> list[str]:
    if (
        not isinstance(value, list)
        or not value
        or any(not isinstance(item, str) or not item for item in value)
        or len(value) != len(set(value))
    ):
        raise CommandProductivityError(
            f"{owner} must contain unique non-empty strings"
        )
    return value


def canonical_fingerprint(document: Any) -> str:
    encoded = json.dumps(
        document,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def validate_contract(document: Any) -> dict[str, int]:
    contract = require_object(document, "CP0 contract")
    required_keys = {
        "schema",
        "phase",
        "status",
        "activation",
        "native_definition_policy",
        "shells",
        "discovery",
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
        require_exact_keys(record, SHELL_RECORD_KEYS, f"shells[{shell}]")
        platforms = require_unique_strings(
            record.get("platforms"), f"shells[{shell}].platforms"
        )
        if (
            set(platforms) != expected_platforms
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

    discoveries = unique_records(contract["discovery"], "discovery")
    if set(discoveries) != set(REQUIRED_SHELLS):
        raise CommandProductivityError(
            f"discovery matrix must be exactly {sorted(REQUIRED_SHELLS)}"
        )
    for shell, record in discoveries.items():
        require_exact_keys(record, DISCOVERY_RECORD_KEYS, f"discovery[{shell}]")
        require_unique_strings(record.get("sources"), f"discovery[{shell}].sources")
        require_unique_strings(
            record.get("profile_candidates"),
            f"discovery[{shell}].profile_candidates",
        )
        if (
            record.get("mutates") is not False
            or record.get("invokes_definitions") is not False
        ):
            raise CommandProductivityError(
                f"{shell} discovery must remain read-only and must not invoke definitions"
            )

    providers = unique_records(contract["providers"], "providers")
    if set(providers) != REQUIRED_PROVIDERS:
        raise CommandProductivityError(
            f"provider matrix must be exactly {sorted(REQUIRED_PROVIDERS)}"
        )
    for provider, record in providers.items():
        require_exact_keys(record, PROVIDER_RECORD_KEYS, f"providers[{provider}]")
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
        provider_shells = require_unique_strings(
            record.get("shells"), f"providers[{provider}].shells"
        )
        if (
            not set(provider_shells).issubset(REQUIRED_SHELLS)
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
    for identifier, record in cases.items():
        require_exact_keys(record, CASE_RECORD_KEYS, f"compatibility_cases[{identifier}]")
        if not str(record.get("expected", "")).strip():
            raise CommandProductivityError(
                "every compatibility case needs an expected result"
            )
    if canonical_fingerprint(contract) != EXPECTED_CONTRACT_SHA256:
        raise CommandProductivityError(
            "CP0 contract differs from the accepted canonical fingerprint"
        )
    return {
        "shells": len(shells),
        "discoveries": len(discoveries),
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
        require_unique_strings(threat_model[field], field)
    if set(threat_model["assets"]) != REQUIRED_ASSETS:
        raise CommandProductivityError(
            f"protected assets must be exactly {sorted(REQUIRED_ASSETS)}"
        )
    if set(threat_model["boundaries"]) != REQUIRED_BOUNDARIES:
        raise CommandProductivityError(
            f"trust boundaries must be exactly {sorted(REQUIRED_BOUNDARIES)}"
        )
    if set(threat_model["review_triggers"]) != REQUIRED_REVIEW_TRIGGERS:
        raise CommandProductivityError(
            f"review triggers must be exactly {sorted(REQUIRED_REVIEW_TRIGGERS)}"
        )
    threats = unique_records(threat_model["threats"], "threats")
    if set(threats) != REQUIRED_THREATS:
        raise CommandProductivityError(
            f"threat catalog must be exactly {sorted(REQUIRED_THREATS)}"
        )
    for identifier, threat in threats.items():
        require_exact_keys(threat, THREAT_RECORD_KEYS, f"threats[{identifier}]")
        for field in ("category", "title"):
            if not isinstance(threat.get(field), str) or not threat[field]:
                raise CommandProductivityError(f"{identifier}.{field} must be non-empty")
        for field in ("controls", "verification"):
            require_unique_strings(threat.get(field), f"{identifier}.{field}")
    if canonical_fingerprint(threat_model) != EXPECTED_THREATS_SHA256:
        raise CommandProductivityError(
            "CP0 threat model differs from the accepted canonical fingerprint"
        )
    return {
        "assets": len(threat_model["assets"]),
        "boundaries": len(threat_model["boundaries"]),
        "threats": len(threats),
        "review_triggers": len(threat_model["review_triggers"]),
    }


def bounded_read_text(path: Path, limit: int, owner: str) -> str:
    if path.is_symlink():
        raise CommandProductivityError(f"{owner} must not be a symbolic link: {path}")
    size = path.stat().st_size
    if size > limit:
        raise CommandProductivityError(
            f"{owner} exceeds the {limit}-byte policy limit: {path} ({size} bytes)"
        )
    with path.open("rb") as source:
        content = source.read(limit + 1)
    if len(content) > limit:
        raise CommandProductivityError(
            f"{owner} grew beyond the {limit}-byte policy limit while reading: {path}"
        )
    return content.decode("utf-8", errors="strict")


def read_lower(path: Path) -> str:
    return bounded_read_text(
        path, SCANNED_SOURCE_MAX_BYTES, "scanned source"
    ).casefold()


def rust_code_without_comments_and_literals(source: str) -> str:
    output: list[str] = []
    index = 0
    block_depth = 0
    state = "code"
    raw_closer = ""
    while index < len(source):
        if state == "line-comment":
            if source[index] in "\r\n":
                output.append(source[index])
                state = "code"
            else:
                output.append(" ")
            index += 1
            continue
        if state == "block-comment":
            if source.startswith("/*", index):
                block_depth += 1
                output.extend((" ", " "))
                index += 2
            elif source.startswith("*/", index):
                block_depth -= 1
                output.extend((" ", " "))
                index += 2
                if block_depth == 0:
                    state = "code"
            else:
                output.append(source[index] if source[index] in "\r\n" else " ")
                index += 1
            continue
        if state == "string":
            character = source[index]
            output.append(character if character in "\r\n" else " ")
            index += 1
            if character == "\\" and index < len(source):
                output.append(source[index] if source[index] in "\r\n" else " ")
                index += 1
            elif character == '"':
                state = "code"
            continue
        if state == "raw-string":
            if source.startswith(raw_closer, index):
                output.extend(" " for _ in raw_closer)
                index += len(raw_closer)
                state = "code"
            else:
                output.append(source[index] if source[index] in "\r\n" else " ")
                index += 1
            continue
        if state == "character":
            character = source[index]
            output.append(character if character in "\r\n" else " ")
            index += 1
            if character == "\\" and index < len(source):
                output.append(source[index] if source[index] in "\r\n" else " ")
                index += 1
            elif character == "'":
                state = "code"
            continue

        if source.startswith("//", index):
            output.extend((" ", " "))
            index += 2
            state = "line-comment"
            continue
        if source.startswith("/*", index):
            output.extend((" ", " "))
            index += 2
            block_depth = 1
            state = "block-comment"
            continue
        raw = re.match(r'(?:b|c)?r(#{0,255})"', source[index:])
        if raw is not None:
            prefix = raw.group(0)
            raw_closer = '"' + raw.group(1)
            output.extend(" " for _ in prefix)
            index += len(prefix)
            state = "raw-string"
            continue
        if source[index] == '"':
            output.append(" ")
            index += 1
            state = "string"
            continue
        character = re.match(r"'(?:\\.|[^\\'\r\n])'", source[index:])
        if character is not None:
            output.append(" ")
            index += 1
            state = "character"
            continue
        output.append(source[index])
        index += 1
    return "".join(output)


def read_rust_code_lower(path: Path) -> str:
    source = bounded_read_text(path, SCANNED_SOURCE_MAX_BYTES, "scanned Rust source")
    return rust_code_without_comments_and_literals(source).casefold()


def normalized_source(path: Path) -> str:
    return re.sub(r"\s+", " ", read_lower(path))


def source_files(root: Path, relative: str) -> list[Path]:
    directory = root / relative
    if not directory.is_dir():
        raise CommandProductivityError(f"required source directory is missing: {relative}")
    if directory.is_symlink():
        raise CommandProductivityError(
            f"required source directory must not be a symbolic link: {relative}"
        )
    files: list[Path] = []
    for entry_count, path in enumerate(directory.rglob("*"), start=1):
        if entry_count > SCANNED_SOURCE_MAX_FILES:
            raise CommandProductivityError(
                f"{relative} exceeds the {SCANNED_SOURCE_MAX_FILES}-file scan limit"
            )
        if path.is_symlink():
            raise CommandProductivityError(
                f"{relative} contains a symbolic link: {path.relative_to(root)}"
            )
        if path.is_file():
            files.append(path)
    return sorted(files)


def workspace_runtime_files(root: Path) -> list[Path]:
    manifest = root / "Cargo.toml"
    with manifest.open("rb") as source:
        workspace = tomllib.load(source)
    members = workspace.get("workspace", {}).get("members")
    if not isinstance(members, list) or not members:
        raise CommandProductivityError("Cargo workspace members must be a non-empty list")
    files: list[Path] = []
    entry_count = 0
    for member in members:
        if not isinstance(member, str) or not member or "*" in member:
            raise CommandProductivityError(
                "CP0 runtime scanning requires explicit Cargo workspace members"
            )
        member_path = Path(member)
        if member_path.is_absolute() or ".." in member_path.parts:
            raise CommandProductivityError(
                f"workspace member escapes the repository boundary: {member}"
            )
        if member == "tools/xtask":
            continue
        member_root = root / member
        source_root = member_root / "src"
        if not source_root.is_dir():
            raise CommandProductivityError(
                f"workspace runtime source directory is missing: {member}/src"
            )
        if member_root.is_symlink() or source_root.is_symlink():
            raise CommandProductivityError(
                f"workspace runtime source must not be a symbolic link: {member}/src"
            )
        for path in source_root.rglob("*"):
            entry_count += 1
            if entry_count > SCANNED_SOURCE_MAX_FILES:
                raise CommandProductivityError(
                    f"runtime workspace exceeds the {SCANNED_SOURCE_MAX_FILES}-file scan limit"
                )
            if path.is_symlink():
                raise CommandProductivityError(
                    f"workspace runtime source contains a symbolic link: {path.relative_to(root)}"
                )
            if path.is_file() and path.suffix == ".rs":
                files.append(path)
        build_script = member_root / "build.rs"
        if build_script.is_file():
            files.append(build_script)
    return sorted(files)


def validate_pure_action_sources(root: Path, runtime_files: list[Path]) -> set[str]:
    present = {
        path.relative_to(root).as_posix()
        for path in runtime_files
        if path.relative_to(root).as_posix().startswith(
            "automexia-devops/src/actions/"
        )
    }
    if not present:
        return set()
    if present != PURE_ACTION_FILES:
        unexpected = sorted(present.symmetric_difference(PURE_ACTION_FILES))
        raise CommandProductivityError(
            f"CP2/CP3/CP4 pure action source set is not the exact reviewed boundary: {unexpected}"
        )
    for relative in sorted(present):
        content = read_lower(root / relative)
        marker = next(
            (item for item in sorted(CP2_PURE_FORBIDDEN_MARKERS) if item in content),
            None,
        )
        if marker is not None:
            raise CommandProductivityError(
                f"{relative} crosses the capability-free CP2/CP3/CP4 model boundary: {marker!r}"
            )
    return present


def validate_persistence_sources(root: Path, runtime_files: list[Path]) -> set[str]:
    present = {
        path.relative_to(root).as_posix()
        for path in runtime_files
        if path.relative_to(root).as_posix().startswith(
            "apps/automexia-terminal/src/automexia/quick_actions/"
        )
    }
    if not present:
        return set()
    expected = (
        CP2_PERSISTENCE_FILES
        | CP31_PUBLICATION_FILES
        | CP32_PACK_FILES
        | CP33_WORKSPACE_FILES
    )
    cp31_present = present & CP31_PUBLICATION_FILES
    cp32_present = present & CP32_PACK_FILES
    cp33_present = present & CP33_WORKSPACE_FILES
    if (
        not CP2_PERSISTENCE_FILES.issubset(present)
        or present - expected
        or (cp31_present and cp31_present != CP31_PUBLICATION_FILES)
        or (cp32_present and cp32_present != CP32_PACK_FILES)
        or (cp33_present and cp33_present != CP33_WORKSPACE_FILES)
    ):
        baseline = CP2_PERSISTENCE_FILES
        if cp31_present:
            baseline |= CP31_PUBLICATION_FILES
        if cp32_present:
            baseline |= CP32_PACK_FILES
        if cp33_present:
            baseline |= CP33_WORKSPACE_FILES
        unexpected = sorted(present.symmetric_difference(baseline))
        raise CommandProductivityError(
            "CP2.1/CP3.1/CP3.2/CP3.3 application source set is not the exact "
            f"reviewed boundary: {unexpected}"
        )
    for relative in sorted(present):
        content = read_rust_code_lower(root / relative)
        markers = (
            CP31_PUBLICATION_FORBIDDEN_MARKERS
            if relative in CP31_PUBLICATION_FILES
            else CP2_PERSISTENCE_FORBIDDEN_MARKERS
        )
        marker = next(
            (item for item in sorted(markers) if item in content),
            None,
        )
        if marker is not None:
            phase = (
                "CP3.1 publication"
                if relative in CP31_PUBLICATION_FILES
                else "CP3.2 pack CLI"
                if relative in CP32_PACK_FILES
                else "CP3.3 import/workspace"
                if relative in CP33_WORKSPACE_FILES
                else "CP2.1 persistence"
            )
            raise CommandProductivityError(
                f"{relative} crosses the {phase}-only capability boundary: {marker!r}"
            )
        if "unsafe {" in content and not relative.endswith("/secure_fs.rs"):
            raise CommandProductivityError(
                f"{relative} adds unsafe code outside the reviewed private-permission adapter"
            )
    wiring = root / "apps/automexia-terminal/src/automexia/mod.rs"
    if "pub mod quick_actions;" not in bounded_read_text(
        wiring, SCANNED_SOURCE_MAX_BYTES, "CP2.1 persistence wiring"
    ):
        raise CommandProductivityError("CP2.1 persistence module wiring is missing")
    return present

def validate_pre_activation(root: Path = ROOT) -> dict[str, int]:
    shell_files = source_files(root, "shell-integration")
    for path in shell_files:
        relative = path.relative_to(root).as_posix()
        if relative in CP1_ALLOWED_SHELL_FILES:
            continue
        shell_text = normalized_source(path)
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
        if len(interactive_files) > SCANNED_SOURCE_MAX_FILES:
            raise CommandProductivityError(
                f"interactive sources exceed the {SCANNED_SOURCE_MAX_FILES}-file scan limit"
            )
    for path in interactive_files:
        content = read_lower(path)
        normalized = re.sub(r"\s+", " ", content)
        for hook in sorted(SHELL_PROVIDER_HOOKS):
            if hook in normalized:
                raise CommandProductivityError(
                    f"{path.relative_to(root).as_posix()} invokes completion/provider work on an interactive path: {hook!r}"
                )
        if any(marker in content for marker in PRODUCTIVITY_MARKERS) and any(
            marker in content for marker in GRID_INFERENCE_MARKERS
        ):
            raise CommandProductivityError(
                f"{path.relative_to(root).as_posix()} couples command productivity to terminal-grid inference"
            )

    runtime_files = workspace_runtime_files(root)
    pure_action_files = validate_pure_action_sources(root, runtime_files)
    persistence_files = validate_persistence_sources(root, runtime_files)
    allowed_runtime_files = (
        pure_action_files
        | persistence_files
        | CP2_PERSISTENCE_WIRING_FILES
        | CP2_ACTIVATION_WIRING_FILES
    )
    for path in runtime_files:
        content = read_lower(path)
        normalized = re.sub(r"\s+", " ", content)
        for hook in sorted(SHELL_PROVIDER_HOOKS):
            if hook in normalized:
                raise CommandProductivityError(
                f"{path.relative_to(root).as_posix()} activates a completion/provider hook before CP1: {hook!r}"
            )
        if path.relative_to(root).as_posix() in allowed_runtime_files:
            continue
        marker = next(
            (item for item in sorted(PRODUCTIVITY_MARKERS) if item in content),
            None,
        )
        if marker is not None:
            raise CommandProductivityError(
                f"{path.relative_to(root).as_posix()} activates command-productivity runtime code during non-runtime CP0: {marker!r}"
            )

    shell_test = read_lower(root / "tools/ci/test_shell_integration.ps1")
    for marker in ("alias docker", "alias kubectl", "function ax", "function kgp"):
        if marker not in shell_test:
            raise CommandProductivityError(
                f"PowerShell integration regression test no longer rejects {marker!r}"
            )
    return {
        "shell_files": len(shell_files),
        "cp1_allowed_shell_files": len(CP1_ALLOWED_SHELL_FILES),
        "cp2_pure_action_files": len(pure_action_files & CP2_PURE_ACTION_FILES),
        "cp4_pure_action_files": len(pure_action_files & CP4_PURE_ACTION_FILES),
        "cp2_persistence_files": len(persistence_files),
        "interactive_files": len(interactive_files),
        "runtime_files": len(runtime_files),
    }


def require_text(path: Path, tokens: set[str]) -> None:
    if not path.is_file():
        raise CommandProductivityError(f"required CP0 document is missing: {path}")
    content = bounded_read_text(path, POLICY_DOCUMENT_MAX_BYTES, "CP0 document")
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
            "14 resource ceilings",
            "seventeen policy tests",
            "### CP0 — decisions, threats, and compatibility",
        },
    )
    require_text(
        root / "docs/COMMAND-PRODUCTIVITY-COMPATIBILITY.md",
        set(REQUIRED_SHELLS)
        | REQUIRED_PROVIDERS
        | {
            "## CP0 acceptance",
            "## Read-only discovery contract",
            "500 ms",
            "256 KiB",
            "4,096-entry",
        },
    )
    require_text(
        root / "docs/COMMAND-PRODUCTIVITY-THREAT-MODEL.md",
        REQUIRED_THREATS
        | {
            "## Security invariants",
            "## Mandatory review triggers",
            "canonical fingerprints",
            "hostile corpus",
        },
    )
    require_text(
        root / "docs/STABILIZATION-ROADMAP.md",
        {
            "#### CP0 implementation ledger",
            "CP0 result: satisfied",
            "14 hard ceilings",
            "11-case hostile corpus",
        },
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
        "tools/ci/test_command_productivity.py": {
            "cp0-hostile-mutations-v1.json",
            "test_versioned_hostile_mutation_corpus_is_rejected",
        },
    }
    for relative, tokens in contracts.items():
        require_text(root / relative, tokens)
    return {"wiring": len(contracts)}


def load_json(path: Path) -> Any:
    return json.loads(
        bounded_read_text(path, POLICY_DOCUMENT_MAX_BYTES, "CP0 JSON fixture")
    )


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
    except (
        CommandProductivityError,
        OSError,
        json.JSONDecodeError,
        tomllib.TOMLDecodeError,
        UnicodeDecodeError,
    ) as error:
        print(f"command productivity CP0 validation failed: {error}", file=sys.stderr)
        return 1
    print(
        "PASS: command productivity CP0/CP1 and bounded CP2.0-CP3.3 model/application boundaries are confined to reviewed allowlists "
        f"(shells={counts['shells']}, providers={counts['providers']}, "
        f"discoveries={counts['discoveries']}, "
        f"cases={counts['cases']}, threats={counts['threats']}, "
        f"shell_files={counts['shell_files']}, "
        f"interactive_files={counts['interactive_files']}, "
        f"runtime_files={counts['runtime_files']}, "
        f"cp2_pure_action_files={counts['cp2_pure_action_files']}, "
        f"cp4_pure_action_files={counts['cp4_pure_action_files']}, "
        f"cp2_persistence_files={counts['cp2_persistence_files']})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
