#!/usr/bin/env python3
"""Exercise the real local Semgrep and Gitleaks executables with canaries."""

from __future__ import annotations

import argparse
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("github_free_assurance", ROOT / "tools/ci/github_free_assurance.py")
assert SPEC and SPEC.loader
ASSURANCE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = ASSURANCE
SPEC.loader.exec_module(ASSURANCE)

ASSURANCE_GIT_LOCAL_ENVIRONMENT = (
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_CONFIG",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
    "GIT_OBJECT_DIRECTORY",
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_IMPLICIT_WORK_TREE",
    "GIT_GRAFT_FILE",
    "GIT_INDEX_FILE",
    "GIT_NO_REPLACE_OBJECTS",
    "GIT_REPLACE_REF_BASE",
    "GIT_PREFIX",
    "GIT_SHALLOW_FILE",
    "GIT_COMMON_DIR",
)


def isolated_git_environment() -> dict[str, str]:
    environment = os.environ.copy()
    for name in ASSURANCE_GIT_LOCAL_ENVIRONMENT:
        environment.pop(name, None)
    for name in tuple(environment):
        if name.startswith(("GIT_CONFIG_KEY_", "GIT_CONFIG_VALUE_")):
            environment.pop(name)
    return environment


def installed_command(name: str) -> list[str] | None:
    policy = ASSURANCE.load_policy()
    try:
        command = ASSURANCE.tool_command(policy, name)
    except ASSURANCE.AssuranceError:
        executable = shutil.which(name)
        return [executable] if executable else None
    return command


def execute_once(
    command: list[str],
    *,
    cwd: Path,
    environment: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        command,
        cwd=cwd,
        env=environment,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=180,
        check=False,
    )


def execute(
    command: list[str],
    *,
    cwd: Path,
    environment: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[str]:
    is_semgrep = Path(command[0]).stem.lower() == "semgrep" or (
        len(command) >= 2 and Path(command[1]).name == "semgrep_entrypoint.py"
    )
    if not is_semgrep:
        return execute_once(command, cwd=cwd, environment=environment)
    # Semgrep adds an RPC socket suffix, so use the same short cache path as
    # production rather than a deeply nested worktree-local directory.
    with ASSURANCE.semgrep_temporary_directory() as temporary:
        semgrep_environment = (
            os.environ.copy() if environment is None else environment.copy()
        )
        semgrep_environment["PATH"] = str(Path(command[0]).parent) + os.pathsep + semgrep_environment.get("PATH", "")
        for name in ("TMP", "TEMP", "TMPDIR"):
            semgrep_environment[name] = temporary
        return execute_once(command, cwd=cwd, environment=semgrep_environment)


class FreeSecurityToolTests(unittest.TestCase):
    def test_historical_exemptions_have_an_exact_reviewed_contract(self) -> None:
        ASSURANCE.validate_history_exemptions()

    def test_history_review_rejects_missing_duplicate_expanded_or_changed_entries(self) -> None:
        source = (ROOT / '.gitleaksignore').read_text(encoding='utf-8')
        entry = sorted(ASSURANCE.HISTORICAL_FINDINGS)[0]
        for candidate in ('', source.replace(entry, ''), source + '\n' + entry,
                          source + '\n' + 'a' * 40 + ':other.txt:generic-api-key:1',
                          source.replace(entry, entry.replace(':196', ':197')) if ':196' in entry
                          else source.replace(entry, 'b' * 40 + entry[40:]),
                          source + '\n#' + 'x' * 4096):
            with self.assertRaises(ASSURANCE.AssuranceError):
                ASSURANCE.validate_history_exemption_text(candidate)

    def test_temporary_git_canary_drops_the_callers_repository_context(self) -> None:
        ambient = {
            name: f"caller-{index}"
            for index, name in enumerate(ASSURANCE_GIT_LOCAL_ENVIRONMENT)
        }
        ambient["AUTOMEXIA_CANARY_SENTINEL"] = "preserved"
        with mock.patch.dict(os.environ, ambient, clear=True):
            environment = isolated_git_environment()
        for name in ASSURANCE_GIT_LOCAL_ENVIRONMENT:
            self.assertNotIn(name, environment)
        self.assertEqual(environment["AUTOMEXIA_CANARY_SENTINEL"], "preserved")

    def test_semgrep_detects_the_shell_evaluator_canary_and_ignores_safe_argv(self) -> None:
        command = installed_command("semgrep")
        if command is None:
            self.skipTest("Semgrep is not installed; run cargo xtask assurance install-tools")
        rule = ROOT / "tools/ci/semgrep-rules.yml"
        unsafe = ROOT / "tests/fixtures/free-assurance/semgrep/shell-evaluation.rs"
        safe = ROOT / "tests/fixtures/free-assurance/semgrep/safe-command.rs"
        literal = ROOT / "tests/fixtures/free-assurance/semgrep/secret-literal.rs"
        detected = execute([*command, "scan", "--config", str(rule), "--json", "--metrics=off", str(unsafe)], cwd=ROOT)
        self.assertEqual(detected.returncode, 0, detected.stderr)
        result = json.loads(detected.stdout)
        self.assertEqual(
            [item["check_id"].rsplit(".", maxsplit=1)[-1] for item in result["results"]],
            ["automexia-rust-shell-evaluation"],
        )
        literal_detected = execute([*command, "scan", "--config", str(rule), "--json", "--metrics=off", str(literal)], cwd=ROOT)
        self.assertEqual(literal_detected.returncode, 0, literal_detected.stderr)
        literal_result = json.loads(literal_detected.stdout)
        self.assertEqual(
            [item["check_id"].rsplit(".", maxsplit=1)[-1] for item in literal_result["results"]],
            ["automexia-rust-secret-literal"],
        )
        clear = execute([*command, "scan", "--config", str(rule), "--json", "--metrics=off", str(safe)], cwd=ROOT)
        self.assertEqual(clear.returncode, 0, clear.stderr)
        self.assertEqual(json.loads(clear.stdout)["results"], [])

    def test_gitleaks_detects_a_committed_canary_without_leaking_it(self) -> None:
        command = installed_command("gitleaks")
        if command is None:
            self.skipTest("Gitleaks is not installed; run cargo xtask assurance install-tools")
        temporary_root = ASSURANCE.process_temporary_directory()
        parent_head = execute(["git", "rev-parse", "HEAD"], cwd=ROOT)
        parent_status = execute(["git", "status", "--porcelain=v1", "-uno"], cwd=ROOT)
        parent_config = execute(
            ["git", "config", "--local", "--null", "--list"], cwd=ROOT
        )
        self.assertEqual(parent_head.returncode, 0, parent_head.stderr)
        self.assertEqual(parent_status.returncode, 0, parent_status.stderr)
        self.assertEqual(parent_config.returncode, 0, parent_config.stderr)
        with tempfile.TemporaryDirectory(prefix="automexia-gitleaks-", dir=temporary_root) as temporary:
            repository = Path(temporary)
            repository_environment = isolated_git_environment()
            for command_line in (
                ["git", "init", "--quiet"],
                ["git", "config", "user.email", "test@example.invalid"],
                ["git", "config", "user.name", "Automexia Scanner Test"],
            ):
                completed = execute(
                    command_line,
                    cwd=repository,
                    environment=repository_environment,
                )
                self.assertEqual(completed.returncode, 0, completed.stderr)
            canary = "gh" + "p_" + ("A1b2C3d4E5f6G7h8I9" + "j0K1l2M3n4O5p6Q7r8")
            generic_canary = 'A1b2C3d4E5f6' + 'G7h8I9j0K1l2M3n4O5p6Q7r8'
            owners = ('AGENTS.md', 'extensions/devops-kubernetes/tests/contracts.rs',
                      'docs/docusaurus.config.js')
            # Exact old fingerprints must not suppress new findings in the same paths.
            shutil.copy2(ROOT / '.gitleaksignore', repository / '.gitleaksignore')
            for owner in owners:
                path = repository / owner
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(f"token={canary}\napi_key={generic_canary}\n", encoding="utf-8")
            for command_line in (["git", "add", "--", *owners], ["git", "commit", "--quiet", "-m", "scanner canary"]):
                completed = execute(
                    command_line,
                    cwd=repository,
                    environment=repository_environment,
                )
                self.assertEqual(completed.returncode, 0, completed.stderr)
            detected = execute(
                [*command, "git", "--redact", "--no-banner", "--config", str(ROOT / ".gitleaks.toml"), "--log-opts=--all",
                 '--report-format=json', '--report-path=findings.json'],
                cwd=repository,
                environment=repository_environment,
            )
            self.assertEqual(detected.returncode, 1)
            self.assertNotIn(canary, detected.stdout + detected.stderr)
            report = (repository / 'findings.json').read_text(encoding='utf-8')
            self.assertNotIn(canary, report)
            self.assertNotIn(generic_canary, report)
            self.assertEqual({item['File'] for item in json.loads(report)}, set(owners))
            self.assertEqual({item['File'] for item in json.loads(report)
                              if item['RuleID'] == 'generic-api-key'}, set(owners))
        current_head = execute(["git", "rev-parse", "HEAD"], cwd=ROOT)
        current_status = execute(["git", "status", "--porcelain=v1", "-uno"], cwd=ROOT)
        current_config = execute(
            ["git", "config", "--local", "--null", "--list"], cwd=ROOT
        )
        self.assertEqual(current_head.returncode, 0, current_head.stderr)
        self.assertEqual(current_status.returncode, 0, current_status.stderr)
        self.assertEqual(current_config.returncode, 0, current_config.stderr)
        self.assertEqual(current_head.stdout, parent_head.stdout)
        self.assertEqual(current_status.stdout, parent_status.stdout)
        self.assertEqual(current_config.stdout, parent_config.stdout)

    def test_gitleaks_excludes_private_local_directory_before_content_scanning(self) -> None:
        command = installed_command("gitleaks")
        if command is None:
            self.skipTest("Gitleaks is not installed; run cargo xtask assurance install-tools")
        temporary_root = ASSURANCE.process_temporary_directory()
        with tempfile.TemporaryDirectory(prefix="automexia-private-scope-", dir=temporary_root) as temporary:
            root = Path(temporary)
            private = root / ".automexia-private"
            private.mkdir()
            canary = "gh" + "p_" + ("A1b2C3d4E5f6G7h8I9" + "j0K1l2M3n4O5p6Q7r8")
            (private / "canary.txt").write_text(f"token={canary}\n", encoding="utf-8")
            completed = execute(
                [*command, "dir", "--redact", "--no-banner", "--timeout=30", "--max-target-megabytes=16", "--config", str(ROOT / ".gitleaks.toml"), str(root)],
                cwd=ROOT,
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertNotIn(canary, completed.stdout + completed.stderr)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tool", choices=("all", "semgrep", "gitleaks"), default="all")
    arguments = parser.parse_args()
    suite = unittest.TestSuite()
    if arguments.tool in {"all", "semgrep"}:
        suite.addTest(FreeSecurityToolTests("test_semgrep_detects_the_shell_evaluator_canary_and_ignores_safe_argv"))
    if arguments.tool in {"all", "gitleaks"}:
        suite.addTest(FreeSecurityToolTests("test_historical_exemptions_have_an_exact_reviewed_contract"))
        suite.addTest(FreeSecurityToolTests("test_history_review_rejects_missing_duplicate_expanded_or_changed_entries"))
        suite.addTest(FreeSecurityToolTests("test_temporary_git_canary_drops_the_callers_repository_context"))
        suite.addTest(FreeSecurityToolTests("test_gitleaks_detects_a_committed_canary_without_leaking_it"))
        suite.addTest(FreeSecurityToolTests("test_gitleaks_excludes_private_local_directory_before_content_scanning"))
    return 0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1


if __name__ == "__main__":
    raise SystemExit(main())
