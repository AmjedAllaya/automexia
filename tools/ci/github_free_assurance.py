#!/usr/bin/env python3
"""Run the GitHub-Free assurance layers locally with pinned, isolated tools.

This runner deliberately does not grant credentials, send source anywhere, or
pretend that vendor-gated GitHub controls have been exercised.  It gives every
contributor the same reproducible checks that can run without those services.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import urllib.error
import urllib.request
import zipfile
from dataclasses import dataclass
from typing import Any, Iterable


ROOT = Path(__file__).resolve().parents[2]
POLICY_PATH = ROOT / "tests" / "assurance" / "github-free-assurance-policy-v1.json"
MAX_POLICY_BYTES = 32 * 1024
MAX_COMMAND_TIMEOUT_SECONDS = 90 * 60
MAX_TOOL_ARCHIVE_BYTES = 128 * 1024 * 1024
MAX_TOOL_BINARY_BYTES = 64 * 1024 * 1024
REQUIRED_POLICY_KEYS = {"schema", "owner", "tool_cache", "profiles", "tools", "external_gates"}
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
    if policy["schema"] != 1 or policy["owner"] != "tools/ci/github_free_assurance.py":
        raise AssuranceError("policy schema or authoritative owner drifted")
    cache = policy["tool_cache"]
    if not isinstance(cache, str) or cache != ".automexia-tools" or Path(cache).is_absolute():
        raise AssuranceError("tool cache must be the relative .automexia-tools directory")
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
    root = (ROOT / policy["tool_cache"]).resolve()
    if root.parent != ROOT.resolve():
        raise AssuranceError("tool cache escapes the repository root")
    return root


def executable(name: str) -> str:
    return f"{name}.exe" if os.name == "nt" else name


def tool_path(policy: dict[str, Any], name: str) -> Path:
    cache = cache_root(policy)
    if name == "semgrep":
        return cache / "semgrep-venv" / ("Scripts" if os.name == "nt" else "bin") / executable("semgrep")
    return cache / "bin" / executable(name)


def tool_command(policy: dict[str, Any], name: str, *arguments: str) -> list[str]:
    binary = tool_path(policy, name)
    if not binary.is_file():
        raise AssuranceError(
            f"{name} is unavailable in the isolated tool cache; run "
            "cargo xtask assurance install-tools"
        )
    if name == "cargo-vet":
        return ["cargo", "vet", *arguments]
    return [str(binary), *arguments]


def process_environment(
    policy: dict[str, Any], *, isolate_cargo_home: bool = True
) -> dict[str, str]:
    environment = os.environ.copy()
    cache = cache_root(policy)
    if isolate_cargo_home:
        environment["CARGO_HOME"] = str(cache / "cargo-home")
    environment["PATH"] = str(cache / "bin") + os.pathsep + environment.get("PATH", "")
    temporary = cache / "tmp"
    temporary.mkdir(parents=True, exist_ok=True)
    environment["TMP"] = str(temporary)
    environment["TEMP"] = str(temporary)
    environment["TMPDIR"] = str(temporary)
    environment["PIP_DISABLE_PIP_VERSION_CHECK"] = "1"
    environment["PIP_NO_INPUT"] = "1"
    environment["PIP_CACHE_DIR"] = str(cache / "pip-cache")
    return environment


def run(command: list[str], label: str, policy: dict[str, Any], *, timeout: int = MAX_COMMAND_TIMEOUT_SECONDS, extra_environment: dict[str, str] | None = None, isolate_cargo_home: bool = True) -> None:
    environment = process_environment(policy, isolate_cargo_home=isolate_cargo_home)
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


def download_binary(policy: dict[str, Any], name: str, download: Download) -> None:
    destination = tool_path(policy, name)
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="automexia-assurance-", dir=cache_root(policy)) as raw_temporary:
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


def install_cargo_tool(policy: dict[str, Any], package: str) -> None:
    if package not in {"cargo-audit", "cargo-deny", "cargo-vet", "zizmor"}:
        raise AssuranceError(f"unsupported Cargo assurance tool {package!r}")
    destination = tool_path(policy, package)
    destination.parent.mkdir(parents=True, exist_ok=True)
    build_parent = Path(ROOT.anchor) if os.name == "nt" else cache_root(policy)
    with tempfile.TemporaryDirectory(
        prefix="automexia-tool-build-", dir=build_parent
    ) as raw_build_root:
        build_root = Path(raw_build_root)
        run(
            [
                "cargo",
                "install",
                "--root",
                str(build_root),
                "--target-dir",
                str(build_root / "target"),
                "--locked",
                "--version",
                REQUIRED_TOOLS[package],
                package,
            ],
            f"install {package}",
            policy,
            timeout=45 * 60,
            extra_environment={"CARGO_HOME": str(build_root / "cargo-home")},
        )
        built = build_root / "bin" / executable(package)
        try:
            built_size = built.stat().st_size
        except OSError as error:
            raise AssuranceError(f"installed {package} binary is unavailable") from error
        if built_size <= 0 or built_size > MAX_TOOL_BINARY_BYTES:
            raise AssuranceError(f"installed {package} binary has an invalid size")
        os.replace(built, destination)
    destination.chmod(destination.stat().st_mode | 0o700)


def install_tools(policy: dict[str, Any]) -> None:
    cache = cache_root(policy)
    cache.mkdir(parents=True, exist_ok=True)
    for name, download in platform_downloads(policy).items():
        if not tool_path(policy, name).is_file():
            download_binary(policy, name, download)
    for package in ("cargo-audit", "cargo-deny", "cargo-vet", "zizmor"):
        name = package
        if not tool_path(policy, name).is_file():
            install_cargo_tool(policy, package)
    semgrep_binary = tool_path(policy, "semgrep")
    venv = cache / "semgrep-venv"
    semgrep_python = venv / ("Scripts" if os.name == "nt" else "bin") / executable("python")
    try:
        semgrep_ready = semgrep_binary.is_file() and subprocess.run(
            [str(semgrep_binary), "--version"],
            cwd=ROOT,
            env=process_environment(policy),
            check=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=120,
        ).returncode == 0
    except (OSError, subprocess.TimeoutExpired):
        semgrep_ready = False
    if not semgrep_ready:
        if not semgrep_python.is_file():
            run([sys.executable, "-m", "venv", str(venv)], "create isolated Semgrep environment", policy, timeout=180)
        run([str(semgrep_python), "-m", "pip", "install", f"semgrep=={REQUIRED_TOOLS['semgrep']}"], "install Semgrep", policy, timeout=45 * 60)
    for name in REQUIRED_TOOLS:
        run(tool_command(policy, name, "--version"), f"verify {name}", policy, timeout=120)
    print("PASS: checksum-pinned local assurance tools are installed in .automexia-tools")


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
    source = "#!/bin/sh\n# Managed by Automexia; run the GitHub-Free local pre-push gate.\ncargo xtask assurance pre-push\n"
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
    print("PASS: installed non-overwriting local pre-push hook")


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
        semgrep_temp_parent = Path(ROOT.anchor) if os.name == "nt" else None
        with tempfile.TemporaryDirectory(prefix="automexia-semgrep-", dir=semgrep_temp_parent) as temporary:
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
        elif args.command == "initialize-vet":
            initialize_vet(policy)
        elif args.command == "install-hook":
            install_hook()
        elif args.command == "audit-history-secrets":
            run(tool_command(policy, "gitleaks", "git", "--redact", "--no-banner", "--timeout=900", "--max-target-megabytes=16", "--config", ".gitleaks.toml", "--log-opts=--all"), "full-history secret audit", policy, timeout=15 * 60)
        else:
            run_profile(policy, args.command)
    except AssuranceError as error:
        print(f"GitHub-Free assurance failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
