"""Explicit post-build native shell smoke; browser opening is never requested."""
import argparse
import os
from pathlib import Path
import shutil
import tempfile
import unittest

import qa_process

ROOT = Path(__file__).resolve().parents[2]
BINARY = None
SHELLS = []


def run(command, environment, cwd=ROOT):
    chunks = bytearray()
    def consume(chunk):
        if len(chunks) + len(chunk) > 65536:
            raise RuntimeError("native Google command output exceeded its bound")
        chunks.extend(chunk)
    result = qa_process.run(command, cwd=cwd, environment=environment,
                            timeout_seconds=25, consume=consume)
    if result.error or result.timed_out:
        raise AssertionError("native command did not complete within its process boundary")
    return result.return_code, bytes(chunks)


class GoogleCommandTests(unittest.TestCase):
    def test_real_search_and_docs_routes_are_offline_and_bounded(self):
        with tempfile.TemporaryDirectory(dir=ROOT / "target/qa", prefix="amx-search-") as temporary:
            env = self.environment(temporary)
            before = set(Path(temporary).rglob("*"))
            routes = [
                ("search", "google", b"https://www.google.com/search?q=fixture+%26+query"),
                ("search", "github", b"https://github.com/search?type=repositories&q=fixture+%26+query"),
                ("search", "youtube", b"https://www.youtube.com/results?search_query=fixture+%26+query"),
                ("search", "ddg", b"https://duckduckgo.com/?q=fixture+%26+query"),
                ("docs", "kubernetes", b"https://www.google.com/search?as_sitesearch=kubernetes.io%2Fdocs&q=fixture+%26+query"),
            ]
            for family, source, expected in routes:
                code, output = run([str(BINARY), family, source, "--print-url", "fixture & query"], env)
                self.assertEqual(code, 0, "browser-search preview failed")
                self.assertEqual(output.strip(), expected)
                for query in ([], ["a" * 4097], ["a"] * 257, ["a\nb"]):
                    code, _ = run([str(BINARY), family, source, "--print-url", *query], env)
                    self.assertNotEqual(code, 0, "invalid search was accepted")
            for family in ("search", "docs"):
                code, _ = run([str(BINARY), family, "unknown", "--print-url", "fixture"], env)
                self.assertNotEqual(code, 0, "unknown route silently fell back")
            self.assertEqual(set(Path(temporary).rglob("*")), before)

    def environment(self, temporary):
        env = {key: os.environ[key] for key in ("PATH", "SystemRoot", "WINDIR", "COMSPEC", "PATHEXT", "WSL_DISTRO_NAME") if key in os.environ}
        env.update(HOME=temporary, USERPROFILE=temporary, LOCALAPPDATA=temporary,
                   APPDATA=temporary, XDG_CONFIG_HOME=temporary,
                   AUTOMEXIA_CONFIG_HOME=temporary, USER="alice", USERNAME="alice",
                   HOSTNAME="devbox", TERM_PROGRAM="Automexia", AUTOMEXIA_PLAIN_LS="1",
                   AUTOMEXIA_CLI=str(BINARY), LC_ALL="C.UTF-8")
        return env

    def test_real_cli_preview_query_and_no_config_writes(self):
        with tempfile.TemporaryDirectory(dir=ROOT / "target/qa", prefix="google-") as temporary:
            before = set(Path(temporary).rglob("*"))
            env = self.environment(temporary)
            code, output = run([str(BINARY), "google", "--print-url", "café & rust", "+#%", "two words"], env)
            self.assertEqual(code, 0, "real Google preview command failed")
            self.assertEqual(output.strip(), b"https://www.google.com/search?q=caf%C3%A9+%26+rust+%2B%23%25+two+words")
            code, output = run([str(BINARY), "google"], env)
            self.assertNotEqual(code, 0)
            self.assertIn(b"amx google", output)
            self.assertEqual(set(Path(temporary).rglob("*")), before)

    def test_native_shell_forwarding_collision_disable_and_repeat_source(self):
        for shell in SHELLS:
            for case in ("normal", "function", "alias", "external", "disabled", "missing", "searches", "local"):
                with self.subTest(shell=shell, case=case), tempfile.TemporaryDirectory(dir=ROOT / "target/qa", prefix="google-shell-") as temporary:
                    env = self.environment(temporary)
                    ps = shell in ("powershell", "pwsh")
                    adapter = "powershell" if ps else shell
                    suffix = {"powershell": "ps1", "bash": "bash", "zsh": "zsh", "fish": "fish"}[adapter]
                    env.update(AMX_TEST_SOURCE=str(ROOT / f"shell-integration/{adapter}/automexia.{suffix}"), AMX_TEST_CASE=case)
                    if case == "local":
                        (Path(temporary) / ".git").mkdir()
                        (Path(temporary) / "Dockerfile").write_text("fixture\n")
                    if case == "external":
                        # Fixed independent executable sentinel proves the wrapper
                        # does not hijack an existing command on PATH.
                        external = Path(temporary) / ("amx.cmd" if ps else "amx")
                        external.write_bytes(b"@echo off\r\necho AMX_EXISTING_OK\r\n" if ps else b"#!/bin/sh\nprintf '%s\\n' AMX_EXISTING_OK\n")
                        external.chmod(0o700)
                        env["PATH"] = temporary + os.pathsep + env.get("PATH", "")
                    fixture = str(ROOT / f"tests/fixtures/google-command/query.{suffix}")
                    arguments = [shutil.which(shell)]
                    arguments += ["-NoLogo", "-NoProfile", "-NonInteractive", "-File", fixture] if ps else (["--noprofile", "--norc", fixture] if shell == "bash" else ["-f", fixture] if shell == "zsh" else ["--no-config", fixture])
                    code, output = run(arguments, env, cwd=Path(temporary) if case == "local" else ROOT)
                    self.assertEqual(code, 0, "native amx scenario failed")
                    marker = b"AMX_OUTPUT_BEGIN"
                    self.assertIn(marker, output, "native fixture did not reach its assertion boundary")
                    actual = output.split(marker, 1)[1].strip()
                    expected = {"normal": b"https://www.google.com/search?q=caf%C3%A9+%26+rust+%2B%23%25+two+words", "function": b"AMX_EXISTING_OK", "alias": b"AMX_EXISTING_OK", "external": b"AMX_EXISTING_OK", "disabled": b"AMX_DISABLED_OK", "missing": b"AMX_MISSING_OK"}.get(case, b"")
                    if case == "searches":
                        actual = actual.replace(b"\r\n", b"\n")
                        expected = b"\n".join([
                            b"https://www.google.com/search?q=fixture+%26+query",
                            b"https://github.com/search?type=repositories&q=fixture+%26+query",
                            b"https://www.youtube.com/results?search_query=fixture+%26+query",
                            b"https://duckduckgo.com/?q=fixture+%26+query",
                            b"https://www.google.com/search?as_sitesearch=kubernetes.io%2Fdocs&q=fixture+%26+query",
                        ])
                    if case == "local":
                        actual = actual.replace(b"\\", b"/")
                        expected = b"./Dockerfile"
                    if shell == "zsh":
                        # Zsh's installed preexec hook announces the command even
                        # in this source fixture. Check its exact frame rather
                        # than stripping arbitrary terminal bytes from the oracle.
                        expected = b"\x1b[0m\x1b]1337;SetUserVar=automexia_prompt_active=MA==\x07\x1b]133;C\x07" + expected
                    self.assertTrue(actual == expected, "native alias result differed from the independent oracle")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", required=True, type=Path)
    options = parser.parse_args()
    BINARY = options.binary.resolve()
    if not BINARY.is_file():
        parser.error("build the Automexia executable first")
    names = ("powershell", "pwsh") if os.name == "nt" else ("bash", "zsh", "fish")
    SHELLS = [name for name in names if shutil.which(name)]
    if not SHELLS:
        parser.error("no native supported shell available")
    (ROOT / "target/qa").mkdir(parents=True, exist_ok=True)
    print("Native Google command shells: " + ", ".join(SHELLS))
    unittest.main(argv=[__file__], verbosity=2)
