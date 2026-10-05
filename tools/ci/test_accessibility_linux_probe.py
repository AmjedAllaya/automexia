#!/usr/bin/env python3
"""Check the native probe's stale-object handling without claiming native evidence."""
from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
from types import ModuleType, SimpleNamespace
import unittest
from unittest.mock import patch


class NativeError(Exception):
    def __init__(self, message, domain="atspi_error", code=0):
        super().__init__(message)
        self.message, self.domain, self.code = message, domain, code


def load_probe():
    spec = importlib.util.spec_from_file_location("native_linux_probe",
        Path(__file__).resolve().parents[2] / "tests/integration/accessibility-linux.py")
    module = importlib.util.module_from_spec(spec)
    pyatspi = ModuleType("pyatspi")
    pyatspi.STATE_DEFUNCT, pyatspi.STATE_ENABLED, pyatspi.STATE_SENSITIVE = range(3)
    gi = ModuleType("gi")
    gi.__path__ = []
    repository = ModuleType("gi.repository")
    repository.Atspi = SimpleNamespace()
    repository.GLib = SimpleNamespace(Error=NativeError)
    with patch.dict(sys.modules, {"pyatspi": pyatspi, "gi": gi, "gi.repository": repository}):
        spec.loader.exec_module(module)
    return module


probe = load_probe()


class LinuxProbeContract(unittest.TestCase):
    def wait(self, reads):
        process = SimpleNamespace(pid=42, poll=lambda: None)
        with patch.object(probe, "owned_application", return_value=object()), \
             patch.object(probe, "read_owned", side_effect=reads) as reader, \
             patch.object(probe.time, "sleep"), \
             patch.object(probe.time, "monotonic", side_effect=range(100)):
            result = probe.wait_for(process, lambda nodes: nodes == ["ready"], "deadline")
            return result, reader.call_count

    def test_explicitly_retired_objects_retry_within_the_original_deadline(self):
        for error in (probe.ReplacedTree(), NativeError("The application no longer exists")):
            with self.subTest(error=type(error).__name__):
                self.assertEqual(self.wait([error, ["ready"]]), (["ready"], 2))

    def test_unexpected_native_errors_and_invalid_data_are_not_ignored(self):
        for error in (NativeError("malformed body"),
                      NativeError("The application no longer exists", code=1),
                      NativeError("The application no longer exists", domain="different"),
                      probe.ProbeFailure("oversized tree")):
            with self.subTest(error=str(error)), self.assertRaises(type(error)):
                self.wait([error, ["ready"]])

    def test_permanent_provider_loss_still_fails_at_the_deadline(self):
        with self.assertRaisesRegex(probe.ProbeFailure, "deadline"):
            self.wait([NativeError("The application no longer exists")] * 30)

    def test_oversized_and_negative_child_counts_fail_without_defunct_state(self):
        state = SimpleNamespace(contains=lambda _state: False)
        for count in (-1, probe.LIMIT + 1):
            node = SimpleNamespace(name="fixture", getRole=lambda: 0,
                getState=lambda: state, get_interfaces=lambda: [], childCount=count)
            with self.subTest(count=count), self.assertRaisesRegex(probe.ProbeFailure, "child count"):
                probe.read_owned(node)

    def test_concealed_labels_cannot_be_hidden_by_the_stale_retry(self):
        state = SimpleNamespace(contains=lambda _state: False)
        node = SimpleNamespace(name=probe.HIDDEN, getRole=lambda: 0,
            getState=lambda: state, get_interfaces=lambda: [], childCount=0)
        with self.assertRaisesRegex(probe.ProbeFailure, "Concealed"):
            probe.read_owned(node)


if __name__ == "__main__":
    unittest.main()
