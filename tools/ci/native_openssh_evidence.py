#!/usr/bin/env python3
"""Validate bounded, redacted native OpenSSH release evidence.

The default command only probes fixed platform-owned OpenSSH executable
locations. It never installs software, starts a service, opens a network
connection, or changes SSH configuration. Release manifests are produced by
controlled native runners and remain private; the repository contains only a
synthetic validator fixture.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform as host_platform
import re
import stat
import subprocess
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "tests/fixtures/session-launch/d0-d3-contract-v5.json"
SYNTHETIC_FIXTURE = (
    ROOT / "tests/fixtures/session-launch/native-openssh-evidence-synthetic-v1.json"
)
MAX_MANIFEST_BYTES = 262_144
MAX_VERSION_BYTES = 512
MAX_SCENARIO_DURATION_MS = 60_000
MAX_TOTAL_DURATION_MS = 30 * 60_000
PLATFORMS = ("windows", "macos", "linux")
ARCHITECTURES = ("x86_64", "aarch64")
SCENARIO_IDS = (
    "direct-alias",
    "explicit-destination",
    "user-and-port",
    "encrypted-key",
    "agent",
    "certificate",
    "agent-session-binding-restricted-key",
    "host-key-new",
    "host-key-known",
    "host-key-changed",
    "post-quantum-key-exchange",
    "weak-crypto-warning",
    "proxy-jump",
    "local-forward",
    "remote-forward",
    "dynamic-forward",
    "tunnel-bind-collision",
    "cancellation",
    "exit-code",
    "hostile-output",
    "offline",
    "shutdown-cleanup",
    "one-ten-fifty-sessions",
)
MANIFEST_KEYS = {
    "schema",
    "evidence_kind",
    "synthetic",
    "contract_sha256",
    "source_commit",
    "platform",
    "architecture",
    "application",
    "fixture",
    "openssh",
    "scenarios",
    "lifecycle",
    "manual_baseline",
    "redaction",
}
APPLICATION_KEYS = {"version", "binary_sha256", "package_sha256"}
FIXTURE_KEYS = {
    "fixture_sha256",
    "server_config_sha256",
    "known_hosts_seed_sha256",
    "random_seed_sha256",
    "loopback_only",
    "internet_disabled",
    "private_workspace",
    "workspace_removed_after",
}
OPENSSH_KEYS = {
    "client_version",
    "server_version",
    "security_policy",
    "post_quantum_kex",
    "weak_crypto_warning",
    "agent_session_binding_restricted_key",
}
SCENARIO_KEYS = {"id", "result", "duration_ms"}
RESOURCE_LIMITS = {
    "connect_latency_p95_ms": (0, 15_000),
    "cancellation_latency_p95_ms": (0, 5_000),
    "shutdown_latency_p95_ms": (0, 10_000),
    "peak_cpu_millicores": (0, 4_000),
    "idle_cpu_millicores_after": (0, 50),
    "peak_memory_bytes": (1, 2 * 1024 * 1024 * 1024),
    "peak_handles": (1, 16_384),
    "peak_processes": (1, 64),
    "peak_ptys": (1, 50),
    "peak_listeners": (1, 1_600),
    "peak_tunnels": (1, 1_600),
    "peak_tasks": (1, 1_024),
    "peak_routes": (1, 50),
    "peak_cache_bytes": (0, 64 * 1024 * 1024),
    "peak_log_bytes": (0, 2 * 1024 * 1024),
    "peak_storage_bytes": (0, 32 * 1024 * 1024),
}
CLEANUP_FIELDS = {
    "owned_children_after",
    "owned_ptys_after",
    "owned_listeners_after",
    "owned_tunnels_after",
    "owned_tasks_after",
    "stale_routes_after",
    "open_handles_delta_after",
    "temporary_secret_files_after",
    "temporary_workspace_bytes_after",
}
LIFECYCLE_KEYS = {"session_counts", *RESOURCE_LIMITS, *CLEANUP_FIELDS}
BASELINE_KEYS = {
    "client_sha256_before",
    "client_sha256_after",
    "user_config_sha256_before",
    "user_config_sha256_after",
    "manual_ssh_before",
    "manual_ssh_after",
    "disable_preserved",
    "uninstall_preserved",
}
REDACTION_KEYS = {
    "canaries_checked",
    "canary_leaks",
    "forbidden_fields_absent",
}
HEX_64 = re.compile(r"^[0-9a-f]{64}$")
COMMIT = re.compile(r"^[0-9a-f]{40}(?:[0-9a-f]{24})?$")
VERSION = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._,+() /-]{0,255}$")


class NativeOpenSshEvidenceError(ValueError):
    """A release-evidence artifact violated the exact contract."""


def bounded_bytes(path: Path, maximum: int = MAX_MANIFEST_BYTES) -> bytes:
    descriptor: int | None = None
    try:
        before = path.lstat()
        if not stat.S_ISREG(before.st_mode):
            raise NativeOpenSshEvidenceError(
                "required native evidence is missing, linked, or not a regular file"
            )
        flags = os.O_RDONLY | getattr(os, "O_BINARY", 0) | getattr(os, "O_NOFOLLOW", 0)
        descriptor = os.open(path, flags)
        opened = os.fstat(descriptor)
        if (
            not stat.S_ISREG(opened.st_mode)
            or (opened.st_dev, opened.st_ino) != (before.st_dev, before.st_ino)
        ):
            raise NativeOpenSshEvidenceError(
                "native evidence identity changed while opening"
            )
        if opened.st_size <= 0 or opened.st_size > maximum:
            raise NativeOpenSshEvidenceError(
                f"native evidence size must be within 1..{maximum} bytes"
            )
        data = bytearray()
        while len(data) <= maximum:
            chunk = os.read(descriptor, min(65_536, maximum + 1 - len(data)))
            if not chunk:
                break
            data.extend(chunk)
        after = os.fstat(descriptor)
        if (
            (after.st_dev, after.st_ino) != (opened.st_dev, opened.st_ino)
            or after.st_size != opened.st_size
            or len(data) != opened.st_size
            or len(data) > maximum
        ):
            raise NativeOpenSshEvidenceError(
                "native evidence changed or grew while reading"
            )
        return bytes(data)
    except NativeOpenSshEvidenceError:
        raise
    except OSError as error:
        raise NativeOpenSshEvidenceError(
            "required native evidence is unavailable"
        ) from error
    finally:
        if descriptor is not None:
            os.close(descriptor)


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise NativeOpenSshEvidenceError(
                f"duplicate native evidence key: {key}"
            )
        result[key] = value
    return result


def contract_sha256(root: Path = ROOT) -> str:
    contract = root / CONTRACT.relative_to(ROOT)
    return hashlib.sha256(bounded_bytes(contract)).hexdigest()


def _exact_object(value: Any, keys: set[str], label: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != keys:
        raise NativeOpenSshEvidenceError(f"{label} keys changed")
    return value


def _bounded_integer(value: Any, minimum: int, maximum: int, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise NativeOpenSshEvidenceError(f"{label} must be an integer")
    if not minimum <= value <= maximum:
        raise NativeOpenSshEvidenceError(
            f"{label} must be within {minimum}..{maximum}"
        )
    return value


def _hash(value: Any, label: str) -> str:
    if not isinstance(value, str) or HEX_64.fullmatch(value) is None:
        raise NativeOpenSshEvidenceError(f"{label} must be a lowercase SHA-256 digest")
    return value


def _version(value: Any, label: str) -> str:
    if not isinstance(value, str) or VERSION.fullmatch(value) is None:
        raise NativeOpenSshEvidenceError(f"{label} is missing or unsafe")
    return value


def current_source_commit(root: Path = ROOT) -> str:
    try:
        completed = subprocess.run(
            ["git", "-C", str(root), "rev-parse", "--verify", "HEAD"],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            timeout=2,
            check=False,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise NativeOpenSshEvidenceError(
            "the current source commit is unavailable"
        ) from error
    output = completed.stdout[:129]
    if completed.returncode != 0 or len(output) > 128:
        raise NativeOpenSshEvidenceError("the current source commit is unavailable")
    try:
        candidate = output.decode("ascii").strip()
    except UnicodeDecodeError as error:
        raise NativeOpenSshEvidenceError(
            "the current source commit is unavailable"
        ) from error
    if COMMIT.fullmatch(candidate) is None:
        raise NativeOpenSshEvidenceError("the current source commit is unavailable")
    try:
        status = subprocess.run(
            [
                "git",
                "-C",
                str(root),
                "diff-index",
                "--quiet",
                "HEAD",
                "--",
            ],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=2,
            check=False,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise NativeOpenSshEvidenceError(
            "the tracked source tree state is unavailable"
        ) from error
    if status.returncode != 0:
        raise NativeOpenSshEvidenceError(
            "the tracked source tree is not clean at the evidence commit"
        )
    return candidate


def load_manifest(path: Path) -> dict[str, Any]:
    try:
        document = json.loads(
            bounded_bytes(path).decode("utf-8"),
            object_pairs_hook=reject_duplicate_keys,
        )
    except UnicodeDecodeError as error:
        raise NativeOpenSshEvidenceError("native evidence must be UTF-8") from error
    return _exact_object(document, MANIFEST_KEYS, "native evidence")


def validate_manifest(
    path: Path,
    *,
    allow_synthetic: bool = False,
    root: Path = ROOT,
) -> dict[str, int | str]:
    document = load_manifest(path)
    if document["schema"] != 1 or document["evidence_kind"] != "native-openssh-release":
        raise NativeOpenSshEvidenceError("native evidence identity changed")
    if not isinstance(document["synthetic"], bool):
        raise NativeOpenSshEvidenceError("synthetic must be a boolean")
    if document["synthetic"] and not allow_synthetic:
        raise NativeOpenSshEvidenceError("synthetic evidence cannot satisfy a release gate")
    if document["contract_sha256"] != contract_sha256(root):
        raise NativeOpenSshEvidenceError("native evidence is not bound to the active contract")
    if not isinstance(document["source_commit"], str) or COMMIT.fullmatch(
        document["source_commit"]
    ) is None:
        raise NativeOpenSshEvidenceError("source_commit must be an exact lowercase commit digest")
    if not document["synthetic"] and document["source_commit"] != current_source_commit(root):
        raise NativeOpenSshEvidenceError(
            "native evidence is not bound to the current source commit"
        )
    if document["platform"] not in PLATFORMS:
        raise NativeOpenSshEvidenceError("platform must be native Windows, macOS, or Linux")
    if document["architecture"] not in ARCHITECTURES:
        raise NativeOpenSshEvidenceError("architecture is outside the release evidence matrix")

    application = _exact_object(
        document["application"], APPLICATION_KEYS, "application"
    )
    _version(application["version"], "application version")
    _hash(application["binary_sha256"], "application binary")
    _hash(application["package_sha256"], "application package")
    if not document["synthetic"] and (
        application["binary_sha256"] == "0" * 64
        or application["package_sha256"] == "0" * 64
    ):
        raise NativeOpenSshEvidenceError(
            "release application hashes cannot use the synthetic sentinel"
        )

    fixture = _exact_object(document["fixture"], FIXTURE_KEYS, "fixture")
    for key in (
        "fixture_sha256",
        "server_config_sha256",
        "known_hosts_seed_sha256",
        "random_seed_sha256",
    ):
        _hash(fixture[key], key)
    if not document["synthetic"] and any(
        fixture[key] == "0" * 64
        for key in (
            "fixture_sha256",
            "server_config_sha256",
            "known_hosts_seed_sha256",
            "random_seed_sha256",
        )
    ):
        raise NativeOpenSshEvidenceError(
            "release fixture hashes cannot use the synthetic sentinel"
        )
    for key in (
        "loopback_only",
        "internet_disabled",
        "private_workspace",
        "workspace_removed_after",
    ):
        if fixture[key] is not True:
            raise NativeOpenSshEvidenceError(f"fixture requirement failed: {key}")

    openssh = _exact_object(document["openssh"], OPENSSH_KEYS, "openssh")
    client_version = _version(openssh["client_version"], "OpenSSH client version")
    server_version = _version(openssh["server_version"], "OpenSSH server version")
    if not document["synthetic"] and (
        "synthetic" in client_version.lower() or "synthetic" in server_version.lower()
    ):
        raise NativeOpenSshEvidenceError(
            "release OpenSSH versions cannot use synthetic labels"
        )
    if openssh["security_policy"] != "platform-supported-current-advisory-review":
        raise NativeOpenSshEvidenceError("OpenSSH security policy review is missing")
    for key in (
        "post_quantum_kex",
        "weak_crypto_warning",
        "agent_session_binding_restricted_key",
    ):
        if openssh[key] is not True:
            raise NativeOpenSshEvidenceError(f"OpenSSH security requirement failed: {key}")

    scenarios = document["scenarios"]
    if not isinstance(scenarios, list) or len(scenarios) != len(SCENARIO_IDS):
        raise NativeOpenSshEvidenceError("native scenario count changed")
    total_duration = 0
    seen: list[str] = []
    for index, item in enumerate(scenarios):
        scenario = _exact_object(item, SCENARIO_KEYS, f"scenario {index}")
        if scenario["id"] != SCENARIO_IDS[index]:
            raise NativeOpenSshEvidenceError("native scenarios are missing, duplicated, or reordered")
        if scenario["result"] != "pass":
            raise NativeOpenSshEvidenceError(f"native scenario did not pass: {scenario['id']}")
        total_duration += _bounded_integer(
            scenario["duration_ms"], 0, MAX_SCENARIO_DURATION_MS, "scenario duration"
        )
        seen.append(scenario["id"])
    if tuple(seen) != SCENARIO_IDS or total_duration > MAX_TOTAL_DURATION_MS:
        raise NativeOpenSshEvidenceError("native scenario duration or identity budget changed")

    lifecycle = _exact_object(document["lifecycle"], LIFECYCLE_KEYS, "lifecycle")
    if lifecycle["session_counts"] != [1, 10, 50]:
        raise NativeOpenSshEvidenceError("native lifecycle must exercise 1, 10, and 50 sessions")
    for key, (minimum, maximum) in RESOURCE_LIMITS.items():
        _bounded_integer(lifecycle[key], minimum, maximum, key)
    for key in sorted(CLEANUP_FIELDS):
        if lifecycle[key] != 0:
            raise NativeOpenSshEvidenceError(f"cleanup invariant is nonzero: {key}")

    baseline = _exact_object(
        document["manual_baseline"], BASELINE_KEYS, "manual baseline"
    )
    for key in (
        "client_sha256_before",
        "client_sha256_after",
        "user_config_sha256_before",
        "user_config_sha256_after",
    ):
        _hash(baseline[key], key)
    if baseline["client_sha256_before"] != baseline["client_sha256_after"]:
        raise NativeOpenSshEvidenceError("OpenSSH client changed during native validation")
    if baseline["user_config_sha256_before"] != baseline["user_config_sha256_after"]:
        raise NativeOpenSshEvidenceError("user SSH configuration changed during native validation")
    for key in (
        "manual_ssh_before",
        "manual_ssh_after",
        "disable_preserved",
        "uninstall_preserved",
    ):
        if baseline[key] is not True:
            raise NativeOpenSshEvidenceError(f"manual baseline requirement failed: {key}")

    redaction = _exact_object(document["redaction"], REDACTION_KEYS, "redaction")
    _bounded_integer(redaction["canaries_checked"], 1, 1_000_000, "redaction canaries")
    if redaction["canary_leaks"] != 0 or redaction["forbidden_fields_absent"] is not True:
        raise NativeOpenSshEvidenceError("native evidence redaction failed")

    return {
        "platform": document["platform"],
        "scenarios": len(scenarios),
        "sessions": 61,
        "synthetic": int(document["synthetic"]),
    }


def _platform_name() -> str | None:
    if os.name == "nt":
        return "windows"
    if sys.platform == "darwin":
        return "macos"
    if sys.platform.startswith("linux"):
        if "microsoft" in host_platform.release().lower():
            return None
        return "linux"
    return None


def _fixed_candidates(platform_name: str) -> dict[str, tuple[Path, ...]]:
    if platform_name == "windows":
        system_root = Path(os.environ.get("SystemRoot", r"C:\Windows"))
        directory = system_root / "System32" / "OpenSSH"
        return {
            "ssh": (directory / "ssh.exe",),
            "ssh-add": (directory / "ssh-add.exe",),
            "ssh-keygen": (directory / "ssh-keygen.exe",),
            "sshd": (directory / "sshd.exe",),
        }
    roots = (Path("/usr/bin"), Path("/bin"), Path("/usr/local/bin"))
    if platform_name == "macos":
        roots += (Path("/opt/homebrew/bin"),)
    return {name: tuple(root / name for root in roots) for name in ("ssh", "ssh-add", "ssh-keygen", "sshd")}


def _fixed_executable(candidates: tuple[Path, ...]) -> Path | None:
    for candidate in candidates:
        if candidate.is_file() and not candidate.is_symlink():
            return candidate
    return None


def _probe_version(executable: Path) -> str | None:
    try:
        completed = subprocess.run(
            [str(executable), "-V"],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=2,
            check=False,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    output = completed.stdout[: MAX_VERSION_BYTES + 1]
    if len(output) > MAX_VERSION_BYTES:
        return None
    value = output.decode("utf-8", "replace").strip().splitlines()
    if not value:
        return None
    candidate = value[0]
    return candidate if VERSION.fullmatch(candidate) else None


def probe_prerequisites() -> dict[str, Any]:
    platform_name = _platform_name()
    if platform_name is None:
        return {
            "status": "external-prerequisite",
            "platform": "unsupported-or-wsl",
            "missing": ["native-platform"],
            "versions": {},
        }
    executables = {
        name: _fixed_executable(candidates)
        for name, candidates in _fixed_candidates(platform_name).items()
    }
    missing = sorted(name for name, executable in executables.items() if executable is None)
    versions: dict[str, str] = {}
    for name in ("ssh", "sshd"):
        executable = executables[name]
        if executable is not None:
            version = _probe_version(executable)
            if version is not None:
                versions[name] = version
            else:
                missing.append(f"{name}-version")
    missing = sorted(set(missing))
    return {
        "status": "ready-for-controlled-native-run" if not missing else "external-prerequisite",
        "platform": platform_name,
        "missing": missing,
        "versions": versions,
    }


def validate_repository_contract(root: Path = ROOT) -> dict[str, int]:
    fixture = root / SYNTHETIC_FIXTURE.relative_to(ROOT)
    result = validate_manifest(fixture, allow_synthetic=True, root=root)
    if result["synthetic"] != 1:
        raise NativeOpenSshEvidenceError("repository native evidence fixture must be synthetic")
    return {"scenarios": int(result["scenarios"]), "platforms": len(PLATFORMS)}


def main() -> int:
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--validate", type=Path, metavar="MANIFEST")
    group.add_argument("--validate-synthetic", type=Path, metavar="MANIFEST")
    group.add_argument("--check-repository", action="store_true")
    group.add_argument("--validate-environment", action="store_true")
    args = parser.parse_args()
    try:
        if args.validate is not None:
            result: dict[str, Any] = validate_manifest(args.validate)
        elif args.validate_synthetic is not None:
            result = validate_manifest(args.validate_synthetic, allow_synthetic=True)
        elif args.check_repository:
            result = validate_repository_contract()
        elif args.validate_environment:
            manifest = os.environ.get("AUTOMEXIA_QA_NATIVE_OPENSSH_EVIDENCE")
            if not manifest:
                raise NativeOpenSshEvidenceError("native evidence environment is missing")
            result = validate_manifest(Path(manifest))
        else:
            result = probe_prerequisites()
    except (NativeOpenSshEvidenceError, OSError, json.JSONDecodeError) as error:
        print(f"native OpenSSH evidence validation failed: {error}", file=sys.stderr)
        return 1
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    return 0 if result.get("status") != "external-prerequisite" else 2


if __name__ == "__main__":
    raise SystemExit(main())
