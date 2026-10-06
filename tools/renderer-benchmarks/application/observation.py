"""Normalize bounded numeric observations; missing completions never become samples."""
from benchmark_model import BenchmarkError, exact, io
from pathlib import Path

MAX_TRACE_BYTES = 4 * 1024 * 1024
CLOCK_TOLERANCE_NS = 1_000_000
KINDS = {'window', 'present', 'input', 'resize', 'search-begin', 'search-end', 'fixture'}


def integer(value, low, high):
    if type(value) is not int or not low <= value <= high:
        raise BenchmarkError('invalid bounded observation number')
    return value


def read(path):
    try:
        return io.read_json(Path(path), MAX_TRACE_BYTES, 'native observation')
    except (io.AssuranceError, OSError, RecursionError, OverflowError) as error:
        raise BenchmarkError('missing or invalid native observation') from error


def clocks(value):
    start = integer(value['wall_start_ns'], 1, 2**64)
    end = integer(value['wall_end_ns'], start, 2**64)
    duration = integer(value['duration_ns'], 1, 300_000_000_000)
    if abs(end - start - duration) > CLOCK_TOLERANCE_NS:
        raise BenchmarkError('wall clock changed during observation')


def validate_trace(value):
    exact(value, {'schema', 'kind', 'complete', 'os', 'arch', 'optimized', 'wall_start_ns',
                  'wall_end_ns', 'duration_ns', 'events'}, 'native trace')
    if (type(value['schema']) is not int or value['schema'] != 1
            or value['kind'] != 'application-observation' or value['complete'] is not True
            or value['os'] not in ('windows', 'linux', 'macos')
            or value['arch'] not in ('x86_64', 'aarch64', 'x86')
            or type(value['optimized']) is not bool):
        raise BenchmarkError('incomplete or unsupported native trace')
    clocks(value)
    if not isinstance(value['events'], list) or not 1 <= len(value['events']) <= 32768:
        raise BenchmarkError('native event budget invalid')
    previous = 0
    for event in value['events']:
        exact(event, {'at_ns', 'window', 'kind', 'a', 'b', 'c'}, 'event')
        previous = integer(event['at_ns'], previous, value['duration_ns'])
        # This campaign owns one window; panes share that window. Foreign
        # windows mean the controlled workload was not followed.
        integer(event['window'], 0, 0)
        if not isinstance(event['kind'], str) or event['kind'] not in KINDS:
            raise BenchmarkError('unknown native observation kind')
        for key in ('a', 'b', 'c'):
            integer(event[key], 0, 2**32 - 1)
    return value


def startup(trace, launch_wall_ns):
    offset = trace['wall_start_ns'] - integer(launch_wall_ns, 1, 2**64)
    if not 0 <= offset <= 60_000_000_000:
        raise BenchmarkError('launch clock does not match application observation')
    result = {}
    for name, kind in [('startup_window', 'window'), ('startup_terminal_frame', 'present')]:
        match = next((e for e in trace['events'] if e['kind'] == kind and e['a'] > 0), None)
        if match is None:
            raise BenchmarkError('startup endpoint missing')
        result[name] = [offset + match['at_ns']]
    return result


def durations(trace, kind, *, panes=1):
    begin = 'search-begin' if kind == 'search' else kind
    end = 'search-end' if kind == 'search' else 'present'
    result, pending = [], None
    for event in trace['events']:
        if event['kind'] == begin:
            if kind == 'input' and event['a'] != 1:
                continue  # Only the fixed x probe; never search/navigation keys.
            if pending is not None:
                raise BenchmarkError('overlapping/coalesced probes cannot prove independent latency')
            pending = event
        elif event['kind'] == end and pending is not None:
            if kind != 'search' and event['a'] != panes:
                continue
            if kind == 'resize' and (event['b'], event['c']) != (pending['a'], pending['b']):
                continue
            duration = event['at_ns'] - pending['at_ns']
            if duration <= 0:
                raise BenchmarkError('invalid probe duration')
            result.append(duration)
            pending = None
    if pending is not None:
        raise BenchmarkError('native probe has no matching completion')
    return result


def fixture_duration(trace, fixture, mode):
    exact(fixture, {'schema', 'mode', 'wall_start_ns', 'wall_end_ns', 'duration_ns',
                    'start_wall_ns', 'write_duration_ns', 'lines', 'image_side'}, 'fixture')
    if type(fixture['schema']) is not int or fixture['schema'] != 1 or fixture['mode'] != mode:
        raise BenchmarkError('fixture identity mismatch')
    clocks(fixture)
    start = integer(fixture['start_wall_ns'], fixture['wall_start_ns'], fixture['wall_end_ns'])
    integer(fixture['write_duration_ns'], 1, fixture['duration_ns'])
    integer(fixture['lines'], 1_000_000 if mode == 'output' else 0, 1_000_000 if mode == 'output' else 0)
    integer(fixture['image_side'], 0 if mode == 'output' else 256, 0 if mode == 'output' else 256)
    bit = {'output': 1, 'kitty': 2, 'iterm2': 4, 'sixel': 8}[mode]
    for event in trace['events']:
        if event['kind'] == 'fixture' and event['a'] & bit and (mode == 'output' or event['b'] > 0):
            elapsed = trace['wall_start_ns'] + event['at_ns'] - start
            if 0 < elapsed <= fixture['duration_ns']:
                return elapsed
    raise BenchmarkError('fixture completion never reached a submitted frame')
