#!/usr/bin/env python3
"""Mutation coverage for the GitHub-Free local assurance runner."""

from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import os
import sys
import tarfile
import tempfile
import unittest
import zipfile
from unittest import mock
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
TEST_TEMP_PARENT = Path(ROOT.anchor) if os.name == "nt" else None
SPEC = importlib.util.spec_from_file_location("github_free_assurance", ROOT / "tools/ci/github_free_assurance.py")
assert SPEC and SPEC.loader
ASSURANCE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = ASSURANCE
SPEC.loader.exec_module(ASSURANCE)


class GitHubFreeAssuranceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.policy = ASSURANCE.load_policy()

    def test_current_policy_and_profile_expansion_are_complete(self) -> None:
        self.assertEqual(self.policy["tools"], ASSURANCE.REQUIRED_TOOLS)
        self.assertEqual(
            ASSURANCE.expand_profile(self.policy, "release-local"),
            [
                "repository-ready", "free-plan-contract", "workflow-static-analysis",
                "dependency-security", "dependency-vetting", "secret-scan", "local-sast",
                "scanner-canaries",
                "release-policy",
            ],
        )
        self.assertIn("fuzz", ASSURANCE.expand_profile(self.policy, "deep-source"))

    def test_missing_security_step_or_tool_version_is_rejected(self) -> None:
        policy = copy.deepcopy(self.policy)
        policy["profiles"]["pre-push"].remove("secret-scan")
        with self.assertRaisesRegex(ASSURANCE.AssuranceError, "pre-push"):
            ASSURANCE.validate_policy_for_test(policy)
        policy = copy.deepcopy(self.policy)
        policy["tools"]["gitleaks"] = "0.0.0"
        with self.assertRaisesRegex(ASSURANCE.AssuranceError, "tool inventory"):
            ASSURANCE.validate_policy_for_test(policy)

    def test_duplicate_policy_keys_and_profile_cycles_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            path = Path(temporary) / "policy.json"
            path.write_text('{"schema": 1, "schema": 2}', encoding="utf-8")
            with self.assertRaisesRegex(ASSURANCE.AssuranceError, "duplicate"):
                ASSURANCE.load_policy(path)
        policy = copy.deepcopy(self.policy)
        policy["profiles"]["pre-push"] = ["release-local"]
        with self.assertRaisesRegex(ASSURANCE.AssuranceError, "pre-push"):
            ASSURANCE.validate_policy_for_test(policy)

    def test_existing_hook_is_never_overwritten(self) -> None:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            hook = Path(temporary) / "pre-push"
            hook.write_text("custom hook\n", encoding="utf-8")
            with self.assertRaisesRegex(ASSURANCE.AssuranceError, "overwrite"):
                ASSURANCE.write_hook(hook)
            self.assertEqual(hook.read_text(encoding="utf-8"), "custom hook\n")

    def test_new_hook_is_executable_and_runs_the_pre_push_profile(self) -> None:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            hook = Path(temporary) / "hooks" / "pre-push"
            ASSURANCE.write_hook(hook)
            self.assertTrue(hook.exists())
            self.assertIn("cargo xtask assurance pre-push", hook.read_text(encoding="utf-8"))
            if os.name != "nt":
                self.assertNotEqual(hook.stat().st_mode & 0o100, 0)

    def test_profile_step_tool_mapping_cannot_omit_a_scanner(self) -> None:
        tools = ASSURANCE.required_tools(ASSURANCE.expand_profile(self.policy, "pre-push"))
        self.assertEqual(tools, {"actionlint", "shellcheck", "zizmor", "cargo-audit", "cargo-deny", "cargo-vet", "gitleaks", "semgrep"})

    def test_actionlint_always_uses_the_pinned_shellcheck_binary(self) -> None:
        def command(_policy: dict[str, object], name: str, *arguments: str) -> list[str]:
            return [name, *arguments]

        with mock.patch.object(ASSURANCE, "tool_command", side_effect=command), mock.patch.object(
            ASSURANCE, "run"
        ) as runner:
            ASSURANCE.run_step(self.policy, "workflow-static-analysis")
        actionlint = runner.call_args_list[0].args[0]
        self.assertEqual(actionlint[0:3], ["actionlint", "-color", "-shellcheck"])
        self.assertEqual(actionlint[3], str(ASSURANCE.tool_path(self.policy, "shellcheck")))

    def test_changed_commit_scan_is_bounded_to_the_upstream_range(self) -> None:
        with mock.patch.object(
            ASSURANCE,
            "git_stdout",
            side_effect=["origin/main", "a" * 40, "b" * 40],
        ):
            self.assertEqual(ASSURANCE.changed_commit_log_options(), f"{'a' * 40}..{'b' * 40}")

    def test_new_branch_scan_uses_the_remote_default_without_an_upstream(self) -> None:
        with mock.patch.object(
            ASSURANCE,
            "git_stdout",
            side_effect=[None, "origin/main", "a" * 40, "b" * 40],
        ):
            self.assertEqual(
                ASSURANCE.changed_commit_log_options(),
                f"{'a' * 40}..{'b' * 40}",
            )

    def test_new_branch_scan_fails_closed_without_a_remote_default(self) -> None:
        with mock.patch.object(
            ASSURANCE, "git_stdout", side_effect=[None, None]
        ), self.assertRaisesRegex(ASSURANCE.AssuranceError, "origin/HEAD"):
            ASSURANCE.changed_commit_log_options()

    def test_history_audit_entrypoint_reaches_the_pinned_full_history_scan(self) -> None:
        def command(_policy: dict[str, object], name: str, *arguments: str) -> list[str]:
            return [name, *arguments]

        # Tool execution is mocked, but CLI routing and the full-history argument
        # remain observable; separate scanner canaries exercise the real binary.
        with mock.patch.object(ASSURANCE, "load_policy", return_value=self.policy), mock.patch.object(
            ASSURANCE, "tool_command", side_effect=command
        ), mock.patch.object(ASSURANCE, "run") as runner:
            self.assertEqual(ASSURANCE.main(["audit-history-secrets"]), 0)
        command_line = runner.call_args.args[0]
        self.assertEqual(command_line[0], "gitleaks")
        self.assertIn("--log-opts=--all", command_line)
        self.assertEqual(runner.call_args.args[1], "full-history secret audit")

    def test_cargo_vet_uses_the_cargo_plugin_contract(self) -> None:
        with mock.patch.object(ASSURANCE.Path, "is_file", return_value=True):
            self.assertEqual(
                ASSURANCE.tool_command(self.policy, "cargo-vet", "init"),
                ["cargo", "vet", "init"],
            )

    def test_dependency_security_uses_the_pinned_cargo_deny_binary(self) -> None:
        def command(_policy: dict[str, object], name: str, *arguments: str) -> list[str]:
            return [name, *arguments]

        with mock.patch.object(ASSURANCE, "tool_command", side_effect=command), mock.patch.object(ASSURANCE, "run") as runner:
            ASSURANCE.run_step(self.policy, "dependency-security")
        commands = [call.args[0] for call in runner.call_args_list]
        self.assertEqual(commands[0][0], "cargo-audit")
        self.assertEqual(commands[1][0], "cargo-deny")

    def test_dependency_scanners_use_one_disposable_short_windows_cargo_home(self) -> None:
        if os.name != "nt":
            self.skipTest("the historical path exhaustion is Windows-specific")
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            deep_root = Path(temporary).joinpath(*(["nested"] * 12))
            deep_root.mkdir(parents=True)
            cargo_homes: list[Path] = []

            def command(
                _policy: dict[str, object], name: str, *arguments: str
            ) -> list[str]:
                return [name, *arguments]

            def capture_run(
                _command: list[str],
                _label: str,
                _policy: dict[str, object],
                **kwargs: object,
            ) -> None:
                environment = kwargs["extra_environment"]
                assert isinstance(environment, dict)
                cargo_home = Path(environment["CARGO_HOME"])
                cargo_homes.append(cargo_home)
                self.assertEqual(cargo_home.parent.parent, Path(ASSURANCE.ROOT.anchor))
                self.assertNotIn(ASSURANCE.cache_root(self.policy), cargo_home.parents)
                for name in ("TMP", "TEMP", "TMPDIR"):
                    self.assertEqual(Path(environment[name]), cargo_home.parent)

            # Cargo Deny clones an advisory Git repository below CARGO_HOME. A
            # deep worktree once exhausted Win32 path space only after that
            # clone began, so both scanners must share one short-lived root.
            with mock.patch.object(ASSURANCE, "ROOT", deep_root), mock.patch.object(
                ASSURANCE, "tool_command", side_effect=command
            ), mock.patch.object(ASSURANCE, "run", side_effect=capture_run):
                ASSURANCE.run_step(self.policy, "dependency-security")
                ASSURANCE.run_step(self.policy, "dependency-vetting")

            self.assertEqual(len(cargo_homes), 3)
            self.assertEqual(cargo_homes[0], cargo_homes[1])
            for cargo_home in cargo_homes:
                self.assertFalse(cargo_home.parent.exists())

    def test_deep_source_profile_fails_closed_off_linux(self) -> None:
        with mock.patch.object(ASSURANCE.platform, "system", return_value="Windows"):
            with self.assertRaisesRegex(ASSURANCE.AssuranceError, "native Linux checkout"):
                ASSURANCE.require_linux_deep_profile()
        with mock.patch.object(ASSURANCE.platform, "system", return_value="Linux"):
            ASSURANCE.require_linux_deep_profile()

    def test_repository_ready_honors_the_xtask_completion_receipt(self) -> None:
        with mock.patch.dict(os.environ, {"AUTOMEXIA_ASSURANCE_READY_DONE": "1"}):
            with mock.patch.object(ASSURANCE, "run") as runner:
                ASSURANCE.run_step(self.policy, "repository-ready")
        runner.assert_not_called()

    def test_direct_repository_ready_keeps_the_contributor_cargo_home(self) -> None:
        with mock.patch.dict(os.environ, {}, clear=False):
            os.environ.pop("AUTOMEXIA_ASSURANCE_READY_DONE", None)
            with mock.patch.object(ASSURANCE, "run") as runner:
                ASSURANCE.run_step(self.policy, "repository-ready")
        self.assertEqual(runner.call_args.args[0], ["cargo", "xtask", "ready"])
        self.assertFalse(runner.call_args.kwargs["isolate_cargo_home"])

    def test_cargo_vet_initialization_is_idempotent_but_rejects_partial_records(self) -> None:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            root = Path(temporary)
            # Exercise all three storage states: absent, complete, and partial.
            # Partial supply-chain records must never be silently regenerated.
            with mock.patch.object(ASSURANCE, "ROOT", root), mock.patch.object(
                ASSURANCE,
                "tool_command",
                side_effect=lambda _policy, name, *arguments: [name, *arguments],
            ), mock.patch.object(ASSURANCE, "run") as runner:
                ASSURANCE.initialize_vet(self.policy)
            self.assertEqual(runner.call_args_list[0].args[0], ["cargo-vet", "init"])
            self.assertEqual(runner.call_args_list[1].args[0], ["cargo-vet", "--locked"])

            records = root / "supply-chain"
            records.mkdir()
            for name in ("audits.toml", "config.toml", "imports.lock"):
                (records / name).write_text("fixture", encoding="utf-8")
            with mock.patch.object(ASSURANCE, "ROOT", root), mock.patch.object(
                ASSURANCE,
                "tool_command",
                side_effect=lambda _policy, name, *arguments: [name, *arguments],
            ), mock.patch.object(ASSURANCE, "run") as verified:
                ASSURANCE.initialize_vet(self.policy)
            self.assertEqual([call.args[0] for call in verified.call_args_list], [["cargo-vet", "--locked"]])

            (records / "imports.lock").unlink()
            with mock.patch.object(ASSURANCE, "ROOT", root):
                with self.assertRaisesRegex(ASSURANCE.AssuranceError, "incomplete"):
                    ASSURANCE.initialize_vet(self.policy)

    def test_semgrep_uses_its_native_virtual_environment_entrypoint(self) -> None:
        with mock.patch.object(ASSURANCE.Path, "is_file", return_value=True):
            command = ASSURANCE.tool_command(self.policy, "semgrep", "--version")
        self.assertEqual(command[0], str(ASSURANCE.tool_path(self.policy, "semgrep")))
        self.assertEqual(command[1:], ["--version"])
        self.assertNotIn("-m", command)

    def test_local_sast_is_bounded_to_rust_source_and_excludes_local_caches(self) -> None:
        def command(_policy: dict[str, object], name: str, *arguments: str) -> list[str]:
            return [name, *arguments]

        # Command construction is the contract under test. Mock the executable
        # lookup so a clean CI checkout proves scope without a populated tool cache.
        with mock.patch.object(ASSURANCE, "tool_command", side_effect=command), mock.patch.object(
            ASSURANCE, "run"
        ) as run:
            ASSURANCE.run_step(self.policy, "local-sast")
        command = run.call_args.args[0]
        self.assertIn("*.rs", command)
        for excluded in (".automexia-tools", ".automexia-private", "target"):
            self.assertIn(excluded, command)
        # Semgrep receives an isolated short-path environment on Windows, while
        # repository caches and generated output remain outside the scan corpus.
        temporary_environment = run.call_args.kwargs["extra_environment"]
        self.assertEqual(set(temporary_environment), {"TMP", "TEMP", "TMPDIR", "PATH"})
        self.assertEqual(
            {Path(temporary_environment[name]).parent for name in ("TMP", "TEMP", "TMPDIR")},
            {Path(ASSURANCE.ROOT.anchor) if os.name == "nt" else Path(tempfile.gettempdir())},
        )
        self.assertTrue(temporary_environment["PATH"].startswith(str(ASSURANCE.tool_path(self.policy, "semgrep").parent) + os.pathsep))

    def test_tool_environment_keeps_cargo_and_temporary_output_in_the_local_cache(self) -> None:
        environment = ASSURANCE.process_environment(self.policy)
        cache = ASSURANCE.cache_root(self.policy)
        self.assertEqual(Path(environment["CARGO_HOME"]), cache / "cargo-home")
        self.assertTrue(environment["PATH"].startswith(str(cache / "bin") + os.pathsep))
        self.assertEqual(Path(environment["PIP_CACHE_DIR"]), cache / "pip-cache")
        for name in ("TMP", "TEMP", "TMPDIR"):
            self.assertEqual(Path(environment[name]), cache / "tmp")

        with mock.patch.dict(os.environ, {"CARGO_HOME": "ambient-cargo-home"}):
            repository_environment = ASSURANCE.process_environment(
                self.policy, isolate_cargo_home=False
            )
        self.assertEqual(repository_environment["CARGO_HOME"], "ambient-cargo-home")

    def test_tool_downloads_are_checksum_pinned_and_bounded(self) -> None:
        downloads = ASSURANCE.platform_downloads(self.policy)
        self.assertEqual(set(downloads), {"actionlint", "gitleaks", "shellcheck"})
        for name, download in downloads.items():
            with self.subTest(tool=name):
                self.assertTrue(download.url.startswith("https://github.com/"))
                self.assertRegex(download.sha256, r"^[0-9a-f]{64}$")
                self.assertLessEqual(ASSURANCE.MAX_TOOL_BINARY_BYTES, ASSURANCE.MAX_TOOL_ARCHIVE_BYTES)

    def test_cargo_tool_install_uses_disposable_short_build_paths(self) -> None:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            deep_root = Path(temporary).joinpath(*(["nested"] * 12))
            deep_root.mkdir(parents=True)
            build_roots: list[Path] = []

            def install_fixture(
                command: list[str],
                _label: str,
                policy: dict[str, object],
                **kwargs: object,
            ) -> None:
                self.assertEqual(policy, self.policy)
                install_root = Path(command[command.index("--root") + 1])
                target_dir = Path(command[command.index("--target-dir") + 1])
                build_roots.append(install_root)
                self.assertEqual(target_dir.parent, install_root)
                if os.name == "nt":
                    self.assertEqual(install_root.parent, Path(ASSURANCE.ROOT.anchor))
                    self.assertNotIn(ASSURANCE.cache_root(self.policy), install_root.parents)
                else:
                    # Keep publication on one filesystem so atomic replacement
                    # cannot degrade into an unsupported cross-device move.
                    self.assertEqual(install_root.parent, ASSURANCE.cache_root(self.policy))
                self.assertEqual(
                    kwargs["extra_environment"],
                    {"CARGO_HOME": str(install_root / "cargo-home")},
                )
                binary = install_root / "bin" / ASSURANCE.executable("cargo-audit")
                binary.parent.mkdir(parents=True)
                binary.write_bytes(b"pinned-cargo-tool")

            # The historical failure occurred only after MSVC received the full
            # worktree/cache/dependency path. The fake compiler independently
            # proves that publication uses its short root and cleans it afterward.
            with mock.patch.object(ASSURANCE, "ROOT", deep_root), mock.patch.object(
                ASSURANCE, "run", side_effect=install_fixture
            ):
                ASSURANCE.install_cargo_tool(self.policy, "cargo-audit")
                installed = ASSURANCE.tool_path(self.policy, "cargo-audit")
                self.assertEqual(installed.read_bytes(), b"pinned-cargo-tool")

            self.assertEqual(len(build_roots), 1)
            self.assertFalse(build_roots[0].exists())

    def test_checksum_pinned_zip_and_tar_members_extract_to_the_isolated_cache(self) -> None:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            root = Path(temporary)
            payload = b"bounded-tool-fixture"
            archives = (
                ("tool.zip", "shellcheck.exe"),
                ("tool.tar.gz", "shellcheck-v0.11.0/shellcheck.exe"),
            )
            for archive_name, member_name in archives:
                with self.subTest(archive=archive_name):
                    archive = root / archive_name
                    if archive.suffix == ".zip":
                        with zipfile.ZipFile(archive, "w") as output:
                            output.writestr(member_name, payload)
                    else:
                        source = root / "shellcheck.exe"
                        source.write_bytes(payload)
                        with tarfile.open(archive, "w:gz") as output:
                            output.add(source, arcname=member_name)
                    download = ASSURANCE.Download(
                        archive.as_uri(),
                        hashlib.sha256(archive.read_bytes()).hexdigest(),
                        member_name,
                    )
                    with mock.patch.object(ASSURANCE, "ROOT", root):
                        ASSURANCE.download_binary(self.policy, "shellcheck", download)
                        self.assertEqual(
                            ASSURANCE.tool_path(self.policy, "shellcheck").read_bytes(),
                            payload,
                        )

    def test_actionlint_self_hosted_labels_are_exactly_declared(self) -> None:
        config = (ROOT / ".github/actionlint.yaml").read_text(encoding="utf-8")
        labels = {
            line.strip()[2:]
            for line in config.splitlines()
            if line.strip().startswith("- ")
        }
        self.assertEqual(
            labels,
            {
                "automexia-assurance",
                "automexia-benchmark",
                "automexia-elevated",
                "automexia-gpu",
                "automexia-openssh-*",
                "automexia-release",
                "automexia-wsl",
                "defender",
                "gpu",
                "wsl",
            },
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
