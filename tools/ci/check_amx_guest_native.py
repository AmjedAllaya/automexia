"""Live Linux lease and installed-tool checks. Never opens a browser or installs tools."""
import argparse
import json
import os
from pathlib import Path
import select
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from urllib.parse import quote

ROOT = Path(__file__).resolve().parents[2]
BRIDGE = ROOT / "shell-integration/amx-tool-bridge.py"
BINARY = None


class GuestNativeTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT / "target/qa", prefix="amx-guest-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / ".git").mkdir()
        self.environment = dict(os.environ, HOME=str(self.root), PATH="/usr/bin:/bin", LC_ALL="C.UTF-8")
        self.environment["AUTOMEXIA_CONFIG_HOME"] = str(self.root)
        self.environment["WSLENV"] = "AUTOMEXIA_CONFIG_HOME/pw"

    def client(self, name="rg"):
        tools = self.root / "clients"
        tools.mkdir(exist_ok=True)
        client = tools / name
        shutil.copyfile(ROOT / "tests/fixtures/amx-tools/native-client.py", client)
        client.chmod(0o700)
        self.environment["PATH"] = str(tools) + ":/usr/bin:/bin"

    def launch(self, arguments, timeout=10):
        request = json.dumps(dict(version=1, program="rg", arguments=arguments, timeout_seconds=timeout))
        child = subprocess.Popen([sys.executable, str(BRIDGE), request], cwd=self.root, env=self.environment,
                                 stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.addCleanup(self.cleanup_child, child)
        return child

    @staticmethod
    def cleanup_child(child):
        if child.stdin is not None and not child.stdin.closed:
            child.stdin.close()
            child.stdin = None
        try:
            child.communicate(timeout=4)
        except subprocess.TimeoutExpired:
            child.kill()
            child.communicate(timeout=2)

    def identity(self, marker, child):
        deadline = time.monotonic() + 5
        while not marker.exists():
            self.assertIsNone(child.poll(), "supervisor exited before native readiness")
            self.assertLess(time.monotonic(), deadline, "native fixture did not acknowledge readiness")
            time.sleep(0.005)
        ids = json.loads(marker.read_text())
        handle = os.pidfd_open(ids[0])
        self.addCleanup(os.close, handle)
        return handle

    def test_lease_eof_termination_and_deadline_retire_exact_guest_child(self):
        self.client()
        for operation in ("eof", "signal", "deadline"):
            with self.subTest(operation=operation):
                marker = self.root / operation
                child = self.launch(["lease", str(marker)], timeout=2 if operation == "deadline" else 10)
                handle = self.identity(marker, child)
                if operation == "eof":
                    child.stdin.close()
                    child.stdin = None
                elif operation == "signal":
                    child.send_signal(signal.SIGTERM)
                # Keep the lease open for signal/deadline; communicate() would
                # close it and mask which cancellation path actually worked.
                child.wait(timeout=5)
                self.assertEqual(child.returncode, 124 if operation == "deadline" else 130)
                self.assertTrue(select.select([handle], [], [], 1)[0], "owned native child remained running")

    def test_leader_exit_retires_pipe_holding_descendant(self):
        self.client()
        marker = self.root / "descendant"
        child = self.launch(["descendant", str(marker)])
        handle = self.identity(marker, child)
        marker.with_suffix(".ack").touch()
        child.wait(timeout=5)
        self.assertEqual(child.returncode, 0)
        self.assertTrue(select.select([handle], [], [], 1)[0])
        child.stdin.close()
        child.stdin = None
        output, _ = child.communicate(timeout=2)
        self.assertEqual(output, b"")

    def test_closed_lease_never_launches_client(self):
        self.client()
        marker = self.root / "never-created"
        child = self.launch(["lease", str(marker)])
        child.stdin.close()
        child.stdin = None
        child.wait(timeout=5)
        self.assertEqual(child.returncode, 130)
        self.assertFalse(marker.exists())

    def invoke(self, arguments):
        hints = []
        if BINARY.suffix.lower() == ".exe":
            hints = ["--amx-wsl-distribution", os.environ["WSL_DISTRO_NAME"], "--amx-wsl-cwd", str(self.root),
                     "--amx-wsl-path", self.environment["PATH"], "--amx-wsl-home", str(self.root)]
        result = subprocess.run([str(BINARY), *hints, *arguments], cwd=self.root,
                                env=self.environment, capture_output=True, timeout=25)
        self.assertLess(len(result.stdout) + len(result.stderr), 65536)
        return result.returncode, result.stdout.replace(b"\r\n", b"\n"), result.stderr

    def test_real_ripgrep_respects_ignore_privacy_binary_and_symlink_rules(self):
        self.assertIsNotNone(shutil.which("rg"), "install ripgrep explicitly to run native search validation")
        (self.root / ".gitignore").write_text("ignored/\n")
        (self.root / "src").mkdir()
        (self.root / "src/Dockerfile").write_text("FROM fixture\nconnection refused\n")
        for directory in ("ignored", ".hidden", ".aws"):
            (self.root / directory).mkdir()
            (self.root / directory / "Dockerfile").write_text("connection refused\n")
        for name in ("credentials.json", ".env", "fixture.pem"):
            (self.root / name).write_text("connection refused\n")
        (self.root / "binary").write_bytes(b"\0connection refused\n")
        (self.root / "linked").symlink_to(self.root / "ignored", target_is_directory=True)
        for arguments, expected in [(["find", "file", "Dockerfile"], b"./src/Dockerfile\n"),
                                    (["find", "text", "connection refused"], b"./src/Dockerfile:2:connection refused\n"),
                                    (["find", "text", "no matches"], b"No matching text.\n")]:
            code, output, _ = self.invoke(arguments)
            self.assertEqual(code, 0, "real project search failed")
            self.assertEqual(output, expected)

    def test_directory_preview_resolves_guest_symlinks_without_desktop_or_writes(self):
        parent = self.root / "container"
        parent.mkdir()
        destination = parent / "folder & Unicode-é"
        destination.mkdir()
        (self.root / "alias").symlink_to(destination, target_is_directory=True)
        (self.root / "file").write_text("not a directory")
        before = set(self.root.rglob("*"))
        # A link into another parent distinguishes real POSIX resolution from
        # lexical '..' removal performed on the host before following the link.
        for target, resolved in (("container/folder & Unicode-é", destination), ("alias", destination),
                                 (str(destination), destination), ("alias/..", parent), ("/", Path("/"))):
            code, output, _ = self.invoke(["open", "--preview", target])
            self.assertEqual(code, 0, "directory preview must resolve the real guest directory")
            plan = json.loads(output)
            self.assertEqual(plan["action"], "open-directory")
            self.assertEqual(plan["execution"], "preview-only")
            expected = str(resolved)
            if BINARY.suffix.lower() == ".exe":
                expected = "\\\\wsl.localhost\\" + os.environ["WSL_DISTRO_NAME"] + expected.replace("/", "\\")
            self.assertTrue(plan["destination"] == expected, "resolved directory differs from the independent native path oracle")
        for target in ("missing", "file", "bad\npath", "x" * 4097):
            code, output, errors = self.invoke(["open", "--preview", target])
            self.assertNotEqual(code, 0)
            self.assertEqual(output, b"")
            self.assertNotIn(str(self.root).encode(), errors)
        self.assertEqual(set(self.root.rglob("*")), before)

    def test_project_python_modules_cannot_execute_during_guest_launch(self):
        # The Windows-backed supervisor is fixed code, but Python's default -c
        # import path otherwise lets the current project replace stdlib modules.
        (self.root / "json.py").write_text("from pathlib import Path\nPath('unexpected-execution').touch()\nraise RuntimeError('fixture blocked')\n")
        (self.root / "Dockerfile").write_text("fixture\n")
        self.environment["PYTHONPATH"] = str(self.root)
        self.environment["WSLENV"] += ":PYTHONPATH/u"
        code, output, _ = self.invoke(["find", "file", "Dockerfile"])
        self.assertFalse((self.root / "unexpected-execution").exists(), "project module executed during tool setup")
        self.assertEqual(code, 0, "guest launch trusted a project Python module")
        self.assertEqual(output, b"./Dockerfile\n")
        code, output, _ = self.invoke(["open", "--preview", "."])
        self.assertEqual(code, 0, "directory probe imported untrusted project code")
        self.assertEqual(json.loads(output)["action"], "open-directory")
        self.assertFalse((self.root / "unexpected-execution").exists())
        code, output, _ = self.invoke(["edit", "--preview", "Dockerfile"])
        self.assertEqual(code, 0, "file probe imported untrusted project code")
        self.assertEqual(json.loads(output)["action"], "edit-file")
        self.assertFalse((self.root / "unexpected-execution").exists())

    def test_editor_preview_resolves_guest_file_links_and_rejects_directories(self):
        target = self.root / "file & Unicode-é.rs"
        target.write_text("fixture\n")
        (self.root / "alias.rs").symlink_to(target)
        for name in (target.name, "alias.rs"):
            code, output, _ = self.invoke(["edit", "--preview", "--line", "42", name])
            self.assertEqual(code, 0, "guest file resolution failed")
            destination = str(target)
            if BINARY.suffix.lower() == ".exe":
                destination = "//wsl.localhost/" + os.environ["WSL_DISTRO_NAME"] + destination
            expected = "vscode://file" + quote(destination, safe="/:&") + ":42:1"
            self.assertTrue(json.loads(output)["destination"] == expected, "guest file identity or position changed")
        for name in (".", "missing"):
            code, _, _ = self.invoke(["edit", "--preview", name])
            self.assertNotEqual(code, 0)
        self.assertEqual(target.read_text(), "fixture\n")

    def test_explain_uses_offline_exact_client_argv_and_never_executes_examples(self):
        self.client("tldr")
        before = set(self.root.rglob("*"))
        code, output, _ = self.invoke(["explain", "tar"])
        self.assertEqual(code, 0, "offline client fixture failed")
        self.assertIn(b"nothing below is executed", output)
        self.assertIn(b"tar -tf {{archive.tar}}", output)
        self.assertEqual(set(self.root.rglob("*")), before)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    options = parser.parse_args()
    BINARY = options.binary.resolve()
    if sys.platform != "linux" or not hasattr(os, "pidfd_open"):
        parser.error("native Linux with pidfd process identity support is required")
    if not BINARY.is_file():
        parser.error("build the Automexia executable first")
    if BINARY.suffix.lower() == ".exe" and not os.environ.get("WSL_DISTRO_NAME"):
        parser.error("the Windows-backed adapter must run inside a real WSL session")
    (ROOT / "target/qa").mkdir(parents=True, exist_ok=True)
    unittest.main(argv=[__file__], verbosity=2)
