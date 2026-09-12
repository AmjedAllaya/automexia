"""Independent ingress tests; live Linux lifecycle checks use their native runner."""
import importlib.util
import json
from pathlib import Path
import sys
import unittest

SOURCE = Path(__file__).resolve().parents[2] / "shell-integration/amx-tool-bridge.py"
SPEC = importlib.util.spec_from_file_location("amx_guest_bridge", SOURCE)
BRIDGE = importlib.util.module_from_spec(SPEC)
# Import the package-owned supervisor without leaving bytecode in the shipped
# shell source tree. Its source-security scan deliberately rejects binary files.
_previous_bytecode_policy = sys.dont_write_bytecode
try:
    sys.dont_write_bytecode = True
    SPEC.loader.exec_module(BRIDGE)
finally:
    sys.dont_write_bytecode = _previous_bytecode_policy


class GuestBridgeTests(unittest.TestCase):
    def test_directory_probe_accepts_one_literal_path_not_source_or_extra_arguments(self):
        request = dict(version=1, program="amx-directory", arguments=["folder & café"], timeout_seconds=10)
        self.assertEqual(BRIDGE.validate_request(json.dumps(request)), request)
        for args in ([], [""], ["one", "two"], ["bad\npath"]):
            with self.assertRaises(ValueError):
                BRIDGE.validate_request(json.dumps(dict(request, arguments=args)))

    def test_exact_request_and_literal_arguments(self):
        request = dict(version=1, program="rg", arguments=["--fixed-strings", "--", "$(fixture); & text"], timeout_seconds=10)
        self.assertEqual(BRIDGE.validate_request(json.dumps(request)), request)

    def test_hostile_and_over_limit_requests(self):
        valid = dict(version=1, program="rg", arguments=["--files"], timeout_seconds=10)
        for field, value in [("version", True), ("version", 2), ("program", "sh"), ("program", "../rg"), ("program", []), ("arguments", "text"), ("arguments", ["x"] * 257), ("arguments", ["x" * 4097]), ("arguments", ["x\ny"]), ("arguments", [0]), ("timeout_seconds", 0), ("timeout_seconds", 61), ("timeout_seconds", True)]:
            with self.subTest(field=field):
                request = dict(valid, **{field: value})
                with self.assertRaises((ValueError, TypeError)):
                    BRIDGE.validate_request(json.dumps(request))
        for raw in ["{}", "null", "[]", "broken", json.dumps(dict(valid, extra=True)), " " * 16385]:
            with self.assertRaises((ValueError, TypeError)):
                BRIDGE.validate_request(raw)

    def test_boundary_deadlines_and_arguments(self):
        for seconds in (1, 60):
            for args in (["x"] * 256, ["é" * 2048]):
                request = dict(version=1, program="tldr", arguments=args, timeout_seconds=seconds)
                self.assertEqual(BRIDGE.validate_request(json.dumps(request, ensure_ascii=False)), request)

    def test_duplicate_fields_cannot_change_the_reviewed_request(self):
        with self.assertRaises(ValueError):
            BRIDGE.validate_request('{"version":1,"program":"rg","program":"tldr","arguments":[],"timeout_seconds":1}')


if __name__ == "__main__":
    unittest.main()
