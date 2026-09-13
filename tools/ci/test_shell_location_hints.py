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
FRAME = re.compile(rb"\x1b\]1337;SetUserVar=automexia_env_(HOME|KUBECONFIG)=([A-Za-z0-9+/=]*)\x07")
HOME_FIXTURE = "/fixture/home"
CONFIG_FIXTURE = "/fixture/one:/fixture/two"


def native_shells():
    candidates = ("powershell", "pwsh") if os.name == "nt" else ("bash", "zsh", "fish")
    return [name for name in candidates if shutil.which(name)]


def run_prompt_fixture(shell, changes, repeat=1, no_encoder=False, unexport=False, disabled=False):
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
            script = 'source "$AUTOMEXIA_HINT_SOURCE" >/dev/null 2>/dev/null\n'
            prompt = "__automexia_fish_prompt"
            marker = "printf '\\036'"
            assign = lambda key, value: f"set -gx {key} '" + value.replace("\\", "\\\\").replace("'", "\\'") + "'"
            remove = lambda key: f"set -e {key}"
            loop = lambda body: f"set -l i 0; while test $i -lt {repeat}; {body}; set i (math $i + 1); end"
            args = [shutil.which(shell), "--no-config"]
        else:
            script = '. "$AUTOMEXIA_HINT_SOURCE" >/dev/null 2>/dev/null\n'
            prompt = "__automexia_pre_prompt" if shell == "bash" else "__automexia_precmd"
            marker = "printf '\\036'"
            assign = lambda key, value: f"export {key}='" + value.replace("'", "'\\''") + "'"
            remove = lambda key: f"unset {key}"
            loop = lambda body: f"for ((i=0;i<{repeat};i++)); do {body}; done"
            args = [shutil.which(shell), "--noprofile", "--norc"] if shell == "bash" else [shutil.which(shell), "-f"]
        if not powershell:
            script += assign("HOME", HOME_FIXTURE) + "\n"
        if disabled:
            script += assign("AUTOMEXIA_CONTEXT_PATH_HINTS", "0") + "\n"
        for value in changes:
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
        started = time.perf_counter()
        result = subprocess.run(args, input=script.encode("utf-8"), env=env, cwd=temporary,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30, check=False)
        if result.returncode:
            raise AssertionError(f"{shell}: native prompt fixture failed (output withheld)")
        if len(result.stdout) > 8 * 1024 * 1024:
            raise AssertionError(f"{shell}: prompt output exceeded fixture budget")
        chunks = result.stdout.split(b"\x1e")[1:]
        for chunk in chunks:
            wire = re.findall(rb"\x1b\]1337;SetUserVar=automexia_env_(pending|HOME|KUBECONFIG)=([A-Za-z0-9+/=]*)\x07", chunk)
            if len(wire) % 4 or not wire:
                raise AssertionError(f"{shell}: missing path-pair commit markers")
            for offset in range(0, len(wire), 4):
                group = wire[offset:offset + 4]
                if [name for name, _ in group] != [b"pending", b"HOME", b"KUBECONFIG", b"pending"] or group[0][1] != b"MQ==" or group[3][1] != b"MA==":
                    raise AssertionError(f"{shell}: unordered path-pair commit markers")
        frames = [[(name.decode("ascii"), base64.b64decode(value, validate=True).decode("utf-8"))
                   for name, value in FRAME.findall(chunk)] for chunk in chunks]
        return frames, time.perf_counter() - started


class ShellLocationHintTests(unittest.TestCase):
    def shells(self):
        available = native_shells()
        if not available:
            self.skipTest("native shell adapters unavailable on this runner")
        return available

    def assert_pair(self, shell, frames, configured, home=HOME_FIXTURE):
        self.assertEqual([name for name, _ in frames], ["HOME", "KUBECONFIG"], f"{shell}: exact hint pair/order")
        values = dict(frames)
        # PowerShell's immutable HOME is validated for shape only here; no live
        # home value is used in assertion diagnostics or recorded fixtures.
        self.assertEqual(values["KUBECONFIG"], configured, f"{shell}: configuration hint mismatch")
        if shell not in ("powershell", "pwsh"):
            self.assertEqual(values["HOME"], home, f"{shell}: home hint mismatch")
        else:
            self.assertTrue(bool(values["HOME"]) == bool(home), "native home hint presence mismatch")

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
                configured = "/fixture/Ω space & ' quote ; $(not-a-command)"
                chunks, _ = run_prompt_fixture(shell, [configured])
                for chunk in chunks:
                    self.assert_pair(shell, chunk, configured)

    def test_byte_boundary_and_controls_fail_closed_as_one_pair(self):
        for shell in self.shells():
            for configured, valid in [("x" * 4095, True), ("x" * 4096, True),
                                      ("x" * 4097, False), ("Ω" * 2049, False), ("one\ntwo", False)]:
                with self.subTest(shell=shell, bytes=len(configured.encode("utf-8"))):
                    chunks, _ = run_prompt_fixture(shell, [configured])
                    for chunk in chunks:
                        self.assert_pair(shell, chunk, configured if valid else "", HOME_FIXTURE if valid else "")

    def test_repeated_prompts_replay_cached_frames_without_encoder(self):
        for shell in self.shells():
            with self.subTest(shell=shell):
                chunks, _ = run_prompt_fixture(shell, [CONFIG_FIXTURE], repeat=200, no_encoder=True)
                self.assert_pair(shell, chunks[0], CONFIG_FIXTURE)
                self.assertEqual(len(chunks[1]), 400)
                for offset in range(0, 400, 2):
                    self.assert_pair(shell, chunks[1][offset:offset + 2], CONFIG_FIXTURE)

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
            oracle.assertEqual(len(chunks[1]), 2000)
            for offset in range(0, 2000, 2):
                oracle.assert_pair(shell, chunks[1][offset:offset + 2], CONFIG_FIXTURE)
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
