#!/usr/bin/env python3
"""Mutation coverage for the GitHub-Free local assurance runner."""

from __future__ import annotations

import copy
import contextlib
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
    def test_fuzz_registration_keeps_reviewed_seeds_and_unique_bounded_campaigns(self) -> None:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            with mock.patch.object(ASSURANCE, "ROOT", Path(temporary)), mock.patch.object(ASSURANCE, "require_linux_deep_profile"), mock.patch.object(ASSURANCE, "run") as run:
                # Load policy before redirecting only the generated-output root;
                # the repository's reviewed history exemptions remain authoritative.
                ASSURANCE.run_step(self.policy, "fuzz")
            commands = [call.args[0] for call in run.call_args_list]
            targets = [command[4] for command in commands]
            self.assertEqual(len(targets), len(set(targets)))
            surface = next(command for command in commands if command[4] == "semantic_surfaces")
            self.assertEqual(surface[5:7], ["fuzz/corpus/semantic_surfaces", "fuzz/seeds/semantic_surfaces"])
            self.assertIn("-max_total_time=120", surface)
            self.assertIn("-rss_limit_mb=768", surface)
            self.assertIn("-timeout=15", surface)
            self.assertTrue((Path(temporary) / "fuzz/corpus/semantic_surfaces").is_dir())

    def setUp(self) -> None:
        self.policy = ASSURANCE.load_policy()

    def test_current_policy_and_profile_expansion_are_complete(self) -> None:
        ASSURANCE.validate_readiness_runner()
        self.assertEqual(self.policy["tools"], ASSURANCE.REQUIRED_TOOLS)
        self.assertEqual(self.policy["schema"], 2)
        self.assertEqual(
            self.policy["tool_cache"], ASSURANCE.TOOL_CACHE_CONTRACT
        )
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

    def test_readiness_deadline_cleanup_and_real_process_tests_cannot_be_weakened(self) -> None:
        source = ASSURANCE.READINESS_RUNNER_PATH.read_text(encoding="utf-8")
        # Each mutation removes one independent part of the failure mode seen in
        # the protected push: deadline, memory bound, descendant ownership, or
        # the real child-process oracle. The production checker must reject all.
        mutations = {
            "deadline relaxation": (
                "Duration::from_secs(30 * 60)",
                "Duration::from_secs(90 * 60)",
            ),
            "output ceiling removal": (
                "SUMMARIZED_CARGO_STDOUT_LIMIT",
                "UNBOUNDED_CARGO_STDOUT",
            ),
            "unix descendant cleanup removal": (
                "command.wrap(ProcessGroup::leader());",
                "",
            ),
            "windows descendant cleanup removal": (
                "command.wrap(JobObject);",
                "",
            ),
            "deadline test removal": (
                "summarized_command_deadline_terminates_the_owned_child",
                "removed_deadline_test",
            ),
            "zero-bound test removal": (
                "summarized_command_rejects_zero_bounds_before_spawn",
                "removed_zero_bound_test",
            ),
            "deadline diagnostics removal": (
                'String::from_utf8_lossy(&error.stdout).contains("bounded-child-waiting")',
                "true",
            ),
        }
        for name, (needle, replacement) in mutations.items():
            with self.subTest(name=name):
                self.assertIn(needle, source)
                mutated = source.replace(needle, replacement, 1)
                with self.assertRaises(ASSURANCE.AssuranceError):
                    ASSURANCE.validate_readiness_runner_source(mutated)

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

    def test_new_hook_is_executable_but_does_not_block_push(self) -> None:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            hook = Path(temporary) / "hooks" / "pre-push"
            ASSURANCE.write_hook(hook)
            self.assertTrue(hook.exists())
            source = hook.read_text(encoding="utf-8")
            self.assertIn("# cargo xtask assurance pre-push", source)
            self.assertNotIn("\ncargo xtask assurance pre-push\n", source)
            self.assertTrue(source.endswith("exit 0\n"))
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
        # Isolate the lease because this mutation suite also runs inside the
        # release profile's already-held process-wide assurance lease.
        with mock.patch.object(ASSURANCE, "load_policy", return_value=self.policy), mock.patch.object(
            ASSURANCE, "tool_command", side_effect=command
        ), mock.patch.object(ASSURANCE, "run") as runner, mock.patch.object(
            ASSURANCE.dev_cache,
            "cache_lease",
            return_value=contextlib.nullcontext(),
        ):
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

    def test_dependency_scanners_share_the_managed_short_cache_environment(self) -> None:
        if os.name != "nt":
            self.skipTest("the historical path exhaustion is Windows-specific")
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            deep_root = Path(temporary).joinpath(*(["nested"] * 12))
            deep_root.mkdir(parents=True)
            cache_override = Path(temporary) / "managed-cache"
            cargo_homes: list[Path] = []
            temporary_roots: list[Path] = []

            def command(
                _policy: dict[str, object], name: str, *arguments: str
            ) -> list[str]:
                return [name, *arguments]

            def capture_subprocess(
                _command: list[str], **kwargs: object
            ) -> mock.Mock:
                environment = kwargs["env"]
                assert isinstance(environment, dict)
                cargo_home = Path(environment["CARGO_HOME"])
                cargo_homes.append(cargo_home)
                roots = {Path(environment[name]) for name in ("TMP", "TEMP", "TMPDIR")}
                self.assertEqual(len(roots), 1)
                temporary_roots.extend(roots)
                return mock.Mock(returncode=0)

            expected_cargo_home = cache_override / "mutable" / "runtime" / "cargo-home"
            expected_temporary_parent = cache_override / "temporary"

            # The real runner must construct the environment. Mocking run()
            # here would repeat the historical blind spot that missed a deep
            # Win32 path only after Cargo Deny cloned its advisory repository.
            with mock.patch.dict(
                os.environ, {"AUTOMEXIA_DEV_CACHE_DIR": str(cache_override)}
            ), mock.patch.object(ASSURANCE, "ROOT", deep_root), mock.patch.object(
                ASSURANCE, "tool_command", side_effect=command
            ), mock.patch.object(
                ASSURANCE.subprocess, "run", side_effect=capture_subprocess
            ):
                ASSURANCE._PROCESS_TEMPORARY = None
                try:
                    ASSURANCE.run_step(self.policy, "dependency-security")
                    ASSURANCE.run_step(self.policy, "dependency-vetting")
                finally:
                    ASSURANCE._PROCESS_TEMPORARY = None

            self.assertEqual(len(cargo_homes), 3)
            self.assertEqual(set(cargo_homes), {expected_cargo_home})
            self.assertEqual(len(temporary_roots), 3)
            for temporary_root in temporary_roots:
                self.assertEqual(temporary_root.parent, expected_temporary_parent)
                self.assertNotIn(deep_root, temporary_root.parents)
            self.assertNotIn(deep_root, expected_cargo_home.parents)

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

    def test_semgrep_uses_its_relocatable_virtual_environment_runtime(self) -> None:
        with mock.patch.object(ASSURANCE.Path, "is_file", return_value=True):
            command = ASSURANCE.tool_command(self.policy, "semgrep", "--version")
        self.assertEqual(
            command[0], str(ASSURANCE.semgrep_python_path(self.policy))
        )
        self.assertEqual(
            command[1:],
            [str(ASSURANCE.semgrep_launcher_path()), "--legacy", "--version"],
        )
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
            len(
                {
                    Path(temporary_environment[name])
                    for name in ("TMP", "TEMP", "TMPDIR")
                }
            ),
            1,
        )
        self.assertEqual(
            Path(temporary_environment["TMP"]).parent,
            ASSURANCE.dev_cache.temporary_root(root=ASSURANCE.ROOT),
        )
        self.assertTrue(temporary_environment["PATH"].startswith(str(ASSURANCE.tool_path(self.policy, "semgrep").parent) + os.pathsep))

    def test_tool_environment_separates_immutable_tools_from_mutable_state(self) -> None:
        environment = ASSURANCE.process_environment(self.policy)
        cache = ASSURANCE.cache_root(self.policy)
        self.assertEqual(
            Path(environment["CARGO_HOME"]),
            ASSURANCE.dev_cache.runtime_root(root=ASSURANCE.ROOT) / "cargo-home",
        )
        self.assertTrue(environment["PATH"].startswith(str(cache / "bin") + os.pathsep))
        self.assertEqual(
            Path(environment["PIP_CACHE_DIR"]),
            ASSURANCE.dev_cache.downloads_root(root=ASSURANCE.ROOT) / "pip",
        )
        for name in ("TMP", "TEMP", "TMPDIR"):
            self.assertEqual(
                Path(environment[name]),
                ASSURANCE.process_temporary_directory(),
            )

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

    def test_install_tools_uses_disposable_managed_staging(self) -> None:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            deep_root = Path(temporary).joinpath(*(["nested"] * 12))
            deep_root.mkdir(parents=True)
            cache_override = Path(temporary) / "managed-cache"
            install_roots: list[Path] = []
            build_roots: list[Path] = []
            commands: list[list[str]] = []

            def capture_run(
                command: list[str],
                _label: str,
                policy: dict[str, object],
                **kwargs: object,
            ) -> None:
                self.assertEqual(policy, self.policy)
                commands.append(command)
                if command[:2] == ["cargo", "install"]:
                    install_roots.append(Path(command[command.index("--root") + 1]))
                    build_roots.append(Path(command[command.index("--target-dir") + 1]))
                self.assertIn("cache", kwargs)

            # Mock only external producers and final publication. The production
            # installer still chooses both temporary paths and must release them
            # after building one complete toolset.
            with mock.patch.dict(
                os.environ, {"AUTOMEXIA_DEV_CACHE_DIR": str(cache_override)}
            ), mock.patch.object(ASSURANCE, "ROOT", deep_root), mock.patch.object(
                ASSURANCE, "toolset_is_valid", side_effect=[False, False, True]
            ), mock.patch.object(
                ASSURANCE, "quarantine_invalid_toolset"
            ), mock.patch.object(
                ASSURANCE.dev_cache,
                "cache_lease",
                return_value=contextlib.nullcontext(),
            ), mock.patch.object(
                ASSURANCE, "platform_downloads", return_value={}
            ), mock.patch.object(
                ASSURANCE,
                "tool_command",
                side_effect=lambda _policy, name, *arguments, **_kwargs: [
                    name,
                    *arguments,
                ],
            ), mock.patch.object(
                ASSURANCE, "run", side_effect=capture_run
            ), mock.patch.object(
                ASSURANCE, "write_toolset_manifest"
            ), mock.patch.object(ASSURANCE, "publish_toolset") as publish:
                ASSURANCE.install_tools(self.policy)

            staging_parent = cache_override / "staging"
            expected_final = (
                cache_override
                / "toolsets"
                / ASSURANCE.dev_cache.toolset_id(self.policy["tools"])
            )
            self.assertEqual(len(install_roots), 4)
            self.assertEqual(len(build_roots), 4)
            self.assertEqual(len(set(install_roots)), 1)
            self.assertEqual(len(set(build_roots)), 1)
            install_root = install_roots[0]
            build_root = build_roots[0]
            self.assertEqual(install_root.parent, staging_parent)
            self.assertEqual(build_root.parent, staging_parent)
            self.assertNotIn(deep_root, install_root.parents)
            self.assertNotIn(deep_root, build_root.parents)
            self.assertFalse(install_root.exists())
            self.assertFalse(build_root.exists())
            self.assertTrue(any(command[:2] == ["cargo", "install"] for command in commands))
            publish.assert_called_once()
            self.assertEqual(publish.call_args.args[0], self.policy)
            self.assertEqual(publish.call_args.args[1], install_root)
            self.assertEqual(publish.call_args.args[2], expected_final)

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
                    cache = root / "toolset"
                    with mock.patch.object(ASSURANCE, "ROOT", root):
                        ASSURANCE.download_binary(
                            self.policy,
                            "shellcheck",
                            download,
                            cache=cache,
                        )
                        self.assertEqual(
                            ASSURANCE.tool_path(
                                self.policy, "shellcheck", cache=cache
                            ).read_bytes(),
                            payload,
                        )
                        self.assertFalse((cache.parent / ".downloads").exists())

    def test_toolset_manifest_detects_entrypoint_and_dependency_mutations(self) -> None:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            cache = Path(temporary) / "toolset"
            (cache / "bin").mkdir(parents=True)
            binary = cache / "bin" / "tool"
            binary.write_bytes(b"verified")
            ASSURANCE.write_toolset_manifest(self.policy, cache)
            self.assertTrue(ASSURANCE.toolset_is_valid(self.policy, cache))
            binary.write_bytes(b"changed")
            self.assertFalse(ASSURANCE.toolset_is_valid(self.policy, cache))
            binary.write_bytes(b"verified")
            manifest = cache / ASSURANCE.TOOLSET_MANIFEST_NAME
            value = json.loads(manifest.read_text(encoding="utf-8"))
            value["descriptor"]["tools"]["semgrep"] = "0.0.0"
            manifest.write_text(json.dumps(value), encoding="utf-8")
            self.assertFalse(ASSURANCE.toolset_is_valid(self.policy, cache))
            manifest.unlink()
            with mock.patch.object(ASSURANCE, "MAX_TOOLSET_FILES", 0):
                with self.assertRaisesRegex(ASSURANCE.AssuranceError, "file ceiling"):
                    ASSURANCE._toolset_tree_digest(cache)

    def test_concurrent_toolset_publication_reuses_only_an_exact_winner(self) -> None:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            root = Path(temporary)
            winner = root / "winner"
            challenger = root / "challenger"
            final = root / "toolsets" / "current"
            for cache in (winner, challenger):
                (cache / "bin").mkdir(parents=True)
                (cache / "bin" / "tool").write_bytes(b"same")
                ASSURANCE.write_toolset_manifest(self.policy, cache)
            ASSURANCE.publish_toolset(self.policy, winner, final)
            ASSURANCE.publish_toolset(self.policy, challenger, final)
            self.assertTrue(ASSURANCE.toolset_is_valid(self.policy, final))

            corrupt = root / "corrupt"
            corrupt.mkdir()
            with mock.patch.object(
                ASSURANCE.os, "replace", side_effect=OSError("exists")
            ), self.assertRaisesRegex(ASSURANCE.AssuranceError, "publication"):
                ASSURANCE.publish_toolset(self.policy, corrupt, root / "missing")

    def test_incomplete_content_addressed_toolset_fails_closed(self) -> None:
        with mock.patch.object(
            ASSURANCE, "load_policy", return_value=self.policy
        ), mock.patch.object(
            ASSURANCE, "toolset_is_valid", return_value=False
        ), mock.patch.object(
            ASSURANCE.dev_cache,
            "cache_lease",
            return_value=contextlib.nullcontext(),
        ):
            self.assertEqual(ASSURANCE.main(["pre-push"]), 1)

    def test_invalid_current_toolset_is_quarantined_only_inside_staging(self) -> None:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            cache_root = Path(temporary) / "cache"
            toolsets = cache_root / "toolsets"
            staging = cache_root / "staging"
            final = toolsets / ASSURANCE.dev_cache.toolset_id(self.policy["tools"])
            final.mkdir(parents=True)
            (final / "partial").write_bytes(b"generated")
            with mock.patch.object(
                ASSURANCE.dev_cache, "cache_root", return_value=cache_root
            ):
                quarantine = ASSURANCE.quarantine_invalid_toolset(self.policy, final)
            self.assertIsNotNone(quarantine)
            assert quarantine is not None
            self.assertEqual(quarantine.parent, staging)
            self.assertFalse(final.exists())
            self.assertEqual((quarantine / "partial").read_bytes(), b"generated")

            outside = Path(temporary) / "outside"
            outside.mkdir()
            with mock.patch.object(
                ASSURANCE.dev_cache, "cache_root", return_value=cache_root
            ), self.assertRaisesRegex(ASSURANCE.AssuranceError, "unexpected"):
                ASSURANCE.quarantine_invalid_toolset(self.policy, outside)

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
