#!/usr/bin/env python3
"""Validate the bounded CP1 native-completion activation contract."""

from __future__ import annotations

import json
from pathlib import Path
import re
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
CONTRACT_PATH = ROOT / "tests/fixtures/command-productivity/cp1-contract-v1.json"
MAX_POLICY_BYTES = 65_536
MAX_SOURCE_BYTES = 1_048_576
SHELLS = {
    "powershell": "managed-explicit-override",
    "bash": "managed-native-first",
    "zsh": "managed-native-first",
    "fish": "managed-native-first",
    "cmd": "native-fallback",
}
PROVIDERS = {
    "git": ("git", "native/provider-owned"),
    "docker": ("docker", "shell-specific"),
    "kubernetes": ("kubectl", "explicit-refresh"),
    "openshift": ("oc", "explicit-refresh"),
    "helm": ("helm", "explicit-refresh"),
    "terraform": ("terraform", "manual-consent-required"),
    "opentofu": ("tofu", "manual-consent-required"),
    "aws": ("aws_completer", "native/provider-owned"),
    "azure": ("az", "native/provider-owned"),
    "gcp": ("gcloud", "native/provider-owned"),
    "openssh": ("ssh", "native/provider-owned"),
}
LIMITS = {
    "provider_deadline_ms": 750,
    "provider_stdout_bytes": 1_048_576,
    "provider_stderr_bytes": 262_144,
    "version_output_bytes": 16_384,
    "config_path_bytes": 4_096,
    "profile_bytes": 1_048_576,
}
OPERATIONS = ["doctor", "enable", "disable", "refresh", "remove"]
CAPABILITIES = {
    "network": False,
    "secret_read": False,
    "terminal_grid_inference": False,
    "provider_on_startup": False,
    "provider_on_keystroke": False,
}
ACTIVATION_FILES = {
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


class Cp1Error(ValueError):
    """The CP1 activation boundary is invalid."""


def bounded_text(path: Path, maximum: int, label: str) -> str:
    if path.is_symlink():
        raise Cp1Error(f"{label} must not be a symbolic link: {path}")
    size = path.stat().st_size
    if size > maximum:
        raise Cp1Error(f"{label} exceeds {maximum} bytes: {path}")
    return path.read_text(encoding="utf-8")


def validate_contract(document: Any) -> dict[str, int]:
    if not isinstance(document, dict):
        raise Cp1Error("CP1 contract must be an object")
    expected_keys = {
        "schema", "phase", "status", "shells", "providers", "limits",
        "operations", "activation_files", "capabilities",
    }
    if set(document) != expected_keys:
        raise Cp1Error("CP1 contract keys changed")
    if (document["schema"], document["phase"], document["status"]) != (1, "CP1", "active"):
        raise Cp1Error("CP1 must remain active schema 1")
    if document["shells"] != SHELLS:
        raise Cp1Error("CP1 shell policy changed")
    actual_providers = {
        key: (value.get("command"), value.get("policy"))
        for key, value in document["providers"].items()
        if isinstance(value, dict)
    }
    if actual_providers != PROVIDERS or len(actual_providers) != len(document["providers"]):
        raise Cp1Error("CP1 provider policy changed")
    if document["limits"] != LIMITS:
        raise Cp1Error("CP1 resource ceilings changed")
    if document["operations"] != OPERATIONS:
        raise Cp1Error("CP1 operations changed")
    if set(document["activation_files"]) != ACTIVATION_FILES or len(document["activation_files"]) != len(ACTIVATION_FILES):
        raise Cp1Error("CP1 activation allowlist changed")
    if document["capabilities"] != CAPABILITIES:
        raise Cp1Error("CP1 capability boundary changed")
    return {"shells": len(SHELLS), "providers": len(PROVIDERS), "activation_files": len(ACTIVATION_FILES)}


def validate_adapter_text(relative: str, text: str) -> None:
    folded = text.casefold()
    if relative.startswith("shell-integration/completion/"):
        if "eval " in folded or "invoke-expression" in folded:
            raise Cp1Error(f"{relative} contains dynamic evaluation")
        required = ["sha256", "disabled", "4096", "192"]
        if relative.endswith(".bash"):
            required += ["complete -p", ". ", "$file", "__automexia_completion_directory_safe", "== /*"]
        elif relative.endswith(".zsh"):
            required += ["_comps", "source", "$file", "__automexia_completion_directory_safe", "== /*"]
        elif relative.endswith(".fish"):
            required += ["complete -c", "source", "$file", "__automexia_completion_directory_safe", "^/"]
        elif relative.endswith(".ps1"):
            required += ["allow-override", "Get-AutomexiaCompletionFileSha256", "Security.Cryptography.SHA256", ". $file", "Test-AutomexiaCompletionDirectorySafe", "UNC root"]
        if relative.endswith((".bash", ".zsh", ".fish")):
            required += ["Application Support/io.github.AmjedAllaya.AutomexiaTerminal"]
        missing = [token for token in required if token.casefold() not in folded]
        if missing:
            raise Cp1Error(f"{relative} is missing adapter controls: {missing}")


def validate_sources(root: Path = ROOT) -> dict[str, int]:
    for relative in ACTIVATION_FILES:
        path = root / relative
        if not path.is_file():
            raise Cp1Error(f"CP1 activation source is missing: {relative}")
        validate_adapter_text(relative, bounded_text(path, MAX_SOURCE_BYTES, "CP1 source"))

    xtask = bounded_text(root / "tools/xtask/src/completion.rs", MAX_SOURCE_BYTES, "CP1 provider manager")
    required = {
        "Duration::from_millis(750)", "MAX_PROVIDER_OUTPUT", "MAX_PROVIDER_STDERR",
        "stdin(Stdio::null())", "start_kill()", "CommandWrap", "ProcessGroup::leader()",
        "JobObject", "bounded_process_terminates_descendants_holding_output_pipes",
        "bounded_process_terminates_pipe_holders_after_leader_exit",
        "validate_refresh_executable", "native .exe/.com", "NamedTempFile", "symlink_metadata",
        "inspect_artifact", "metadata-mismatch", "revalidate_executable", "MAX_METADATA_BYTES",
        "Application Support", "completion configuration root must be absolute",
        "--allow-native-override", "native/provider-owned", "manual-consent-required",
        "transition_digest_bytes", "runtime_artifact_digest",
        "env_clear", "filtered_provider_path", "NO_COLOR",
        "provider_process_does_not_inherit_ambient_secret_environment",
        "sanitize_diagnostic", "bidirectional control characters",
        "absolute local PATH entry", "local Windows drive",
    }
    missing = sorted(token for token in required if token not in xtask)
    if missing:
        raise Cp1Error(f"CP1 provider manager is missing controls: {missing}")
    if re.search(r"\b(reqwest|ureq|hyper|curl)\b", xtask, re.IGNORECASE):
        raise Cp1Error("CP1 provider manager must not contain a network client")

    wiring = {
        "tools/xtask/src/main.rs": {"completion::dispatch", "completion COMMAND [OPTIONS]"},
        "tools/ci/validate_repository.py": {"validate_command_productivity_cp1"},
        ".github/workflows/ci.yml": {"test_command_productivity_cp1.py", "fish"},
        "docs/COMMAND-PRODUCTIVITY.md": {"CP1 status", "cargo xtask completion doctor"},
        "docs/TESTING.md": {"Command-productivity CP1"},
        "tools/ci/measure_completion_adapter.py": {
            "WARMUPS = 5", "SAMPLES = 20", "MAX_P95_MS = 50.0",
            "time.perf_counter_ns", "subprocess.run", "DEADLINE_SECONDS",
        },
        "tools/ci/test_measure_completion_adapter.py": {
            "test_p95_uses_nearest_rank", "test_exact_runner_rejects_unbounded_diagnostics",
        },
        "shell-integration/install-unix.sh": {
            "schema=3", "Application Support/io.github.AmjedAllaya.AutomexiaTerminal",
            "config root must be absolute", "grep -Fqx", "bash_source_line",
        },
        "shell-integration/uninstall-unix.sh": {
            "system_name", "io.github.AmjedAllaya.AutomexiaTerminal",
            "All destructive targets are validated before the first mutation",
            "assert_owned_file", "config root must be absolute",
        },
        "shell-integration/install-windows.ps1": {
            "Test-MarkedBlockBody", "Get-AutomexiaPowerShellHook",
            "Assert-AutomexiaRealDirectory", "schema=3",
        },
        "shell-integration/uninstall-windows.ps1": {
            "Test-AutomexiaAbsoluteWindowsPath", "Assert-AutomexiaOwnedFile",
            "Remove-AutomexiaOwnedFile",
        },
        "tools/ci/test_shell_sources.sh": {
            "stale-owned-source-line", "relative-root", "config-link",
            "Application Support/io.github.AmjedAllaya.AutomexiaTerminal",
        },
        "tools/ci/test_shell_integration.sh": {
            "interrupted refresh", "4097",
        },
        "tools/ci/test_zsh_integration.zsh": {
            "interrupted refresh", "4097",
        },
        "tools/ci/test_fish_integration.fish": {
            "interrupted refresh", "4097",
        },
        "tools/ci/test_shell_integration.ps1": {
            "relative-config-root", "interrupted refresh", "4097",
            "remote persistence root",
        },
    }
    for relative, tokens in wiring.items():
        text = bounded_text(root / relative, MAX_SOURCE_BYTES, "CP1 wiring")
        missing = sorted(token for token in tokens if token not in text)
        if missing:
            raise Cp1Error(f"{relative} is missing CP1 wiring: {missing}")
    return {"sources": len(ACTIVATION_FILES) + 1, "wiring": len(wiring)}


def validate_repository(root: Path = ROOT) -> dict[str, int]:
    contract = json.loads(bounded_text(root / CONTRACT_PATH.relative_to(ROOT), MAX_POLICY_BYTES, "CP1 contract"))
    counts = validate_contract(contract)
    counts.update(validate_sources(root))
    return counts


def main() -> int:
    try:
        counts = validate_repository()
    except (Cp1Error, OSError, UnicodeError, json.JSONDecodeError) as error:
        print(f"command productivity CP1 validation failed: {error}", file=sys.stderr)
        return 1
    print(
        "PASS: command productivity CP1 native completion is bounded and allowlisted "
        f"(shells={counts['shells']}, providers={counts['providers']}, "
        f"activation_files={counts['activation_files']}, wiring={counts['wiring']})"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
