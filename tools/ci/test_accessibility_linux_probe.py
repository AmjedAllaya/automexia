#!/usr/bin/env python3
"""Check the native probe's stale-object handling without claiming native evidence."""
from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import sys
import tempfile
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
    def test_concealed_pixel_oracle_requires_blank_hidden_and_nonblank_visible_rows(self):
        state = {'panels': [{'grid_origin': [4, 44], 'cell_width': 8, 'cell_height': 24}]}
        with patch.object(probe, 'command', side_effect=['20', '1']):
            probe.verify_concealed_pixels(Path('fixture.png'), state, 800, 600)
        for counts in (['20', '2'], ['1', '1'], ['bad', '1']):
            with self.subTest(counts=counts), patch.object(probe, 'command', side_effect=counts), self.assertRaises(probe.ProbeFailure):
                probe.verify_concealed_pixels(Path('fixture.png'), state, 800, 600)

    def test_native_capture_requires_matching_control_and_two_stable_images(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            state = {'sequence': 7, 'last_control': 'ready', 'renderer_backend': 'cpu', 'scale_factor': 1.0}
            snapshot = root / 'snapshot.json'
            snapshot.write_text(json.dumps(state))
            calls = []
            def native(args):
                calls.append(args[0])
                if args[0] == '/usr/bin/import':
                    (root / 'case.pending.png').write_bytes(b'fixed-pixels')
                return '80 24'
            with patch.object(probe, 'command', side_effect=native), patch.object(probe.time, 'sleep'):
                probe.capture_window('1', snapshot, root, 'case', 'ready', 'fixture')
            self.assertEqual(calls.count('/usr/bin/import'), 2)
            report = json.loads((root / 'case.json').read_bytes())
            self.assertEqual((report['frame_generation'], report['width']), (7, 80))
            self.assertNotIn(str(root), json.dumps(report))
            with self.assertRaises(probe.ProbeFailure):
                probe.capture_window('1', snapshot, root, 'case', 'ready', 'fixture')
            with self.assertRaises(probe.ProbeFailure):
                probe.capture_bytes(snapshot, 1)

    def test_stale_presented_control_cannot_publish_pixels(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            snapshot = root / 'snapshot.json'
            snapshot.write_text(json.dumps({'last_control': 'old'}))
            with patch.object(probe, 'command') as native, patch.object(probe.time, 'sleep'), \
                 patch.object(probe.time, 'monotonic', side_effect=range(20)), self.assertRaises(probe.ProbeFailure):
                probe.capture_window('1', snapshot, root, 'case', 'new', 'fixture')
            native.assert_not_called()
            self.assertFalse((root / 'case.png').exists())

    def test_scaling_changes_during_capture_do_not_publish_misattributed_pixels(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            snapshot = root / 'snapshot.json'
            state = {'sequence': 7, 'last_control': 'ready', 'renderer_backend': 'cpu', 'scale_factor': 1.0}
            snapshot.write_text(json.dumps(state))
            def native(_args):
                (root / 'case.pending.png').write_bytes(b'fixed-pixels')
                state['scale_factor'] = 3.0 - state['scale_factor']
                snapshot.write_text(json.dumps(state))
                return ''
            with patch.object(probe, 'command', side_effect=native), patch.object(probe.time, 'sleep'), \
                 patch.object(probe.time, 'monotonic', side_effect=range(20)), self.assertRaises(probe.ProbeFailure):
                probe.capture_window('1', snapshot, root, 'case', 'ready', 'fixture')
            self.assertFalse((root / 'case.png').exists())

    def test_shell_echo_cannot_satisfy_or_violate_output_oracles(self):
        command = probe.shell_fixture_command()
        self.assertNotIn(probe.FIXTURE, command)
        self.assertNotIn(probe.HIDDEN, command)
        self.assertLess(len(command), 2048)
        payload = command.removeprefix("printf '%b' '").removesuffix("'")
        octets = payload.split('\\0')[1:]
        self.assertEqual(bytes(int(value, 8) for value in octets).decode(),
                         probe.FIXTURE + '\n\x1b[8m' + probe.HIDDEN + '\x1b[0m\n')

    def wait(self, reads):
        process = SimpleNamespace(pid=42, poll=lambda: None)
        with patch.object(probe, "owned_application", return_value=object()), \
             patch.object(probe, "read_owned", side_effect=reads) as reader, \
             patch.object(probe.time, "sleep"), \
             patch.object(probe.time, "monotonic", side_effect=range(100)):
            result = probe.wait_for(process, lambda nodes: nodes == ["ready"], "deadline")
            return result, reader.call_count

    def test_explicitly_retired_objects_retry_within_the_original_deadline(self):
        for error in (probe.ReplacedTree(), NativeError("The application no longer exists"),
                      NativeError("Unknown object '/org/a11y/atspi/accessible/0/590295810358705651712'", code=1)):
            with self.subTest(error=type(error).__name__):
                self.assertEqual(self.wait([error, ["ready"]]), (["ready"], 2))

    def test_unexpected_native_errors_and_invalid_data_are_not_ignored(self):
        for error in (NativeError("malformed body"),
                      NativeError("The application no longer exists", code=1),
                      NativeError("The application no longer exists", domain="different"),
                      NativeError("Unknown object '/unrelated/0/1'", code=1),
                      NativeError("Unknown object '/org/a11y/atspi/accessible/0/1'", code=0),
                      probe.ProbeFailure("oversized tree")):
            with self.subTest(error=str(error)), self.assertRaises(type(error)):
                self.wait([error, ["ready"]])

    def test_permanent_provider_loss_still_fails_at_the_deadline(self):
        with self.assertRaisesRegex(probe.ProbeFailure, "deadline"):
            self.wait([NativeError("The application no longer exists")] * 30)
        with self.assertRaisesRegex(probe.ProbeFailure, "deadline"):
            self.wait([NativeError("Unknown object '/org/a11y/atspi/accessible/0/1'", code=1)] * 30)

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
