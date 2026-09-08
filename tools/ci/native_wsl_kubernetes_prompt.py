"""Real kubectl writes -> WSL filesystem -> packaged helper, with no cluster access."""
from __future__ import annotations

import argparse
import base64
import json
import os
import pathlib
import re
import statistics
import subprocess
import sys
import tempfile
import time


def run(argv: list[str], timeout: float = 15) -> subprocess.CompletedProcess[bytes]:
    try:
        result = subprocess.run(argv, input=b"", capture_output=True, timeout=timeout, check=False)
    except (OSError, subprocess.TimeoutExpired):
        raise RuntimeError("native fixture process unavailable or timed out") from None
    if len(result.stdout) > 16384 or len(result.stderr) > 16384:
        raise RuntimeError("native fixture response exceeds its limit")
    return result


def guest(action: str, value: str | None, shell: str = "bash") -> None:
    if action == "create":
        with tempfile.NamedTemporaryFile(prefix="automexia-prompt-fixture-Ω & ", suffix=".json", delete=False) as file:
            config = {
                "apiVersion": "v1", "kind": "Config", "current-context": "fixture",
                "contexts": [{"name": "fixture", "context": {"cluster": "fixture", "user": "fixture", "namespace": "before"}}],
                "clusters": [{"name": "fixture", "cluster": {"server": "https://127.0.0.1:9"}}],
                "users": [{"name": "fixture", "user": {"token": "fixture-private-canary", "exec": {
                    "apiVersion": "client.authentication.k8s.io/v1", "command": "must-never-run-fixture", "interactiveMode": "Never"}}}],
            }
            file.write(json.dumps(config).encode())
            print(file.name)
        return
    # Cleanup and writes are restricted to the exact regular fixture file under
    # the OS temporary root, never a shell's real kubeconfig or a recursive tree.
    path = pathlib.Path(value or "").absolute()
    if path.parent != pathlib.Path(tempfile.gettempdir()) or not path.name.startswith("automexia-prompt-fixture-") or path.suffix != ".json" or path.is_symlink() or not path.is_file():
        raise RuntimeError("invalid native fixture target")
    if action == "delete":
        path.unlink()
    elif action == "mutate":
        result = run(["kubectl", "--kubeconfig", str(path), "config", "set-context", "--current", "--namespace=sandbox"])
        if result.returncode:
            raise RuntimeError("native kubectl fixture mutation failed")
    elif action == "clear":
        path.write_text("{}", encoding="utf-8")
    elif action == "prompt":
        source = pathlib.Path(__file__).resolve().parents[2] / f"shell-integration/{shell}/automexia.{shell}"
        with tempfile.TemporaryDirectory(prefix="automexia-prompt-home-") as home:
            environment = dict(os.environ, HOME=home, XDG_CONFIG_HOME=home, AUTOMEXIA_CONFIG_HOME=home,
                               USER="alice", KUBECONFIG=str(path), TERM_PROGRAM="Automexia",
                               AUTOMEXIA_PLAIN_LS="1", AUTOMEXIA_FIXTURE_SOURCE=str(source))
            if shell == "fish":
                argv = ["fish", "--no-config"]
                script = 'source "$AUTOMEXIA_FIXTURE_SOURCE" >/dev/null 2>/dev/null\n__automexia_fish_prompt\n'
            else:
                argv = ["bash", "--noprofile", "--norc"] if shell == "bash" else ["zsh", "-f"]
                hook = "__automexia_pre_prompt" if shell == "bash" else "__automexia_precmd"
                script = '. "$AUTOMEXIA_FIXTURE_SOURCE" >/dev/null 2>/dev/null\n' + hook + "\n"
            result = subprocess.run(argv, input=script.encode(), env=environment, cwd=home,
                                    capture_output=True, timeout=10, check=False)
            if result.returncode or len(result.stdout) > 32768:
                raise RuntimeError("native prompt hook failed")
            frames = re.findall(rb"\x1b\]1337;SetUserVar=automexia_env_(HOME|KUBECONFIG)=([A-Za-z0-9+/=]*)\x07", result.stdout)
            if len(frames) != 2:
                raise RuntimeError("native prompt did not publish exactly two location frames")
            hints = {key.decode(): base64.b64decode(encoded, validate=True).decode() for key, encoded in frames}
            if hints != {"HOME": home, "KUBECONFIG": str(path)}:
                raise RuntimeError("native prompt path projection mismatch")
            print(json.dumps(hints))
    else:
        raise RuntimeError("invalid native fixture action")


def verify(binary: pathlib.Path, distro: str, report: pathlib.Path) -> None:
    source = pathlib.Path(__file__).resolve()
    if source.drive:
        guest_script = "/mnt/" + source.drive[0].lower() + source.as_posix()[2:]
    else:
        raise RuntimeError("this rehearsal requires a Windows host and native WSL")
    prefix = ["wsl.exe", "--distribution", distro, "--exec", "python3", guest_script, "--guest"]
    created = run([*prefix, "create"])
    if created.returncode:
        raise RuntimeError("native fixture creation failed")
    fixture = created.stdout.decode("utf-8").strip()
    request = {
        "session_id": 7, "cwd": "/tmp", "title": "", "distro": distro,
        "os_version": None, "shell_name": "bash", "shell_user": "alice",
        "shell_path": "/bin/bash", "shell_integration": True, "shell_pid": 1,
        "environment": {"HOME": "/fixture", "KUBECONFIG": fixture},
    }
    samples: list[float] = []

    def read(expected: dict | None) -> None:
        start = time.perf_counter()
        result = run([str(binary), "--internal-prompt-context-v1", json.dumps(request)], timeout=5)
        samples.append((time.perf_counter() - start) * 1000)
        if result.returncode or result.stderr:
            raise RuntimeError("native helper startup failed")
        if json.loads(result.stdout) != expected or b"fixture-private-canary" in result.stdout:
            raise RuntimeError("native helper projection mismatch or disclosure")

    try:
        for shell in ["bash", "zsh", "fish"]:
            emitted = run([*prefix, "prompt", "--value", fixture, "--shell", shell])
            if emitted.returncode:
                raise RuntimeError("native shell path-hint rehearsal failed")
            request["environment"] = json.loads(emitted.stdout)
            request["shell_name"] = shell
            read({"context": "fixture", "namespace": "before"})
        if run([*prefix, "mutate", "--value", fixture]).returncode:
            raise RuntimeError("native fixture mutation failed")
        for _ in range(20):
            read({"context": "fixture", "namespace": "sandbox"})
        if run([*prefix, "clear", "--value", fixture]).returncode:
            raise RuntimeError("native fixture clear failed")
        read(None)
        invalid = run([str(binary), "--internal-prompt-context-v1", "not-json"])
        if invalid.returncode != 2 or invalid.stdout or invalid.stderr:
            raise RuntimeError("invalid helper request was not rejected silently")
    finally:
        if run([*prefix, "delete", "--value", fixture]).returncode:
            raise RuntimeError("native fixture cleanup failed")
    # Never retain request arguments, temporary paths, usernames or raw output.
    evidence = {"status": "pass", "platform": "Windows-host/WSL-filesystem", "samples": len(samples),
                "median_ms": statistics.median(samples), "p95_ms": sorted(samples)[int(len(samples) * .95)],
                "max_ms": max(samples), "scope": "real helper process, not desktop frames or input latency"}
    report.parent.mkdir(parents=True, exist_ok=True)
    report.write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(evidence))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--guest", choices=["create", "mutate", "clear", "delete", "prompt"])
    parser.add_argument("--shell", choices=["bash", "zsh", "fish"], default="bash")
    parser.add_argument("--value")
    parser.add_argument("--binary", type=pathlib.Path)
    parser.add_argument("--distro")
    parser.add_argument("--report", type=pathlib.Path)
    args = parser.parse_args()
    try:
        if args.guest:
            guest(args.guest, args.value, args.shell)
        elif args.binary and args.distro and args.report:
            verify(args.binary.resolve(), args.distro, args.report)
        else:
            raise RuntimeError("binary, distribution and report are required")
    except RuntimeError as error:
        # RuntimeError messages above are fixed logical operation labels.
        print(f"FAIL: native Kubernetes prompt rehearsal: {error}", file=sys.stderr)
        return 1
    except (OSError, ValueError):
        print("FAIL: native Kubernetes prompt rehearsal; fixture or helper contract failed", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
