"""Native CMD prompt expansion, with fictional identity and no installed profile."""
import base64
import hashlib
import os
from pathlib import Path
import re
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


@unittest.skipUnless(os.name == "nt", "requires native Windows CMD")
class CmdPromptTests(unittest.TestCase):
    def test_interactive_prompt_preserves_input_marker_for_long_unicode_identity(self):
        for glyph in (None, ">", "\u03bb"):
            with self.subTest(glyph=glyph or "default"):
                self.check_prompt(glyph)

    def test_interactive_prompt_keeps_maximum_identity_registration_complete(self):
        self.check_prompt(">", maximum=True)

    def check_prompt(self, glyph, maximum=False):
        # CMD's actual prompt renderer truncates its format at 511 UTF-16 units.
        # Echoing %PROMPT% in a /C batch does not exercise that behavior.
        user = "fixture-" + "\u00e9\u754c" * 40
        executable = "C:\\AutomexiaFixtures\\" + "nested-\u00e9\u754c" * 50 + "cmd.exe"
        if maximum:
            user, executable = "u" * 4096, "p" * 4096
        encode = lambda value: base64.b64encode(value.encode("utf-8")).decode("ascii")
        reference = hashlib.sha256((user + "\0" + executable).encode()).hexdigest()[:32]
        with tempfile.TemporaryDirectory(prefix="automexia-cmd-prompt-") as folder:
            fixture = Path(folder)
            # Only the alias loader is a fixture. The tracked CMD startup and
            # native interactive prompt expansion both run unchanged.
            source = (ROOT / "shell-integration/cmd/automexia.cmd").read_text()
            (fixture / "automexia.cmd").write_bytes(source.replace("\r\n", "\n").replace("\n", "\r\n").encode("ascii"))
            (fixture / "automexia-alias-loader.ps1").write_text("Write-Output 'UNSAFE_PERMISSIONS|||'", encoding="ascii")
            (fixture / "next").mkdir()
            env = dict(os.environ)
            env.update(TERM_PROGRAM="Automexia", AUTOMEXIA_PLAIN_LS="1", AUTOMEXIA_AMX="0",
                       AUTOMEXIA_CMD_USER_BASE64=encode(user), AUTOMEXIA_CMD_PATH_BASE64=encode(executable),
                       AUTOMEXIA_CMD_REFERENCE_V1="1", AUTOMEXIA_CMD_REFERENCE=reference,
                       AUTOMEXIA_CMD_REFERENCE_BASE64=encode(reference),
                       AUTOMEXIA_CONFIG_HOME=str(fixture / "empty-config"))
            env.pop("AUTOMEXIA_CMD_PROMPT_GLYPH", None)
            if glyph is not None:
                env["AUTOMEXIA_CMD_PROMPT_GLYPH"] = glyph
            command = Path(os.environ["SystemRoot"]) / "System32/cmd.exe"
            result = subprocess.run([str(command), "/D", "/Q", "/K", "chcp 65001>nul & automexia.cmd"],
                                    cwd=fixture, env=env, input=b"cd next\r\nset PROMPT\r\nexit\r\n",
                                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=20,
                                    creationflags=subprocess.CREATE_NO_WINDOW)
            self.assertEqual(result.returncode, 0)
            self.assertEqual(result.stdout.count(b"\x1b]133;B\x1b\\"), 3,
                             "native CMD truncated or omitted the input marker")
            self.assertEqual(result.stdout.count(b"\x1b]133;A\x1b\\"), 3)
            formats = re.findall(rb"PROMPT=([^\r\n]*)", result.stdout)
            self.assertEqual(len(formats), 1)
            self.assertLess(len(formats[0].decode("utf-8").encode("utf-16-le")) // 2, 511)
            self.assertTrue((glyph or ">").encode("utf-8") + b"\x1b[0m \x1b]133;B" in result.stdout,
                            "input glyph or marker is missing")
            self.assertTrue(b"]7;file:///" + str(fixture / "next").encode() in result.stdout,
                            "directory change did not update OSC 7")
            self.assertEqual(result.stdout.count(b"SetUserVar=automexia_cmd_ref_v1=" + encode(reference).encode()), 5)
            self.assertTrue(b"SetUserVar=automexia_cmd_user_v1_" + reference.encode() + b"=" + encode(user).encode() in result.stdout,
                            "user registration is missing or truncated")
            self.assertTrue(b"SetUserVar=automexia_cmd_path_v1_" + reference.encode() + b"=" + encode(executable).encode() in result.stdout,
                            "executable registration is missing or truncated")


if __name__ == "__main__":
    unittest.main()
