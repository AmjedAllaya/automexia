#!/usr/bin/env python3
"""Regression tests for the bounded Phase 0 QA evidence runner."""

from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import pathlib
import subprocess
import sys
import tempfile
import time
import threading
import unittest
from unittest import mock
import zipfile

MODULE_PATH = pathlib.Path(__file__).with_name("qa.py")
SPEC = importlib.util.spec_from_file_location("automexia_qa", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load tools/ci/qa.py")
QA = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(QA)


class QaRunnerTests(unittest.TestCase):
    def test_cli_artifact_announcements_never_include_the_checkout_root(self) -> None:
        # Stub only external execution/host observations. The real CLI still
        # writes and packages its reports, so this exercises the leaking path.
        for status in ("pass", "fail"):
            with self.subTest(status=status), tempfile.TemporaryDirectory() as temporary:
                root = pathlib.Path(temporary)
                junit = root / "target" / "nextest" / "ci" / "junit.xml"
                junit.parent.mkdir(parents=True)
                junit.write_text("<testsuites/>", encoding="utf-8")
                output = io.StringIO()
                observed_commands = {}

                def external_step(_run_dir, _index, name, _command, **_kwargs):
                    observed_commands[name] = _command
                    return {"name": name, "command": "fixture command", "status": status if name == "component-host" else "pass",
                            "required": True, "duration_seconds": 0}

                with contextlib.ExitStack() as stack:
                    stack.enter_context(mock.patch.object(QA, "ROOT", root))
                    stack.enter_context(mock.patch.object(sys, "argv", ["qa.py", "--full", "--bundle"]))
                    stack.enter_context(mock.patch.dict(QA.os.environ, {"AUTOMEXIA_QA_RUN_LABEL": "fixture-run"}, clear=True))
                    stack.enter_context(mock.patch.object(QA, "git_value", return_value="a" * 40))
                    stack.enter_context(mock.patch.object(QA, "dirty_fingerprint", return_value="b" * 64))
                    stack.enter_context(mock.patch.object(QA, "collect_host_manifest", return_value={}))
                    stack.enter_context(mock.patch.object(QA, "safe_version", return_value="fixture"))
                    stack.enter_context(mock.patch.object(QA, "run_step", side_effect=external_step))
                    stack.enter_context(contextlib.redirect_stdout(output))
                    result = QA.main()

                self.assertEqual(result, 0 if status == "pass" else 1)
                self.assertEqual(observed_commands["component-host"],
                                 ["cargo", "nextest", "run", "-p", "automexia-ecosystem-runtime", "--all-features", "--locked", "--profile", "default"])
                summary = json.loads((root / "target/qa/fixture-run/summary.json").read_text(encoding="utf-8"))
                self.assertEqual(summary["required_failures"], [] if status == "pass" else ["component-host"])
                self.assertEqual(QA.STEP_TIMEOUT_SECONDS["component-host"], 900)
                self.assertTrue("Evidence report: target/qa/fixture-run/report.html" in output.getvalue(),
                                "report announcement must use its repository-relative logical location")
                self.assertTrue("Evidence bundle: target/qa/fixture-run.zip" in output.getvalue(),
                                "bundle announcement must use its repository-relative logical location")
                self.assertFalse(str(root) in output.getvalue(), "local root reached CLI output")
                self.assertTrue((root / "target/qa/fixture-run/report.html").is_file())
                self.assertTrue((root / "target/qa/fixture-run.zip").is_file())

    def test_dirty_fingerprint_changes_when_only_existing_dirty_content_changes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            (root / "fixture.txt").write_text("first", encoding="utf-8")
            with mock.patch.object(QA, "ROOT", root), mock.patch.object(QA, "source_status_bytes", return_value=b" M fixture.txt\0"):
                before = QA.dirty_fingerprint()
                (root / "fixture.txt").write_text("other", encoding="utf-8")
                after = QA.dirty_fingerprint()
            self.assertNotEqual(before, after, "status-only hashes cannot identify tested source")

    def test_source_content_identity_covers_untracked_rename_delete_and_unicode(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            name = "fixture space-界.txt"
            (root / name).write_bytes(b"first")
            raw = b"R  " + name.encode() + b"\0old.txt\0?? untracked.txt\0"
            first = QA.fingerprint_status_contents(raw, root)
            (root / "untracked.txt").write_bytes(b"content")
            second = QA.fingerprint_status_contents(raw, root)
            self.assertNotEqual(first, second)
            (root / name).unlink()
            self.assertNotEqual(second, QA.fingerprint_status_contents(raw, root))
            self.assertEqual(QA.fingerprint_status_contents(b"", root), QA.fingerprint_status_contents(b"", root))

    def test_real_git_inventory_and_same_status_content_edits_have_distinct_identities(self) -> None:
        # This disposable empty repository has no commit, hooks or remote and
        # never stages or changes the contributor's actual working tree.
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            subprocess.run(["git", "init", "--quiet", "--template=", str(root)], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            fixture = root / "fixture space-界.txt"
            fixture.write_bytes(b"first")
            with mock.patch.object(QA, "ROOT", root):
                before = QA.dirty_fingerprint()
                raw_before = QA.source_status_bytes()
                fixture.write_bytes(b"other")
                after = QA.dirty_fingerprint()
                self.assertEqual(raw_before, QA.source_status_bytes())
                self.assertNotEqual(before, "unavailable")
                self.assertNotEqual(after, "unavailable")
                self.assertNotEqual(before, after)

    def test_source_content_identity_fails_closed_on_limits_and_unsafe_inventory(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            (root / "fixture").write_bytes(b"1234")
            for raw in (b"bad", b"?? ../outside\0", b"?? /outside\0", b"?? D:/outside\0", b"?? .git/config\0", b"R  fixture\0"):
                with self.subTest(raw=raw), self.assertRaises((ValueError, StopIteration)):
                    QA.fingerprint_status_contents(raw, root)
            for limit, value in (("MAX_SOURCE_FILE_BYTES", 3), ("MAX_SOURCE_TOTAL_BYTES", 3), ("MAX_SOURCE_FILES", 0), ("MAX_SOURCE_STATUS_BYTES", 3)):
                with mock.patch.object(QA, limit, value), self.assertRaises(ValueError):
                    QA.fingerprint_status_contents(b" M fixture\0", root)
            for limit in ("MAX_SOURCE_FILE_BYTES", "MAX_SOURCE_TOTAL_BYTES"):
                with mock.patch.object(QA, limit, 4):
                    self.assertEqual(len(QA.fingerprint_status_contents(b" M fixture\0", root)), 64)

    def test_source_identity_rejects_changed_or_unavailable_evidence(self) -> None:
        valid = ("a" * 40, "b" * 64)
        self.assertTrue(QA.source_identity_stable(valid, valid))
        for other in (("c" * 40, valid[1]), (valid[0], "c" * 64), ("unavailable", valid[1]), (valid[0], "unavailable"), ("", "")):
            self.assertFalse(QA.source_identity_stable(valid, other))
            if "unavailable" in other or other == ("", ""):
                self.assertFalse(QA.source_identity_stable(other, other))
        with mock.patch.object(QA, "source_status_bytes", return_value=None):
            self.assertEqual(QA.dirty_fingerprint(), "unavailable")
        invalid_commit = ("a" * 41, "b" * 64)
        self.assertFalse(QA.source_identity_stable(invalid_commit, invalid_commit))

    def test_source_status_capture_has_real_process_byte_and_deadline_bounds(self) -> None:
        spawn = subprocess.Popen
        children = []
        def fixture_process(_command, **kwargs):
            if _command[0] != "git":
                return spawn(_command, **kwargs)
            child = spawn([sys.executable, "-c", code], **kwargs)
            children.append(child)
            return child
        for code, expected, should_timeout in (
            ("import sys;sys.stdout.buffer.write(b'?? fixture\\0')", b"?? fixture\0", False),
            ("import sys;sys.stdout.buffer.write(b'x'*65536)", None, False),
            ("import time;time.sleep(30)", None, True),
        ):
            # Mock only Git's executable output, not pipe ownership, collection,
            # process termination or the resulting byte/deadline oracle.
            with (
                self.subTest(expected=expected),
                mock.patch.object(QA.subprocess, "Popen", side_effect=fixture_process),
                mock.patch.object(QA, "MAX_SOURCE_STATUS_BYTES", 1024),
                mock.patch.object(QA, "SOURCE_STATUS_TIMEOUT_SECONDS", 0.5 if should_timeout else 10),
                mock.patch.object(QA, "terminate_process_tree", wraps=QA.terminate_process_tree) as cleanup,
            ):
                self.assertEqual(QA.source_status_bytes(), expected)
                self.assertEqual(cleanup.called, should_timeout)
            self.assertIsNotNone(children[-1].poll())
            self.assertFalse(any(thread.name == "automexia-qa-source-status" for thread in threading.enumerate()))

    def test_qa_main_fails_the_report_when_source_changes_even_if_all_commands_pass(self) -> None:
        for final, expected in (("b" * 64, 0), ("c" * 64, 1), ("unavailable", 1)):
            with self.subTest(final=final), tempfile.TemporaryDirectory() as temporary:
                root = pathlib.Path(temporary)
                def successful_step(_directory, _index, name, *_args, **_kwargs):
                    return {"name": name, "status": "pass", "required": True}
                with (
                    mock.patch.object(QA, "ROOT", root),
                    mock.patch.object(sys, "argv", ["qa.py", "--full"]),
                    mock.patch.dict(QA.os.environ, {"AUTOMEXIA_QA_RUN_LABEL": "identity-fixture"}, clear=True),
                    mock.patch.object(QA, "run_step", side_effect=successful_step),
                    mock.patch.object(QA, "collect_junit", return_value={"name": "junit", "status": "pass", "required": True}),
                    mock.patch.object(QA, "dirty_fingerprint", side_effect=["b" * 64, final]),
                    mock.patch.object(QA, "git_value", return_value="a" * 40),
                    mock.patch.object(QA, "safe_version", return_value="fixture-version"),
                    mock.patch.object(QA, "collect_host_manifest", return_value={}),
                    contextlib.redirect_stdout(io.StringIO()),
                ):
                    self.assertEqual(QA.main(), expected)
                report = json.loads((root / "target/qa/identity-fixture/summary.json").read_text())
                self.assertEqual(report["status"], "pass" if expected == 0 else "fail")
                self.assertEqual(report["required_failures"], [] if expected == 0 else ["source-identity-stable"])

    @staticmethod
    def run_step_quiet(*args, **kwargs):
        with contextlib.redirect_stdout(io.StringIO()):
            with contextlib.redirect_stderr(io.StringIO()):
                return QA.run_step(*args, **kwargs)

    def test_timeout_kills_the_complete_process_tree(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            run_dir = pathlib.Path(temporary)
            sentinel = run_dir / "descendant-survived.txt"
            child_code = (
                "import pathlib,time;"
                "time.sleep(1);"
                f"pathlib.Path({str(sentinel)!r}).write_text('survived')"
            )
            parent_code = (
                "import subprocess,time;"
                f"subprocess.Popen([{sys.executable!r}, '-c', {child_code!r}]);"
                "print('started', flush=True);"
                "time.sleep(30)"
            )
            started = time.monotonic()
            result = self.run_step_quiet(
                run_dir,
                1,
                "timeout-contract",
                [sys.executable, "-c", parent_code],
                timeout_seconds=0.25,
            )
            elapsed = time.monotonic() - started
            time.sleep(1.25)
            self.assertEqual(result["status"], "fail")
            self.assertTrue(result["timed_out"])
            self.assertLess(elapsed, 15)
            self.assertIn("TimeoutExpired", str(result["error"]))
            self.assertFalse(sentinel.exists())

    def test_local_prefix_redaction_handles_case_and_escaped_spellings(self) -> None:
        # Failed assertion messages can lowercase an escaped traceback. Keep
        # workspace labels more specific than the enclosing home prefix.
        with (
            mock.patch.object(QA, "ROOT", pathlib.PureWindowsPath(r"Q:\Fixture\Work[1]")),
            mock.patch.object(QA.pathlib.Path, "home", return_value=pathlib.PureWindowsPath(r"Q:\Fixture")),
        ):
            redact = QA.make_redactor()
        for prefix, label in ((r"Q:\Fixture\Work[1]", "<WORKSPACE>"), (r"Q:\Fixture", "<HOME>")):
            for separator in ("\\", "/", "\\\\", "\\" * 4, "\\" * 8):
                spelled = prefix.replace("\\", separator)
                suffix = separator + "tools" + separator + "Case.py"
                for transform in (str, str.lower, str.upper, str.swapcase):
                    with self.subTest(label=label, separator=separator, case=transform.__name__):
                        self.assertEqual(redact(transform(spelled) + suffix), label + suffix)
        self.assertEqual(redact(r"Q:\Fixture\Work1\file"), r"<HOME>\Work1\file")
        self.assertEqual(redact(r"Q:\Unrelated\Case.py"), r"Q:\Unrelated\Case.py")

    def test_failed_subprocess_log_redacts_case_folded_tracebacks_without_hiding_failure(self) -> None:
        with (
            mock.patch.object(QA, "ROOT", pathlib.PureWindowsPath(r"Q:\Fixture\Work[1]")),
            mock.patch.object(QA.pathlib.Path, "home", return_value=pathlib.PureWindowsPath(r"Q:\Fixture")),
        ):
            fixture_redact = QA.make_redactor()
        original_redact = QA.REDACT
        message = r"AssertionError: q:\\fixture\\work[1]\\tools\\check.py"
        code = f"import sys; print({message!r}); sys.exit(7)"
        with tempfile.TemporaryDirectory() as temporary, mock.patch.object(
            QA, "REDACT", lambda value: original_redact(fixture_redact(value))
        ):
            run_dir = pathlib.Path(temporary)
            result = self.run_step_quiet(
                run_dir, 1, "failed-redaction-contract", [sys.executable, "-c", code], timeout_seconds=30
            )
            payload = (run_dir / str(result["log"])).read_text(encoding="utf-8")
        self.assertEqual(result["status"], "fail")
        self.assertEqual(result["return_code"], 7)
        self.assertEqual(payload, r"AssertionError: <WORKSPACE>\\tools\\check.py" + "\n")
        self.assertNotIn("q:", str(result["command"]))

    def test_logs_are_redacted_and_exactly_bounded(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            run_dir = pathlib.Path(temporary)
            code = (
                "import sys;"
                f"print('authorization: Bearer phase-zero-secret', {str(QA.ROOT)!r});"
                "line='x'*4090+'\\n';"
                f"[sys.stdout.write(line) for _ in range({QA.MAX_LOG_BYTES // 4090 + 8})]"
            )
            result = self.run_step_quiet(
                run_dir,
                1,
                "bounded-log-contract",
                [sys.executable, "-c", code],
                timeout_seconds=30,
            )
            log_path = run_dir / str(result["log"])
            payload = log_path.read_text(encoding="utf-8")
            self.assertEqual(result["status"], "pass")
            self.assertTrue(result["log_truncated"])
            self.assertLessEqual(log_path.stat().st_size, QA.MAX_LOG_BYTES)
            self.assertIn("<REDACTED>", payload)
            self.assertIn("<WORKSPACE>", payload)
            self.assertNotIn("phase-zero-secret", payload)
            self.assertNotIn(str(QA.ROOT), str(result["command"]))
            self.assertNotIn(str(QA.ROOT).replace("\\", "\\\\"), str(result["command"]))
            self.assertIn("log truncated at 2 MiB", payload)

    def test_controlled_run_label_is_bounded_and_portable(self) -> None:
        source = MODULE_PATH.read_text(encoding="utf-8")
        self.assertIn("AUTOMEXIA_QA_RUN_LABEL", source)
        self.assertIn("[A-Za-z0-9][A-Za-z0-9._-]*", source)
        self.assertIn("len(requested_run_id) > 96", source)

    def test_s1_assurance_is_always_tested_and_controlled_evidence_is_optional(self) -> None:
        source = MODULE_PATH.read_text(encoding="utf-8")
        self.assertIn('"s1-assurance-policy"', source)
        self.assertIn('"s1-assurance-mutations"', source)
        self.assertIn('"AUTOMEXIA_QA_S1_EVIDENCE"', source)
        self.assertIn('"--require-complete"', source)
        self.assertIn('"s1-release-assurance-evidence"', source)

    def test_host_manifest_is_allowlisted_and_path_free(self) -> None:
        manifest = QA.collect_host_manifest()
        expected = {
            "os",
            "release",
            "architecture",
            "python",
            "shells",
            "wsl",
            "display",
            "dpi",
            "gpu",
            "renderer_backend",
            "power_profile",
        }
        self.assertEqual(set(manifest), expected)
        encoded = json.dumps(manifest)
        self.assertNotIn(str(QA.ROOT), encoded)
        self.assertNotIn(str(pathlib.Path.home()), encoded)

    def test_portable_bundle_excludes_private_artifacts(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            run_dir = root / "run"
            run_dir.mkdir()
            (run_dir / "summary.json").write_text("{}\n", encoding="utf-8")
            (run_dir / "terminal.png").write_bytes(b"private pixels")
            (run_dir / "trace.etl").write_bytes(b"private trace")
            (run_dir / "lcov.info").write_text("SF:private/path\n", encoding="utf-8")
            bundle = root / "evidence.zip"
            QA.build_bundle(run_dir, bundle)
            with zipfile.ZipFile(bundle) as archive:
                names = archive.namelist()
                self.assertTrue(any(name.endswith("summary.json") for name in names))
                self.assertTrue(
                    any(name.endswith("bundle-manifest.json") for name in names)
                )
                self.assertFalse(any(name.endswith(".png") for name in names))
                self.assertFalse(any(name.endswith(".etl") for name in names))
                self.assertFalse(any(name.endswith(".info") for name in names))
                manifest_name = next(
                    name for name in names if name.endswith("bundle-manifest.json")
                )
                manifest = json.loads(archive.read(manifest_name))
                self.assertEqual(len(manifest["excluded"]), 3)

    def test_controlled_benchmark_matrix_covers_every_declared_target(self) -> None:
        source = MODULE_PATH.read_text(encoding="utf-8")
        expected = (
            '"benchmark-app"',
            '"benchmark-image"',
            '"image_preview"',
            '"benchmark-vt"',
            '"benchmark-channel"',
            '"benchmark-pty"',
            '"teletypewriter"',
            '"pty_io"',
            '"benchmark-ssh-inventory"',
            '"automexia-devops-ssh"',
            '"openssh_inventory"',
            '"benchmark-quick-actions"',
            '"automexia-command-productivity"',
            '"quick_actions"',
            '"benchmark-connection-planning"',
            '"automexia-connectivity"',
            '"connection_planning"',
            '"benchmark-quick-action-store"',
            '"quick_action_store"',
        )
        for contract in expected:
            self.assertIn(contract, source)
        self.assertEqual(QA.STEP_TIMEOUT_SECONDS["benchmark-image"], 7200)
        self.assertEqual(QA.STEP_TIMEOUT_SECONDS["github-free-assurance-policy"], 120)
        self.assertEqual(QA.STEP_TIMEOUT_SECONDS["python-contract-mutations"], 600)
        self.assertEqual(QA.STEP_TIMEOUT_SECONDS["github-free-assurance-mutations"], 120)
        self.assertEqual(QA.STEP_TIMEOUT_SECONDS["repository-walker-cache-scope"], 120)
        self.assertEqual(QA.STEP_TIMEOUT_SECONDS["rustsec-exception-policy"], 120)

    def test_benchmark_compiler_target_is_removed_after_success_and_failure(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            run_dir = pathlib.Path(temporary)
            target = run_dir / "benchmark-target"

            def complete(*_args, **_kwargs):
                target.mkdir()
                (target / "large-generated-artifact").write_bytes(b"generated")
                return {"status": "pass"}

            with mock.patch.object(QA, "run_step", side_effect=complete):
                results, next_index = QA.run_benchmark_matrix(
                    run_dir, 7, (("benchmark", ["ignored"]),)
                )
            self.assertEqual(results, [{"status": "pass"}])
            self.assertEqual(next_index, 8)
            self.assertFalse(target.exists())

            def fail(*_args, **_kwargs):
                target.mkdir()
                (target / "partial-artifact").write_bytes(b"generated")
                raise RuntimeError("simulated benchmark failure")

            with mock.patch.object(QA, "run_step", side_effect=fail), self.assertRaisesRegex(
                RuntimeError, "simulated benchmark failure"
            ):
                QA.run_benchmark_matrix(
                    run_dir, 9, (("benchmark", ["ignored"]),)
                )
            self.assertFalse(target.exists())
        self.assertEqual(QA.STEP_TIMEOUT_SECONDS["benchmark-pty"], 7200)
        self.assertEqual(
            QA.STEP_TIMEOUT_SECONDS["benchmark-ssh-inventory"], 7200
        )
        self.assertEqual(
            QA.STEP_TIMEOUT_SECONDS["benchmark-quick-actions"], 7200
        )
        self.assertEqual(
            QA.STEP_TIMEOUT_SECONDS["benchmark-connection-planning"], 7200
        )
        self.assertEqual(
            QA.STEP_TIMEOUT_SECONDS["benchmark-quick-action-store"], 7200
        )

    def test_local_qa_discovers_the_complete_python_contract_suite(self) -> None:
        source = MODULE_PATH.read_text(encoding="utf-8")
        for token in (
            '"python-contract-mutations"',
            '"unittest"',
            '"discover"',
            '"tools/ci"',
            '"test_*.py"',
        ):
            self.assertIn(token, source)


if __name__ == "__main__":
    unittest.main(verbosity=2)
