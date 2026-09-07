#!/usr/bin/env python3
"""Verify the compiler actually selected by rustup and the workflow contract."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import threading
import tomllib

from qa import popen_group_options, terminate_process_tree


ROOT = Path(__file__).resolve().parents[2]
NIGHTLY = "nightly-2026-08-25"
MAX_FILE_BYTES = 256 * 1024
MAX_PROBE_BYTES = 4096
PROBE_TIMEOUT = 20
VERSION = re.compile(r"[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}")
RUST_WORKFLOWS = ("ci.yml", "release.yml", "nightly.yml", "linux-early-access.yml")
COMPILER_EXECUTABLE_ENV = ("RUSTC", "RUSTDOC", "CARGO_BUILD_RUSTC", "CARGO_BUILD_RUSTDOC")
CACHE_DOCS = ("docs/CI-ASSURANCE.md", "docs/CI-BUILD-PERFORMANCE-PLAN.md", ".github/FREE-PRIVATE-PRODUCTION-SETUP.md")
COMPILER_DOCS = (".github/TOOLCHAIN-POLICY.md", ".github/PATCH-REPORT.md")
PYTHON_SETUP = "actions/setup-python@ece7cb06caefa5fff74198d8649806c4678c61a1"


class ToolchainError(ValueError):
    """A required compiler identity or workflow invariant is unproven."""


def read_text(path: Path) -> str:
    try:
        if path.is_symlink() or not path.is_file():
            raise ToolchainError("compiler policy input must be a regular file")
        with path.open("rb") as stream:
            raw = stream.read(MAX_FILE_BYTES + 1)
        if not raw or len(raw) > MAX_FILE_BYTES:
            raise ToolchainError("compiler policy input exceeds its byte limit")
        return raw.decode("utf-8")
    except (OSError, UnicodeError) as error:
        raise ToolchainError("compiler policy input is unavailable") from error


def versions(root: Path = ROOT) -> tuple[str, str]:
    try:
        toolchain = tomllib.loads(read_text(root / "rust-toolchain.toml"))["toolchain"]
        manifest = tomllib.loads(read_text(root / "Cargo.toml"))
        pin = toolchain["channel"]
        minimum = manifest["workspace"]["package"]["rust-version"]
        if "path" in toolchain or (root / "rust-toolchain").exists():
            raise ToolchainError("a local or legacy toolchain must not shadow the pin")
        if not all(isinstance(v, str) and VERSION.fullmatch(v) for v in (pin, minimum)):
            raise ToolchainError("compiler pin and MSRV must be exact release versions")
        if tuple(map(int, minimum.split("."))) > tuple(map(int, pin.split("."))):
            raise ToolchainError("development compiler is below the declared MSRV")
        return pin, minimum
    except (KeyError, TypeError, tomllib.TOMLDecodeError) as error:
        raise ToolchainError("compiler pin or MSRV policy is malformed") from error


def probe(command: list[str], *, timeout: float = PROBE_TIMEOUT,
          maximum: int = MAX_PROBE_BYTES) -> str:
    """Read bounded tool identity, never copy arbitrary tool output into errors."""
    data = bytearray()
    overflow = threading.Event()
    try:
        process = subprocess.Popen(command, cwd=ROOT, stdout=subprocess.PIPE,
                                   stderr=subprocess.STDOUT, **popen_group_options())
    except OSError as error:
        raise ToolchainError("compiler identity tool is unavailable") from error

    def collect() -> None:
        try:
            with process.stdout:
                while chunk := process.stdout.read(1024):
                    if len(data) + len(chunk) > maximum:
                        overflow.set()
                        terminate_process_tree(process)
                        return
                    data.extend(chunk)
        except (OSError, ValueError):
            overflow.set()

    reader = threading.Thread(target=collect, name="automexia-compiler-probe")
    reader.start()
    failed = False
    try:
        failed = process.wait(timeout=timeout) != 0
    except subprocess.TimeoutExpired:
        failed = True
        terminate_process_tree(process)
    finally:
        reader.join(timeout=timeout)
        if reader.is_alive():
            terminate_process_tree(process)
            reader.join(timeout=PROBE_TIMEOUT)
            failed = True
    if failed or overflow.is_set() or reader.is_alive():
        raise ToolchainError("compiler identity probe failed or exceeded its bounds")
    try:
        return bytes(data).decode("utf-8", errors="strict")
    except UnicodeError as error:
        raise ToolchainError("compiler identity is not UTF-8") from error


def verify(kind: str = "development", root: Path = ROOT) -> dict[str, str]:
    pin, minimum = versions(root)
    expected = {"development": pin, "msrv": minimum, "nightly": NIGHTLY}[kind]
    if os.environ.get("RUSTUP_TOOLCHAIN") != expected:
        raise ToolchainError("RUSTUP_TOOLCHAIN must explicitly select the expected compiler")
    if any(os.environ.get(key) for key in COMPILER_EXECUTABLE_ENV):
        raise ToolchainError("custom compiler executables are not release identity evidence")
    active = probe(["rustup", "show", "active-toolchain"]).split()
    if not active or not active[0].startswith(expected + "-"):
        raise ToolchainError("effective rustup selection differs from the required compiler")
    receipt = {"kind": kind, "toolchain": expected}
    for tool in ("rustc", "cargo"):
        output = probe([tool, "--version", "--verbose"])
        matches = re.findall(r"(?m)^release: ([0-9]+\.[0-9]+\.[0-9]+(?:-nightly)?)$", output)
        hashes = re.findall(r"(?m)^commit-hash: ([0-9a-f]{40})$", output)
        if len(matches) != 1 or len(hashes) != 1:
            raise ToolchainError("compiler identity has missing or duplicate version fields")
        if (kind == "nightly" and not matches[0].endswith("-nightly")) or (
            kind != "nightly" and matches[0] != expected
        ):
            raise ToolchainError("actual compiler or Cargo version differs from policy")
        receipt[tool] = matches[0]
        receipt[tool + "_commit"] = hashes[0]
    return receipt


def load_workflow(path: Path) -> dict:
    import yaml

    class UniqueLoader(yaml.SafeLoader):
        def compose_node(self, parent, index):
            if self.check_event(yaml.AliasEvent):
                raise ToolchainError("compiler workflow aliases are not supported")
            self.node_count = getattr(self, "node_count", 0) + 1
            self.node_depth = getattr(self, "node_depth", 0) + 1
            if self.node_count > 16384 or self.node_depth > 48:
                raise ToolchainError("compiler workflow exceeds structural bounds")
            try:
                return super().compose_node(parent, index)
            finally:
                self.node_depth -= 1

    def mapping(loader, node):
        result = {}
        for key_node, value_node in node.value:
            key = loader.construct_object(key_node)
            if not isinstance(key, (str, bool)) or key in result:
                raise ToolchainError("workflow contains a duplicate or invalid mapping key")
            result[key] = loader.construct_object(value_node)
        return result

    UniqueLoader.add_constructor("tag:yaml.org,2002:map", mapping)
    try:
        result = yaml.load(read_text(path), Loader=UniqueLoader)
    except yaml.YAMLError as error:
        raise ToolchainError("workflow compiler contract is malformed") from error
    if not isinstance(result, dict) or not isinstance(result.get("jobs"), dict):
        raise ToolchainError("workflow compiler contract has no jobs")
    return result


def reject_compiler_executable_override(environment: dict) -> None:
    if not isinstance(environment, dict):
        raise ToolchainError("compiler environment must be a mapping")
    if any(key in environment for key in COMPILER_EXECUTABLE_ENV):
        raise ToolchainError("compiler executable overrides are not allowed in workflows")


def validate_documentation(root: Path, pin: str) -> None:
    expected_cache = f"automexia-rust-{pin}-v2"
    for name in CACHE_DOCS:
        labels = set(re.findall(r"automexia-rust-[0-9.]+-v[0-9]+", read_text(root / name)))
        if labels != {expected_cache}:
            raise ToolchainError("compiler cache documentation differs from policy")
    for name in COMPILER_DOCS:
        source = read_text(root / name)
        claimed = re.findall(r"\bRust\s+\*{0,2}([0-9]+\.[0-9]+\.[0-9]+)\b", source)
        if "rust-toolchain.toml" not in source or any(version != pin for version in claimed):
            raise ToolchainError("compiler documentation differs from policy")


def _validate_policy(root: Path) -> int:
    pin, minimum = versions(root)
    verified_jobs = 0
    for name in RUST_WORKFLOWS:
        workflow = load_workflow(root / ".github/workflows" / name)
        reject_compiler_executable_override(workflow.get("env", {}))
        if workflow.get("env", {}).get("RUSTUP_TOOLCHAIN") != pin:
            raise ToolchainError(f"{name}: workflow must explicitly select the repository compiler pin")
        for job_number, (job_id, job) in enumerate(workflow["jobs"].items(), 1):
            location = f"{name}/job-{job_number}"
            reject_compiler_executable_override(job.get("env", {}))
            steps = job.get("steps", [])
            if not isinstance(steps, list) or not all(isinstance(step, dict) for step in steps):
                raise ToolchainError(f"{name}: malformed compiler job")
            selection = job.get("env", {}).get("RUSTUP_TOOLCHAIN", pin)
            kind = "nightly" if name == "nightly.yml" and job_id in {"fuzz", "miri", "sanitizers"} else "development"
            if selection != (NIGHTLY if kind == "nightly" else pin):
                raise ToolchainError(f"{location}: job compiler override differs from policy")
            verified = False
            python_ready = False
            uses_rust = False
            for step in steps:
                if str(step.get('uses', '')).startswith('actions/setup-python@'):
                    if (step['uses'] != PYTHON_SETUP
                            or step.get('with') != {'python-version': '3.12'}
                            or step.get('if') is not None
                            or step.get('continue-on-error') is not None):
                        raise ToolchainError(f"{location}: Python bootstrap differs from policy")
                    python_ready = True
                reject_compiler_executable_override(step.get("env", {}))
                command = step.get("run", "")
                if not isinstance(command, str):
                    raise ToolchainError(f"{location}: malformed compiler command")
                live_lines = "\n".join(line for line in command.splitlines() if not line.lstrip().startswith("#"))
                if re.search(r"\b(?:RUSTC|RUSTDOC|CARGO_BUILD_RUSTC|CARGO_BUILD_RUSTDOC)\s*=", live_lines):
                    raise ToolchainError(f"{location}: compiler executable override is not allowed")
                if re.search(r"\brustup\s+default\b", live_lines):
                    raise ToolchainError(f"{location}: global rustup default is not explicit selection")
                if re.search(r"\bRUSTUP_TOOLCHAIN\s*=|\brustup\s+override\b", live_lines):
                    raise ToolchainError(f"{location}: commands must not mutate compiler selection")
                step_selection = step.get("env", {}).get("RUSTUP_TOOLCHAIN", selection)
                is_msrv = step.get("id") == "msrv"
                if step_selection != (minimum if is_msrv else selection):
                    raise ToolchainError(f"{location}: step shadows the verified compiler")
                verification = f"python tools/ci/rust_toolchain.py verify --kind {'msrv' if is_msrv else kind}"
                if verification in live_lines:
                    if not python_ready:
                        raise ToolchainError(f"{location}: Python bootstrap must precede compiler verification")
                    if step.get("if") is not None or step.get("continue-on-error") is not None:
                        raise ToolchainError(f"{location}: compiler verification cannot be skipped or ignored")
                    lines = [line.strip() for line in live_lines.splitlines()]
                    if lines.count(verification) != 1:
                        raise ToolchainError(f"{location}: compiler verification must be a mandatory command")
                    position = lines.index(verification)
                    if re.search(r"\bcargo\s|\brustc\s", "\n".join(lines[:position])):
                        raise ToolchainError(f"{location}: Rust use precedes compiler verification")
                    # pwsh needs an immediate native-exit check; Bash must retain errexit.
                    if step.get("shell") == "pwsh":
                        if lines[position + 1:position + 2] != ["if ($LASTEXITCODE -ne 0) { throw 'compiler identity verification failed' }"]:
                            raise ToolchainError(f"{location}: compiler failure propagation is missing")
                    elif step.get("shell") != "bash" or "set -euo pipefail" not in lines[:position] or re.search(r"\bset\s+\+e\b|\bset\s+\+o\s+errexit\b", live_lines):
                        raise ToolchainError(f"{location}: compiler failure propagation is missing")
                    if not is_msrv:
                        verified = True
                if re.search(r"\bcargo(?:\s|\s*\+)|\brustc\s", live_lines):
                    uses_rust = True
                    if not verified:
                        raise ToolchainError(f"{location}: Rust use precedes compiler verification")
                    overrides = re.findall(r"\bcargo\s+\+([^\s]+)", live_lines)
                    if any(value != NIGHTLY or kind != "nightly" for value in overrides):
                        raise ToolchainError(f"{location}: unreviewed command-line compiler override")
                if is_msrv and not (verification in live_lines and "cargo check --workspace --all-targets --all-features --locked" in live_lines):
                    raise ToolchainError("MSRV must verify and check the locked supported workspace")
            if uses_rust:
                verified_jobs += 1
            cache = job.get("env", {}).get("SCCACHE_GHA_VERSION")
            if cache is not None and cache != f"automexia-rust-{pin}-v2":
                raise ToolchainError(f"{location}: compiler cache identity differs from the pin")
    quality = load_workflow(root / ".github/workflows/ci.yml")["jobs"]["quality"]
    if sum(step.get("id") == "msrv" for step in quality["steps"]) != 1:
        raise ToolchainError("CI must retain one explicit MSRV verification/build step")
    validate_documentation(root, pin)
    return verified_jobs


def validate_policy(root: Path = ROOT) -> int:
    try:
        return _validate_policy(root)
    except (KeyError, TypeError, AttributeError, RecursionError) as error:
        raise ToolchainError("compiler workflow structure is invalid") from error


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("check-policy", "verify"))
    parser.add_argument("--kind", choices=("development", "msrv", "nightly"), default="development")
    args = parser.parse_args()
    try:
        result = {"verified_jobs": validate_policy()} if args.command == "check-policy" else verify(args.kind)
        print(json.dumps(result, sort_keys=True))
        return 0
    except ToolchainError as error:
        print(f"Rust toolchain check failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
