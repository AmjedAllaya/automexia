#!/usr/bin/env python3
"""Measure bounded native-shell adapter registration overhead."""

from __future__ import annotations

import argparse
import math
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time


ROOT = Path(__file__).resolve().parents[2]
WARMUPS = 5
SAMPLES = 20
DEADLINE_SECONDS = 2.0
MAX_STDERR_BYTES = 65_536
MAX_P95_MS = 50.0
ADAPTERS = {
    "fish": ROOT / "shell-integration/completion/fish/automexia-completion.fish",
}


def percentile_95(values: list[float]) -> float:
    if not values:
        raise ValueError("at least one timing sample is required")
    ordered = sorted(values)
    return ordered[math.ceil(len(ordered) * 0.95) - 1]


def run_exact(command: list[str], environment: dict[str, str]) -> float:
    started = time.perf_counter_ns()
    completed = subprocess.run(
        command,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        env=environment,
        check=False,
        timeout=DEADLINE_SECONDS,
    )
    elapsed_ms = (time.perf_counter_ns() - started) / 1_000_000
    if len(completed.stderr) > MAX_STDERR_BYTES:
        raise RuntimeError("adapter benchmark exceeded its stderr ceiling")
    if completed.returncode != 0:
        summary = completed.stderr.decode("utf-8", errors="replace").splitlines()
        raise RuntimeError(
            f"adapter benchmark failed with {completed.returncode}: "
            f"{summary[0] if summary else 'no stderr'}"
        )
    return elapsed_ms


def measure_fish(adapter: Path) -> float:
    executable = shutil.which("fish")
    if not executable:
        raise RuntimeError("Fish is not installed")
    environment = os.environ.copy()
    baseline = [executable, "--no-config", "-c", "true"]
    candidate = [
        executable,
        "--no-config",
        "-c",
        'source "$argv[1]"',
        str(adapter),
    ]
    overheads: list[float] = []
    for iteration in range(WARMUPS + SAMPLES):
        baseline_ms = run_exact(baseline, environment)
        candidate_ms = run_exact(candidate, environment)
        if iteration >= WARMUPS:
            overheads.append(max(0.0, candidate_ms - baseline_ms))
    return percentile_95(overheads)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--shell", choices=sorted(ADAPTERS), required=True)
    args = parser.parse_args()
    adapter = ADAPTERS[args.shell]
    if adapter.is_symlink() or not adapter.is_file():
        print(f"completion adapter is missing or linked: {adapter}", file=sys.stderr)
        return 1
    try:
        p95_ms = measure_fish(adapter)
    except (OSError, RuntimeError, subprocess.TimeoutExpired, ValueError) as error:
        print(f"completion adapter benchmark failed: {error}", file=sys.stderr)
        return 1
    if p95_ms > MAX_P95_MS:
        print(
            f"completion adapter p95 exceeded {MAX_P95_MS:.0f} ms: "
            f"shell={args.shell} p95={p95_ms:.2f} ms",
            file=sys.stderr,
        )
        return 1
    print(f"shell={args.shell} adapter-p95={p95_ms:.2f}ms budget={MAX_P95_MS:.0f}ms")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
