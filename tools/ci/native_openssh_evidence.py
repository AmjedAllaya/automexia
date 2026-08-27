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
CONTRACT = ROOT / "tests/fixtures/session-launch/d0-d3-contract-v7.json"
SYNTHETIC_FIXTURE = (
    ROOT / "tests/fixtures/session-launch/native-openssh-evidence-synthetic-v2.json"
)
HISTORICAL_SYNTHETIC_FIXTURES = (
    (
        ROOT / "tests/fixtures/session-launch/native-openssh-evidence-synthetic-v1.json",
        "2d2b1c8448eba5aa8149b078ef598271d3c2e4be7712dd26b8bc73b7ce1d511d",
    ),
)
MAX_MANIFEST_BYTES = 262_144
MAX_RELEASE_ARTIFACT_BYTES = 4 * 1024 * 1024 * 1024
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
    "client_sha256",
    "server_sha256",
    "ssh_add_sha256",
    "ssh_keygen_sha256",
    "security_policy",
    "upstream_security_baseline",
    "advisory_review_sha256",
    "package_provenance_sha256",
    "post_quantum_kex",
    "weak_crypto_warning",
    "agent_session_binding_restricted_key",
    "agent_lock_session_binding_fix",
    "pending_remote_forward_cleanup_fix",
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


def _is_link_or_reparse(path: Path, metadata: os.stat_result) -> bool:
    reparse_flag = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)
    attributes = getattr(metadata, "st_file_attributes", 0)
    return path.is_symlink() or bool(reparse_flag and attributes & reparse_flag)


class NativeOpenSshEvidenceError(ValueError):
    """A release-evidence artifact violated the exact contract."""


def bounded_bytes(path: Path, maximum: int = MAX_MANIFEST_BYTES) -> bytes:
    descriptor: int | None = None
    try:
        before = path.lstat()
        if not stat.S_ISREG(before.st_mode) or _is_link_or_reparse(path, before):
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


def release_file_sha256(
    path: Path, maximum: int = MAX_RELEASE_ARTIFACT_BYTES
) -> str:
    """Hash one exact regular release file without exposing its path."""
    descriptor: int | None = None
    try:
        before = path.lstat()
        if not stat.S_ISREG(before.st_mode) or _is_link_or_reparse(path, before):
            raise NativeOpenSshEvidenceError(
                "required release artifact is missing, linked, or not a regular file"
            )
        flags = os.O_RDONLY | getattr(os, "O_BINARY", 0) | getattr(os, "O_NOFOLLOW", 0)
        descriptor = os.open(path, flags)
        opened = os.fstat(descriptor)
        if (
            not stat.S_ISREG(opened.st_mode)
            or (opened.st_dev, opened.st_ino) != (before.st_dev, before.st_ino)
        ):
            raise NativeOpenSshEvidenceError(
                "release artifact identity changed while opening"
            )
        if opened.st_size <= 0 or opened.st_size > maximum:
            raise NativeOpenSshEvidenceError(
                f"release artifact size must be within 1..{maximum} bytes"
            )
        digest = hashlib.sha256()
        consumed = 0
        while consumed <= maximum:
            chunk = os.read(descriptor, min(1024 * 1024, maximum + 1 - consumed))
            if not chunk:
                break
            digest.update(chunk)
            consumed += len(chunk)
        after = os.fstat(descriptor)
        if (
            (after.st_dev, after.st_ino) != (opened.st_dev, opened.st_ino)
            or after.st_size != opened.st_size
            or consumed != opened.st_size
            or consumed > maximum
        ):
            raise NativeOpenSshEvidenceError(
                "release artifact changed or grew while reading"
            )
        return digest.hexdigest()
    except NativeOpenSshEvidenceError:
        raise
    except OSError as error:
        raise NativeOpenSshEvidenceError(
            "required release artifact is unavailable"
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
    return validate_document(
        load_manifest(path), allow_synthetic=allow_synthetic, root=root
    )


def validate_document(
    document: dict[str, Any],
    *,
    allow_synthetic: bool = False,
    root: Path = ROOT,
) -> dict[str, int | str]:
    """Validate one identity-stable manifest snapshot."""
    document = _exact_object(document, MANIFEST_KEYS, "native evidence")
    if document["schema"] != 2 or document["evidence_kind"] != "native-openssh-release":
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
    tool_hash_keys = (
        "client_sha256",
        "server_sha256",
        "ssh_add_sha256",
        "ssh_keygen_sha256",
    )
    provenance_hash_keys = (
        "advisory_review_sha256",
        "package_provenance_sha256",
    )
    for key in (*tool_hash_keys, *provenance_hash_keys):
        _hash(openssh[key], f"OpenSSH {key}")
    if not document["synthetic"] and any(
        openssh[key] == "0" * 64 for key in (*tool_hash_keys, *provenance_hash_keys)
    ):
        raise NativeOpenSshEvidenceError(
            "release OpenSSH hashes cannot use the synthetic sentinel"
        )
    if not document["synthetic"] and (
        "synthetic" in client_version.lower() or "synthetic" in server_version.lower()
    ):
        raise NativeOpenSshEvidenceError(
            "release OpenSSH versions cannot use synthetic labels"
        )
    if openssh["security_policy"] != "platform-supported-current-advisory-review":
        raise NativeOpenSshEvidenceError("OpenSSH security policy review is missing")
    if openssh["upstream_security_baseline"] != "OpenSSH-10.5-2026-08-11":
        raise NativeOpenSshEvidenceError("OpenSSH upstream security baseline changed")
    for key in (
        "post_quantum_kex",
        "weak_crypto_warning",
        "agent_session_binding_restricted_key",
        "agent_lock_session_binding_fix",
        "pending_remote_forward_cleanup_fix",
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
    if not document["synthetic"] and any(
        baseline[key] == "0" * 64
        for key in (
            "client_sha256_before",
            "client_sha256_after",
            "user_config_sha256_before",
            "user_config_sha256_after",
        )
    ):
        raise NativeOpenSshEvidenceError(
            "release manual baseline hashes cannot use the synthetic sentinel"
        )
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


def _architecture_name() -> str | None:
    machine = host_platform.machine().strip().lower()
    if machine in {"amd64", "x86_64"}:
        return "x86_64"
    if machine in {"arm64", "aarch64"}:
        return "aarch64"
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
        try:
            metadata = candidate.lstat()
        except OSError:
            continue
        if stat.S_ISREG(metadata.st_mode) and not _is_link_or_reparse(
            candidate, metadata
        ):
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
    if completed.returncode != 0 or len(output) > MAX_VERSION_BYTES:
        return None
    value = output.decode("utf-8", "replace").strip().splitlines()
    if not value:
        return None
    candidate = value[0]
    return candidate if VERSION.fullmatch(candidate) else None


def _probe_application_version(executable: Path) -> str | None:
    """Read the exact clap version without starting the GUI or a shell."""
    try:
        completed = subprocess.run(
            [str(executable), "--version"],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=5,
            check=False,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    output = completed.stdout[: MAX_VERSION_BYTES + 1]
    if completed.returncode != 0 or len(output) > MAX_VERSION_BYTES:
        return None
    lines = output.decode("utf-8", "replace").strip().splitlines()
    if len(lines) != 1:
        return None
    prefix = "automexia "
    candidate = lines[0]
    version = candidate[len(prefix) :] if candidate.startswith(prefix) else ""
    return version if VERSION.fullmatch(version) else None


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


def validate_controlled_environment(
    manifest: Path,
    *,
    application_binary: Path,
    application_package: Path,
    advisory_review: Path,
    package_provenance: Path,
    expected_commit: str,
    root: Path = ROOT,
) -> dict[str, int | str]:
    """Bind a private real manifest to the executing host and exact artifacts."""
    if COMMIT.fullmatch(expected_commit) is None:
        raise NativeOpenSshEvidenceError(
            "requested source commit must be an exact lowercase commit digest"
        )
    document = load_manifest(manifest)
    result = validate_document(document, root=root)
    if document["synthetic"]:
        raise NativeOpenSshEvidenceError(
            "synthetic evidence cannot satisfy controlled validation"
        )
    if document["source_commit"] != expected_commit:
        raise NativeOpenSshEvidenceError(
            "native evidence is not bound to the requested source commit"
        )

    platform_name = _platform_name()
    if platform_name is None or document["platform"] != platform_name:
        raise NativeOpenSshEvidenceError(
            "native platform does not match the release evidence"
        )
    architecture = _architecture_name()
    if architecture is None or document["architecture"] != architecture:
        raise NativeOpenSshEvidenceError(
            "native architecture does not match the release evidence"
        )

    executables = {
        name: _fixed_executable(candidates)
        for name, candidates in _fixed_candidates(platform_name).items()
    }
    if any(executable is None for executable in executables.values()):
        raise NativeOpenSshEvidenceError(
            "fixed native OpenSSH tools are unavailable"
        )
    client = executables["ssh"]
    server = executables["sshd"]
    if client is None or server is None:
        raise NativeOpenSshEvidenceError(
            "fixed native OpenSSH tools are unavailable"
        )
    client_version = _probe_version(client)
    server_version = _probe_version(server)
    openssh = document["openssh"]
    if client_version is None or client_version != openssh["client_version"]:
        raise NativeOpenSshEvidenceError(
            "native OpenSSH client version does not match the release evidence"
        )
    if server_version is None or server_version != openssh["server_version"]:
        raise NativeOpenSshEvidenceError(
            "native OpenSSH server version does not match the release evidence"
        )

    application = document["application"]
    if release_file_sha256(application_binary) != application["binary_sha256"]:
        raise NativeOpenSshEvidenceError(
            "application binary does not match the release evidence"
        )
    if release_file_sha256(application_package) != application["package_sha256"]:
        raise NativeOpenSshEvidenceError(
            "application package does not match the release evidence"
        )
    if _probe_application_version(application_binary) != application["version"]:
        raise NativeOpenSshEvidenceError(
            "application version does not match the release evidence"
        )
    tool_hashes = {
        "ssh": "client_sha256",
        "sshd": "server_sha256",
        "ssh-add": "ssh_add_sha256",
        "ssh-keygen": "ssh_keygen_sha256",
    }
    for tool, manifest_key in tool_hashes.items():
        executable = executables[tool]
        if executable is None or release_file_sha256(executable) != openssh[manifest_key]:
            raise NativeOpenSshEvidenceError(
                "native OpenSSH tool does not match the release evidence"
            )
    if release_file_sha256(advisory_review) != openssh["advisory_review_sha256"]:
        raise NativeOpenSshEvidenceError(
            "OpenSSH advisory review does not match the release evidence"
        )
    if release_file_sha256(package_provenance) != openssh["package_provenance_sha256"]:
        raise NativeOpenSshEvidenceError(
            "OpenSSH package provenance does not match the release evidence"
        )
    client_hash = openssh["client_sha256"]
    baseline = document["manual_baseline"]
    if (
        client_hash != baseline["client_sha256_before"]
        or client_hash != baseline["client_sha256_after"]
    ):
        raise NativeOpenSshEvidenceError(
            "native OpenSSH client does not match the manual baseline"
        )

    return {
        **result,
        "architecture": architecture,
        "artifacts": 8,
        "host_bound": 1,
    }


def validate_repository_contract(root: Path = ROOT) -> dict[str, int]:
    for historical, expected_digest in HISTORICAL_SYNTHETIC_FIXTURES:
        path = root / historical.relative_to(ROOT)
        if hashlib.sha256(bounded_bytes(path)).hexdigest() != expected_digest:
            raise NativeOpenSshEvidenceError(
                "historical native OpenSSH evidence fixture changed"
            )
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
            binary = os.environ.get("AUTOMEXIA_QA_NATIVE_OPENSSH_BINARY")
            package = os.environ.get("AUTOMEXIA_QA_NATIVE_OPENSSH_PACKAGE")
            advisory_review = os.environ.get(
                "AUTOMEXIA_QA_NATIVE_OPENSSH_ADVISORY_REVIEW"
            )
            package_provenance = os.environ.get(
                "AUTOMEXIA_QA_NATIVE_OPENSSH_PACKAGE_PROVENANCE"
            )
            commit = os.environ.get(
                "AUTOMEXIA_QA_NATIVE_OPENSSH_EXPECTED_COMMIT"
            )
            if not all(
                (
                    manifest,
                    binary,
                    package,
                    advisory_review,
                    package_provenance,
                    commit,
                )
            ):
                raise NativeOpenSshEvidenceError(
                    "controlled native evidence environment is incomplete"
                )
            result = validate_controlled_environment(
                Path(manifest),
                application_binary=Path(binary),
                application_package=Path(package),
                advisory_review=Path(advisory_review),
                package_provenance=Path(package_provenance),
                expected_commit=commit,
            )
        else:
            result = probe_prerequisites()
    except (NativeOpenSshEvidenceError, OSError, json.JSONDecodeError) as error:
        print(f"native OpenSSH evidence validation failed: {error}", file=sys.stderr)
        return 1
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    return 0 if result.get("status") != "external-prerequisite" else 2


if __name__ == "__main__":
    raise SystemExit(main())
