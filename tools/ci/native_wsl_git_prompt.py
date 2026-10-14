"""Native Git metadata through the bounded Windows/WSL prompt helper."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
import time

from native_wsl_shell_matrix import CheckFailure, run


def git(directory: Path, *arguments: str) -> bytes:
    # A fixture must not inherit repository overrides, signing or user hooks.
    environment = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
    environment.update(HOME=str(directory / "home"), XDG_CONFIG_HOME=str(directory / "home"),
                       GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull, GIT_TERMINAL_PROMPT="0")
    return run(["git", "-c", "init.templateDir=", "-c", "commit.gpgSign=false",
                "-c", "core.hooksPath=" + str(directory / "home"), *arguments], environment=environment)


def guest(action: str, value: str | None) -> None:
    if action == "create":
        directory = Path(tempfile.mkdtemp(prefix="automexia-git-fixture-", dir="/tmp"))
        try:
            (directory / "home").mkdir()
            repository = directory / "repo"
            git(directory, "init", "-q", "-b", "fixture-first", str(repository))
            git(directory, "-C", str(repository), "-c", "user.name=Fixture", "-c",
                "user.email=fixture@example.invalid", "commit", "-q", "--allow-empty", "-m", "fixture")
            git(directory, "-C", str(repository), "worktree", "add", "-q", "-b", "fixture-worktree",
                str(directory / "worktree"))
            (directory / "linked").symlink_to(repository, target_is_directory=True)
            (directory / "outside").mkdir()
            (directory / "config").write_text(json.dumps({
                "current-context": "fixture", "contexts": [{"name": "fixture",
                "context": {"namespace": "sandbox"}}]}), encoding="utf-8")
            print(directory)
        except Exception:
            shutil.rmtree(directory)
            raise
        return
    directory = Path(value or "")
    if (directory.parent != Path("/tmp") or not directory.name.startswith("automexia-git-fixture-")
            or directory.is_symlink() or not directory.is_dir()):
        raise ValueError("Invalid fixture directory")
    if action == "delete":
        shutil.rmtree(directory)
    elif action == "branch":
        git(directory, "-C", str(directory / "repo"), "switch", "-q", "-c", "fixture-second")
    elif action == "detach":
        git(directory, "-C", str(directory / "repo"), "checkout", "-q", "--detach")
        print(git(directory, "-C", str(directory / "repo"), "rev-parse", "HEAD").decode().strip()[:8])


def verify(binary: Path, distro: str, report: Path) -> None:
    source = Path(__file__).resolve()
    script = run(["wsl.exe", "--distribution", distro, "--exec", "wslpath", "-a", str(source)]).decode().strip()
    prefix = ["wsl.exe", "--distribution", distro, "--exec", "python3", script, "--guest"]
    directory = run([*prefix, "create"]).decode().strip()
    session = {"session_id": 7, "cwd": directory + "/repo", "title": "", "distro": distro,
               "shell_name": "bash", "shell_user": "fixture", "shell_path": "/bin/bash",
               "shell_integration": True, "shell_pid": 1,
               "environment": {"KUBECONFIG": directory + "/config"}}
    checks = 0
    measurements = []
    console_required = binary.name.casefold() == "amx.exe"
    console_checked = False

    def read(relative: str, expected: str | None, enabled: bool = True, kube: bool = False) -> None:
        nonlocal checks
        session["cwd"] = directory + "/" + relative
        request = {"session": session, "git": enabled, "kubernetes": kube}
        measurement = {"check": checks + 1, "status": "failed"}
        started = time.perf_counter()
        try:
            output = run([str(binary), "--internal-prompt-context-v2", json.dumps(request)], timeout=5,
                         operation=f"Git prompt check {checks + 1}")
            result = json.loads(output)
            expected_kube = {"context": "fixture", "namespace": "sandbox"} if kube else None
            if result != {"git_branch": expected, "kubernetes": expected_kube}:
                raise ValueError("Native WSL Git projection mismatch")
            measurement["status"] = "passed"
            checks += 1
        except CheckFailure as error:
            measurement["failure"] = str(error)
            raise
        finally:
            measurement["elapsed_ms"] = (time.perf_counter() - started) * 1000
            measurements.append(measurement)

    try:
        read("repo", "fixture-first")
        read("repo", "fixture-first", kube=True)
        read("repo", None, False, True)
        read("worktree", "fixture-worktree")
        run([*prefix, "branch", "--value", directory])
        read("repo", "fixture-second")
        read("repo", None, False)
        read("outside", None)
        read("linked", None)
        detached = run([*prefix, "detach", "--value", directory]).decode().strip()
        read("repo", detached)
        if console_required:
            verify_console_entrypoint(binary)
            console_checked = True
    finally:
        cleaned = False
        try:
            run([*prefix, "delete", "--value", directory])
            cleaned = True
        finally:
            # Hash after execution: reading the image before the first launch
            # would change the cold-start conditions being recorded.
            with binary.open("rb") as image:
                image_digest = hashlib.sha256()
                for chunk in iter(lambda: image.read(1024 * 1024), b""):
                    image_digest.update(chunk)
                digest = image_digest.hexdigest()
            report.parent.mkdir(parents=True, exist_ok=True)
            report.write_text(json.dumps({
                "status": "passed" if checks == 9 and cleaned and (not console_required or console_checked) else "failed", "checks": checks,
                "console_entrypoint": "passed" if console_checked else "failed" if console_required else "not_applicable",
                "helper_launches": measurements, "binary_bytes": binary.stat().st_size,
                "binary_sha256": digest, "deadline_seconds": 5, "cleanup": "passed" if cleaned else "failed",
                "scope": "native Git and application helper; timings include process startup; no GUI pixels",
            }) + "\n")
    print(f"PASS: {checks} native WSL Git prompt checks")


def verify_console_entrypoint(binary: Path) -> None:
    # The package console entrypoint must serve the restricted protocol itself.
    # A missing desktop sibling makes accidental forwarding fail observably.
    with tempfile.TemporaryDirectory(prefix="amx-prompt-entry-") as temporary:
        copied = Path(temporary) / "amx.exe"
        shutil.copyfile(binary, copied)
        request = {"session": {"session_id": 7, "cwd": "/tmp", "title": "",
                   "distro": "Fixture", "shell_name": "bash", "shell_user": "fixture",
                   "shell_path": "/bin/bash", "shell_integration": True, "shell_pid": 1},
                   "git": False, "kubernetes": False}
        result = run([str(copied), "--internal-prompt-context-v2", json.dumps(request)],
                     timeout=5, operation="Console prompt entrypoint")
        if json.loads(result) != {"git_branch": None, "kubernetes": None}:
            raise ValueError("Console prompt entrypoint did not serve the bounded protocol")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--guest", choices=("create", "delete", "branch", "detach"))
    parser.add_argument("--value")
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--distro")
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    try:
        if args.report:
            args.report.parent.mkdir(parents=True, exist_ok=True)
            args.report.write_text('{"status":"failed"}\n')
        if args.guest:
            guest(args.guest, args.value)
        elif sys.platform == "win32" and args.binary and args.distro and args.report:
            verify(args.binary.resolve(), args.distro, args.report)
        else:
            raise ValueError("Explicit Windows fixture arguments required")
    except (OSError, ValueError, RuntimeError) as error:
        print("FAIL: native WSL Git fixture or helper contract", file=sys.stderr)
        if isinstance(error, CheckFailure):
            print(str(error), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
