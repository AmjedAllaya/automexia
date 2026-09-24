"""Native prompt hooks; decode wire bytes independently, never print shell output."""
import base64
import json
import os
from pathlib import Path
import re
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
import unittest

ROOT = Path(__file__).resolve().parents[2]
HINT_NAMES = ["HOME", "KUBECONFIG"]
WINDOWS_HINTS = {"HOMEDRIVE": "C:", "HOMEPATH": "\\fixture\\drive-home", "USERPROFILE": "C:\\fixture\\profile"}
FRAME = re.compile(rb"\x1b\]1337;SetUserVar=automexia_env_(HOME|KUBECONFIG|HOMEDRIVE|HOMEPATH|USERPROFILE)=([A-Za-z0-9+/=]*)\x07")
HOME_FIXTURE = "/fixture/home"
CONFIG_FIXTURE = "/fixture/one:/fixture/two"
IDENTITY_NAMES = {"automexia_shell", "automexia_shell_name", "automexia_shell_user",
                  "automexia_shell_path", "automexia_distro", "automexia_os_version"}
USER_VAR = re.compile(rb"\x1b\]1337;SetUserVar=([a-zA-Z0-9_]+)=([A-Za-z0-9+/=]*)(?:\x07|\x1b\\)")


def committed_identity_frames(shell, wire):
    """Decode real OSC independently; errors never include identity values."""
    hints = [] if shell == "cmd" else HINT_NAMES + (list(WINDOWS_HINTS) if shell in ("powershell", "pwsh") else [])
    required = IDENTITY_NAMES | {"automexia_env_" + name for name in hints}
    pending = "automexia_env_pending"
    frames, current = [], None
    for name, encoded in USER_VAR.findall(wire):
        name = name.decode("ascii")
        if name not in required and name != pending:
            continue
        try:
            value = base64.b64decode(encoded, validate=True).decode("utf-8")
        except (ValueError, UnicodeError):
            raise AssertionError(f"{shell}: invalid metadata encoding (value withheld)") from None
        if name == pending:
            if value == "1" and current is None:
                current = {}
            elif value == "0" and current is not None:
                if set(current) != required:
                    raise AssertionError(f"{shell}: incomplete identity/location frame")
                frames.append(current)
                current = None
            else:
                raise AssertionError(f"{shell}: invalid identity frame boundary")
        elif current is None:
            raise AssertionError(f"{shell}: identity or location published outside its frame")
        elif name in current:
            raise AssertionError(f"{shell}: duplicate identity or location field")
        else:
            current[name] = value
    if current is not None:
        raise AssertionError(f"{shell}: incomplete identity frame")
    return frames


def native_shells():
    candidates = ("powershell", "pwsh") if os.name == "nt" else ("bash", "zsh", "fish")
    return [name for name in candidates if shutil.which(name)]


def run_prompt_fixture(shell, changes, repeat=1, no_encoder=False, unexport=False, disabled=False, location_changes=None, inspect_identity=False, identity_environment=None):
    """Run real hooks in one shell so cache invalidation cannot hide behind restart."""
    powershell = shell in ("powershell", "pwsh")
    adapter = "powershell" if powershell else shell
    suffix = {"powershell": "ps1", "bash": "bash", "zsh": "zsh", "fish": "fish"}[adapter]
    temporary_root = ROOT / "target/qa/shell-location-fixtures"
    temporary_root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="fixture-", dir=temporary_root) as temporary:
        env = {key: os.environ[key] for key in ("PATH", "SystemRoot", "WINDIR", "COMSPEC", "PATHEXT") if key in os.environ}
        env.update(HOME=temporary, USERPROFILE=temporary, XDG_CONFIG_HOME=temporary,
                   AUTOMEXIA_CONFIG_HOME=temporary, TERM_PROGRAM="Automexia", AUTOMEXIA_PLAIN_LS="1",
                   AUTOMEXIA_HINT_SOURCE=str(ROOT / f"shell-integration/{adapter}/automexia.{suffix}"),
                   USER="alice", HOSTNAME="devbox", WSL_DISTRO_NAME="Fixture-Distro", LC_ALL="C.UTF-8")
        if identity_environment is not None:
            if set(identity_environment) - {"WSL_DISTRO_NAME", "USER"}:
                raise ValueError("unsupported identity fixture key")
            env.update(identity_environment)
        if powershell:
            # Console writes bypass PowerShell's pipeline redirection. Discard all
            # startup bytes by the explicit marker, never by an output snapshot.
            script = ". $env:AUTOMEXIA_HINT_SOURCE\n"
            prompt = "prompt | Out-Null"
            marker = "[Console]::Write([char]30)"
            assign = lambda key, value: f"$env:{key} = '" + value.replace("'", "''") + "'"
            remove = lambda key: f"Remove-Item Env:{key} -ErrorAction SilentlyContinue"
            loop = lambda body: f"for ($i=0; $i -lt {repeat}; $i++) {{ {body} }}"
            args = [shutil.which(shell), "-NoLogo", "-NoProfile", "-NonInteractive", "-Command", "-"]
        elif shell == "fish":
            script = 'source "$AUTOMEXIA_HINT_SOURCE"' + ('\n' if inspect_identity else ' >/dev/null 2>/dev/null\n')
            prompt = "__automexia_fish_prompt"
            marker = "printf '\\036'"
            assign = lambda key, value: f"set -gx {key} '" + value.replace("\\", "\\\\").replace("'", "\\'") + "'"
            remove = lambda key: f"set -e {key}"
            loop = lambda body: f"set -l i 0; while test $i -lt {repeat}; {body}; set i (math $i + 1); end"
            args = [shutil.which(shell), "--no-config"]
        else:
            script = '. "$AUTOMEXIA_HINT_SOURCE"' + ('\n' if inspect_identity else ' >/dev/null 2>/dev/null\n')
            prompt = "__automexia_pre_prompt" if shell == "bash" else "__automexia_precmd"
            marker = "printf '\\036'"
            assign = lambda key, value: f"export {key}='" + value.replace("'", "'\\''") + "'"
            remove = lambda key: f"unset {key}"
            loop = lambda body: f"for ((i=0;i<{repeat};i++)); do {body}; done"
            args = [shutil.which(shell), "--noprofile", "--norc"] if shell == "bash" else [shutil.which(shell), "-f"]
        script += assign("HOME", HOME_FIXTURE) + "\n"
        if powershell:
            for key, value in WINDOWS_HINTS.items():
                script += assign(key, value) + "\n"
        if disabled:
            script += assign("AUTOMEXIA_CONTEXT_PATH_HINTS", "0") + "\n"
        if location_changes is not None and len(location_changes) != len(changes):
            raise ValueError("location changes must match prompt fixture steps")
        for step, value in enumerate(changes):
            for key, hint in (location_changes[step] if location_changes else {}).items():
                if key not in ["HOME", *WINDOWS_HINTS]:
                    raise ValueError("unsupported location fixture key")
                script += (remove(key) if hint is None else assign(key, hint)) + "\n"
            script += (remove("KUBECONFIG") if value is None else assign("KUBECONFIG", value)) + "\n"
            if unexport:
                script += {"bash": "export -n KUBECONFIG", "zsh": "typeset +x KUBECONFIG",
                           "fish": "set -gu KUBECONFIG $KUBECONFIG"}[shell] + "\n"
            script += marker + "\n" + prompt + "\n"
        if no_encoder:
            # After warming the cache, removal of all executable lookup paths
            # proves repeated prompts do not depend on another base64 process.
            script += assign("PATH", "/fixture/no-executables") + "\n"
        script += marker + "\n" + loop(prompt) + "\n"
        payload = script.encode("utf-8")
        if powershell:
            # Windows PowerShell decodes redirected stdin using the console code
            # page. A BOM-bearing fixture exercises Unicode values themselves.
            fixture = Path(temporary) / "prompt-fixture.ps1"
            fixture.write_text(script, encoding="utf-8-sig")
            args[-2:] = ["-File", str(fixture)]
            payload = None
        started = time.perf_counter()
        result = subprocess.run(args, input=payload, env=env, cwd=temporary,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30, check=False)
        if result.returncode:
            raise AssertionError(f"{shell}: native prompt fixture failed (output withheld)")
        if len(result.stdout) > 8 * 1024 * 1024:
            raise AssertionError(f"{shell}: prompt output exceeded fixture budget")
        all_chunks = result.stdout.split(b"\x1e")
        chunks = all_chunks[1:]
        if inspect_identity:
            # Include source-time publication in the oracle even though ordinary
            # prompt-count assertions begin after the first fixture marker.
            startup = committed_identity_frames(shell, all_chunks[0])
            if shell != "fish" and not startup:
                raise AssertionError(f"{shell}: missing complete startup identity")
            identity_frames = [committed_identity_frames(shell, chunk) for chunk in chunks]
            if any(not frames for frames in identity_frames):
                raise AssertionError(f"{shell}: missing complete prompt identity")
        for chunk in chunks:
            names = HINT_NAMES + (list(WINDOWS_HINTS) if powershell else [])
            wire = re.findall(rb"\x1b\]1337;SetUserVar=automexia_env_(pending|HOME|KUBECONFIG|HOMEDRIVE|HOMEPATH|USERPROFILE)=([A-Za-z0-9+/=]*)\x07", chunk)
            width = len(names) + 2
            if len(wire) % width or not wire:
                raise AssertionError(f"{shell}: missing path-pair commit markers")
            for offset in range(0, len(wire), width):
                group = wire[offset:offset + width]
                if [name.decode("ascii") for name, _ in group] != ["pending", *names, "pending"] or group[0][1] != b"MQ==" or group[-1][1] != b"MA==":
                    raise AssertionError(f"{shell}: unordered path-pair commit markers")
        frames = [[(name.decode("ascii"), base64.b64decode(value, validate=True).decode("utf-8"))
                   for name, value in FRAME.findall(chunk)] for chunk in chunks]
        return (identity_frames if inspect_identity else frames), time.perf_counter() - started


class ShellLocationHintTests(unittest.TestCase):
    def shells(self):
        available = native_shells()
        if not available:
            self.skipTest("native shell adapters unavailable on this runner")
        return available

    def assert_pair(self, shell, frames, configured, home=HOME_FIXTURE):
        windows = shell in ("powershell", "pwsh")
        self.assertEqual([name for name, _ in frames], HINT_NAMES + (list(WINDOWS_HINTS) if windows else []), f"{shell}: exact location hint order")
        values = dict(frames)
        self.assertEqual(values["KUBECONFIG"], configured, f"{shell}: configuration hint mismatch")
        # Never include an accidentally inherited host value in diagnostics.
        self.assertTrue(values["HOME"] == home, f"{shell}: exported home hint mismatch")
        if windows:
            for key, expected in WINDOWS_HINTS.items():
                self.assertTrue(values[key] == (expected if home else ""), f"{shell}: Windows home candidate mismatch")

    def test_identity_activation_and_locations_commit_together_on_startup_and_replay(self):
        for shell in self.shells():
            with self.subTest(shell=shell):
                groups, _ = run_prompt_fixture(shell, [CONFIG_FIXTURE], repeat=3,
                                               no_encoder=True, inspect_identity=True)
                self.assertEqual([len(group) for group in groups], [1, 3])
                expected_name = "PowerShell" if shell in ("powershell", "pwsh") else shell
                for frames in groups:
                    for frame in frames:
                        self.assertEqual(frame["automexia_shell"], "1")
                        self.assertEqual(frame["automexia_shell_name"], expected_name)
                        self.assertEqual(frame["automexia_env_KUBECONFIG"], CONFIG_FIXTURE)

    def test_empty_identity_is_explicit_and_cached_without_an_encoder(self):
        for shell in self.shells():
            with self.subTest(shell=shell):
                groups, _ = run_prompt_fixture(shell, [CONFIG_FIXTURE], repeat=3,
                    no_encoder=True, inspect_identity=True,
                    identity_environment={"WSL_DISTRO_NAME": ""})
                for frames in groups:
                    for frame in frames:
                        self.assertEqual(frame["automexia_distro"], "",
                                         f"{shell}: an empty identity must clear previous metadata")

    @unittest.skipUnless(os.name == "nt", "native CMD is Windows-only")
    def test_cmd_startup_and_cached_prompt_commit_after_interrupted_metadata(self):
        temporary_root = ROOT / "target/qa/shell-location-fixtures"
        temporary_root.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="cmd-frame-", dir=temporary_root) as temporary:
            fixture = Path(temporary)
            source = (ROOT / "shell-integration/cmd/automexia.cmd").read_text(encoding="ascii")
            expected = {"automexia_shell": "1", "automexia_shell_name": "CMD",
                        "automexia_shell_user": "alice", "automexia_shell_path": r"C:\Fixture\cmd.exe",
                        "automexia_distro": "", "automexia_os_version": ""}
            for placeholder, key in (("__AUTOMEXIA_CMD_USER_BASE64__", "automexia_shell_user"),
                                     ("__AUTOMEXIA_CMD_PATH_BASE64__", "automexia_shell_path")):
                source = source.replace(placeholder, base64.b64encode(expected[key].encode()).decode())
            (fixture / "automexia.cmd").write_bytes(source.replace("\r\n", "\n").replace("\n", "\r\n").encode("ascii"))
            shutil.copyfile(ROOT / "shell-integration/cmd/automexia-alias-loader.ps1", fixture / "automexia-alias-loader.ps1")
            interrupted = b"\x1b]1337;SetUserVar=automexia_env_pending=MQ==\x1b\\"
            # Execute a fixed batch file with a fictional identity. Native CMD
            # parses the real hook; replay then uses only its cached PROMPT.
            probe = (b'@echo off\r\n<nul set /p "=' + interrupted + b'"\r\n'
                     b'call automexia.cmd\r\necho \x1e\r\nset "PATH="\r\n'
                     b'echo %PROMPT%\r\necho %PROMPT%\r\nexit /b 0\r\n')
            (fixture / "probe.cmd").write_bytes(probe)
            env = {key: os.environ[key] for key in ("SystemRoot", "WINDIR", "TEMP", "TMP") if key in os.environ}
            system = Path(os.environ["SystemRoot"]) / "System32"
            command = system / "cmd.exe"
            env.update(PATH=os.pathsep.join([str(system), str(system / "WindowsPowerShell/v1.0")]),
                       COMSPEC=str(command), HOME=temporary, USERPROFILE=temporary,
                       TERM_PROGRAM="Automexia", AUTOMEXIA_PLAIN_LS="1", AUTOMEXIA_AMX="0",
                       AUTOMEXIA_CONFIG_HOME=str(fixture / "empty-config"), AUTOMEXIA_CMD_PROMPT_GLYPH="+")
            result = subprocess.run([str(command), "/D", "/Q", "/C", "probe.cmd"], cwd=fixture, env=env,
                                    stdin=subprocess.DEVNULL, capture_output=True, timeout=30, check=False)
            self.assertEqual(result.returncode, 0, "native CMD fixture failed (output withheld)")
            self.assertLess(len(result.stdout), 64 * 1024, "CMD fixture output exceeds budget")
            chunks = result.stdout.split(b"\x1e")
            self.assertEqual(len(chunks), 2, "native CMD fixture marker missing")
            self.assertTrue(chunks[0].startswith(interrupted), "interrupted frame fixture missing")
            startup = committed_identity_frames("cmd", chunks[0][len(interrupted):])
            replay = committed_identity_frames("cmd", chunks[1])
            self.assertEqual(startup, [expected], "CMD startup identity is incomplete")
            self.assertEqual(replay, [expected, expected], "CMD cached identity is incomplete")
            self.assertLess(len(chunks[0]), 8192, "CMD startup frame exceeds command bounds")
            pending = [value for name, value in USER_VAR.findall(result.stdout) if name == b"automexia_env_pending"]
            self.assertEqual(pending, [b"MQ==", b"MQ==", b"MA==", b"MQ==", b"MA==", b"MQ==", b"MA=="],
                             "CMD must finish new frames after interrupted metadata")

    def test_actual_prompt_updates_clears_and_restores_same_directory(self):
        for shell in self.shells():
            with self.subTest(shell=shell):
                changes = [CONFIG_FIXTURE, "/fixture/child", CONFIG_FIXTURE, "", None]
                chunks, _ = run_prompt_fixture(shell, changes)
                self.assertEqual(len(chunks), len(changes) + 1)
                for chunk, expected in zip(chunks, changes + [None]):
                    self.assert_pair(shell, chunk, expected or "")

    def test_unicode_metacharacters_are_data_not_commands(self):
        for shell in self.shells():
            with self.subTest(shell=shell):
                configured = "/fixture/Î© space & ' quote ; $(not-a-command)"
                chunks, _ = run_prompt_fixture(shell, [configured])
                for chunk in chunks:
                    self.assert_pair(shell, chunk, configured)

    def test_byte_boundary_and_controls_fail_closed_as_one_pair(self):
        for shell in self.shells():
            for configured, valid in [("x" * 4095, True), ("x" * 4096, True),
                                      ("x" * 4097, False), ("Î©" * 2049, False), ("one\ntwo", False)]:
                with self.subTest(shell=shell, bytes=len(configured.encode("utf-8"))):
                    chunks, _ = run_prompt_fixture(shell, [configured])
                    for chunk in chunks:
                        self.assert_pair(shell, chunk, configured if valid else "", HOME_FIXTURE if valid else "")

    def test_repeated_prompts_replay_cached_frames_without_encoder(self):
        for shell in self.shells():
            with self.subTest(shell=shell):
                chunks, _ = run_prompt_fixture(shell, [CONFIG_FIXTURE], repeat=200, no_encoder=True)
                self.assert_pair(shell, chunks[0], CONFIG_FIXTURE)
                width = 5 if shell in ("powershell", "pwsh") else 2
                self.assertEqual(len(chunks[1]), 200 * width)
                for offset in range(0, 200 * width, width):
                    self.assert_pair(shell, chunks[1][offset:offset + width], CONFIG_FIXTURE)

    def test_windows_candidates_update_clear_and_restore_without_restart(self):
        shells = [shell for shell in self.shells() if shell in ("powershell", "pwsh")]
        if not shells:
            self.skipTest("Windows candidate publication requires native PowerShell")
        expected = [dict(HOME=HOME_FIXTURE, **WINDOWS_HINTS),
                    dict(HOME="", HOMEDRIVE="D:", HOMEPATH=r"\fixture\changed", USERPROFILE=""),
                    dict(HOME=HOME_FIXTURE, **WINDOWS_HINTS)]
        for shell in shells:
            with self.subTest(shell=shell):
                chunks, _ = run_prompt_fixture(shell, ["", "", ""], location_changes=expected)
                for chunk, values in zip(chunks, expected + expected[-1:]):
                    actual = dict(chunk)
                    for key, value in values.items():
                        self.assertTrue(actual.get(key) == value, f"{shell}: live candidate update mismatch")

    def test_each_invalid_windows_candidate_clears_the_complete_snapshot(self):
        shells = [shell for shell in self.shells() if shell in ("powershell", "pwsh")]
        if not shells:
            self.skipTest("Windows candidate publication requires native PowerShell")
        for shell in shells:
            for key in ["HOME", *WINDOWS_HINTS]:
                with self.subTest(shell=shell, key=key):
                    chunks, _ = run_prompt_fixture(shell, [CONFIG_FIXTURE], location_changes=[{key: "invalid\nvalue"}])
                    for chunk in chunks:
                        self.assert_pair(shell, chunk, "", "")

    def test_opt_out_clears_both_hints(self):
        for shell in self.shells():
            with self.subTest(shell=shell):
                chunks, _ = run_prompt_fixture(shell, [CONFIG_FIXTURE], disabled=True)
                for chunk in chunks:
                    self.assert_pair(shell, chunk, "", "")

    def test_unexported_configuration_is_not_the_child_environment(self):
        shells = [shell for shell in self.shells() if shell in ("bash", "zsh", "fish")]
        if not shells:
            self.skipTest("export attributes require native Unix shells")
        for shell in shells:
            with self.subTest(shell=shell):
                chunks, _ = run_prompt_fixture(shell, [CONFIG_FIXTURE], unexport=True)
                for chunk in chunks:
                    self.assert_pair(shell, chunk, "")

    @unittest.skipIf(os.name == "nt", "native Unix checker failure injection")
    def test_zsh_startup_does_not_populate_unrelated_path_commands(self):
        executable = shutil.which("zsh")
        if not executable:
            self.skipTest("native Zsh is unavailable")
        parent = ROOT / "target/qa/shell-location-fixtures"
        parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=parent) as temporary:
            sentinel = Path(temporary) / "automexia-fixture-unrelated-command"
            sentinel.write_text("#!/bin/sh\nexit 97\n", encoding="utf-8")
            sentinel.chmod(0o700)
            env = {"PATH": "/usr/bin:/bin:" + temporary, "HOME": temporary,
                   "USER": "alice", "WSL_DISTRO_NAME": "Fixture-Distro",
                   "TERM_PROGRAM": "Automexia", "AUTOMEXIA_PLAIN_LS": "1",
                   "AUTOMEXIA_CONFIG_HOME": temporary, "XDG_CONFIG_HOME": temporary,
                   "FIXTURE_SOURCE": str(ROOT / "shell-integration/zsh/automexia.zsh")}
            # The unrelated executable is after every required native utility.
            # Call the real encoder once in this process to observe its hash
            # table: normal frame capture uses a subshell and hides that table.
            # A full commands-array scan hashes it; targeted lookups do not.
            script = (b'before="$options[hashcmds]:$options[hashdirs]"\n'
                      b'source "$FIXTURE_SOURCE" >/dev/null 2>/dev/null\n'
                      b'__automexia_set_user_var fixture x >/dev/null\n'
                      b'__automexia_precmd >/dev/null\n'
                      b'[[ "$before" == "$options[hashcmds]:$options[hashdirs]" ]] || exit 96\n'
                      b'builtin hash\n')
            result = subprocess.run([executable, "-f"], input=script, env=env, cwd=temporary,
                                    capture_output=True, timeout=10, check=False)
            self.assertEqual(result.returncode, 0, "Zsh startup/hash-option preservation failed")
            self.assertFalse(sentinel.name.encode() in result.stdout,
                             "prompt startup enumerated unrelated PATH entries")

    @unittest.skipIf(os.name == "nt", "native Unix checker failure injection")
    def test_shell_gate_preserves_failure_and_emits_redacted_diagnostic(self):
        parent = ROOT / "target/qa/shell-location-fixtures"
        parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=parent) as temporary:
            executable = Path(temporary) / "git"
            executable.write_text("#!/bin/sh\nexit 7\n", encoding="utf-8")
            executable.chmod(0o700)
            environment = dict(os.environ, PATH=temporary + os.pathsep + os.environ.get("PATH", ""))
            result = subprocess.run([shutil.which("bash"), str(ROOT / "tools/ci/test_shell_sources.sh")],
                                    cwd=ROOT, env=environment, capture_output=True, timeout=10, check=False)
            self.assertEqual(result.returncode, 7)
            self.assertIn(b"FAIL: shell-source gate at line", result.stderr)
            self.assertFalse(str(ROOT).encode() in result.stderr or temporary.encode() in result.stderr)


def benchmark(report):
    """Startup-inclusive fixture cost, not interactive input or frame latency."""
    rows = []
    oracle = ShellLocationHintTests()
    for shell in oracle.shells():
        samples = []
        startup_samples = []
        for _ in range(5):
            first, startup_elapsed = run_prompt_fixture(shell, [CONFIG_FIXTURE], repeat=1)
            oracle.assertEqual(len(first), 2)
            for chunk in first:
                oracle.assert_pair(shell, chunk, CONFIG_FIXTURE)
            startup_samples.append(startup_elapsed * 1000)
            chunks, elapsed = run_prompt_fixture(shell, [CONFIG_FIXTURE], repeat=1000, no_encoder=True)
            oracle.assertEqual(len(chunks), 2)
            width = 5 if shell in ("powershell", "pwsh") else 2
            oracle.assertEqual(len(chunks[1]), 1000 * width)
            for offset in range(0, 1000 * width, width):
                oracle.assert_pair(shell, chunks[1][offset:offset + width], CONFIG_FIXTURE)
            samples.append(elapsed * 1000)
        rows.append({"shell": shell, "samples": 5, "prompts_per_sample": 1000,
                     "median_startup_and_two_prompts_ms": statistics.median(startup_samples),
                     "max_startup_and_two_prompts_ms": max(startup_samples),
                     "median_total_ms": statistics.median(samples), "max_total_ms": max(samples)})
    evidence = {"status": "pass", "scope": "native process startup plus 1000 validated cached prompt replays; not UI latency", "results": rows}
    report.parent.mkdir(parents=True, exist_ok=True)
    report.write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(evidence))


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--benchmark":
        benchmark(Path(sys.argv[2]))
    else:
        unittest.main()
