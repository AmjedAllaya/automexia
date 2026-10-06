"""Controlled-runner adapter; publish only allowlisted, validated JSON artifacts."""
import argparse
import json
import os
from pathlib import Path
import re
import sys
import uuid

from benchmark_model import BenchmarkError, compare, load, summarize, validate_profile, write
from collector import collect, ROOT


def main():
    key = os.environ.get('AUTOMEXIA_APPLICATION_RUN_KEY', '')
    if re.fullmatch(r'[0-9]+-[0-9]+', key) is None:
        raise BenchmarkError('controlled campaign run identity missing')
    output = ROOT / 'target/performance' / f'application-{key}'
    output.mkdir(parents=True, exist_ok=False)
    profile = validate_profile(json.loads(os.environ['AUTOMEXIA_APPLICATION_PROFILE']))
    fonts = json.loads(os.environ['AUTOMEXIA_APPLICATION_FONTS'])
    if not isinstance(fonts, list) or not 1 <= len(fonts) <= 1024 or any(not isinstance(p, str) for p in fonts):
        raise BenchmarkError('controlled font inventory missing')
    private = ROOT / '.automexia-private' / f'application-ci-{uuid.uuid4().hex}'
    private.mkdir(parents=True)
    write(private / 'profile.json', profile)
    candidate_path = output / 'candidate.json'
    result = collect(argparse.Namespace(profile=private / 'profile.json', font_file=fonts,
                     binary=ROOT / 'target/release' / ('automexia.exe' if os.name == 'nt' else 'automexia'),
                     work_dir=private / 'runs', output=candidate_path, runs=5, scenario=None,
                     desktop=True, diagnostic=False))
    candidate = load(candidate_path)
    write(output / 'summary.json', dict(schema=1, kind='application-summary', commit=candidate['commit'], metrics=summarize(candidate)))
    baseline = os.environ.get('AUTOMEXIA_APPLICATION_BASELINE', '')
    if not baseline:
        write(output / 'comparison.json', dict(schema=1, kind='application-comparison', status='baseline-not-configured'))
        print('Application evidence collected; no regression baseline is configured.')
        return result
    comparison = compare(load(Path(baseline)), candidate)
    write(output / 'comparison.json', comparison)
    print('Application comparison: ' + comparison['status'])
    return result or (0 if comparison['status'] == 'pass' else 1 if comparison['status'] == 'regression' else 2)


if __name__ == '__main__':
    try:
        raise SystemExit(main())
    except (BenchmarkError, OSError, ValueError, KeyError) as error:
        # Do not print paths, environment values or raw runner configuration.
        print(f'Controlled application campaign failed ({type(error).__name__}).', file=sys.stderr)
        raise SystemExit(2)
