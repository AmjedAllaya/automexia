"""Contained application campaigns with disposable settings and exact argv."""
import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import platform
import secrets
import stat
import subprocess
import sys
import time

from benchmark_model import BenchmarkError, METRICS, validate_evidence, validate_profile, write
from observation import clocks, durations, fixture_duration, read, startup, validate_trace
import native
import qa_process
import qa
from workload import SEARCH_QUERY

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
SCENARIOS = ('idle', 'input', 'resize', 'search', 'four-pane', 'output', 'kitty', 'iterm2', 'sixel')
SCENARIO_METRICS = {
    'idle': ('startup_window', 'startup_terminal_frame', 'idle_rss', 'idle_cpu', 'idle_gpu'),
    'input': ('input_present',), 'resize': ('resize_reflow',), 'search': ('search',),
    'four-pane': ('four_pane_rss', 'four_pane_input_present'), 'output': ('output_1m',),
    'kitty': ('image_kitty', 'image_rss_delta'), 'iterm2': ('image_iterm2', 'image_rss_delta'),
    'sixel': ('image_sixel', 'image_rss_delta'),
}
CONFIG = '''confirm-before-quit = false
working-dir = {cwd}
[window]
width = 1280
height = 720
opacity = 1.0
[session-recovery]
enabled = false
[shell]
program = {python}
args = [{workload}, {mode}, "--seconds", {seconds}]
[bindings]
keys = [
  {{ key = "F5", action = "SplitRight" }},
  {{ key = "F6", action = "SplitDown" }},
  {{ key = "F8", action = "SearchForward" }}
]
'''
INTERACTIVE = {'input', 'resize', 'search', 'four-pane'}


def sha_file(path):
    path = Path(path)
    before = path.stat()
    if not stat.S_ISREG(before.st_mode) or not 0 < before.st_size <= 1024 * 1024 * 1024:
        raise BenchmarkError('benchmark input is not a bounded regular file')
    digest = hashlib.sha256()
    with open(path, 'rb') as stream:
        opened = os.fstat(stream.fileno())
        if (opened.st_dev, opened.st_ino, opened.st_size) != (before.st_dev, before.st_ino, before.st_size):
            raise BenchmarkError('benchmark input changed while opening')
        size = 0
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            size += len(block)
            if size > before.st_size:
                raise BenchmarkError('benchmark input grew while hashing')
            digest.update(block)
    after = path.stat()
    if size != before.st_size or (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns) != (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns):
        raise BenchmarkError('benchmark input changed while hashing')
    return digest.hexdigest()


def fingerprints(font_files):
    # Stable logical paths in the template, exact collector/fixture contents,
    # Python identity and font bytes. Temporary paths/nonces never define cohorts.
    files = sorted(HERE.glob('*.py'))
    workload = hashlib.sha256()
    for path in files:
        workload.update(path.name.encode()); workload.update(bytes.fromhex(sha_file(path)))
    for path in (ROOT / 'tools/ci/qa_process.py', ROOT / 'tools/ci/qa.py', ROOT / 'apps/automexia-terminal/src/application_benchmarks.rs'):
        workload.update(path.name.encode()); workload.update(bytes.fromhex(sha_file(path)))
    workload.update(platform.python_version().encode())
    workload.update(bytes.fromhex(sha_file(sys.executable)))
    if not 1 <= len(font_files) <= 1024:
        raise BenchmarkError('provide the pinned font files, including fallbacks')
    if sum(Path(path).stat().st_size for path in font_files) > 512 * 1024 * 1024:
        raise BenchmarkError('font inventory exceeds the campaign byte limit')
    fonts = hashlib.sha256()
    for digest in sorted(sha_file(path) for path in font_files):
        fonts.update(bytes.fromhex(digest))
    return dict(config_sha256=hashlib.sha256(CONFIG.encode()).hexdigest(),
                workload_sha256=workload.hexdigest(), fonts_sha256=fonts.hexdigest())


def source_identity():
    commit = qa.git_value('rev-parse', 'HEAD')
    status = qa.source_status_bytes()
    if len(commit) != 40 or any(c not in '0123456789abcdef' for c in commit) or status is None:
        raise BenchmarkError('bounded source identity unavailable')
    return commit, bool(status), qa.fingerprint_status_contents(status, ROOT)


def environment(directory, mode):
    keys = ('SystemRoot', 'WINDIR', 'PATH', 'PATHEXT', 'COMSPEC', 'DISPLAY', 'XAUTHORITY',
            'WAYLAND_DISPLAY', 'XDG_RUNTIME_DIR', 'DBUS_SESSION_BUS_ADDRESS')
    env = {key: os.environ[key] for key in keys if key in os.environ}
    for name in ('HOME', 'USERPROFILE', 'LOCALAPPDATA', 'APPDATA', 'XDG_CONFIG_HOME', 'XDG_CACHE_HOME',
                 'XDG_DATA_HOME', 'XDG_STATE_HOME', 'TMP', 'TEMP', 'TMPDIR'):
        env[name] = str(directory)
    env.update(AUTOMEXIA_CONFIG_HOME=str(directory), AUTOMEXIA_SHELL_INTEGRATION='0',
               AUTOMEXIA_BENCHMARK_TRACE=str(directory / 'trace.json'),
               AUTOMEXIA_BENCHMARK_TOKEN=secrets.token_hex(16),
               AUTOMEXIA_BENCHMARK_FIXTURE_OUTPUT=str(directory / 'fixture.json'),
               PYTHONDONTWRITEBYTECODE='1', PYTHONUTF8='1', LANG='C.UTF-8')
    return env


def run_worker(args):
    directory = Path(args.directory)
    mode = 'interactive' if args.scenario in INTERACTIVE else args.scenario
    config = CONFIG.format(cwd=json.dumps(str(directory)), python=json.dumps(sys.executable),
                           workload=json.dumps(str(HERE / 'workload.py')), mode=json.dumps(mode),
                           seconds=json.dumps('12' if mode == 'idle' else '45'))
    (directory / 'config.toml').write_text(config, encoding='utf-8')
    start_wall, start_mono = time.time_ns(), time.monotonic_ns()
    # This worker and all descendants are contained by the existing qa_process
    # supervisor. A worker exception/timeout is retired by that same owner.
    process = subprocess.Popen([args.binary], cwd=directory, env=environment(directory, mode),
                               stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    counter = native.Resources(process)
    metrics, phase = {}, None
    gpu, driver = None, None
    if args.scenario == 'idle':
        try:
            gpu = native.NvidiaGpu(args.gpu, args.gpu_driver)
        except (OSError, AttributeError, UnicodeError):
            pass
    try:
        if args.scenario in INTERACTIVE:
            driver = native.driver(process)
            time.sleep(4)  # Fixed warmup; trace verifies the resulting pane count.
            if args.scenario == 'four-pane':
                for key in ('F5', 'F6', 'F5'):
                    driver.key(key); time.sleep(.5)
                time.sleep(4)
                metrics['four_pane_rss'] = [counter.sample()[0]]
            phase = time.time_ns()
            if args.scenario in ('input', 'four-pane'):
                for _ in range(50):
                    driver.key('x'); time.sleep(.15)
            elif args.scenario == 'resize':
                for index in range(12):
                    driver.resize(1100 + (index % 2) * 100, 640 + (index % 2) * 80)
                    time.sleep(.3)
            else:
                driver.key('F8'); time.sleep(.3)
                for key in SEARCH_QUERY:
                    driver.key(key); time.sleep(.15)
            time.sleep(.5)
            # End fixed children through their own protocol. Killing a busy PTY
            # is a lifecycle stress scenario, not part of these latency probes.
            if args.scenario == 'search':
                driver.key('Escape'); time.sleep(.2)
            for _ in range(4 if args.scenario == 'four-pane' else 1):
                driver.key('q'); time.sleep(.5)
        elif args.scenario == 'idle':
            time.sleep(5)
            before = counter.sample()
            resident, utilization = [], []
            for _ in range(20):
                time.sleep(.25)
                resident.append(counter.sample()[0])
                if gpu is not None:
                    try:
                        utilization.append(gpu.sample())
                    except OSError:
                        gpu.close(); gpu = None; utilization.clear()
            after = counter.sample()
            metrics['idle_rss'] = resident
            metrics['idle_cpu'] = [(after[1] - before[1]) / (after[2] - before[2]) * 100]
            if utilization:
                metrics['idle_gpu'] = utilization
        elif args.scenario in ('kitty', 'iterm2', 'sixel'):
            # Fixed fixture warmup is 3 s. Sample before image submission and
            # after its decode window; final trace must prove an image frame.
            time.sleep(2)
            before = counter.sample()[0]
            time.sleep(2)
            metrics['image_rss_delta'] = [max(0, counter.sample()[0] - before)]
        process.wait(timeout=75)
    finally:
        if driver is not None:
            getattr(driver, 'dispose', lambda: None)()
        if gpu is not None:
            gpu.close()
    if process.returncode != 0:
        raise BenchmarkError('application did not exit successfully')
    finish_wall, finish_mono = time.time_ns(), time.monotonic_ns()
    clocks(dict(wall_start_ns=start_wall, wall_end_ns=finish_wall, duration_ns=finish_mono-start_mono))
    trace = validate_trace(read(directory / 'trace.json'))
    if trace['optimized'] is not (not args.diagnostic):
        raise BenchmarkError('application optimization does not match campaign profile')
    if trace['os'] != {'win32': 'windows', 'darwin': 'macos'}.get(sys.platform, sys.platform):
        raise BenchmarkError('application OS identity mismatch')
    windows = [event for event in trace['events'] if event['kind'] == 'window']
    if len(windows) != 1:
        raise BenchmarkError('native campaign did not create exactly one window')
    write(directory / 'geometry.json', dict(width=windows[0]['a'], height=windows[0]['b'], scale_milli=windows[0]['c']))
    if args.scenario == 'idle':
        metrics.update(startup(trace, start_wall))
    elif args.scenario in INTERACTIVE:
        trace['events'] = [e for e in trace['events'] if trace['wall_start_ns'] + e['at_ns'] >= phase]
        if args.scenario in ('input', 'four-pane'):
            name = 'input_present' if args.scenario == 'input' else 'four_pane_input_present'
            metrics[name] = durations(trace, 'input', panes=1 if args.scenario == 'input' else 4)
        else:
            metrics['resize_reflow' if args.scenario == 'resize' else 'search'] = durations(trace, args.scenario)
    else:
        elapsed = fixture_duration(trace, read(directory / 'fixture.json'), args.scenario)
        metrics['output_1m' if args.scenario == 'output' else f'image_{args.scenario}'] = [
            1_000_000 * 1_000_000_000 / elapsed if args.scenario == 'output' else elapsed]
    for name, samples in metrics.items():
        if len(samples) < METRICS[name].minimum_samples:
            raise BenchmarkError('native scenario produced too few completed probes')
    write(directory / 'samples.json', metrics)


def collect(args):
    profile = validate_profile(read(args.profile))
    observed_os = {'win32': 'windows', 'darwin': 'macos'}.get(sys.platform, sys.platform)
    arch = {'AMD64': 'x86_64', 'arm64': 'aarch64'}.get(platform.machine(), platform.machine())
    if (profile['os'] != observed_os or profile['arch'] != arch or profile['os_version'] != platform.release()
            or profile['logical_cpus'] != os.cpu_count() or 'application-benchmarks' not in profile['features']
            or profile['build_profile'] != ('debug-diagnostic' if args.diagnostic else 'release')):
        raise BenchmarkError('machine/build identity differs from pinned profile')
    for key, value in fingerprints(args.font_file).items():
        if profile[key] != value:
            raise BenchmarkError(f'pinned {key} differs from measured inputs')
    binary = Path(args.binary).resolve(strict=True)
    root = Path(args.work_dir).absolute()
    # Existing/link destinations are not reused. All temporary configuration and
    # traces are private campaign-owned files; only evidence.json is publishable.
    for parent in root.parents:
        if parent.is_symlink() or (hasattr(parent, 'is_junction') and parent.is_junction()):
            raise BenchmarkError('campaign path traverses a link')
    root.mkdir(parents=True, exist_ok=False)
    artifact = sha_file(binary)
    source_before = source_identity()
    commit, dirty, _ = source_before
    dirty = dirty or args.diagnostic
    metrics = {name: {'status': 'unavailable', 'reason': 'not-run'} for name in METRICS}
    collected = {name: [] for name in METRICS}
    failures = []
    for scenario in SCENARIOS:
        if args.scenario and scenario not in args.scenario:
            continue
        if scenario in INTERACTIVE and not args.desktop:
            failures.append({'scenario': scenario, 'reason': 'unsupported'})
            for name in SCENARIO_METRICS[scenario]:
                metrics[name] = dict(status='unavailable', reason='unsupported')
            continue
        for iteration in range(args.runs):
            directory = root / f'{scenario}-{iteration:02d}'
            directory.mkdir()
            command = [sys.executable, str(Path(__file__).resolve()), '--worker', '--binary', str(binary),
                       '--directory', str(directory), '--scenario', scenario,
                       '--gpu', profile['gpu'], '--gpu-driver', profile['gpu_driver']]
            if args.diagnostic:
                command.append('--diagnostic')
            result = qa_process.run(command, cwd=ROOT, timeout_seconds=120, consume=lambda _: None)
            reason = 'timeout' if result.timed_out else 'failed'
            if result.return_code != 0 or result.error or result.timed_out:
                failures.append({'scenario': scenario, 'run': iteration, 'reason': reason})
                for name in SCENARIO_METRICS[scenario]:
                    metrics[name] = dict(status='unavailable', reason=reason)
                print(f'{scenario} run {iteration+1}: {reason}', flush=True)
                break
            samples = read(directory / 'samples.json')
            geometry = read(directory / 'geometry.json')
            if geometry['scale_milli'] != profile['scale_milli']:
                raise BenchmarkError('native scale differs from pinned profile')
            for name, series in samples.items():
                if name not in METRICS:
                    raise BenchmarkError('worker returned unknown metric')
                # Image RSS is the worst of the three protocol deltas for each
                # independent run; retain its complete protocol-specific raw data.
                if name == 'image_rss_delta' and len(collected[name]) > iteration:
                    collected[name][iteration] = [max(collected[name][iteration][0], series[0])]
                else:
                    collected[name].append(series)
            print(f'{scenario} run {iteration+1}: observed', flush=True)
    if sha_file(binary) != artifact:
        raise BenchmarkError('application artifact changed during measurement')
    if fingerprints(args.font_file) != {key: profile[key] for key in ('config_sha256', 'workload_sha256', 'fonts_sha256')}:
        raise BenchmarkError('benchmark inputs changed during measurement')
    if source_identity() != source_before:
        raise BenchmarkError('source contents or revision changed during measurement')
    # Partial campaigns remain explicit and cannot pass comparison. All raw
    # successful runs remain in the private campaign for diagnosing failures.
    for name, series in collected.items():
        if len(series) == args.runs:
            metrics[name] = dict(status='measured', unit=METRICS[name].unit, runs=series)
        elif name == 'idle_gpu' and not any(f['scenario'] == 'idle' for f in failures):
            metrics[name] = dict(status='unavailable', reason='sensor-unavailable')
    if any(f['scenario'] in ('kitty', 'iterm2', 'sixel') for f in failures) or (
            args.scenario and not {'kitty', 'iterm2', 'sixel'}.issubset(args.scenario)):
        if metrics['image_rss_delta']['status'] == 'measured':
            metrics['image_rss_delta'] = dict(status='unavailable', reason='not-run')
    document = dict(schema=1, kind='application-benchmark', commit=commit, dirty=dirty,
                    artifact_sha256=artifact, measured_at_utc=dt.datetime.now(dt.timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ'),
                    profile=profile, metrics=metrics)
    write(root / 'campaign-status.json', dict(schema=1, failures=failures))
    write(args.output, validate_evidence(document))
    return 2 if failures else 0


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--worker', action='store_true', required=True)
    parser.add_argument('--binary', required=True)
    parser.add_argument('--directory', required=True)
    parser.add_argument('--scenario', choices=SCENARIOS, required=True)
    parser.add_argument('--diagnostic', action='store_true')
    parser.add_argument('--gpu', required=True)
    parser.add_argument('--gpu-driver', required=True)
    args = parser.parse_args()
    try:
        run_worker(args)
    except (BenchmarkError, OSError, subprocess.SubprocessError, ValueError) as error:
        # Private diagnosis is bounded and never copied to public evidence.
        write(Path(args.directory) / 'failure.json', {'type': type(error).__name__, 'message': str(error)[:2000]})
        print('Native application scenario failed; no successful sample was published.', file=sys.stderr)
        raise SystemExit(2)
