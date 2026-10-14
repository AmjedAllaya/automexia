"""Native shell regressions for bounded, private, digest-verified artifacts.

Run with --shell bash|zsh|fish inside an approved Linux/macOS test guest.
"""
from __future__ import annotations

import argparse
import contextlib
import hashlib
import io
import os
from pathlib import Path
import shutil
import sys
import tempfile
import unittest

from create_cp31_alias_fixture import create
import qa_process

ROOT = Path(__file__).resolve().parents[2]
SHELL = "bash"


@unittest.skipUnless(os.name == "posix", "Native shells run in the explicit Linux/macOS job")
class ShellArtifactTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="automexia artifact ")
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        self.config = self.directory / "config"
        with contextlib.redirect_stdout(io.StringIO()):
            generation = create(self.config, "axt", "first")
        self.alias_root = self.config / "generated/aliases"
        self.generation = self.alias_root / "generations" / generation
        self.alias_file = self.generation / SHELL / f"automexia-aliases.{SHELL}"
        self.manifest = self.generation / "generation.manifest"
        self.completion = self.config / f"generated/completion/{SHELL}/kubectl.{SHELL}"
        self.completion.parent.mkdir(parents=True)
        code = {"bash": 'complete -W managed-candidate kubectl',
                "zsh": 'compdef _gnu_generic kubectl',
                "fish": 'complete -c kubectl -a managed-candidate'}[SHELL]
        self.completion.write_text(code + "\n")
        self.digest = self.completion.with_suffix(self.completion.suffix + ".sha256")
        self.digest.write_text(hashlib.sha256(self.completion.read_bytes()).hexdigest() + "\n")
        self.environment = dict(os.environ, HOME=str(self.directory),
                                AUTOMEXIA_CONFIG_HOME=str(self.config), TERM_PROGRAM="Automexia",
                                AUTOMEXIA_SHELL_INTEGRATION="1", AUTOMEXIA_AMX="0", AUTOMEXIA_PLAIN_LS="1")
        for name in ("AUTOMEXIA_COMPLETION_DISABLED", "AUTOMEXIA_COMPLETION_ADAPTER_FISH_LOADED"):
            self.environment.pop(name, None)

    def probe(self, owner: str, accepted: bool):
        source = ROOT / (f"shell-integration/{SHELL}/automexia.{SHELL}" if owner == "aliases"
                         else f"shell-integration/completion/{SHELL}/automexia-completion.{SHELL}")
        # The program is fixed test code. Source paths travel only as argv.
        if SHELL == "fish":
            program = 'source "$argv[1]" >/dev/null; '
            program += ('test "$__automexia_alias_state" = ready' if owner == "aliases"
                        else 'contains -- kubectl $__automexia_completion_loaded')
            command = [SHELL, "--no-config", "-c", program, str(source)]
        else:
            program = ('compdef() { :; }; typeset -A _comps; ' if SHELL == "zsh" else '')
            program += 'source "$1" >/dev/null; '
            program += ('[[ $__automexia_alias_state == ready ]]' if owner == "aliases"
                        else '[[ $__automexia_completion_loaded == kubectl ]]')
            command = [SHELL, "-f" if SHELL == "zsh" else "--noprofile", "-c", program,
                       "artifact-fixture", str(source)]
        self.run_probe(command, accepted)

    def run_probe(self, command: list[str], accepted: bool):
        if SHELL == "bash":
            # Match interactive Bash's alias lookup in this noninteractive probe.
            command = list(command)
            program = command.index("-c") + 1
            command[program] = "shopt -s expand_aliases; " + command[program]
        result = qa_process.run(command, cwd=self.directory, environment=self.environment,
                                timeout_seconds=10, consume=lambda chunk: None)
        self.assertFalse(result.timed_out, "shell artifact probe timed out")
        self.assertIsNone(result.error)
        self.assertEqual(result.return_code, 0 if accepted else 1)

    def hash_failure(self):
        binary = self.directory / "bin"
        binary.mkdir()
        self.marker = self.directory / "hash-called"
        available = [(name, shutil.which(name)) for name in ("sha256sum", "shasum", "openssl")]
        selected = next(((name, path) for name, path in available if path), None)
        if selected is None:
            self.fail("SHA-256 fixture prerequisite is missing")
        name, executable = selected
        # Both paths are passed through environment; never interpolate data as code.
        stub = binary / name
        stub.write_text('#!/bin/sh\nprintf x >> "$HASH_MARKER"\n"$REAL_HASH" "$@"\nexit 7\n')
        stub.chmod(0o700)
        self.environment.update(PATH=str(binary) + os.pathsep + os.environ["PATH"],
                                HASH_MARKER=str(self.marker), REAL_HASH=executable)

    def test_valid_private_aliases(self):
        self.probe("aliases", True)

    def test_fish_hash_selection_does_not_autoload_shell_functions(self):
        if SHELL != "fish":
            self.skipTest("Fish autoload contract")
        autoload = self.directory / "autoload"
        autoload.mkdir()
        marker = self.directory / "autoload-called"
        self.environment["HASH_AUTOLOAD_MARKER"] = str(marker)
        for name in ("sha256sum", "shasum", "openssl"):
            (autoload / (name + ".fish")).write_text(
                'printf x >>"$HASH_AUTOLOAD_MARKER"\n'
                f'function {name}\n    command {name} $argv\nend\n')
        source = ROOT / "shell-integration/fish/automexia.fish"
        program = ('set -p fish_function_path "$argv[2]"; '
                   'source "$argv[1]" >/dev/null; '
                   'test "$__automexia_alias_state" = ready; or exit 1; '
                   'contains -- kubectl $__automexia_completion_loaded; or exit 1; '
                   '__automexia_alias_hash_file "$argv[3]" >/dev/null; or exit 1; '
                   'printf fixture | __automexia_alias_hash_stream >/dev/null; or exit 1')
        self.run_probe(["fish", "--no-config", "-c", program, str(source), str(autoload),
                        str(self.alias_file)], True)
        self.assertFalse(marker.exists(), "hash lookup autoloaded a shell function")

    def test_valid_completion(self):
        self.probe("completion", True)

    def test_reload_preserves_a_changed_managed_definition(self):
        source = ROOT / f"shell-integration/{SHELL}/automexia.{SHELL}"
        if SHELL == "fish":
            program = ('source "$argv[1]" >/dev/null; function axt; printf changed; end; '
                       'if automexia_aliases_reload >/dev/null; exit 2; end; test (axt) = changed')
            command = [SHELL, "--no-config", "-c", program, str(source)]
        else:
            program = ('source "$1" >/dev/null; alias axt="printf changed"; '
                       'if automexia_aliases_reload >/dev/null; then exit 2; fi; '
                       '[[ $(builtin alias axt) == *changed* ]]')
            command = [SHELL, "-f" if SHELL == "zsh" else "--noprofile", "-c", program,
                       "artifact-fixture", str(source)]
        self.run_probe(command, True)

    def test_new_generation_does_not_remove_a_changed_managed_function(self):
        # Give Bash/Zsh the private function emitted for a structured alias.
        # Keep the content-addressed fixture internally consistent.
        name = "axt" if SHELL == "fish" else "_automexia_cp3_0123456789abcdef"
        if SHELL != "fish":
            body = (f"{name}() {{ printf first; }}\nalias axt='{name}'\n").encode()
            self.alias_file.write_bytes(body)
            lines = self.manifest.read_text().splitlines()
            index = 6 if SHELL == "bash" else 7
            fields = lines[index].split("|")
            fields[2] = hashlib.sha256(body).hexdigest()
            lines[index] = "|".join(fields)
            data = ("\n".join(lines) + "\n").encode()
            self.manifest.write_bytes(data)
            destination = self.generation.parent / hashlib.sha256(data).hexdigest()
            self.assertTrue(self.generation.resolve().is_relative_to(self.directory.resolve()))
            self.assertTrue(destination.resolve().is_relative_to(self.directory.resolve()))
            self.generation.rename(destination)
            (self.alias_root / "current").write_text(destination.name + "\n")
        source = ROOT / f"shell-integration/{SHELL}/automexia.{SHELL}"
        creator = ROOT / "tools/ci/create_cp31_alias_fixture.py"
        if SHELL == "fish":
            program = ('source "$argv[1]" >/dev/null; function axt; printf changed; end; '
                       'python3 "$argv[2]" --config-root "$AUTOMEXIA_CONFIG_HOME" --alias ayt --value second >/dev/null; or exit 3; '
                       'if automexia_aliases_reload >/dev/null; exit 2; end; '
                       'not functions --query ayt; and test (axt) = changed')
            command = [SHELL, "--no-config", "-c", program, str(source), str(creator)]
        else:
            program = ('source "$1" >/dev/null; _automexia_cp3_0123456789abcdef() { printf changed; }; '
                       'python3 "$2" --config-root "$AUTOMEXIA_CONFIG_HOME" --alias ayt --value second >/dev/null || exit 3; '
                       'if automexia_aliases_reload >/dev/null; then exit 2; fi; '
                       '! builtin alias ayt >/dev/null 2>&1 && [[ $(_automexia_cp3_0123456789abcdef) == changed ]]')
            command = [SHELL, "-f" if SHELL == "zsh" else "--noprofile", "-c", program,
                       "artifact-fixture", str(source), str(creator)]
        self.run_probe(command, True)

    def test_alias_artifact_tampering(self):
        self.alias_file.write_bytes(self.alias_file.read_bytes() + b"# changed\n")
        self.probe("aliases", False)

    def test_alias_manifest_tampering(self):
        self.manifest.write_bytes(self.manifest.read_bytes() + b"# changed\n")
        self.probe("aliases", False)

    def test_digest_valid_manifest_rejects_malformed_records(self):
        original = self.manifest.read_bytes()
        extra_fields = original.decode().splitlines()
        index = {"bash": 6, "zsh": 7, "fish": 8}[SHELL]
        extra_fields[index] += "|"
        for kind, data in (("missing terminator", original.rstrip(b"\n")),
                           ("extra record", original + b"\n"), ("trailing NUL", original + b"\0"),
                           ("extra shell field", ("\n".join(extra_fields) + "\n").encode())):
            with self.subTest(kind=kind):
                self.manifest.write_bytes(data)
                destination = self.generation.parent / hashlib.sha256(data).hexdigest()
                self.assertTrue(self.generation.resolve().is_relative_to(self.directory.resolve()))
                self.assertTrue(destination.resolve().is_relative_to(self.directory.resolve()))
                self.generation.rename(destination)
                self.generation = destination
                self.manifest = destination / "generation.manifest"
                (self.alias_root / "current").write_text(destination.name + "\n")
                self.probe("aliases", False)

    def test_alias_permissions_are_rechecked(self):
        for path in (self.config / "generated", self.alias_root,
                     self.alias_root / "generations", self.alias_root / "current",
                     self.generation, self.alias_file.parent, self.manifest, self.alias_file):
            with self.subTest(entry=path.name):
                mode = path.stat().st_mode
                path.chmod(0o777)
                self.probe("aliases", False)
                path.chmod(mode)

    def test_alias_pointer_rejects_trailing_bytes(self):
        pointer = self.alias_root / "current"
        original = pointer.read_bytes()
        for tail in (b"extra", b"\n", b"\0"):
            with self.subTest(tail=tail):
                pointer.write_bytes(original + tail)
                self.probe("aliases", False)

    def test_alias_limits_precede_hashing(self):
        # Remove the unrelated completion fixture so the marker observes only aliases.
        self.completion.unlink()
        self.hash_failure()
        for path, ceiling in ((self.manifest, 65536), (self.alias_file, 1114112)):
            with self.subTest(ceiling=ceiling):
                original = path.read_bytes()
                path.write_bytes(b"#" * (ceiling + 1))
                self.probe("aliases", False)
                self.assertFalse(self.marker.exists(), "oversized artifact reached hashing")
                path.write_bytes(original)

    def test_alias_hash_failure_rejects_even_matching_output(self):
        self.hash_failure()
        self.probe("aliases", False)

    def test_completion_limits_precede_hashing(self):
        self.hash_failure()
        for path, ceiling in ((self.completion, 1114112), (self.digest, 192)):
            with self.subTest(ceiling=ceiling):
                original = path.read_bytes()
                path.write_bytes(b"#" * (ceiling + 1))
                self.probe("completion", False)
                self.assertFalse(self.marker.exists(), "oversized artifact reached hashing")
                path.write_bytes(original)

    def test_completion_hash_failure_rejects_even_matching_output(self):
        self.hash_failure()
        self.probe("completion", False)

    def test_completion_tampering(self):
        self.completion.write_bytes(self.completion.read_bytes() + b"# changed\n")
        self.probe("completion", False)

    def test_linked_artifact_is_rejected(self):
        for owner, path in (("aliases", self.alias_file), ("completion", self.completion)):
            with self.subTest(owner=owner):
                target = self.directory / (owner + "-target")
                target.write_bytes(path.read_bytes())
                target.chmod(0o600)
                path.unlink()
                path.symlink_to(target)
                self.probe(owner, False)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--shell", required=True, choices=("bash", "zsh", "fish"))
    args, remaining = parser.parse_known_args()
    SHELL = args.shell
    if os.name != "posix" or shutil.which(SHELL) is None:
        parser.error("Requested native shell is unavailable")
    unittest.main(argv=[sys.argv[0], *remaining])
