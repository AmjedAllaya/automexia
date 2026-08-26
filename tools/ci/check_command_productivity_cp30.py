#!/usr/bin/env python3
"""Validate the capability-free CP3.0 shell projection compiler."""

from __future__ import annotations

import json
from pathlib import Path
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "tests/fixtures/command-productivity/cp30-contract-v1.json"
MAX_POLICY_BYTES = 262_144
EXPECTED_LIMITS = {
    "generated_file_bytes": 1_048_576,
    "collision_entries": 4_096,
    "observation_entries": 256,
    "cmd_typed_bindings": 9,
}
EXPECTED_SHELL_MODES = {
    "powershell": ["command-alias", "wrapper-function"],
    "bash": ["command-alias", "wrapper-function"],
    "zsh": ["command-alias", "wrapper-function"],
    "fish": ["abbreviation", "wrapper-function"],
    "cmd": ["doskey-macro"],
}
EXPECTED_CAPABILITIES = {
    "activation": False,
    "profile_read": False,
    "profile_write": False,
    "filesystem": False,
    "process": False,
    "environment": False,
    "network": False,
    "secret_read": False,
    "implicit_exact_override": False,
}
EXPECTED_OBSERVATIONS = {
    "collision_complete": True,
    "completion_complete": True,
    "tool_complete": True,
    "same_action_owner_fingerprint_required": True,
    "tool_health_retained_in_decisions": True,
}
EXPECTED_INTEGRITY = {
    "source_digest_recomputed": True,
    "structured_manifest_verified": True,
    "body_digest_verified": True,
}
EXPECTED_PUBLICATION = {
    "activation": "disabled",
    "profile_hook": False,
    "rollback_anchor": "previous-artifact-digest",
    "uninstall_owner": "CP3.1-exact-managed-artifact",
}

EXPECTED_MODEL_FILES = [
    "automexia-command-productivity/src/actions/activation.rs",
    "automexia-command-productivity/src/actions/mod.rs",
    "automexia-command-productivity/src/actions/model.rs",
    "automexia-command-productivity/src/actions/projection.rs",
    "automexia-command-productivity/src/actions/validation.rs",
]
PROJECTION_FORBIDDEN = {
    "std::env",
    "std::fs",
    "std::net",
    "std::process",
    "command::new",
    "dirs::",
    "notify::",
    "unsafe {",
    "invoke-expression",
}
PROJECTION_REQUIRED = {
    "PROJECTION_SCHEMA_VERSION",
    "MAX_GENERATED_FILE_BYTES",
    "MAX_COLLISION_ENTRIES",
    "MAX_OBSERVATION_ENTRIES",
    "MAX_CMD_TYPED_BINDINGS",
    "canonical_projection_source_digest",
    "compile_shell_projection",
    "verify_projection_artifact",
    "render_powershell",
    "render_bash",
    "render_zsh",
    "render_fish",
    "render_cmd",
    "activation_enabled: false",
    "previous_artifact_digest",
    "ExactOverrideConsent",
    "IncompleteCollisionInventory",
    "IncompleteCompletionInventory",
    "IncompleteToolInventory",
    "SourceDigestMismatch",
    "ToolInventory",
    "owner_fingerprint",
    "tool: Option<ToolHealth>",
}

class Cp30Error(ValueError):
    pass


def bounded_text(path: Path, maximum: int = MAX_POLICY_BYTES) -> str:
    if path.is_symlink() or not path.is_file():
        raise Cp30Error(f"required regular file is missing or linked: {path}")
    if path.stat().st_size > maximum:
        raise Cp30Error(f"CP3.0 evidence exceeds {maximum} bytes: {path}")
    data = path.read_bytes()
    if len(data) > maximum:
        raise Cp30Error(f"CP3.0 evidence grew while reading: {path}")
    return data.decode("utf-8")


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    document: dict[str, Any] = {}
    for key, value in pairs:
        if key in document:
            raise Cp30Error(f"duplicate CP3.0 contract key: {key}")
        document[key] = value
    return document


def load_contract(path: Path = CONTRACT) -> dict[str, Any]:
    document = json.loads(bounded_text(path), object_pairs_hook=reject_duplicate_keys)
    expected_keys = {
        "schema", "phase", "status", "limits", "shell_modes", "capabilities",
        "observations", "integrity", "publication", "model_files", "test_file",
        "required_tests", "fuzz_target", "benchmark", "documents",
    }
    if not isinstance(document, dict) or set(document) != expected_keys:
        raise Cp30Error("CP3.0 contract keys changed")
    if (document["schema"], document["phase"], document["status"]) != (
        1, "CP3.0", "active-pure-compiler-activation-disabled"
    ):
        raise Cp30Error("CP3.0 contract identity changed")
    if document["limits"] != EXPECTED_LIMITS:
        raise Cp30Error("CP3.0 resource ceilings changed")
    if document["shell_modes"] != EXPECTED_SHELL_MODES:
        raise Cp30Error("CP3.0 shell serializer matrix changed")
    if document["capabilities"] != EXPECTED_CAPABILITIES:
        raise Cp30Error("CP3.0 capability boundary changed")
    if document["observations"] != EXPECTED_OBSERVATIONS:
        raise Cp30Error("CP3.0 observation contract changed")
    if document["integrity"] != EXPECTED_INTEGRITY:
        raise Cp30Error("CP3.0 integrity contract changed")
    if document["publication"] != EXPECTED_PUBLICATION:
        raise Cp30Error("CP3.0 publication/rollback contract changed")
    if document["model_files"] != EXPECTED_MODEL_FILES:
        raise Cp30Error("CP3.0 pure model source boundary changed")
    for key in ("required_tests", "documents"):
        values = document[key]
        if not isinstance(values, list) or not values or len(values) != len(set(values)):
            raise Cp30Error(f"CP3.0 {key} must contain unique entries")
    return document


def require_tokens(relative: str, tokens: set[str], root: Path = ROOT) -> str:
    source = bounded_text(root / relative)
    missing = sorted(token for token in tokens if token not in source)
    if missing:
        raise Cp30Error(f"{relative} is missing CP3.0 evidence: {missing}")
    return source


def validate_sources(document: dict[str, Any], root: Path = ROOT) -> dict[str, int]:
    projection_path = "automexia-command-productivity/src/actions/projection.rs"
    projection = require_tokens(projection_path, PROJECTION_REQUIRED, root)
    lowered = projection.casefold()
    marker = next(
        (item for item in sorted(PROJECTION_FORBIDDEN) if item in lowered),
        None,
    )
    if marker:
        raise Cp30Error(f"{projection_path} crosses the pure compiler boundary: {marker}")

    tests = bounded_text(root / document["test_file"])
    missing_tests = sorted(
        name for name in document["required_tests"] if f"fn {name}(" not in tests
    )
    if missing_tests:
        raise Cp30Error(f"CP3.0 required tests are missing: {missing_tests}")

    require_tokens(
        document["fuzz_target"],
        {
            "fuzz_target!", "compile_shell_projection", "verify_projection_artifact",
            "ShellKind::Cmd", "CompletionHealth::Blocked",
            "ToolHealth::Unsupported", "tampered.content",
        },
        root,
    )
    require_tokens(
        document["benchmark"],
        {"quick_action_projection_compile_bash_256", "compile_shell_projection"},
        root,
    )
    for relative in document["documents"]:
        require_tokens(relative, {"CP3.0"}, root)
    return {
        "model_files": len(document["model_files"]),
        "shells": len(document["shell_modes"]),
        "tests": len(document["required_tests"]),
        "documents": len(document["documents"]),
    }


def validate_repository(root: Path = ROOT) -> dict[str, int]:
    contract_path = root / CONTRACT.relative_to(ROOT)
    document = load_contract(contract_path)
    return validate_sources(document, root)


def main() -> int:
    try:
        counts = validate_repository()
    except (Cp30Error, OSError, UnicodeError, json.JSONDecodeError) as error:
        print(f"CP3.0 projection validation failed: {error}", file=sys.stderr)
        return 1
    print(
        "PASS: CP3.0 keeps five deterministic shell serializers bounded, "
        f"capability-free, fuzzed, benchmarked, and non-activated ({counts})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
