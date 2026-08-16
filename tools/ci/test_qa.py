#!/usr/bin/env python3
"""Regression tests for the bounded Phase 0 QA evidence runner."""

from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import pathlib
import sys
import tempfile
import time
import unittest
import zipfile

MODULE_PATH = pathlib.Path(__file__).with_name("qa.py")
SPEC = importlib.util.spec_from_file_location("automexia_qa", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load tools/ci/qa.py")
QA = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(QA)


class QaRunnerTests(unittest.TestCase):
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
            '"automexia-devops"',
            '"quick_actions"',
        )
        for contract in expected:
            self.assertIn(contract, source)
        self.assertEqual(QA.STEP_TIMEOUT_SECONDS["benchmark-image"], 7200)
        self.assertEqual(QA.STEP_TIMEOUT_SECONDS["benchmark-pty"], 7200)
        self.assertEqual(
            QA.STEP_TIMEOUT_SECONDS["benchmark-ssh-inventory"], 7200
        )
        self.assertEqual(
            QA.STEP_TIMEOUT_SECONDS["benchmark-quick-actions"], 7200
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
