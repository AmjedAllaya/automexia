#!/usr/bin/env python3
"""Install final packages in disposable official distro containers; no host app launch."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import uuid

from test_packaged_shells import verify

IMAGES = {
    "ubuntu": "ubuntu:22.04@sha256:5ec03bb3441e8b0bf3b4f9cd4629a1ae763010dc3035bb8da3ae6cf026486401",
    "debian": "debian:12-slim@sha256:7c7b2c966bc9ee8cedfeef67e0e279108992c77681fa595db4a9d65c06ccc587",
    "fedora": "fedora:43@sha256:a651ddf48ea28a06ed4e1e6519f51c9f47e7a5a138722ade87369b8fbb7e5b42",
    "opensuse": "opensuse/leap:16.0@sha256:7f3aeccb6a613c0fc1690d3d3e5eb493fd477ee4587de20640017fc36d72f9b0",
}


def run(argv: list[str], timeout: int = 240) -> bytes:
    result = subprocess.run(argv, check=False, capture_output=True, timeout=timeout)
    if result.returncode:
        # Package-manager diagnostics contain only public packages and container paths.
        detail = result.stderr[-8192:].decode("utf-8", errors="replace")
        raise RuntimeError(f"{Path(argv[0]).name} exited {result.returncode}: {detail}")
    return result.stdout


def inside(family: str, version: str) -> None:
    packages = Path("/packages")
    deb = family in ("ubuntu", "debian")
    candidates = list(packages.glob("*.deb" if deb else "*.rpm"))
    if len(candidates) != 1:
        raise ValueError("expected exactly one native package")
    package = str(candidates[0])
    if deb:
        run(["apt-get", "update"])
        run(["apt-get", "install", "-y", "--no-install-recommends", "bash", "zsh", "fish", package])
        uninstall = ["apt-get", "remove", "-y", "automexia-terminal"]
    elif family == "fedora":
        run(["dnf", "install", "-y", "bash", "zsh", "fish", package])
        uninstall = ["dnf", "remove", "-y", "automexia-terminal"]
    else:
        run(["zypper", "--non-interactive", "refresh"])
        run(["zypper", "--non-interactive", "install", "--allow-unsigned-rpm", "bash", "zsh", "fish", package])
        uninstall = ["zypper", "--non-interactive", "remove", "automexia-terminal"]
    for name in ("automexia", "amx"):
        if version.encode() not in run(["/usr/bin/" + name, "--version"], timeout=15):
            raise ValueError("installed version mismatch")
    results = verify(Path("/usr/share/automexia/shell-integration"))
    run(uninstall)
    if Path("/usr/bin/automexia").exists():
        raise ValueError("uninstall left the executable behind")
    print(json.dumps({"distro": family, "installed_package": "pass", "shells": results}))


def host(packages: Path, version: str) -> None:
    if os.environ.get("GITHUB_ACTIONS") != "true" or os.environ.get("RUNNER_OS") != "Linux":
        raise ValueError("distro tests require a disposable hosted Linux runner")
    scripts = Path(__file__).resolve().parent
    for family, image in IMAGES.items():
        # Bootstrap only Python; all remaining package operations use exact argv.
        bootstrap = (["apt-get", "update"], ["apt-get", "install", "-y", "python3"]) if family in ("ubuntu", "debian") else (
            (["dnf", "install", "-y", "python3"],) if family == "fedora" else
            (["zypper", "--non-interactive", "install", "python3"],))
        # A fixed, repository-owned shell script is needed before Python exists.
        commands = "\n".join(" ".join(command) for command in bootstrap)
        commands += '\nexec python3 /qa/tools/ci/test_linux_distros.py --inside "$1" --version "$2"\n'
        with tempfile.TemporaryDirectory(prefix="automexia-distro-") as temporary:
            entry = Path(temporary) / "entry.sh"
            entry.write_text("#!/bin/sh\nset -eu\n" + commands, encoding="utf-8")
            name = "automexia-package-test-" + uuid.uuid4().hex
            try:
                result = run(["docker", "run", "--rm", "--name", name, "--init", "--memory", "3g", "--cpus", "2",
                              "--pids-limit", "256", "--mount", f"type=bind,src={packages.resolve()},dst=/packages,readonly",
                              "--mount", f"type=bind,src={scripts},dst=/qa/tools/ci,readonly",
                              "--mount", f"type=bind,src={entry},dst=/entry.sh,readonly",
                              image, "sh", "/entry.sh", family, version], timeout=600)
            finally:
                # Retain ownership if the Docker client times out before --rm.
                subprocess.run(["docker", "rm", "--force", name], check=False,
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=30)
            # Keep only the bounded structured result, not package-manager output.
            line = result.decode("utf-8").splitlines()[-1]
            report = json.loads(line)
            if report.get("distro") != family or report.get("installed_package") != "pass":
                raise ValueError("missing distro package result")
            print(json.dumps(report), flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--packages", type=Path)
    parser.add_argument("--version", required=True)
    parser.add_argument("--inside", choices=IMAGES)
    args = parser.parse_args()
    if args.inside:
        inside(args.inside, args.version)
    elif args.packages:
        host(args.packages, args.version)
    else:
        parser.error("--packages is required outside the container")
