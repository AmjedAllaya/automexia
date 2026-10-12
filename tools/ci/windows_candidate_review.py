#!/usr/bin/env python3
"""Replay the owned Windows release checks against an identified candidate run."""

import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

import yaml

ROOT = Path(__file__).resolve().parents[2]
STEPS = (
    "Verify, execute, install, and uninstall exact final package",
    "Exercise fresh installed Windows settings and policy recovery",
)
STRESS_SCRIPT = r'''$ErrorActionPreference = 'Stop'
$package = @(Get-ChildItem artifacts -File -Filter *.msi)
if ($package.Count -ne 1) { throw 'Expected one final MSI package' }
$digest = (Get-FileHash -LiteralPath $package[0].FullName -Algorithm SHA256).Hash.ToLowerInvariant()
& powershell.exe -NoProfile -File tests/integration/windows-package-review.ps1 -RepositoryRoot $env:GITHUB_WORKSPACE -PackagePath $package[0].FullName -PackageSha256 $digest -CandidateVersion $env:RELEASE_VERSION -CandidateCommit $env:CANDIDATE_COMMIT -PackageKind msi -Policy Restricted -StressOutputHighlighting
if ($LASTEXITCODE -ne 0) { throw 'Focused Windows setting stress failed' }
'''


def validate_source(run: dict, repository: str, commit: str) -> None:
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise ValueError("Candidate commit must be a full SHA")
    if (run.get("head_sha") != commit or run.get("head_branch") != "main"
            or run.get("event") != "workflow_dispatch"
            or run.get("path") != ".github/workflows/first-alpha.yml"
            or run.get("repository", {}).get("full_name") != repository):
        raise ValueError("Candidate run is not the requested main release build")


def release_scripts() -> list[str]:
    workflow = yaml.safe_load((ROOT / ".github/workflows/first-alpha.yml").read_text())
    result = []
    for name in STEPS:
        matches = [step["run"] for job in workflow["jobs"].values()
                   for step in job.get("steps", []) if step.get("name") == name]
        if len(matches) != 1:
            raise ValueError("Expected one owning release verification step")
        script = matches[0].replace("${{ matrix.runner_arch }}", "X64").replace(
            "${{ matrix.arch }}", "x86_64")
        if "${{" in script:
            raise ValueError("Unresolved release workflow expression")
        if name == STEPS[1]:
            argument = "-CandidateVersion $env:RELEASE_VERSION"
            if script.count(argument) != 2:
                raise ValueError("Candidate provenance arguments changed")
            script = script.replace(argument, argument + " -CandidateCommit $env:CANDIDATE_COMMIT")
        result.append(script)
    return result


def main() -> None:
    if (os.environ.get("GITHUB_ACTIONS") != "true"
            or os.environ.get("RUNNER_ENVIRONMENT") != "github-hosted"
            or os.environ.get("RUNNER_OS") != "Windows"
            or os.environ.get("RUNNER_ARCH") != "X64"):
        raise SystemExit("Candidate replay requires disposable hosted Windows x64")
    run_id = os.environ["CANDIDATE_RUN"]
    if not re.fullmatch(r"[0-9]{1,20}", run_id):
        raise ValueError("Candidate run must be a numeric Actions run ID")
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+-alpha", os.environ["RELEASE_VERSION"]):
        raise ValueError("Candidate version must identify an alpha")
    repository = os.environ["GITHUB_REPOSITORY"]
    response = subprocess.run(
        ["gh", "api", f"repos/{repository}/actions/runs/{run_id}"],
        check=True, capture_output=True, text=True, timeout=30)
    validate_source(json.loads(response.stdout), repository, os.environ["CANDIDATE_COMMIT"])
    subprocess.run(["gh", "run", "download", run_id, "--repo", repository,
                    "--name", "packages-windows-x86_64", "--dir", "artifacts"],
                   check=True, timeout=120)
    child_env = {key: value for key, value in os.environ.items()
                 if key not in {"GH_TOKEN", "GITHUB_TOKEN"}}
    scripts = [STRESS_SCRIPT] if os.environ.get("CANDIDATE_STRESS") == "true" else release_scripts()
    with tempfile.TemporaryDirectory(prefix="automexia-candidate-", dir=os.environ["RUNNER_TEMP"]) as folder:
        for index, script in enumerate(scripts):
            path = Path(folder) / f"verify-{index}.ps1"
            path.write_text(script, encoding="utf-8")
            subprocess.run(["pwsh", "-NoProfile", "-File", str(path)],
                           check=True, env=child_env, timeout=720)


if __name__ == "__main__":
    main()
