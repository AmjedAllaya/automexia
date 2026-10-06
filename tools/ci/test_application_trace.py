"""Exercise real trace normalization, including incomplete native observations."""
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'renderer-benchmarks' / 'application'))
from benchmark_model import BenchmarkError
from observation import validate_trace, durations, startup, fixture_duration
import workload


def event(kind, at, a=0, b=0, c=0):
    return dict(kind=kind, at_ns=at, window=0, a=a, b=b, c=c)


def trace():
    return dict(schema=1, kind='application-observation', complete=True, os='windows', arch='x86_64',
                optimized=True, wall_start_ns=1_000_000_000, wall_end_ns=1_010_000_000,
                duration_ns=10_000_000,
                events=[event('window', 100, 1280, 720, 1000), event('present', 200, 1, 1280, 720),
                        event('input', 300, 1), event('present', 500, 1, 1280, 720)])


class ObservationTests(unittest.TestCase):
    def test_startup_and_input_use_measured_endpoints(self):
        value = validate_trace(trace())
        self.assertEqual(startup(value, 999_999_900), {'startup_window': [200], 'startup_terminal_frame': [300]})
        self.assertEqual(durations(value, 'input', panes=1), [200])

    def test_unfinished_wrong_pane_and_coalesced_input_fail(self):
        for events in ([event('input', 300, 1)],
                       [event('input', 300, 1), event('present', 500, 4, 1280, 720)],
                       [event('input', 300, 1), event('input', 350, 1), event('present', 500, 1, 1280, 720)]):
            value = trace(); value['events'] = value['events'][:2] + events
            with self.assertRaises(BenchmarkError):
                durations(validate_trace(value), 'input', panes=1)

    def test_resize_matches_geometry_and_search_pairs(self):
        value = trace()
        value['events'] += [event('resize', 1000, 800, 600), event('present', 1100, 1, 1280, 720),
                            event('present', 1300, 1, 800, 600), event('search-begin', 1500), event('search-end', 1600)]
        self.assertEqual(durations(validate_trace(value), 'resize'), [300])
        self.assertEqual(durations(value, 'search'), [100])

    def test_trace_rejects_incomplete_clock_jump_and_untrusted_shapes(self):
        mutations = [('complete', False), ('events', [event('unknown', 5)]), ('schema', True),
                     ('duration_ns', 10**200), ('wall_end_ns', 1_050_000_000), ('events', []),
                     ('events', [event('present', 100, 1), event('window', 50)]),
                     ('events', [event('present', 10_000_001, 1)])]
        for key, replacement in mutations:
            value = trace(); value[key] = replacement
            with self.subTest(key=key), self.assertRaises(BenchmarkError):
                validate_trace(value)

    def test_image_requires_decoded_overlay_and_completion_after_start(self):
        value = trace(); value['events'] += [event('fixture', 2000, 2, 1)]
        fixture = dict(schema=1, mode='kitty', wall_start_ns=1_000_001_000,
                       start_wall_ns=1_000_001_000, wall_end_ns=1_000_005_000,
                       duration_ns=4000, write_duration_ns=500, lines=0, image_side=256)
        self.assertEqual(fixture_duration(value, fixture, 'kitty'), 1000)
        value['events'][-1]['b'] = 0
        with self.assertRaises(BenchmarkError):
            fixture_duration(value, fixture, 'kitty')

    def test_throughput_workload_is_exact_and_bounded(self):
        count = size = 0
        for chunk in workload.output_chunks():
            self.assertLessEqual(len(chunk), 65536)
            count += chunk.count(b'\n'); size += len(chunk)
        self.assertEqual((count, size), (1_000_000, 64_000_000))

    def test_image_payloads_are_fixed_and_chunked(self):
        import struct
        self.assertEqual(struct.unpack('>II', workload.png()[16:24]), (256, 256))
        kitty = list(workload.image_chunks('kitty'))
        self.assertGreater(len(kitty), 1)
        self.assertTrue(all(len(part) < 4200 for part in kitty))
        self.assertIn(b'm=0;', kitty[-1])
        self.assertIn(b'width=256px', b''.join(workload.image_chunks('iterm2')))
        self.assertTrue(b''.join(workload.image_chunks('sixel')).endswith(b'\x1b\\'))


if __name__ == '__main__':
    unittest.main()
