"""Explicit post-build native shell smoke; browser opening is never requested."""
import argparse
import json
import os
from pathlib import Path
import shutil
import tempfile
import unittest
from urllib.parse import quote

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
    def test_real_repository_navigation_is_offline_exact_and_read_only(self):
        with tempfile.TemporaryDirectory(dir=ROOT / "target/qa", prefix="amx-repo-") as temporary:
            root = Path(temporary).resolve()
            env = self.environment(temporary)
            git = shutil.which("git")
            self.assertIsNotNone(git, "installed Git is required for repository navigation validation")
            code, _ = run([git, "-c", "init.templateDir=", "init", "--quiet", str(root)], env, cwd=root)
            self.assertEqual(code, 0, "temporary repository initialization failed")
            for name, remote in (("origin", "git@github.com:example-org/fixture-repo.git"),
                                 ("upstream", "https://gitlab.com/example-group/subgroup/fixture-repo.git"),
                                 ("mixed-hub", "ssh://git@GitHub.COM/Example-Org/Fixture-Repo.git"),
                                 ("mixed-lab", "ssh://git@GitLab.COM/Example-Group/Subgroup/Fixture-Repo.git")):
                code, _ = run([git, "remote", "add", name, remote], env, cwd=root)
                self.assertEqual(code, 0)
            hints = []
            if os.name != "nt" and BINARY.suffix.lower() == ".exe":
                hints = ["--amx-wsl-distribution", os.environ["WSL_DISTRO_NAME"], "--amx-wsl-cwd", str(root),
                         "--amx-wsl-path", "/usr/bin:/bin", "--amx-wsl-home", str(root)]
            before = {p.relative_to(root): p.read_bytes() for p in root.rglob("*") if p.is_file()}
            for args, expected in (([], "https://github.com/example-org/fixture-repo"),
                                   (["issues"], "https://github.com/example-org/fixture-repo/issues"),
                                   (["issues", "--remote", "upstream"], "https://gitlab.com/example-group/subgroup/fixture-repo/-/issues"),
                                   (["--remote", "mixed-hub"], "https://github.com/Example-Org/Fixture-Repo"),
                                   (["issues", "--remote", "mixed-lab"], "https://gitlab.com/Example-Group/Subgroup/Fixture-Repo/-/issues")):
                code, output = run([str(BINARY), *hints, "repo", "--preview", *args], env, cwd=root)
                self.assertEqual(code, 0, "real repository preview failed")
                plan = json.loads(output)
                self.assertEqual(plan, dict(action="browse-repository", destination=expected, execution="preview-only"))
            code, output = run([str(BINARY), *hints, "repo", "--preview", "--remote", "missing"], env, cwd=root)
            self.assertNotEqual(code, 0, "missing remote must not select another remote")
            self.assertNotIn(str(root).encode(), output)
            self.assertEqual({p.relative_to(root): p.read_bytes() for p in root.rglob("*") if p.is_file()}, before)
            code, _ = run([git, "remote", "set-url", "origin", "https://alice:fixture@github.com/example/fixture"], env, cwd=root)
            self.assertEqual(code, 0)
            code, output = run([str(BINARY), *hints, "repo", "--preview"], env, cwd=root)
            self.assertNotEqual(code, 0)
            self.assertNotIn(b"alice:fixture", output, "credentials must not appear in diagnostics")

    def test_real_editor_preview_preserves_file_position_and_never_writes(self):
        with tempfile.TemporaryDirectory(dir=ROOT / "target/qa", prefix="amx-edit-") as temporary:
            root = Path(temporary).resolve()
            target = root / "source & Unicode-é%23.rs"
            target.write_text("fixture\n")
            env = self.environment(temporary)
            guest = os.name != "nt" and BINARY.suffix.lower() == ".exe"
            hints = []
            if guest:
                hints = ["--amx-wsl-distribution", os.environ["WSL_DISTRO_NAME"], "--amx-wsl-cwd", str(root),
                         "--amx-wsl-path", "/usr/bin:/bin", "--amx-wsl-home", str(root)]
            before = {p.relative_to(root): p.read_bytes() for p in root.rglob("*") if p.is_file()}
            for path in (target.name, str(target)):
                code, output = run([str(BINARY), *hints, "edit", "--preview", "--line", "42", "--column", "7", path], env, cwd=root)
                self.assertEqual(code, 0, "editor preview must traverse the real CLI without launching an editor")
                plan = json.loads(output)
                destination = str(target)
                if guest:
                    destination = "//wsl.localhost/" + os.environ["WSL_DISTRO_NAME"] + destination
                elif os.name == "nt":
                    destination = "/" + destination.replace("\\", "/")
                self.assertTrue(plan["destination"] == "vscode://file" + quote(destination, safe="/:&") + ":42:7",
                                "editor destination or coordinates changed")
                self.assertEqual(plan["action"], "edit-file")
                self.assertEqual(plan["editor"], "vscode")
                self.assertEqual(plan["execution"], "preview-only")
            self.assertEqual({p.relative_to(root): p.read_bytes() for p in root.rglob("*") if p.is_file()}, before)

    def test_real_editor_config_override_disable_and_invalid_targets(self):
        with tempfile.TemporaryDirectory(dir=ROOT / "target/qa", prefix="amx-edit-config-") as temporary:
            root = Path(temporary).resolve()
            (root / "--literal.rs").write_text("fixture\n")
            (root / "project.code-workspace").write_text('{}')
            env = self.environment(temporary)
            hints = []
            if os.name != "nt" and BINARY.suffix.lower() == ".exe":
                hints = ["--amx-wsl-distribution", os.environ["WSL_DISTRO_NAME"], "--amx-wsl-cwd", str(root),
                         "--amx-wsl-path", "/usr/bin:/bin", "--amx-wsl-home", str(root)]
            def invoke(*arguments):
                return run([str(BINARY), *hints, "edit", "--preview", *arguments], env, cwd=root)
            config = root / "amx.toml"
            config.write_text("version = 1\neditor = 'vscode-insiders'\n")
            code, output = invoke("--", "--literal.rs")
            self.assertEqual(code, 0, "user-root editor preference was not used")
            self.assertEqual(json.loads(output)["editor"], "vscode-insiders")
            code, output = invoke("--editor", "vscode", "--", "--literal.rs")
            self.assertEqual(code, 0)
            self.assertEqual(json.loads(output)["editor"], "vscode")
            for target in ("missing", ".", "project.code-workspace", "bad\npath", "bad\u202efile", "x" * 4097):
                code, output = invoke(target)
                self.assertNotEqual(code, 0)
                self.assertNotIn(str(root).encode(), output)
            for value in ("disabled", "unrecognized"):
                config.write_text("version = 1\neditor = '" + value + "'\n")
                before = config.read_bytes()
                code, output = invoke("--editor", "vscode", "--", "--literal.rs")
                self.assertNotEqual(code, 0, "disabled or invalid config silently fell back")
                self.assertEqual(config.read_bytes(), before)
                self.assertNotIn(str(root).encode(), output)
            self.assertEqual((root / "--literal.rs").read_text(), "fixture\n")

    def test_real_directory_preview_is_exact_and_never_writes(self):
        with tempfile.TemporaryDirectory(dir=ROOT / "target/qa", prefix="amx-open-") as temporary:
            root = Path(temporary).resolve()
            folder = root / "folder & Unicode-é"
            folder.mkdir()
            (root / "file").write_text("not an executable")
            env = self.environment(temporary)
            before = set(root.rglob("*"))
            hints = []
            guest = os.name != "nt" and BINARY.suffix.lower() == ".exe"
            if guest:
                hints = ["--amx-wsl-distribution", os.environ["WSL_DISTRO_NAME"], "--amx-wsl-cwd", str(root),
                         "--amx-wsl-path", "/usr/bin:/bin", "--amx-wsl-home", str(root)]
            for args, expected in (([], root), ([folder.name], folder), ([str(folder)], folder), (["--", folder.name], folder)):
                code, output = run([str(BINARY), *hints, "open", "--preview", *args], env, cwd=root)
                self.assertEqual(code, 0, "native directory preview failed")
                plan = json.loads(output)
                target = str(expected)
                if guest:
                    target = "\\\\wsl.localhost\\" + os.environ["WSL_DISTRO_NAME"] + target.replace("/", "\\")
                self.assertTrue(plan["destination"] == target, "native directory changed identity")
                self.assertEqual(plan["action"], "open-directory")
                self.assertEqual(plan["execution"], "preview-only")
            for target in ("file", "missing", "bad\npath", "bad\u202epath", "x" * 4097):
                code, output = run([str(BINARY), *hints, "open", "--preview", target], env, cwd=root)
                self.assertNotEqual(code, 0)
                self.assertNotIn(str(root).encode(), output)
            self.assertEqual(set(root.rglob("*")), before)

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
        if os.name != "nt" and BINARY.suffix.lower() == ".exe":
            # The Windows application must read this isolated test root, not the
            # host user's real preferences. /p translates and /w is guest-to-host.
            env["WSLENV"] = "AUTOMEXIA_CONFIG_HOME/pw"
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
            for case in ("normal", "function", "alias", "external", "disabled", "missing", "searches", "local", "directory", "editor", "repository"):
                with self.subTest(shell=shell, case=case), tempfile.TemporaryDirectory(dir=ROOT / "target/qa", prefix="google-shell-") as temporary:
                    env = self.environment(temporary)
                    ps = shell in ("powershell", "pwsh")
                    adapter = "powershell" if ps else shell
                    suffix = {"powershell": "ps1", "bash": "bash", "zsh": "zsh", "fish": "fish"}[adapter]
                    env.update(AMX_TEST_SOURCE=str(ROOT / f"shell-integration/{adapter}/automexia.{suffix}"), AMX_TEST_CASE=case)
                    if case == "local":
                        (Path(temporary) / ".git").mkdir()
                        (Path(temporary) / "Dockerfile").write_text("fixture\n")
                    if case == "directory":
                        (Path(temporary) / "folder & Unicode-é").mkdir()
                    if case == "editor":
                        (Path(temporary) / "source & Unicode-é.rs").write_text("fixture\n")
                    if case == "repository":
                        git = shutil.which("git")
                        self.assertIsNotNone(git, "installed Git is required for the native repository scenario")
                        code, _ = run([git, "-c", "init.templateDir=", "init", "--quiet", temporary], env, cwd=Path(temporary))
                        self.assertEqual(code, 0)
                        code, _ = run([git, "remote", "add", "origin", "git@github.com:example-org/fixture-repo.git"], env, cwd=Path(temporary))
                        self.assertEqual(code, 0)
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
                    code, output = run(arguments, env, cwd=Path(temporary) if case in ("local", "directory", "editor", "repository") else ROOT)
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
                    if case == "directory":
                        destination = str((Path(temporary) / "folder & Unicode-é").resolve())
                        if os.name != "nt" and BINARY.suffix.lower() == ".exe":
                            destination = "\\\\wsl.localhost\\" + os.environ["WSL_DISTRO_NAME"] + destination.replace("/", "\\")
                        expected = json.dumps(dict(action="open-directory", destination=destination, execution="preview-only"), ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
                    if case == "editor":
                        destination = str((Path(temporary) / "source & Unicode-é.rs").resolve())
                        if os.name != "nt" and BINARY.suffix.lower() == ".exe":
                            destination = "//wsl.localhost/" + os.environ["WSL_DISTRO_NAME"] + destination
                        elif os.name == "nt":
                            destination = "/" + destination.replace("\\", "/")
                        destination = "vscode://file" + quote(destination, safe="/:&") + ":42:7"
                        expected = json.dumps(dict(action="edit-file", editor="vscode", destination=destination, execution="preview-only"), ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
                    if case == "repository":
                        expected = b'{"action":"browse-repository","destination":"https://github.com/example-org/fixture-repo/issues","execution":"preview-only"}'
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
