"""Collector isolation, workload identity, native resources and failure coverage."""
from pathlib import Path
import os
import io
import subprocess
import sys
import time
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'renderer-benchmarks/application'))
import collector
import native
from macos_driver import MacDriver
import workload
from benchmark_model import BenchmarkError, METRICS


def unix_ui_probe():
    import importlib.util
    source = Path(__file__).resolve().parents[2] / 'tests/integration/unix-session-ui.py'
    spec = importlib.util.spec_from_file_location('unix_ui_fixture', source)
    probe = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(probe)
    return probe


class CollectorTests(unittest.TestCase):
    def test_native_wait_wakes_pending_frames_only_and_keeps_its_deadline(self):
        probe = unix_ui_probe()
        frames = iter([{"sequence": 1, "settings": {"ready": False}}, {"sequence": 2, "settings": {"ready": True}}])
        wakes = []
        with patch.object(probe.time, "monotonic", side_effect=[0.0, .01, .1]), \
                patch.object(probe.time, "sleep"):
            actual = probe.wait_for_native_state(lambda: next(frames),
                lambda s: s["settings"]["ready"], "fixture wait", 1.0,
                lambda: wakes.append(True))
        self.assertTrue(actual["settings"]["ready"])
        self.assertEqual(len(wakes), 1)
        wakes.clear()
        with patch.object(probe.time, "monotonic", side_effect=[0.0, .01, 1.0]), \
                patch.object(probe.time, "sleep"), self.assertRaises(probe.Failure) as failure:
            probe.wait_for_native_state(lambda: {"sequence": 1, "settings": {"ready": False}},
                lambda s: s["settings"]["ready"], "fixture deadline", 1.0,
                lambda: wakes.append(True))
        self.assertEqual(len(wakes), 1)
        self.assertFalse(failure.exception.observed["settings"]["ready"])

    def test_native_startup_wait_does_not_wake_an_unmapped_window(self):
        probe = unix_ui_probe()
        for initial in ({}, {"sequence": 0}):
            frames = iter([initial, {"sequence": 1}])
            with patch.object(probe.time, "sleep"), patch.object(probe.time, "monotonic", side_effect=[0, .01, .1]):
                def premature_wake():
                    self.fail("pointer wake attempted before the first native frame")
                state = probe.wait_for_native_state(lambda: next(frames),
                    lambda s: s.get("sequence", 0) > 0, "startup", 30, premature_wake)
            self.assertEqual(state["sequence"], 1)

    def test_native_failure_state_keeps_modal_readiness_without_user_values(self):
        import json
        probe = unix_ui_probe()
        state = probe.native_failure_state({"settings": {
            "open": True, "ready": False, "list_focused": True,
            "save_pending": "private path", "query": "private query",
            "confirmation": {"title": "private title"}}})
        self.assertEqual(state["settings"], {"open": True, "ready": False,
                         "list_focused": True, "confirmation_visible": True})
        self.assertNotIn("private", json.dumps(state))

    def test_native_color_cancellation_diagnostics_are_bounded_integers_only(self):
        probe = unix_ui_probe()
        keys = ("catalog_revision", "last_color_dismissal", "previous_color_dismissal", "color_open_generation")
        for value in (None, True, -1, 2**64, "private path", {"draft": "private"}):
            state = probe.native_failure_state({"settings": dict.fromkeys(keys, value)})["settings"]
            for key in keys:
                self.assertNotIn(key, state)
        state = probe.native_failure_state({"settings": {
            "catalog_revision": 7, "last_color_dismissal": 1900}})["settings"]
        self.assertEqual(state["catalog_revision"], 7)
        self.assertEqual(state["last_color_dismissal"], 1900)

    def test_native_picker_trace_retains_bounded_redacted_transitions(self):
        import json
        probe = unix_ui_probe()
        history = []
        probe.record_picker_state(history, {"settings": {"catalog_revision": 1}})
        self.assertEqual(history, [])
        for revision in range(40):
            state = {"settings": {"catalog_revision": revision, "color_open_generation": 2,
                "previous_color_dismissal": 100, "query": "private query", "color_editor": {
                    "draft_color": "private color", "palette": {"focus": "Swatch(0)", "browsing": True}}}}
            probe.record_picker_state(history, state)
            probe.record_picker_state(history, state)
        self.assertEqual(len(history), 32)
        self.assertEqual(history[0]["catalog_revision"], 8)
        self.assertEqual(history[-1]["previous_color_dismissal"], 100)
        probe.record_picker_state(history, {"settings": {"catalog_revision": 40, "last_color_dismissal": 200}})
        self.assertNotIn("color_focus", history[-1])
        error = probe.Failure("fixed picker failure")
        error.picker_history = history
        report = probe.failure_details(error)
        self.assertEqual(report["picker_history"], history)
        self.assertNotIn("private", json.dumps(report))

    def test_native_color_failure_keeps_only_fixed_focus_and_browsing(self):
        import json
        probe = unix_ui_probe()
        for focus in ('Hex', 'Suggested', 'Swatch(0)', 'Swatch(128)', 'private text', 'Swatch(1234)'):
            state = probe.native_failure_state({'settings': {'color_editor': {
                'draft_color': 'private color', 'palette': {'focus': focus, 'browsing': True,
                                                          'controls': ['private values']}}}})
            self.assertEqual(state['settings']['color_browsing'], True)
            if focus in ('private text', 'Swatch(1234)'):
                self.assertNotIn('color_focus', state['settings'])
            else:
                self.assertEqual(state['settings']['color_focus'], focus)
            self.assertNotIn('private', json.dumps(state))

    def test_native_failure_appearance_rejects_private_or_nonfinite_values(self):
        probe = unix_ui_probe()
        source = {"appearance": {"window_alpha": 1.0, "clear": [0.1, 0.2, 0.3, 0.5], "path": "private"}}
        self.assertEqual(probe.native_failure_state(source)["appearance"],
                         {"window_alpha": 1.0, "clear": [0.1, 0.2, 0.3, 0.5]})
        for clear in ([0, 0, 0, float("nan")], [0, 0, 0, "private"], [1, 2, 3], [0, 0, 0, True]):
            self.assertEqual(probe.native_failure_state({"appearance": {
                "window_alpha": "private", "clear": clear}})["appearance"], {})

    def test_native_pixel_reader_converts_display_profile_before_sampling(self):
        probe = unix_ui_probe()
        tail = ['-crop', '1x1+80+60', '-depth', '8', 'rgb:-']
        self.assertEqual(probe.native_pixel_args('magick', Path('fixture.png'), 80, 60, macos=False),
                         ['magick', 'fixture.png'] + tail)
        self.assertEqual(probe.native_pixel_args('magick', Path('fixture.png'), 80, 60, macos=True),
                         ['magick', 'fixture.png', '-profile',
                          '/System/Library/ColorSync/Profiles/sRGB Profile.icc'] + tail)

    def test_presented_pixels_wait_for_compositor_without_accepting_persistent_errors(self):
        probe = unix_ui_probe()
        expected = [2, 11, 22]
        frames = iter([[2, 6, 14], expected])
        with patch.object(probe.time, 'monotonic', side_effect=[0.0, 0.1]), \
                patch.object(probe.time, 'sleep') as sleep:
            self.assertEqual(probe.presented_rgb(lambda: next(frames), expected, 6), expected)
            sleep.assert_called_once_with(.05)
        wrong = [22, 59, 83]
        with patch.object(probe.time, 'monotonic', side_effect=[0.0, 0.1, 6.0]), \
                patch.object(probe.time, 'sleep') as sleep:
            self.assertEqual(probe.presented_rgb(lambda: wrong, expected, 6), wrong)
            sleep.assert_called_once_with(.05)
        with patch.object(probe.time, 'monotonic', return_value=0.0), \
                patch.object(probe.time, 'sleep') as sleep:
            self.assertEqual(probe.presented_rgb(lambda: expected, expected, 6), expected)
            sleep.assert_not_called()

    def test_native_appearance_profile_isolation_and_cleanup_on_success_and_failure(self):
        probe = unix_ui_probe()
        with TemporaryDirectory() as parent:
            root = Path(parent)
            normal = root / 'normal-profile'
            normal.mkdir()
            sentinel = normal / 'user-preferences-v16.toml'
            sentinel.write_bytes(b'visual preferences must survive unchanged')
            original = sentinel.read_bytes()
            for fail in (False, True):
                with patch.dict(os.environ, {'AUTOMEXIA_CONFIG_HOME': str(normal),
                                'RIO_CONFIG_HOME': str(normal), 'XDG_STATE_HOME': str(normal),
                                'AUTOMEXIA_VISUAL_TEST_FIXTURE': 'inherited',
                                'AUTOMEXIA_BENCHMARK_TRACE': str(normal / 'trace'),
                                'FAKE_CREDENTIAL': 'must-not-be-inherited'}):
                    try:
                        with probe.fixture_workspace() as (temporary, diagnostics):
                            home = Path(temporary)
                            config = home / 'config'
                            config.mkdir()
                            env = probe.fixture_environment(home, config, '/bin/bash', home/'control', home/'snapshot', 1.0)
                            self.assertEqual(env['AUTOMEXIA_CONFIG_HOME'], str(config))
                            for name in ('RIO_CONFIG_HOME', 'HOME', 'USERPROFILE', 'LOCALAPPDATA', 'APPDATA',
                                         'XDG_CONFIG_HOME', 'XDG_DATA_HOME', 'XDG_STATE_HOME', 'XDG_CACHE_HOME',
                                         'TMPDIR', 'TMP', 'TEMP'):
                                self.assertTrue(Path(env[name]).is_relative_to(home), name)
                            for name in ('FAKE_CREDENTIAL', 'AUTOMEXIA_VISUAL_TEST_FIXTURE', 'AUTOMEXIA_BENCHMARK_TRACE'):
                                self.assertNotIn(name, env)
                            state = config / 'state'
                            state.mkdir()
                            for name in ('user-preferences-v16.toml', 'user-preferences-v16.previous.toml'):
                                (state/name).write_text('[visual.interface]\nopacity = 0\n')
                            if fail:
                                raise probe.Failure('injected native failure')
                    except probe.Failure:
                        self.assertTrue(fail)
                    self.assertTrue(diagnostics['removed'])
                    self.assertFalse(home.exists())
                    self.assertEqual(sentinel.read_bytes(), original)
                    self.assertEqual(os.environ['AUTOMEXIA_CONFIG_HOME'], str(normal))

    def test_macos_caption_probe_is_owned_bounded_and_releases_references(self):
        driver = object.__new__(MacDriver)
        driver.process = SimpleNamespace(pid=42)
        driver.window = 'window'
        driver.check = lambda: None
        driver.string = lambda value: value
        released, pressed = [], []
        tree = {'window': {'AXChildren': ['button']},
                'button': {'AXTitle': 'Close window', 'AXRole': 'AXButton'}}
        driver.copy = lambda element, name, optional=False: tree[element].get(name)
        driver.cf = SimpleNamespace(CFArrayGetCount=len,
            CFArrayGetValueAtIndex=lambda values, index: values[index],
            CFRetain=lambda value: value, CFRelease=released.append, CFEqual=lambda a, b: a == b)
        def owner(element, output):
            output._obj.value = tree[element].get('pid', 42)
            return 0
        driver.ax = SimpleNamespace(AXUIElementGetPid=owner)
        driver.action = lambda element, name: pressed.append((element, name))
        self.assertTrue(driver.caption('Close window', press=True))
        self.assertEqual(pressed, [('button', 'AXPress')])
        self.assertIn('button', released)
        pressed.clear()
        for mutation in ('foreign', 'ambiguous', 'oversized'):
            if mutation == 'foreign':
                tree['button']['pid'] = 43
            else:
                tree['button']['pid'] = 42
                tree['window']['AXChildren'] = ['button'] * (2 if mutation == 'ambiguous' else 1025)
            with self.assertRaises(BenchmarkError):
                driver.caption('Close window', press=True)
            self.assertEqual(pressed, [])
        with self.assertRaises(BenchmarkError):
            driver.caption('Run command', press=True)

    def test_macos_pointer_window_lookup_rejects_foreign_or_ambiguous_owners(self):
        import ctypes
        import macos_driver

        driver = object.__new__(MacDriver)
        driver.process = SimpleNamespace(pid=42)
        driver.check = lambda: None
        released = []
        def number(pointer, kind, output):
            self.assertEqual(kind, 4)
            output._obj.value = pointer[0]
            return True
        driver.cf = SimpleNamespace(CFArrayGetCount=len,
            CFArrayGetValueAtIndex=lambda values, index: values[index],
            CFDictionaryGetValue=lambda record, key: (record[key],),
            CFNumberGetValue=number, CFRelease=released.append)
        fake_ct = SimpleNamespace(c_void_p=SimpleNamespace(in_dll=lambda library, name: SimpleNamespace(value=name)),
                                  c_int64=ctypes.c_int64, byref=ctypes.byref)
        def record(pid, layer, number):
            return dict(kCGWindowOwnerPID=pid, kCGWindowLayer=layer, kCGWindowNumber=number)
        own, foreign, overlay = record(42, 0, 7), record(43, 0, 8), record(42, 1, 9)
        with patch.object(macos_driver, 'ct', fake_ct):
            for records, expected in [([own, foreign, overlay], 7), ([foreign], None),
                                      ([own, dict(own)], None), ([record(42, 0, 0)], None),
                                      ([foreign] * 513, None)]:
                driver.cg = SimpleNamespace(CGWindowListCopyWindowInfo=lambda options, relative: records)
                if expected is None:
                    with self.assertRaises(BenchmarkError):
                        driver.owned_window_number()
                else:
                    self.assertEqual(driver.owned_window_number(), expected)
                self.assertIs(released[-1], records)
        self.assertEqual(len(released), 5)

    def test_macos_pointer_hit_test_rejects_foreign_points_and_releases_handles(self):
        from macos_driver import Point
        driver = object.__new__(MacDriver)
        driver.process = SimpleNamespace(pid=42)
        driver.check = lambda: None
        released = []
        driver.cf = SimpleNamespace(CFRelease=released.append)
        def hit(system, x, y, output):
            output._obj.value = 2
            return 0
        for pid in (42, 43):
            def owner(element, output):
                output._obj.value = pid
                return 0
            driver.ax = SimpleNamespace(AXUIElementCreateSystemWide=lambda: 1,
                AXUIElementSetMessagingTimeout=lambda element, seconds: 0,
                AXUIElementCopyElementAtPosition=hit, AXUIElementGetPid=owner)
            if pid == 42:
                driver.check_pointer_owner(Point(20, 30))
            else:
                with self.assertRaises(BenchmarkError):
                    driver.check_pointer_owner(Point(20, 30))
            self.assertEqual(released[-2:], [2, 1])
        with self.assertRaises(BenchmarkError):
            driver.check_pointer_owner(Point(float('nan'), 30))
        self.assertEqual(len(released), 4)

    def test_macos_caption_pointer_waits_for_exact_owned_hit_without_posting_input(self):
        from macos_driver import Point
        driver = object.__new__(MacDriver)
        driver.process = SimpleNamespace(pid=42)
        driver.check = lambda: None
        points, released, attributes = [], [], []
        def point(x, y):
            points.append((x, y))
            return Point(x + 20, y + 30)
        driver.pointer_point = point
        driver.string = lambda text: text
        driver.cf = SimpleNamespace(CFRelease=released.append, CFEqual=lambda a, b: a == b)
        driver.cg = SimpleNamespace(CGEventPost=lambda *args: self.fail('readiness posted input'))
        def hit(system, x, y, output):
            self.assertEqual((x, y), (60, 80))
            output._obj.value = 2
            return 0
        for pid, role, title, ready in ((43, 'AXButton', 'Restore window', False),
                                       (42, 'AXWindow', 'Restore window', False),
                                       (42, 'AXButton', 'Maximize window', False),
                                       (42, 'AXButton', None, False),
                                       (42, 'AXButton', 'Restore window', True)):
            released.clear()
            attributes.clear()
            def owner(element, output):
                output._obj.value = pid
                return 0
            def copy(element, name, optional=False):
                attributes.append(name)
                return {'AXRole': role, 'AXTitle': title}[name]
            driver.copy = copy
            driver.ax = SimpleNamespace(AXUIElementCreateSystemWide=lambda: 1,
                AXUIElementSetMessagingTimeout=lambda element, seconds: 0,
                AXUIElementCopyElementAtPosition=hit, AXUIElementGetPid=owner)
            self.assertIs(driver.pointer_caption_ready('Restore window', 40, 50), ready)
            self.assertEqual(released[-2:], [2, 1])
            if pid == 43:
                self.assertEqual(attributes, [])
                self.assertEqual(released, [2, 1])
            else:
                expected = [role] + ([title, 'AXButton', 'Restore window'] if title else [])
                self.assertEqual(released, expected + [2, 1])
        self.assertEqual(points, [(40, 50)] * 5)
        with self.assertRaises(BenchmarkError):
            driver.pointer_caption_ready('Run command', 40, 50)
        self.assertEqual(len(points), 5)
        released.clear()
        driver.check = lambda: (_ for _ in ()).throw(BenchmarkError('focus changed'))
        with self.assertRaises(BenchmarkError):
            driver.pointer_caption_ready('Restore window', 40, 50)
        self.assertEqual(released, [])

    def test_caption_probe_waits_for_windowserver_hit_after_size_has_settled(self):
        probe = unix_ui_probe()
        state = {'sequence': 21, 'scale_factor': 1, 'window_width': 1200,
                 'window_height': 800, 'chrome': {'maximized': False,
                 'controls_x': 1074, 'button_width': 42, 'header_height': 44}}
        calls = []
        for ready in (False, True):
            def hit(label, x, y):
                calls.append((label, x, y))
                return ready
            driver = SimpleNamespace(window_size=lambda: (1200, 800), pointer_caption_ready=hit)
            self.assertIs(probe.caption_pointer_ready(state, driver), ready)
        self.assertEqual(calls, [('Maximize window', 1137, 22)] * 2)
        calls.clear()
        driver.window_size = lambda: (1920, 959)
        self.assertFalse(probe.caption_pointer_ready(state, driver))
        self.assertEqual(calls, [])

    def test_macos_native_click_pairs_release_and_stops_before_foreign_press(self):
        driver = object.__new__(MacDriver)
        driver.window = 9
        driver.check = lambda: None
        driver.owned_window_number = lambda: 7
        driver.copy = lambda element, name: name
        def geometry(value, kind, output):
            if kind == 1:
                output._obj.x, output._obj.y = 20, 30
            else:
                output._obj.width, output._obj.height = 800, 600
            return True
        driver.ax = SimpleNamespace(AXValueGetValue=geometry)
        for mode in ('normal', 'foreign-before-press', 'dispatch-error'):
            released, posted, checked = [], [], []
            driver.cf = SimpleNamespace(CFRelease=released.append)
            def check(point):
                checked.append((point.x, point.y))
                if mode == 'foreign-before-press' and len(checked) == 2:
                    raise BenchmarkError('foreign point')
            def post(tap, event):
                self.assertEqual(tap, 1)
                posted.append(event)
                if mode == 'dispatch-error' and event == 1:
                    raise BenchmarkError('dispatch unavailable')
            driver.check_pointer_owner = check
            driver.cg = SimpleNamespace(CGEventCreateMouseEvent=lambda source, kind, point, button: kind,
                CGEventSetFlags=lambda event, flags: None,
                CGEventSetIntegerValueField=lambda event, field, value: None, CGEventPost=post)
            with patch('macos_driver.time.sleep'):
                if mode == 'normal':
                    driver.click(40, 50)
                else:
                    with self.assertRaises(BenchmarkError):
                        driver.click(40, 50)
            self.assertEqual(checked, [(60, 80), (60, 80)])
            self.assertEqual(posted, [5] if mode == 'foreign-before-press' else [5, 1, 2])
            self.assertEqual(released, ['AXPosition', 'AXSize', 5, 1, 2])

    def test_macos_click_acknowledges_motion_before_press_and_aborts_on_timeout(self):
        from macos_driver import Point
        driver = object.__new__(MacDriver)
        driver.pointer_point = lambda x, y: Point(x, y)
        driver.owned_window_number = lambda: 7
        driver.check_pointer_owner = lambda point: None
        for acknowledged in (True, False):
            posted, released = [], []
            driver.cf = SimpleNamespace(CFRelease=released.append)
            driver.cg = SimpleNamespace(CGEventCreateMouseEvent=lambda source, kind, point, button: kind,
                CGEventSetFlags=lambda event, flags: None,
                CGEventSetIntegerValueField=lambda event, field, value: None,
                CGEventPost=lambda tap, event: posted.append(event))
            def wait_for_motion():
                self.assertEqual(posted, [5])
                if not acknowledged:
                    raise BenchmarkError('pointer acknowledgement timed out')
            with patch('macos_driver.time.sleep') as sleep:
                if acknowledged:
                    driver.click(40, 50, wait_for_motion=wait_for_motion)
                else:
                    with self.assertRaises(BenchmarkError):
                        driver.click(40, 50, wait_for_motion=wait_for_motion)
                sleep.assert_not_called()
            self.assertEqual(posted, [5, 1, 2] if acknowledged else [5])
            self.assertEqual(released, [5, 1, 2])

    def test_native_failure_report_excludes_content_and_nonfinite_geometry(self):
        probe = unix_ui_probe()
        report = probe.native_failure_state({'pointer': {'x': 12, 'y': float('nan'), 'text': 'private'},
            'chrome': {'header_height': 32, 'palette_x': 'private', 'maximized': False},
            'window_width': 800, 'scale_factor': float('inf'), 'visible_text': 'private',
            'panels': [{'current_directory': 'private'}]})
        self.assertEqual(report, {'pointer': {'x': 12},
            'chrome': {'header_height': 32, 'maximized': False}, 'window_width': 800})

    def test_caption_probe_handles_initially_maximized_windows_and_rejects_old_frames(self):
        probe = unix_ui_probe()
        geometry = {'scale_factor': 1, 'window_width': 1200, 'window_height': 800}
        for initial in (False, True):
            before = dict(geometry, sequence=20, chrome={'maximized': initial})
            label, expected, sequence = probe.caption_transition(before)
            self.assertEqual(label, 'Restore window' if initial else 'Maximize window')
            self.assertIs(expected, not initial)
            self.assertFalse(probe.caption_transition_presented(before, expected, sequence, (1200, 800)))
            self.assertFalse(probe.caption_transition_presented(
                dict(before, chrome={'maximized': expected}), expected, sequence, (1200, 800)))
            self.assertTrue(probe.caption_transition_presented(
                dict(before, sequence=21, chrome={'maximized': expected}), expected, sequence, (1200, 800)))
        for state in ({}, {'sequence': 1, 'chrome': {'maximized': 1}},
                      {'sequence': True, 'chrome': {'maximized': False}}):
            with self.assertRaises(probe.Failure):
                probe.caption_transition(state)

    def test_caption_probe_rejects_early_zoom_flag_until_native_geometry_is_presented(self):
        probe = unix_ui_probe()
        # Captured Intel sequence: the zoom flag advanced, but the old 1200px
        # layout still placed the next click at x=1137 instead of x=1857.
        for scale in (1, 2):
            for maximized, old_size, native_size in (
                    (True, (1200, 800), (1920, 959)),
                    (False, (1920, 959), (1200, 800))):
                state = {'sequence': 21, 'chrome': {'maximized': maximized},
                         'scale_factor': scale, 'window_width': old_size[0] * scale,
                         'window_height': old_size[1] * scale}
                self.assertFalse(probe.caption_transition_presented(state, maximized, 20, native_size))
                state['window_width'] = native_size[0] * scale
                self.assertFalse(probe.caption_transition_presented(state, maximized, 20, native_size))
                state['window_height'] = native_size[1] * scale
                self.assertTrue(probe.caption_transition_presented(state, maximized, 20, native_size))

    def test_macos_native_size_is_owned_validated_and_releases_its_reference(self):
        driver = object.__new__(MacDriver)
        driver.window = 9
        checked, released = [], []
        driver.check = lambda: checked.append(True)
        driver.copy = lambda element, name: name
        driver.cf = SimpleNamespace(CFRelease=released.append)
        for width, height, available in ((1920, 959, True), (0, 800, True),
                                        (1200, -1, True), (float('nan'), 800, True),
                                        (1200, float('inf'), True), (1200, 800, False)):
            def read(value, kind, output):
                self.assertEqual(value, 'AXSize')
                self.assertEqual(kind, 2)
                output._obj.width, output._obj.height = width, height
                return available
            driver.ax = SimpleNamespace(AXValueGetValue=read)
            if available and width == 1920:
                self.assertEqual(driver.window_size(), (1920, 959))
            else:
                with self.assertRaises(BenchmarkError):
                    driver.window_size()
        self.assertEqual(len(checked), 6)
        self.assertEqual(released, ['AXSize'] * 6)
        driver.check = lambda: (_ for _ in ()).throw(BenchmarkError('foreign window'))
        with self.assertRaises(BenchmarkError):
            driver.window_size()
        self.assertEqual(len(released), 6)

    def test_native_search_replacement_does_not_navigate_back_from_empty_query(self):
        probe = unix_ui_probe()
        for previous, value, expected in (
            (0, "", []),
            (0, "Text size", ['T', 'e', 'x', 't', 'space', 's', 'i', 'z', 'e']),
            (9, "", ['ctrl+a', 'BackSpace']),
            (9, "Inherit shared", ['ctrl+a', 'BackSpace', 'I', 'n', 'h', 'e', 'r', 'i', 't', 'space', 's', 'h', 'a', 'r', 'e', 'd']),
        ):
            keys = []
            with self.subTest(previous=previous, value=value):
                probe.replace_search_query(keys.append, previous, value)
                self.assertEqual(keys, expected)

    def test_theme_probe_rejects_placeholder_or_incomplete_inventory(self):
        probe = unix_ui_probe()
        for state in [{}, {'settings': {}}, {'settings': {'gallery': None}}]:
            self.assertFalse(probe.theme_gallery_ready(state))
        for gallery in [{}, {'count': 1, 'busy': False}, {'count': 5, 'busy': False},
                        {'count': 6, 'busy': True}, {'count': 6}]:
            with self.subTest(gallery=gallery):
                self.assertFalse(probe.theme_gallery_ready({'settings': {'gallery': gallery}}))
        self.assertTrue(probe.theme_gallery_ready({'settings': {'gallery': {'count': 6, 'busy': False}}}))

    def test_native_geometry_waits_for_scale_and_size_in_either_event_order(self):
        probe = unix_ui_probe()
        ready = {'scale_factor': 1.5, 'window_width': 1800, 'window_height': 1200}
        self.assertTrue(probe.native_geometry_ready(ready, 1.5, (1200, 800)))
        self.assertTrue(probe.native_geometry_ready(ready, None))
        for key, value in [('scale_factor', 1), ('scale_factor', 0), ('scale_factor', float('nan')),
                           ('scale_factor', None), ('window_width', 1200), ('window_height', 800),
                           ('window_width', float('inf')), ('window_height', None)]:
            with self.subTest(key=key, value=value):
                self.assertFalse(probe.native_geometry_ready(dict(ready, **{key: value}), 1.5, (1200, 800)))
        self.assertFalse(probe.native_geometry_ready({}, 1.5))

    def test_macos_pointer_probe_is_bounded_to_the_owned_window(self):
        self.assertTrue(MacDriver.point_in_window(0, 0, 800, 600))
        self.assertTrue(MacDriver.point_in_window(799.5, 599.5, 800, 600))
        for point in [(800, 0), (0, 600), (-1, 0), (0, -1), (float('nan'), 0), (0, float('inf'))]:
            self.assertFalse(MacDriver.point_in_window(*point, 800, 600))
        self.assertFalse(MacDriver.point_in_window(1, 1, float('nan'), 600))
        self.assertFalse(MacDriver.point_in_window(1, 1, 800, 0))

    def test_wayland_input_requires_unique_owned_focused_window(self):
        probe = unix_ui_probe()
        own = {"type": "con", "pid": 42, "id": 7, "focused": True}
        other = {"type": "con", "pid": 43, "id": 8, "focused": False}
        tree = {"nodes": [{"floating_nodes": [own, other]}]}
        self.assertEqual(probe.WaylandDisplay.owned_window(tree, 42, True), own)
        floating = dict(own, type="floating_con")
        self.assertEqual(probe.WaylandDisplay.owned_window({"floating_nodes": [floating]}, 42, True), floating)
        for invalid in [
            {"nodes": [other]}, {"nodes": [own, dict(own)]},
            {"nodes": [dict(own, focused=False)]}, {"nodes": [dict(own, id="7; exec unwanted")]},
            {"nodes": [own] * 513},
        ]:
            with self.subTest(tree=invalid if len(invalid["nodes"]) < 4 else "oversized"):
                with self.assertRaises(probe.Failure):
                    probe.WaylandDisplay.owned_window(invalid, 42, True)

    def test_native_tag_inheritance_waits_for_saved_geometry(self):
        probe = unix_ui_probe()
        def frame(height, saving=False, ready=True, roster=False):
            return {"settings": {"ready": ready, "save_pending": saving, "targets": [
                {"id": "tags.slot.user.page", "bounds": [10, 20, 40, height], "roster": roster}]}}
        for value in ({}, frame(10), frame(25, saving=True), frame(25, ready=False), frame(25, roster=True)):
            self.assertFalse(probe.tag_geometry_saved(value, "tags.slot.user.page", 25))
        self.assertTrue(probe.tag_geometry_saved(frame(25), "tags.slot.user.page", 25))

    def test_wayland_click_waits_for_presented_pointer_before_pressing(self):
        probe = unix_ui_probe()
        display = object.__new__(probe.WaylandDisplay)
        process = SimpleNamespace(pid=42, poll=lambda: None)
        rect = {"x": 40, "y": 50, "width": 1200, "height": 800}
        stale = {"scale_factor": 1.5, "window_width": 1800, "window_height": 1200,
                 "pointer": {"x": 19.5, "y": 600}}
        ready = dict(stale, pointer={"x": 900, "y": 450})
        self.assertFalse(probe.native_pointer_ready(stale, 600, 300, (1200, 800)))
        self.assertTrue(probe.native_pointer_ready(ready, 600, 300, (1200, 800)))
        self.assertFalse(probe.native_pointer_ready(dict(ready, window_width=900), 600, 300, (1200, 800)))
        frames, acknowledged = iter([stale, ready]), []
        def read():
            value = next(frames)
            acknowledged.append(value is ready)
            return value
        def post(*args):
            if "press" in args:
                self.assertEqual(acknowledged, [False, True])
        with patch.object(display, "window", return_value={"rect": rect}), \
                patch.object(display, "ipc", side_effect=post) as ipc, patch.object(probe.time, "sleep"):
            display.click(process, 600, 300, read)
            self.assertEqual(ipc.call_args_list[-2].args, ("seat", "seat0", "cursor", "press", "button1"))
            self.assertEqual(ipc.call_args_list[-1].args, ("seat", "seat0", "cursor", "release", "button1"))
        with patch.object(display, "window", return_value={"rect": rect}), \
                patch.object(display, "ipc") as ipc, patch.object(probe.time, "sleep"), \
                patch.object(probe.time, "monotonic", side_effect=[0, .1, 5]):
            with self.assertRaises(probe.Failure):
                display.click(process, 600, 300, lambda: stale)
            self.assertEqual(ipc.call_count, 1)  # Movement only; never click a stale target.

    def test_wayland_unsupported_inputs_never_post_keys(self):
        probe = unix_ui_probe()
        display = object.__new__(probe.WaylandDisplay)
        process = SimpleNamespace(pid=42, poll=lambda: None)
        with patch.object(display, "window", return_value={"id": 7}), \
             patch.object(probe.subprocess, "run") as run:
            for chord in ["ctrl+q", "ctrl+shift+p; bad", "", "Escape+ctrl"]:
                with self.assertRaises(probe.Failure):
                    display.key(process, chord)
            for size in [(0, 520), (720, float("nan")), (1601, 520), (720, 1201)]:
                with self.assertRaises(probe.Failure):
                    display.resize(process, *size)
            run.assert_not_called()

    def test_wayland_control_wake_is_owned_motion_without_clicks_or_pty_keys(self):
        probe = unix_ui_probe()
        display = object.__new__(probe.WaylandDisplay)
        process = SimpleNamespace(pid=42, poll=lambda: None)
        rect = {"x": 40, "y": 50, "width": 1200, "height": 800}
        with patch.object(display, "window", return_value={"rect": rect}) as window, \
                patch.object(display, "ipc") as ipc:
            for sequence in (1, 2, 3):
                display.wake_control(process, sequence)
                window.assert_called_with(process, focused=True)
                ipc.assert_called_with("seat", "seat0", "cursor", "set", str(52 + sequence % 2), "450")
            self.assertEqual(ipc.call_count, 3)
            window.side_effect = probe.Failure("owned window lost focus")
            with self.assertRaises(probe.Failure):
                display.wake_control(process, 4)
            self.assertEqual(ipc.call_count, 3)

    def test_wayland_keys_use_owned_persistent_compositor_keyboard(self):
        probe = unix_ui_probe()
        display = object.__new__(probe.WaylandDisplay)
        display.tools = {"xdotool": "/fixture/bin/xdotool"}
        display.input_env = {"DISPLAY": ":fixture", "XAUTHORITY": "/fixture/auth"}
        display.input_window = "24"
        process = SimpleNamespace(pid=42, poll=lambda: None)
        with patch.object(display, "window", return_value={"id": 7}) as window, \
             patch.object(probe.subprocess, "run") as run:
            for chord in ["ctrl+shift+F1", "ctrl+shift+F3", "alt+j", "alt+shift+j", "x", "a", "F3", "Return",
                          "BackSpace", "space", *"connectTextsizeInheritshared1250"]:
                display.key(process, chord)
                window.assert_called_with(process, focused=True)
                run.assert_called_with(
                    ["/fixture/bin/xdotool", "windowfocus", "24", "key", "--clearmodifiers", chord],
                    env=display.input_env, check=True, capture_output=True, timeout=5)

    def test_wayland_compositor_environment_never_reuses_the_user_display(self):
        probe = unix_ui_probe()
        with patch.object(probe.shutil, "which", side_effect=lambda name: "/fixture/bin/" + name):
            for scale in [0, -1, float("nan"), 4]:
                with self.assertRaises(probe.Failure):
                    probe.WaylandDisplay(scale)
            display = probe.WaylandDisplay(1.5)
        def inspect_launch(*args, **kwargs):
            self.assertEqual(args[0][0], "/fixture/bin/xvfb-run")
            self.assertIn("-a", args[0])
            self.assertTrue(any("-nolisten tcp" in arg for arg in args[0]))
            self.assertTrue(kwargs["start_new_session"])
            self.assertNotIn("DISPLAY", display.env)
            self.assertNotIn("WAYLAND_DISPLAY", display.env)
            self.assertNotIn("WAYLAND_SOCKET", display.env)
            self.assertNotIn("SWAYSOCK", display.env)
            raise OSError("fixture launch failure")
        with patch.dict(probe.os.environ, {"DISPLAY": ":0", "WAYLAND_DISPLAY": "wayland-0", "WAYLAND_SOCKET": "4",
                                         "SWAYSOCK": "/fixture/user-socket", "XDG_RUNTIME_DIR": "/fixture/user-runtime"}), \
             patch.object(probe.subprocess, "Popen", side_effect=inspect_launch):
            with self.assertRaises(OSError):
                with display:
                    self.fail("failed compositor was admitted")
        self.assertFalse(Path(display.temporary.name).exists())
        self.assertTrue(display.log.closed)

    @unittest.skipIf(os.name == 'nt', 'Unix native fixture')
    def test_unix_ui_fixture_uses_the_canonical_launch_directory(self):
        from contextlib import nullcontext

        probe = unix_ui_probe()
        class LaunchReached(Exception):
            pass
        captured = {}
        def launch(*args, **kwargs):
            captured.update(cwd=kwargs['cwd'], home=kwargs['env']['HOME'])
            raise LaunchReached()
        with TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            home = root / 'real'
            home.mkdir()
            alias = root / 'alias'
            alias.symlink_to(home, target_is_directory=True)
            with patch.object(probe, 'fixture_workspace', return_value=nullcontext((str(alias), {}))), \
                 patch.object(probe.subprocess, 'Popen', side_effect=launch), \
                 self.assertRaises(LaunchReached):
                probe.run_case(root / 'unused-binary', root / 'captures', 'cpu', 'default', 1.0, 'default')
            self.assertEqual(captured['cwd'], home)
            self.assertEqual(captured['home'], str(home))
            self.assertTrue((home / 'config/config.toml').is_file())
            for rc in ('.bashrc', '.bash_profile', '.zshrc'):
                text = (home / rc).read_text()
                self.assertIn("USER=fixture", text)
                self.assertIn("PS1='fixture> '", text)
            fish = (home / '.config/fish/config.fish').read_text()
            self.assertIn('set -gx USER fixture', fish)
            self.assertIn("function fish_prompt; printf 'fixture> '; end", fish)

    @unittest.skipIf(os.name == 'nt', 'Unix native fixture')
    def test_native_cleanup_retries_actual_directory_enumeration_race(self):
        probe = unix_ui_probe()
        original = os.rmdir
        injected = False
        home = None

        def late_native_cache(path, *args, **kwargs):
            nonlocal injected
            if home is not None and Path(path) == home and not injected:
                injected = True
                (home / 'late-cache').write_text('synthetic cache')
            return original(path, *args, **kwargs)

        with patch.object(os, 'rmdir', side_effect=late_native_cache):
            with probe.fixture_workspace() as (temporary, diagnostics):
                home = Path(temporary)
                (home / 'initial').write_text('synthetic fixture')
        self.assertTrue(injected)
        self.assertEqual(diagnostics, {'retries': 1, 'removed': True})
        self.assertFalse(home.exists())

    def test_native_cleanup_is_bounded_and_never_hides_other_failures(self):
        import errno
        probe = unix_ui_probe()
        for code, attempts in ((errno.ENOTEMPTY, 5), (errno.EACCES, 1)):
            with self.subTest(errno=code), TemporaryDirectory() as parent:
                temporary = probe.tempfile.TemporaryDirectory(dir=parent)
                try:
                    with patch.object(probe.tempfile, 'TemporaryDirectory', return_value=temporary), \
                         patch.object(temporary, 'cleanup', side_effect=OSError(code, 'synthetic')) as cleanup, \
                         patch.object(probe.time, 'sleep'):
                        with self.assertRaises(OSError) as raised:
                            with probe.fixture_workspace():
                                pass
                        self.assertEqual(raised.exception.errno, code)
                        self.assertEqual(cleanup.call_count, attempts)
                finally:
                    temporary.cleanup()
        with TemporaryDirectory() as parent:
            temporary = probe.tempfile.TemporaryDirectory(dir=parent)
            cleanup_real = temporary.cleanup
            first = True
            def cleanup_after_race():
                nonlocal first
                if first:
                    first = False
                    raise OSError(errno.ENOTEMPTY, 'synthetic')
                cleanup_real()
            try:
                with patch.object(probe.tempfile, 'TemporaryDirectory', return_value=temporary), \
                     patch.object(temporary, 'cleanup', side_effect=cleanup_after_race), \
                     patch.object(probe.time, 'sleep'):
                    with self.assertRaisesRegex(probe.Failure, 'owned shell survived'):
                        with probe.fixture_workspace():
                            raise probe.Failure('owned shell survived')
            finally:
                temporary.cleanup()

    def test_native_startup_waits_for_fragmented_cwd_before_asserting_its_value(self):
        probe = unix_ui_probe()
        panel = {'active': True, 'shell_integration': True, 'current_directory': None,
                 'cursor_line_text': '\u03bb', 'raw_cursor_line_text': '\u03bb', 'raw_cursor_prompt_id': 7}
        state = {'panels': [panel], 'prompt_active': True, 'latest_prompt_id': 7}
        self.assertFalse(probe.integrated_ready(state))
        panel['current_directory'] = '/fixture/actual'
        self.assertTrue(probe.integrated_ready(state))
        # Readiness never substitutes the expected directory. The caller must
        # still reject a real clone launched somewhere else.
        self.assertNotEqual(panel['current_directory'], '/fixture/expected')
        state['prompt_active'] = False
        self.assertFalse(probe.integrated_ready(state))

    def test_native_prompt_readiness_waits_for_the_presented_input_row(self):
        probe = unix_ui_probe()
        # Bash publishes prompt-active/A and the path before Readline emits PS1.
        panel = {'active': True, 'shell_integration': True, 'current_directory': '/fixture',
                 'cursor_line_text': '', 'raw_cursor_line_text': '', 'raw_cursor_prompt_id': 7}
        state = {'panels': [panel], 'prompt_active': True, 'latest_prompt_id': 7}
        self.assertFalse(probe.integrated_ready(state))
        panel['raw_cursor_line_text'] = 'λ'
        self.assertFalse(probe.integrated_ready(state))
        panel['cursor_line_text'] = 'λ'
        self.assertTrue(probe.integrated_ready(state))
        # A raw grid can advance while the previous presented frame is retained.
        panel['raw_cursor_prompt_id'] = 8
        self.assertFalse(probe.integrated_ready(state))
        state['latest_prompt_id'] = 8
        self.assertTrue(probe.integrated_ready(state))
        panel['raw_cursor_line_text'] = 'λ unexpected input'
        self.assertFalse(probe.integrated_ready(state))
        panel.update(shell_name='fish', cursor_line_text='fixture>', raw_cursor_line_text='fixture>')
        self.assertTrue(probe.integrated_ready(state))
        panel['raw_cursor_line_text'] = 'λ'
        self.assertFalse(probe.integrated_ready(state))

    def test_native_failure_diagnostics_keep_errno_but_never_paths_or_output(self):
        import errno
        import json

        probe = unix_ui_probe()
        error = OSError(errno.ENOTEMPTY, 'private terminal content', '/private/example')
        details = probe.failure_details(error)
        self.assertEqual(details, {'failure': 'OSError', 'errno': errno.ENOTEMPTY, 'operations': []})
        self.assertNotIn('private', json.dumps(details))
        # An actual fixture launch error includes a fixed source owner/line,
        # while foreign frames, messages and the executable path stay private.
        with TemporaryDirectory() as temporary:
            with patch.object(probe.subprocess, 'Popen', side_effect=error):
                try:
                    probe.run_case(Path(temporary) / 'private-binary', Path(temporary) / 'captures',
                                   'cpu', 'default', 1.0, 'default')
                except OSError as failure:
                    details = probe.failure_details(failure)
                else:
                    self.fail('the native launch error was swallowed')
        self.assertTrue(details['operations'])
        self.assertTrue(all(item.startswith('native-ui:') for item in details['operations']))
        self.assertNotIn('private', json.dumps(details))
        self.assertNotIn(temporary, json.dumps(details))

    def test_macos_shortcuts_use_native_modifier_flags_and_reject_ambiguous_keys(self):
        self.assertEqual(MacDriver.key_spec('meta+shift+p'), (35, (1 << 20) | (1 << 17), 'p'))
        for key, code in (('F1', 122), ('F3', 99), ('F5', 96), ('F6', 97), ('F8', 100)):
            with self.subTest(key=key):
                self.assertEqual(MacDriver.key_spec(key), (code, 1 << 23, key))
                self.assertEqual(MacDriver.key_spec('meta+shift+' + key),
                                 (code, (1 << 20) | (1 << 17) | (1 << 23), key))
        self.assertEqual(MacDriver.key_spec('Escape'), (53, 0, 'Escape'))
        self.assertEqual(MacDriver.key_spec('Return'), (36, 0, 'Return'))
        self.assertEqual(MacDriver.key_spec('Down'), (125, 0, 'Down'))
        self.assertEqual(MacDriver.key_spec('x'), (7, 0, 'x'))
        for key in ('meta+meta+p', 'unknown+p', 'meta+unsupported', '', 'meta+'):
            with self.subTest(key=key), self.assertRaises(BenchmarkError):
                MacDriver.key_spec(key)

    def test_macos_input_requires_the_exact_focused_window(self):
        driver = object.__new__(MacDriver)
        driver.process = SimpleNamespace(pid=42, poll=lambda: None)
        driver.application, driver.window = 'app', 'owned-window'
        released = []
        driver.cf = SimpleNamespace(CFRelease=released.append,
            CFBooleanGetValue=lambda value: value == 'true', CFEqual=lambda a, b: a == b)
        def owner(element, output):
            output._obj.value = 42
            return 0
        driver.ax = SimpleNamespace(AXUIElementGetPid=owner)
        for front, focused, allowed in (('true', 'owned-window', True),
                                        ('false', 'owned-window', False),
                                        ('true', 'other-window', False),
                                        ('true', None, False)):
            with self.subTest(front=front, focused=focused):
                released.clear()
                driver.copy = lambda element, name, optional=False: {
                    'AXFrontmost': front, 'AXFocusedWindow': focused}[name]
                if allowed:
                    driver.check()
                else:
                    with self.assertRaises(BenchmarkError):
                        driver.check()
                self.assertIn(front, released)
                if front == 'true' and focused:
                    self.assertIn(focused, released)

    def test_macos_function_key_press_and_release_preserve_native_flags(self):
        driver = object.__new__(MacDriver)
        driver.process = SimpleNamespace(pid=42)
        driver.check = lambda: None
        posted, flags, released = [], [], []
        driver.cf = SimpleNamespace(CFRelease=released.append)
        driver.cg = SimpleNamespace(
            CGEventCreateKeyboardEvent=lambda source, code, down: (code, down),
            CGEventSetFlags=lambda event, value: flags.append((event, value)),
            CGEventKeyboardSetUnicodeString=lambda *args: self.fail('function key is not text'),
            CGEventPostToPid=lambda pid, event: posted.append((pid, event)))
        driver.key('meta+shift+F3')
        events = [(99, True), (99, False)]
        self.assertEqual(posted, [(42, event) for event in events])
        self.assertEqual(flags, [(event, (1 << 23) | (1 << 20) | (1 << 17)) for event in events])
        self.assertEqual(released, events)

    def test_native_focus_failure_evidence_is_read_only_and_content_free(self):
        driver = object.__new__(MacDriver)
        driver.process = SimpleNamespace(pid=42, poll=lambda: None)
        driver.application, driver.window = 'app', 'owned-window'
        released = []
        driver.cf = SimpleNamespace(CFRelease=released.append,
            CFBooleanGetValue=lambda value: value == 'true', CFEqual=lambda a, b: a == b)
        def owner(element, output):
            output._obj.value = 42
            return 0
        driver.ax = SimpleNamespace(AXUIElementGetPid=owner)
        driver.copy = lambda element, name, optional=False: {
            'AXFrontmost': 'false', 'AXFocusedWindow': 'other-window'}[name]
        observed = driver.focus_snapshot()
        self.assertEqual(observed, {'process_alive': True, 'window_owned': True,
                                   'app_frontmost': False, 'owned_window_focused': False})
        self.assertEqual(released, ['false', 'other-window'])
        driver.copy = lambda *args, **kwargs: None
        self.assertEqual(driver.focus_snapshot(), {'process_alive': True, 'window_owned': True,
            'app_frontmost_unavailable': True, 'owned_window_focused_unavailable': True})
        probe = unix_ui_probe()
        error = probe.Failure('fixed failure')
        error.native_focus = dict(observed, private_title='private', process_pid=42)
        self.assertEqual(probe.failure_details(error)['native_focus'], observed)
        error.native_focus = {'process_alive': 'private', 'window_owned': 42}
        self.assertEqual(probe.failure_details(error)['native_focus'], {})

    def test_macos_native_picker_search_supports_every_fixed_query_key(self):
        # Exercise the exact native palette query, including repeated letters.
        for key, code in zip("connect", (8, 31, 45, 45, 14, 8, 17)):
            with self.subTest(key=key):
                self.assertEqual(MacDriver.key_spec(key), (code, 0, key))

    def test_macos_tag_customization_supports_all_fixture_keys(self):
        codes = {'t': 17, 'e': 14, 'x': 7, 's': 1, 'i': 34, 'z': 6,
                 'n': 45, 'h': 4, 'r': 15, 'a': 0, 'd': 2,
                 '1': 18, '2': 19, '5': 23, '0': 29, 'space': 49, 'BackSpace': 51}
        for text in ('Text size', 'Inherit shared', '125', '50'):
            for char in text:
                key = 'space' if char == ' ' else char
                base = key.lower() if len(key) == 1 else key
                flags = (1 << 17) if char.isupper() else 0
                self.assertEqual(MacDriver.key_spec(key), (codes[base], flags, base))
        self.assertEqual(MacDriver.key_spec('BackSpace'), (51, 0, 'BackSpace'))
        self.assertEqual(MacDriver.key_spec('shift+T'), (17, 1 << 17, 't'))
        with self.assertRaises(BenchmarkError):
            MacDriver.key_spec('shift+shift+T')

    def test_native_search_query_matches_the_actual_fixture_history(self):
        with TemporaryDirectory() as temporary:
            output = io.BytesIO()
            environment = {'AUTOMEXIA_BENCHMARK_TOKEN': 'a' * 32,
                           'AUTOMEXIA_BENCHMARK_FIXTURE_OUTPUT': str(Path(temporary) / 'fixture.json')}
            with patch.dict(os.environ, environment), \
                 patch.object(workload.sys, 'stdout', SimpleNamespace(buffer=output)), \
                 patch.object(workload.time, 'sleep'), patch.object(workload, 'interactive'):
                self.assertEqual(workload.main(['interactive', '--seconds', '5']), 0)
            rows = output.getvalue().splitlines()
            self.assertEqual(len(rows), 1000)
            self.assertTrue(all(collector.SEARCH_QUERY.encode() in row for row in rows))

    def test_hidden_and_infrastructure_windows_are_never_input_targets(self):
        self.assertTrue(native.application_surface('native-window', 1280, 720, 0, 10))
        for args in [('Winit Thread Event Target', 1280, 720, 0, 10),
                     ('native-window', 1280, 720, 1, 10), ('native-window', 0, 0, 0, 10),
                     ('native-window', 1280, 720, 0, 0)]:
            self.assertFalse(native.application_surface(*args))

    def test_scenarios_cover_exact_metric_roster(self):
        self.assertEqual(set(collector.SCENARIOS), set(collector.SCENARIO_METRICS))
        self.assertEqual(set(METRICS), {name for names in collector.SCENARIO_METRICS.values() for name in names})

    def test_dirty_source_identity_includes_current_untracked_bytes(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / 'fixture.py'
            source.write_text('before', encoding='utf-8')
            with patch.object(collector, 'ROOT', root), \
                 patch.object(collector.qa, 'git_value', return_value='a' * 40), \
                 patch.object(collector.qa, 'source_status_bytes', return_value=b'?? fixture.py\0'):
                before = collector.source_identity()
                source.write_text('changed', encoding='utf-8')
                after = collector.source_identity()
            self.assertTrue(before[1])
            self.assertEqual(before[:2], after[:2])
            self.assertNotEqual(before[2], after[2])

    def test_missing_source_inventory_cannot_be_clean_evidence(self):
        with patch.object(collector.qa, 'git_value', return_value='a' * 40), \
             patch.object(collector.qa, 'source_status_bytes', return_value=None):
            with self.assertRaises(BenchmarkError):
                collector.source_identity()

    def test_fingerprint_rejects_empty_or_nonregular_input(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            empty = root / 'empty'
            empty.touch()
            for path in (root, empty):
                with self.assertRaises(BenchmarkError):
                    collector.sha_file(path)

    def test_environment_drops_credentials_and_user_settings(self):
        with patch.dict(os.environ, {'FAKE_CREDENTIAL': 'never-copy', 'AUTOMEXIA_CONFIG_HOME': 'user',
                                     'RIO_CONFIG_HOME': 'legacy', 'SSH_AUTH_SOCK': 'agent'}, clear=True):
            env = collector.environment(Path('isolated'), 'idle')
        self.assertNotIn('FAKE_CREDENTIAL', env)
        self.assertNotIn('RIO_CONFIG_HOME', env)
        self.assertNotIn('SSH_AUTH_SOCK', env)
        self.assertEqual(env['AUTOMEXIA_CONFIG_HOME'], 'isolated')
        self.assertEqual(env['HOME'], 'isolated')
        self.assertEqual(len(env['AUTOMEXIA_BENCHMARK_TOKEN']), 32)

    def test_config_is_valid_toml_exact_argv_and_recovery_disabled(self):
        import json
        import tomllib
        value = tomllib.loads(collector.CONFIG.format(cwd=json.dumps('fixture directory'),
                 python=json.dumps('python executable'), workload=json.dumps('workload.py'),
                 mode=json.dumps('interactive'), seconds=json.dumps('45')))
        self.assertEqual(value['shell']['args'], ['workload.py', 'interactive', '--seconds', '45'])
        self.assertFalse(value['session-recovery']['enabled'])
        self.assertFalse(value['confirm-before-quit'])
        self.assertEqual(len(value['bindings']['keys']), 3)

    def test_font_fingerprint_is_content_based_not_checkout_path(self):
        with TemporaryDirectory() as temporary:
            left, right = Path(temporary)/'one', Path(temporary)/'two'
            left.write_bytes(b'fixture-font'); right.write_bytes(b'fixture-font')
            self.assertEqual(collector.fingerprints([left]), collector.fingerprints([right]))
            right.write_bytes(b'changed-font')
            self.assertNotEqual(collector.fingerprints([left])['fonts_sha256'], collector.fingerprints([right])['fonts_sha256'])

    def test_native_resource_sensor_tracks_owned_process_then_rejects_exit(self):
        if sys.platform not in ('win32', 'linux', 'darwin'):
            self.skipTest('native resource adapter unavailable')
        # Popen completion does not mean the child has initialized: Linux can
        # legitimately expose zero RSS before Python maps its working set.
        # Keep the real counter assertions; acknowledge fixture readiness and
        # retain the child until sampling instead of racing a fixed sleep.
        with TemporaryDirectory() as temporary:
            ready = Path(temporary) / 'ready'
            script = 'from pathlib import Path; import sys; Path(sys.argv[1]).write_bytes(b"ready"); sys.stdin.buffer.read(1)'
            with subprocess.Popen([sys.executable, '-c', script, str(ready)],
                                  stdin=subprocess.PIPE, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL) as process:
                try:
                    deadline = time.monotonic() + 5
                    while time.monotonic() < deadline and process.poll() is None:
                        if ready.is_file() and ready.read_bytes() == b'ready':
                            break
                        time.sleep(.01)
                    else:
                        self.fail('resource fixture did not acknowledge readiness')
                    counter = native.Resources(process)
                    rss, cpu, clock = counter.sample()
                    self.assertGreater(rss, 0); self.assertGreaterEqual(cpu, 0); self.assertGreater(clock, 0)
                    process.stdin.close()
                    self.assertEqual(process.wait(timeout=5), 0)
                    with self.assertRaises(BenchmarkError):
                        counter.sample()
                finally:
                    if process.poll() is None:
                        process.terminate(); process.wait(timeout=5)

    def test_no_interactive_driver_is_silently_emulated_on_unsupported_os(self):
        with patch.object(native.os, 'name', 'posix'), patch.object(native.sys, 'platform', 'unsupported'):
            with self.assertRaises(BenchmarkError):
                native.driver(object())


if __name__ == '__main__':
    unittest.main()
