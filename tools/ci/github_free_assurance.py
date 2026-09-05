#!/usr/bin/env python3
"""Run the GitHub-Free assurance layers locally with pinned, isolated tools.

This runner deliberately does not grant credentials, send source anywhere, or
pretend that vendor-gated GitHub controls have been exercised.  It gives every
contributor the same reproducible checks that can run without those services.
"""

from __future__ import annotations

import argparse
import atexit
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import re
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import time
import urllib.error
import urllib.request
import zipfile
from dataclasses import dataclass
from typing import Any, Iterable


_DEV_CACHE_SPEC = importlib.util.spec_from_file_location(
    "automexia_dev_cache", Path(__file__).with_name("dev_cache.py")
)
if _DEV_CACHE_SPEC is None or _DEV_CACHE_SPEC.loader is None:
    raise RuntimeError("development cache owner is unavailable")
dev_cache = importlib.util.module_from_spec(_DEV_CACHE_SPEC)
sys.modules[_DEV_CACHE_SPEC.name] = dev_cache
_DEV_CACHE_SPEC.loader.exec_module(dev_cache)


ROOT = Path(__file__).resolve().parents[2]
POLICY_PATH = ROOT / "tests" / "assurance" / "github-free-assurance-policy-v1.json"
MAX_POLICY_BYTES = 32 * 1024
MAX_COMMAND_TIMEOUT_SECONDS = 90 * 60
MAX_TOOL_ARCHIVE_BYTES = 128 * 1024 * 1024
MAX_TOOL_BINARY_BYTES = 64 * 1024 * 1024
REQUIRED_POLICY_KEYS = {"schema", "owner", "tool_cache", "profiles", "tools", "external_gates"}
TOOL_CACHE_CONTRACT = "shared-content-addressed-v1"
TOOLSET_MANIFEST_SCHEMA = 1
TOOLSET_MANIFEST_NAME = "toolset-manifest.json"
MAX_TOOLSET_MANIFEST_BYTES = 64 * 1024
MAX_TOOLSET_FILES = 200_000
MAX_TOOLSET_HASH_BYTES = 2 * 1024 * 1024 * 1024
REQUIRED_TOOLS = {
    "actionlint": "1.7.12",
    "cargo-audit": "0.22.0",
    "cargo-deny": "0.20.2",
    "cargo-vet": "0.10.2",
    "gitleaks": "8.30.1",
    "semgrep": "1.175.0",
    "shellcheck": "0.11.0",
    "zizmor": "1.21.0",
}


class AssuranceError(ValueError):
    """The local assurance contract is missing or unsafe."""


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise AssuranceError(f"duplicate policy key {key!r}")
        result[key] = value
    return result


def read_bounded(path: Path, maximum: int = MAX_POLICY_BYTES) -> str:
    try:
        size = path.stat().st_size
    except OSError as error:
        raise AssuranceError(f"required file is unavailable: {path.relative_to(ROOT)}") from error
    if size <= 0 or size > maximum:
        raise AssuranceError(f"{path.relative_to(ROOT)} must be within 1..{maximum} bytes")
    return path.read_text(encoding="utf-8")


def load_policy(path: Path = POLICY_PATH) -> dict[str, Any]:
    try:
        policy = json.loads(read_bounded(path), object_pairs_hook=reject_duplicate_keys)
    except json.JSONDecodeError as error:
        raise AssuranceError(f"invalid GitHub-Free assurance policy: {error}") from error
    if not isinstance(policy, dict) or set(policy) != REQUIRED_POLICY_KEYS:
        raise AssuranceError(f"policy must define exactly {sorted(REQUIRED_POLICY_KEYS)}")
    if policy["schema"] != 2 or policy["owner"] != "tools/ci/github_free_assurance.py":
        raise AssuranceError("policy schema or authoritative owner drifted")
    cache = policy["tool_cache"]
    if cache != TOOL_CACHE_CONTRACT:
        raise AssuranceError("tool cache must use the shared content-addressed contract")
    tools = policy["tools"]
    if tools != REQUIRED_TOOLS:
        raise AssuranceError("pinned assurance tool inventory drifted")
    profiles = policy["profiles"]
    if not isinstance(profiles, dict) or set(profiles) != {"pre-push", "release-local", "deep-source"}:
        raise AssuranceError("policy must define exactly pre-push, release-local, and deep-source")
    for name, steps in profiles.items():
        if not isinstance(name, str) or not isinstance(steps, list) or not steps:
            raise AssuranceError("profiles must contain non-empty step lists")
        if any(not isinstance(step, str) or not step for step in steps) or len(set(steps)) != len(steps):
            raise AssuranceError(f"profile {name} has invalid or duplicate steps")
    if set(profiles["pre-push"]) != {
        "repository-ready", "free-plan-contract", "workflow-static-analysis",
        "dependency-security", "dependency-vetting", "secret-scan", "local-sast",
        "scanner-canaries",
    }:
        raise AssuranceError("pre-push profile does not cover the required local gates")
    if profiles["release-local"] != ["pre-push", "release-policy"]:
        raise AssuranceError("release-local must extend pre-push with release policy")
    if profiles["deep-source"] != ["pre-push", "miri", "sanitizers", "fuzz"]:
        raise AssuranceError("deep-source must extend pre-push with source assurance")
    external = policy["external_gates"]
    if not isinstance(external, list) or len(external) < 4 or any(not isinstance(item, str) or not item for item in external):
        raise AssuranceError("external GitHub-Free gates must remain explicit")
    return policy


def validate_policy_for_test(policy: dict[str, Any]) -> dict[str, Any]:
    """Exercise the production parser for an in-memory policy mutation."""
    temporary_parent = Path(ROOT.anchor) if os.name == "nt" else None
    with tempfile.TemporaryDirectory(
        prefix="automexia-policy-", dir=temporary_parent
    ) as temporary:
        path = Path(temporary) / "policy.json"
        path.write_text(json.dumps(policy), encoding="utf-8")
        return load_policy(path)


def cache_root(policy: dict[str, Any]) -> Path:
    try:
        return dev_cache.toolset_root(policy["tools"], root=ROOT)
    except dev_cache.CacheError as error:
        raise AssuranceError(str(error)) from error


def _sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    try:
        with path.open("rb") as source:
            while chunk := source.read(1024 * 1024):
                hasher.update(chunk)
    except OSError as error:
        raise AssuranceError("could not hash a shared toolset file") from error
    return hasher.hexdigest()


def _toolset_tree_digest(cache: Path) -> tuple[str, int, int]:
    if not cache.is_dir() or dev_cache._is_reparse_or_link(cache):
        raise AssuranceError("shared toolset root is unavailable or unsafe")
    hasher = hashlib.sha256()
    file_count = 0
    byte_count = 0
    directory_count = 0
    pending = [cache]
    while pending:
        directory = pending.pop()
        directory_count += 1
        if directory_count > dev_cache.MAX_DIRECTORIES:
            raise AssuranceError("shared toolset exceeds its directory ceiling")
        try:
            entries = sorted(
                os.scandir(directory), key=lambda entry: entry.name, reverse=True
            )
        except OSError as error:
            raise AssuranceError("could not inventory the shared toolset") from error
        for entry in entries:
            path = Path(entry.path)
            try:
                metadata = entry.stat(follow_symlinks=False)
            except OSError as error:
                raise AssuranceError("could not inspect the shared toolset") from error
            if entry.is_symlink() or (
                getattr(metadata, "st_file_attributes", 0)
                & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)
            ):
                raise AssuranceError(
                    "shared toolsets cannot contain links or reparse points"
                )
            if entry.is_dir(follow_symlinks=False):
                pending.append(path)
                continue
            if not entry.is_file(follow_symlinks=False):
                raise AssuranceError(
                    "shared toolsets can contain only files and directories"
                )
            relative = path.relative_to(cache)
            if relative.as_posix() == TOOLSET_MANIFEST_NAME:
                continue
            size = metadata.st_size
            file_count += 1
            byte_count += size
            if file_count > MAX_TOOLSET_FILES:
                raise AssuranceError("shared toolset exceeds its file ceiling")
            if byte_count > MAX_TOOLSET_HASH_BYTES:
                raise AssuranceError("shared toolset exceeds its byte ceiling")
            encoded_path = relative.as_posix().encode("utf-8")
            hasher.update(len(encoded_path).to_bytes(8, "big"))
            hasher.update(encoded_path)
            hasher.update(size.to_bytes(8, "big"))
            hasher.update(bytes.fromhex(_sha256_file(path)))
    return hasher.hexdigest(), file_count, byte_count


def _toolset_manifest(
    policy: dict[str, Any],
    cache: Path,
    *,
    created_unix: int,
) -> dict[str, object]:
    tree_sha256, file_count, byte_count = _toolset_tree_digest(cache)
    downloads = platform_downloads(policy)
    return {
        "schema": TOOLSET_MANIFEST_SCHEMA,
        "descriptor": dev_cache.toolset_descriptor(policy["tools"]),
        "created_unix": created_unix,
        "tree_sha256": tree_sha256,
        "file_count": file_count,
        "byte_count": byte_count,
        "provenance": {
            "archives": {
                name: {"url": download.url, "sha256": download.sha256}
                for name, download in sorted(downloads.items())
            },
            "cargo_tools": ["cargo-audit", "cargo-deny", "cargo-vet", "zizmor"],
            "python_tools": {"semgrep": REQUIRED_TOOLS["semgrep"]},
            "signature_status": "checksum-pinned archives; package-manager provenance retained",
        },
    }


def write_toolset_manifest(policy: dict[str, Any], cache: Path) -> None:
    manifest = _toolset_manifest(
        policy, cache, created_unix=int(time.time())
    )
    payload = (json.dumps(manifest, sort_keys=True, indent=2) + "\n").encode("utf-8")
    if len(payload) > MAX_TOOLSET_MANIFEST_BYTES:
        raise AssuranceError("shared toolset manifest exceeds its byte ceiling")
    temporary = cache / f".{TOOLSET_MANIFEST_NAME}.{os.getpid()}.tmp"
    try:
        temporary.write_bytes(payload)
        os.replace(temporary, cache / TOOLSET_MANIFEST_NAME)
    except OSError as error:
        temporary.unlink(missing_ok=True)
        raise AssuranceError("could not publish the shared toolset manifest") from error


def toolset_is_valid(policy: dict[str, Any], cache: Path | None = None) -> bool:
    cache = cache_root(policy) if cache is None else cache
    manifest_path = cache / TOOLSET_MANIFEST_NAME
    try:
        if not manifest_path.is_file():
            return False
        size = manifest_path.stat().st_size
        if size <= 0 or size > MAX_TOOLSET_MANIFEST_BYTES:
            return False
        manifest = json.loads(
            manifest_path.read_text(encoding="utf-8"),
            object_pairs_hook=reject_duplicate_keys,
        )
        if not isinstance(manifest, dict):
            return False
        created = manifest.get("created_unix")
        if not isinstance(created, int) or created <= 0:
            return False
        return manifest == _toolset_manifest(
            policy, cache, created_unix=created
        )
    except (AssuranceError, OSError, UnicodeError, json.JSONDecodeError):
        return False


def executable(name: str) -> str:
    return f"{name}.exe" if os.name == "nt" else name


def tool_path(
    policy: dict[str, Any], name: str, *, cache: Path | None = None
) -> Path:
    cache = cache_root(policy) if cache is None else cache
    if name == "semgrep":
        return cache / "semgrep-venv" / ("Scripts" if os.name == "nt" else "bin") / executable("semgrep")
    return cache / "bin" / executable(name)


def semgrep_python_path(
    policy: dict[str, Any], *, cache: Path | None = None
) -> Path:
    cache = cache_root(policy) if cache is None else cache
    return (
        cache
        / "semgrep-venv"
        / ("Scripts" if os.name == "nt" else "bin")
        / executable("python")
    )


def semgrep_launcher_path() -> Path:
    return ROOT / "tools" / "ci" / "semgrep_entrypoint.py"


def tool_command(
    policy: dict[str, Any],
    name: str,
    *arguments: str,
    cache: Path | None = None,
) -> list[str]:
    binary = tool_path(policy, name, cache=cache)
    if not binary.is_file():
        raise AssuranceError(
            f"{name} is unavailable in the isolated tool cache; run "
            "cargo xtask assurance install-tools"
        )
    if name == "cargo-vet":
        return ["cargo", "vet", *arguments]
    if name == "semgrep":
        python = semgrep_python_path(policy, cache=cache)
        launcher = semgrep_launcher_path()
        if not python.is_file() or not launcher.is_file():
            raise AssuranceError(
                "semgrep runtime is unavailable in the isolated tool cache; run "
                "cargo xtask assurance install-tools"
            )
        # pip console launchers embed their staging path. The venv interpreter
        # and source-owned entrypoint remain valid after atomic publication.
        return [str(python), str(launcher), "--legacy", *arguments]
    return [str(binary), *arguments]


_PROCESS_TEMPORARY: tuple[Path, Path] | None = None


def process_temporary_directory() -> Path:
    global _PROCESS_TEMPORARY
    root = dev_cache.temporary_root(root=ROOT)
    dev_cache.ensure_cache_directory(root)
    if (
        _PROCESS_TEMPORARY is None
        or _PROCESS_TEMPORARY[0] != root
        or not _PROCESS_TEMPORARY[1].is_dir()
    ):
        path = Path(tempfile.mkdtemp(prefix=f"run-{os.getpid()}-", dir=root))
        _PROCESS_TEMPORARY = (root, path)
        atexit.register(shutil.rmtree, path, ignore_errors=True)
    return _PROCESS_TEMPORARY[1]


def semgrep_temporary_directory() -> tempfile.TemporaryDirectory[str]:
    """Create a short isolated directory for Semgrep's local RPC transport."""
    root = dev_cache.temporary_root(root=ROOT)
    dev_cache.ensure_cache_directory(root)
    return tempfile.TemporaryDirectory(prefix="sg-", dir=root)


def process_environment(
    policy: dict[str, Any],
    *,
    isolate_cargo_home: bool = True,
    cache: Path | None = None,
) -> dict[str, str]:
    environment = os.environ.copy()
    cache = cache_root(policy) if cache is None else cache
    runtime = dev_cache.runtime_root(root=ROOT)
    downloads = dev_cache.downloads_root(root=ROOT)
    dev_cache.ensure_cache_directory(runtime)
    dev_cache.ensure_cache_directory(downloads)
    if isolate_cargo_home:
        environment["CARGO_HOME"] = str(runtime / "cargo-home")
    environment["PATH"] = str(cache / "bin") + os.pathsep + environment.get("PATH", "")
    temporary = process_temporary_directory()
    environment["TMP"] = str(temporary)
    environment["TEMP"] = str(temporary)
    environment["TMPDIR"] = str(temporary)
    environment["PIP_DISABLE_PIP_VERSION_CHECK"] = "1"
    environment["PIP_NO_INPUT"] = "1"
    environment["PIP_CACHE_DIR"] = str(downloads / "pip")
    environment["PYTHONDONTWRITEBYTECODE"] = "1"
    return environment


def run(
    command: list[str],
    label: str,
    policy: dict[str, Any],
    *,
    timeout: int = MAX_COMMAND_TIMEOUT_SECONDS,
    extra_environment: dict[str, str] | None = None,
    isolate_cargo_home: bool = True,
    cache: Path | None = None,
) -> None:
    environment = process_environment(
        policy, isolate_cargo_home=isolate_cargo_home, cache=cache
    )
    if extra_environment:
        environment.update(extra_environment)
    print(f"RUN: {label}")
    try:
        completed = subprocess.run(
            command,
            cwd=ROOT,
            env=environment,
            check=False,
            timeout=timeout,
        )
    except FileNotFoundError as error:
        raise AssuranceError(f"{label} could not start: {command[0]}") from error
    except subprocess.TimeoutExpired as error:
        raise AssuranceError(f"{label} exceeded its {timeout}-second bound") from error
    if completed.returncode != 0:
        raise AssuranceError(f"{label} failed with exit code {completed.returncode}")


@dataclass(frozen=True)
class Download:
    url: str
    sha256: str
    member: str


def platform_downloads(policy: dict[str, Any]) -> dict[str, Download]:
    del policy
    system = platform.system().lower()
    machine = platform.machine().lower()
    machine = {"amd64": "x64", "x86_64": "x64", "aarch64": "arm64", "arm64": "arm64"}.get(machine, machine)
    key = f"{system}-{machine}"
    actionlint = {
        "windows-x64": Download("https://github.com/rhysd/actionlint/releases/download/v1.7.12/actionlint_1.7.12_windows_amd64.zip", "6e7241b51e6817ea6a047693d8e6fed13b31819c9a0dd6c5a726e1592d22f6e9", "actionlint.exe"),
        "linux-x64": Download("https://github.com/rhysd/actionlint/releases/download/v1.7.12/actionlint_1.7.12_linux_amd64.tar.gz", "8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8", "actionlint"),
        "linux-arm64": Download("https://github.com/rhysd/actionlint/releases/download/v1.7.12/actionlint_1.7.12_linux_arm64.tar.gz", "325e971b6ba9bfa504672e29be93c24981eeb1c07576d730e9f7c8805afff0c6", "actionlint"),
        "darwin-x64": Download("https://github.com/rhysd/actionlint/releases/download/v1.7.12/actionlint_1.7.12_darwin_amd64.tar.gz", "5b44c3bc2255115c9b69e30efc0fecdf498fdb63c5d58e17084fd5f16324c644", "actionlint"),
        "darwin-arm64": Download("https://github.com/rhysd/actionlint/releases/download/v1.7.12/actionlint_1.7.12_darwin_arm64.tar.gz", "aba9ced2dee8d27fecca3dc7feb1a7f9a52caefa1eb46f3271ea66b6e0e6953f", "actionlint"),
    }
    shellcheck = {
        "windows-x64": Download("https://github.com/koalaman/shellcheck/releases/download/v0.11.0/shellcheck-v0.11.0.zip", "8a4e35ab0b331c85d73567b12f2a444df187f483e5079ceffa6bda1faa2e740e", "shellcheck.exe"),
        "linux-x64": Download("https://github.com/koalaman/shellcheck/releases/download/v0.11.0/shellcheck-v0.11.0.linux.x86_64.tar.gz", "b7af85e41cc99489dcc21d66c6d5f3685138f06d34651e6d34b42ec6d54fe6f6", "shellcheck-v0.11.0/shellcheck"),
        "linux-arm64": Download("https://github.com/koalaman/shellcheck/releases/download/v0.11.0/shellcheck-v0.11.0.linux.aarch64.tar.gz", "68a8133197a50beb8803f8d42f9908d1af1c5540d4bb05fdfca8c1fa47decefc", "shellcheck-v0.11.0/shellcheck"),
        "darwin-x64": Download("https://github.com/koalaman/shellcheck/releases/download/v0.11.0/shellcheck-v0.11.0.darwin.x86_64.tar.gz", "c2c15e08df0e8fbc374c335b230a7ee958c313fa5714817a59aa59f1aa594f51", "shellcheck-v0.11.0/shellcheck"),
        "darwin-arm64": Download("https://github.com/koalaman/shellcheck/releases/download/v0.11.0/shellcheck-v0.11.0.darwin.aarch64.tar.gz", "339b930feb1ea764467013cc1f72d09cd6b869ebf1013296ba9055ab2ffbd26f", "shellcheck-v0.11.0/shellcheck"),
    }
    gitleaks = {
        "windows-x64": Download("https://github.com/gitleaks/gitleaks/releases/download/v8.30.1/gitleaks_8.30.1_windows_x64.zip", "d29144deff3a68aa93ced33dddf84b7fdc26070add4aa0f4513094c8332afc4e", "gitleaks.exe"),
        "linux-x64": Download("https://github.com/gitleaks/gitleaks/releases/download/v8.30.1/gitleaks_8.30.1_linux_x64.tar.gz", "551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb", "gitleaks"),
        "linux-arm64": Download("https://github.com/gitleaks/gitleaks/releases/download/v8.30.1/gitleaks_8.30.1_linux_arm64.tar.gz", "e4a487ee7ccd7d3a7f7ec08657610aa3606637dab924210b3aee62570fb4b080", "gitleaks"),
        "darwin-x64": Download("https://github.com/gitleaks/gitleaks/releases/download/v8.30.1/gitleaks_8.30.1_darwin_x64.tar.gz", "dfe101a4db2255fc85120ac7f3d25e4342c3c20cf749f2c20a18081af1952709", "gitleaks"),
        "darwin-arm64": Download("https://github.com/gitleaks/gitleaks/releases/download/v8.30.1/gitleaks_8.30.1_darwin_arm64.tar.gz", "b40ab0ae55c505963e365f271a8d3846efbc170aa17f2607f13df610a9aeb6a5", "gitleaks"),
    }
    try:
        return {
            "actionlint": actionlint[key],
            "gitleaks": gitleaks[key],
            "shellcheck": shellcheck[key],
        }
    except KeyError as error:
        raise AssuranceError(f"no checksum-pinned assurance tool archive exists for {key}") from error


def download_binary(
    policy: dict[str, Any],
    name: str,
    download: Download,
    *,
    cache: Path | None = None,
) -> None:
    cache = cache_root(policy) if cache is None else cache
    destination = tool_path(policy, name, cache=cache)
    destination.parent.mkdir(parents=True, exist_ok=True)
    # Keep transient archives beside the caller-owned staging cache. This
    # preserves one cleanup owner and lets recovery/tests operate without an
    # implicit dependency on a discoverable Git checkout.
    temporary_parent = cache.parent / ".downloads"
    dev_cache.ensure_cache_directory(temporary_parent)
    with tempfile.TemporaryDirectory(
        prefix="download-", dir=temporary_parent
    ) as raw_temporary:
        temporary = Path(raw_temporary)
        archive = temporary / "tool-archive"
        try:
            with urllib.request.urlopen(download.url, timeout=60) as response, archive.open("wb") as output:
                content_length = response.headers.get("Content-Length")
                if content_length is not None and (not content_length.isdigit() or int(content_length) > MAX_TOOL_ARCHIVE_BYTES):
                    raise AssuranceError(f"{name} archive exceeds its byte ceiling")
                hasher = hashlib.sha256()
                size = 0
                while chunk := response.read(1024 * 1024):
                    size += len(chunk)
                    if size > MAX_TOOL_ARCHIVE_BYTES:
                        raise AssuranceError(f"{name} archive exceeds its byte ceiling")
                    hasher.update(chunk)
                    output.write(chunk)
        except (OSError, urllib.error.URLError) as error:
            raise AssuranceError(f"could not download pinned {name} archive") from error
        if size == 0:
            raise AssuranceError(f"{name} archive is empty")
        digest = hasher.hexdigest()
        if digest != download.sha256:
            raise AssuranceError(f"{name} archive checksum mismatch")
        extracted = temporary / executable(name)
        try:
            if download.url.endswith(".zip"):
                with zipfile.ZipFile(archive) as source:
                    member = source.getinfo(download.member)
                    if member.is_dir() or member.file_size <= 0 or member.file_size > MAX_TOOL_BINARY_BYTES:
                        raise AssuranceError(f"{name} archive member is invalid")
                    member_size = member.file_size
                    with source.open(member) as input_file, extracted.open("wb") as output:
                        shutil.copyfileobj(input_file, output, length=1024 * 1024)
            else:
                with tarfile.open(archive, mode="r:gz") as source:
                    member = source.getmember(download.member)
                    if not member.isfile() or member.size <= 0 or member.size > MAX_TOOL_BINARY_BYTES:
                        raise AssuranceError(f"{name} archive member is invalid")
                    member_size = member.size
                    input_file = source.extractfile(member)
                    if input_file is None:
                        raise AssuranceError(f"{name} archive member cannot be read")
                    with input_file, extracted.open("wb") as output:
                        shutil.copyfileobj(input_file, output, length=1024 * 1024)
        except (KeyError, OSError, tarfile.TarError, zipfile.BadZipFile) as error:
            raise AssuranceError(f"{name} archive cannot be safely extracted") from error
        if extracted.stat().st_size != member_size:
            raise AssuranceError(f"{name} extracted byte count changed")
        os.replace(extracted, destination)
    destination.chmod(destination.stat().st_mode | 0o700)
    try:
        temporary_parent.rmdir()
    except OSError:
        # A concurrent producer or a prior failed download owns any remaining
        # entry; bounded cache GC handles it after the grace period.
        pass


def publish_toolset(
    policy: dict[str, Any], staging: Path, final: Path
) -> None:
    dev_cache.ensure_cache_directory(final.parent)
    try:
        os.replace(staging, final)
    except OSError as error:
        if not toolset_is_valid(policy, final):
            raise AssuranceError("shared toolset publication failed") from error


def quarantine_invalid_toolset(policy: dict[str, Any], final: Path) -> Path | None:
    if not final.exists():
        return None
    expected_parent = dev_cache.toolsets_root(root=ROOT)
    if final.parent != expected_parent or final.name != dev_cache.toolset_id(policy["tools"]):
        raise AssuranceError("refusing to quarantine an unexpected toolset path")
    if not final.is_dir() or dev_cache._is_reparse_or_link(final):
        raise AssuranceError("invalid assurance toolset is not a safe cache directory")
    staging_parent = dev_cache.staging_root(root=ROOT)
    dev_cache.ensure_cache_directory(staging_parent)
    quarantine = staging_parent / (
        f"invalid-toolset-{final.name[:12]}-{os.getpid()}-{time.time_ns()}"
    )
    try:
        os.replace(final, quarantine)
    except OSError as error:
        raise AssuranceError("could not quarantine the invalid assurance toolset") from error
    print("WARNING: quarantined an invalid generated assurance toolset for bounded cleanup")
    return quarantine


def install_tools(policy: dict[str, Any]) -> None:
    final = cache_root(policy)
    if toolset_is_valid(policy, final):
        print("PASS: shared pinned assurance toolset is already complete")
        return
    staging_parent = dev_cache.staging_root(root=ROOT)
    dev_cache.ensure_cache_directory(staging_parent)
    prefix = f"toolset-{dev_cache.toolset_id(policy['tools'])[:12]}-"
    with dev_cache.cache_lease("assurance-tools", root=ROOT):
        if toolset_is_valid(policy, final):
            print("PASS: shared pinned assurance toolset is already complete")
            return
        quarantine_invalid_toolset(policy, final)
        with tempfile.TemporaryDirectory(
            prefix=prefix, dir=staging_parent
        ) as raw_staging, tempfile.TemporaryDirectory(
            prefix="compile-", dir=staging_parent
        ) as raw_build:
            staging = Path(raw_staging)
            build_root = Path(raw_build)
            for name, download in platform_downloads(policy).items():
                download_binary(
                    policy, name, download, cache=staging
                )
            for package in ("cargo-audit", "cargo-deny", "cargo-vet", "zizmor"):
                run(
                    [
                        "cargo",
                        "install",
                        "--root",
                        str(staging),
                        "--target-dir",
                        str(build_root),
                        "--locked",
                        "--version",
                        REQUIRED_TOOLS[package],
                        package,
                    ],
                    f"install {package}",
                    policy,
                    timeout=45 * 60,
                    cache=staging,
                )
            venv = staging / "semgrep-venv"
            semgrep_python = (
                venv
                / ("Scripts" if os.name == "nt" else "bin")
                / executable("python")
            )
            run(
                [sys.executable, "-m", "venv", str(venv)],
                "create isolated Semgrep environment",
                policy,
                timeout=180,
                cache=staging,
            )
            run(
                [
                    str(semgrep_python),
                    "-m",
                    "pip",
                    "install",
                    f"semgrep=={REQUIRED_TOOLS['semgrep']}",
                ],
                "install Semgrep",
                policy,
                timeout=45 * 60,
                cache=staging,
            )
            for name in REQUIRED_TOOLS:
                run(
                    tool_command(policy, name, "--version", cache=staging),
                    f"verify {name}",
                    policy,
                    timeout=120,
                    cache=staging,
                )
            write_toolset_manifest(policy, staging)
            if not toolset_is_valid(policy, staging):
                raise AssuranceError(
                    "constructed shared assurance toolset failed validation"
                )
            publish_toolset(policy, staging, final)
    print("PASS: shared content-addressed assurance toolset is complete")


def initialize_vet(policy: dict[str, Any]) -> None:
    records = ROOT / "supply-chain"
    required_records = ("audits.toml", "config.toml", "imports.lock")
    present = [records / name for name in required_records if (records / name).is_file()]
    if present and len(present) != len(required_records):
        raise AssuranceError("Cargo Vet baseline is incomplete; restore or review supply-chain before retrying")
    if not present:
        run(tool_command(policy, "cargo-vet", "init"), "initialize Cargo Vet trust policy", policy, timeout=15 * 60)
    else:
        print("INFO: existing source-controlled Cargo Vet baseline will be verified without rewriting it")
    run(tool_command(policy, "cargo-vet", "--locked"), "verify Cargo Vet trust policy", policy, timeout=15 * 60)
    print("PASS: Cargo Vet trust baseline is initialized and verified; review supply-chain changes before committing")


def write_hook(path: Path) -> None:
    if path.exists():
        raise AssuranceError(f"refusing to overwrite existing Git hook: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    source = (
        "#!/bin/sh\n"
        "# Automexia local pre-push assurance is temporarily opt-in.\n"
        "# Run this explicitly when desired:\n"
        "# cargo xtask assurance pre-push\n"
        "exit 0\n"
    )
    temporary = path.with_name(f".{path.name}.tmp")
    temporary.write_text(source, encoding="utf-8", newline="\n")
    temporary.chmod(0o700)
    os.replace(temporary, path)


def install_hook() -> None:
    completed = subprocess.run(["git", "rev-parse", "--git-path", "hooks/pre-push"], cwd=ROOT, check=False, capture_output=True, text=True, timeout=30)
    if completed.returncode != 0:
        raise AssuranceError("could not resolve the repository pre-push hook path")
    hook = Path(completed.stdout.strip())
    if not hook.is_absolute():
        hook = ROOT / hook
    write_hook(hook)
    print("PASS: installed non-blocking Automexia pre-push placeholder")


def expand_profile(policy: dict[str, Any], profile: str, seen: tuple[str, ...] = ()) -> list[str]:
    profiles = policy["profiles"]
    if profile not in profiles:
        raise AssuranceError(f"unknown assurance profile {profile!r}")
    if profile in seen:
        raise AssuranceError("assurance profiles contain a cycle")
    expanded: list[str] = []
    for step in profiles[profile]:
        if step in profiles:
            expanded.extend(expand_profile(policy, step, (*seen, profile)))
        else:
            expanded.append(step)
    if len(set(expanded)) != len(expanded):
        raise AssuranceError(f"assurance profile {profile} expands duplicate work")
    return expanded


def required_tools(steps: Iterable[str]) -> set[str]:
    tools: set[str] = set()
    for step in steps:
        if step == "workflow-static-analysis":
            tools.update(("actionlint", "shellcheck", "zizmor"))
        elif step == "dependency-security":
            tools.update(("cargo-audit", "cargo-deny"))
        elif step == "dependency-vetting":
            tools.add("cargo-vet")
        elif step == "secret-scan":
            tools.add("gitleaks")
        elif step == "local-sast":
            tools.add("semgrep")
        elif step == "scanner-canaries":
            tools.update(("gitleaks", "semgrep"))
    return tools


def git_stdout(*arguments: str) -> str | None:
    completed = subprocess.run(
        ["git", *arguments],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
        timeout=30,
    )
    if completed.returncode != 0:
        return None
    value = completed.stdout.strip()
    return value or None


def changed_commit_log_options() -> str:
    upstream = git_stdout("rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{upstream}")
    if upstream is None:
        # A lone revision makes `git log` traverse all reachable history. New
        # branches therefore anchor to the fetched origin default instead of
        # turning a pre-push delta scan into an accidental repository audit.
        upstream = git_stdout(
            "symbolic-ref", "--quiet", "--short", "refs/remotes/origin/HEAD"
        )
        if upstream is None:
            raise AssuranceError(
                "the branch has no upstream and origin/HEAD is unavailable; "
                "fetch the origin default branch before secret scanning"
            )
    merge_base = git_stdout("merge-base", "HEAD", upstream)
    head = git_stdout("rev-parse", "HEAD")
    full_sha = re.compile(r"[0-9a-f]{40}")
    if (
        merge_base is None
        or head is None
        or full_sha.fullmatch(merge_base) is None
        or full_sha.fullmatch(head) is None
    ):
        raise AssuranceError("could not resolve the upstream commit range for secret scanning")
    return f"{merge_base}..{head}"


def run_step(policy: dict[str, Any], step: str) -> None:
    if step == "repository-ready":
        if os.environ.get("AUTOMEXIA_ASSURANCE_READY_DONE") == "1":
            print("INFO: repository readiness was completed by the owning xtask process")
        else:
            # The isolated Cargo home belongs to pinned assurance tools. Reusing
            # it for the product build changes native dependency source roots
            # and can race or invalidate the contributor target unexpectedly.
            run(
                ["cargo", "xtask", "ready"],
                "repository readiness",
                policy,
                isolate_cargo_home=False,
            )
    elif step == "free-plan-contract":
        run([sys.executable, ".github/scripts/check_action_pins.py"], "immutable GitHub Action pins", policy, timeout=180)
        run([sys.executable, ".github/scripts/check_free_plan_contract.py"], "GitHub-Free workflow policy", policy, timeout=180)
        run([sys.executable, "tools/ci/test_free_plan_contract.py"], "GitHub-Free workflow mutation tests", policy, timeout=180)
    elif step == "workflow-static-analysis":
        run(
            tool_command(
                policy,
                "actionlint",
                "-color",
                "-shellcheck",
                str(tool_path(policy, "shellcheck")),
            ),
            "GitHub Actions syntax and shell semantics",
            policy,
            timeout=180,
        )
        run(
            tool_command(
                policy,
                "zizmor",
                "--collect",
                "workflows",
                "--strict-collection",
                "--persona",
                "auditor",
                "--min-severity",
                "high",
                "--min-confidence",
                "medium",
                "--no-online-audits",
                "--format",
                "github",
                ".",
            ),
            "GitHub Actions security",
            policy,
            timeout=180,
        )
    elif step == "dependency-security":
        run(tool_command(policy, "cargo-audit", "audit", "--deny", "warnings"), "RustSec dependency audit", policy, timeout=15 * 60)
        run(tool_command(policy, "cargo-deny", "--locked", "--color", "never", "check", "--hide-inclusion-graph"), "dependency/license/source policy", policy, timeout=15 * 60)
    elif step == "dependency-vetting":
        run(tool_command(policy, "cargo-vet", "--locked"), "dependency audit trust policy", policy, timeout=15 * 60)
    elif step == "secret-scan":
        log_options = changed_commit_log_options()
        run(tool_command(policy, "gitleaks", "git", "--redact", "--no-banner", "--timeout=900", "--max-target-megabytes=16", "--config", ".gitleaks.toml", f"--log-opts={log_options}"), "introduced-commit secret scan", policy, timeout=15 * 60)
        run(tool_command(policy, "gitleaks", "dir", "--redact", "--no-banner", "--timeout=900", "--max-target-megabytes=16", "--config", ".gitleaks.toml", "."), "working-tree secret scan", policy, timeout=15 * 60)
    elif step == "local-sast":
        with semgrep_temporary_directory() as temporary:
            temporary_environment = {name: temporary for name in ("TMP", "TEMP", "TMPDIR")}
            temporary_environment["PATH"] = str(tool_path(policy, "semgrep").parent) + os.pathsep + process_environment(policy)["PATH"]
            run(tool_command(policy, "semgrep", "scan", "--config", "tools/ci/semgrep-rules.yml", "--error", "--metrics=off", "--include", "*.rs", "--exclude", ".automexia-tools", "--exclude", ".automexia-private", "--exclude", "target", "--exclude", "tests/fixtures/free-assurance/semgrep", "."), "local Rust SAST", policy, timeout=15 * 60, extra_environment=temporary_environment)
    elif step == "scanner-canaries":
        run([sys.executable, "tools/ci/test_free_security_tools.py", "--tool", "all"], "real scanner canaries", policy, timeout=10 * 60)
    elif step == "release-policy":
        run([sys.executable, "tools/ci/release_trust.py", "--check-policy"], "release artifact trust policy", policy, timeout=180)
        run([sys.executable, "tools/ci/test_release_trust.py"], "release artifact trust mutation tests", policy, timeout=300)
        run([sys.executable, "tools/ci/stable_release.py", "check-policy"], "stable release provenance policy", policy, timeout=180)
        run([sys.executable, "tools/ci/test_stable_release.py"], "stable release provenance mutation tests", policy, timeout=300)
    elif step == "miri":
        require_linux_deep_profile()
        for test in ("simd_utf8::tests", "simd_base64::tests", "performer::parser::tests"):
            run(["cargo", "+nightly-2026-08-25", "miri", "test", "-p", "rio-vt", "--lib", "--no-default-features", "--locked", test], f"Miri {test}", policy, timeout=15 * 60, extra_environment={"MIRIFLAGS": "-Zmiri-disable-isolation"})
    elif step == "sanitizers":
        require_linux_deep_profile()
        for sanitizer in ("address", "thread"):
            environment = {"RUSTFLAGS": f"-Zsanitizer={sanitizer}", "RUSTDOCFLAGS": f"-Zsanitizer={sanitizer}"}
            run(["cargo", "+nightly-2026-08-25", "test", "-p", "rio-vt", "--lib", "--no-default-features", "-Zbuild-std", "--target", "x86_64-unknown-linux-gnu", "--locked"], f"{sanitizer} sanitizer rio-vt", policy, timeout=30 * 60, extra_environment=environment)
    elif step == "fuzz":
        require_linux_deep_profile()
        for target in ("vt_parser", "osc_metadata", "control_string_bounds", "image_decoder", "openssh_inventory", "config_migration", "semantic_classification", "label_sanitization", "quick_action_projection", "quick_action_packs", "quick_action_imports", "connection_planning", "ecosystem_bundle", "ghostty_keybindings", "ghostty_migration"):
            run(["cargo", "+nightly-2026-08-25", "fuzz", "run", target, "--target", "x86_64-unknown-linux-gnu", "--", "-max_total_time=120", "-rss_limit_mb=768", "-timeout=15"], f"bounded fuzz {target}", policy, timeout=10 * 60)
    else:
        raise AssuranceError(f"profile references unknown step {step!r}")


def require_linux_deep_profile() -> None:
    if platform.system() != "Linux":
        raise AssuranceError("deep-source runs on a native Linux checkout; use a supported Linux or WSL environment")


def run_profile(policy: dict[str, Any], profile: str) -> None:
    if not toolset_is_valid(policy):
        raise AssuranceError(
            "shared pinned toolset is unavailable; run "
            "cargo xtask assurance install-tools"
        )
    steps = expand_profile(policy, profile)
    for name in sorted(required_tools(steps)):
        run(tool_command(policy, name, "--version"), f"verify required {name}", policy, timeout=120)
    for step in steps:
        run_step(policy, step)
    print(f"PASS: GitHub-Free local assurance profile {profile} completed")


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("check-policy", "install-tools", "initialize-vet", "install-hook", "audit-history-secrets", "pre-push", "release-local", "deep-source"))
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    try:
        policy = load_policy()
        if args.command == "check-policy":
            cache_root(policy)
            print("PASS: GitHub-Free local assurance policy is structurally valid")
        elif args.command == "install-tools":
            install_tools(policy)
        elif args.command == "install-hook":
            install_hook()
        else:
            with dev_cache.cache_lease("assurance", root=ROOT):
                if args.command == "initialize-vet":
                    initialize_vet(policy)
                elif args.command == "audit-history-secrets":
                    run(tool_command(policy, "gitleaks", "git", "--redact", "--no-banner", "--timeout=900", "--max-target-megabytes=16", "--config", ".gitleaks.toml", "--log-opts=--all"), "full-history secret audit", policy, timeout=15 * 60)
                else:
                    run_profile(policy, args.command)
    except (AssuranceError, dev_cache.CacheError) as error:
        print(f"GitHub-Free assurance failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
