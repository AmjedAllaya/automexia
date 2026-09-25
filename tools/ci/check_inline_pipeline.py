#!/usr/bin/env python3
"""Run focused inline-table checks through Automexia's existing QA process owner.

Missing tools, zero-test matches, timeouts, and failed cleanup are failures.
No online services, Docker, Kubernetes, or GUI automation are required.
"""
from __future__ import annotations
import argparse
import importlib.util
import json
import math
import os
from pathlib import Path
import re
import shutil
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
MAX_LOG = 16 * 1024 * 1024
RESULT = re.compile(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;")


def test_count(output: str) -> int:
    if "test result: FAILED" in output:
        raise ValueError("a failed Rust test summary was observed")
    values = [(int(p), int(f), int(i)) for p, f, i in RESULT.findall(output)]
    if not values or any(f for _, f, _ in values) or sum(p for p, _, _ in values) == 0:
        raise ValueError("successful, nonzero executed test count was not observed")
    return sum(p for p, _, _ in values)


# Parse only recognized terminal decoration; retain raw bytes in the log.
# A bare carriage return is not silently erased: it can change visible meaning.
SGR = re.compile(r"\x1b\[[0-9;:]*m")
PYTHON_SUMMARY = re.compile(
    r"(?m)^Ran ([0-9]{1,9}) tests? in [0-9]+(?:\.[0-9]+)?[ \t]*s[ \t]*$"
)


def python_test_count(output: str) -> int:
    """Accept one complete, successful unittest run with executed tests.

    Windows CRLF and ANSI color are presentation, not test outcomes. Do not
    weaken the exit-code/timeout/cleanup checks in verify(), accept a bare OK,
    or count skipped/expected-failure runs as a fully passing self-test gate.
    """
    plain = SGR.sub("", output.replace("\r\n", "\n"))
    if "\r" in plain or "\x1b" in plain:
        raise ValueError("unrecognized control sequences in Python test output")
    summaries = list(PYTHON_SUMMARY.finditer(plain))
    if len(summaries) != 1:
        raise ValueError("exactly one complete Python test summary is required")
    summary = summaries[0]
    count = int(summary[1])
    if count <= 0:
        raise ValueError("Python self-tests executed zero tests")
    if plain[summary.end():].strip() != "OK":
        raise ValueError("Python self-tests lack a final plain OK status (no skips/failures allowed)")
    if re.search(r"(?m)^(?:FAILED\b|FAIL:|ERROR:|Traceback \(|OK(?:\s|$))", plain[:summary.start()]):
        raise ValueError("conflicting failure or success status in Python self-test output")
    return count


def load_process_owner(root: Path):
    path = root / "tools/ci/qa_process.py"
    if not path.is_file() or path.is_symlink():
        raise RuntimeError("tools/ci/qa_process.py is required as the contained QA process owner")
    spec = importlib.util.spec_from_file_location("automexia_inline_qa_process", path)
    if spec is None or spec.loader is None:
        raise RuntimeError("cannot load existing QA process owner")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def groups(baseline: bool, native: bool):
    if baseline:
        return [
            ("baseline-model", ["test", "--locked", "-p", "automexia-ui-model", "--test", "tables"], True),
            ("baseline-capture", ["test", "--locked", "-p", "automexia-terminal", "--lib", "inline_tables::tests"], True),
            ("baseline-renderer", ["test", "--locked", "-p", "automexia-terminal", "--bin", "automexia", "renderer::inline_table_tests"], True),
        ]
    result = [
        ("model-new", ["test", "--locked", "-p", "automexia-ui-model", "--test", "inline_pipeline"], True),
        ("model-existing", ["test", "--locked", "-p", "automexia-ui-model", "--test", "tables"], True),
        ("display-new", ["test", "--locked", "-p", "rio-vt", "--test", "display_capture_pipeline"], True),
        ("capture-new-and-existing", ["test", "--locked", "-p", "automexia-terminal", "--lib", "inline_tables::"], True),
        ("focused-viewer-existing", ["test", "--locked", "-p", "automexia-terminal", "--lib", "table_output::tests"], True),
        ("renderer-new-and-existing", ["test", "--locked", "-p", "automexia-terminal", "--bin", "automexia", "renderer::inline_table_tests"], True),
    ]
    if native:
        result.append(("native-host", ["test", "--locked", "-p", "automexia-terminal", "--test", "inline_pipeline_native", "--", "--test-threads=1"], True))
    return result


def verify(root: Path, report_dir: Path, *, baseline=False, native=True, bench=False,
           ready=False, timeout=1800.0, wsl_distro=None):
    if not math.isfinite(timeout) or timeout <= 0:
        raise ValueError("timeout must be positive and finite")
    cargo = shutil.which("cargo")
    if not cargo:
        raise RuntimeError("cargo is missing; install the repository's pinned Rust toolchain first")
    if wsl_distro and (os.name != "nt" or wsl_distro.startswith("-") or any(ord(c) < 32 for c in wsl_distro)):
        raise ValueError("--wsl-distro requires a Windows host and a valid explicit distro name")
    owner = load_process_owner(root)
    report_dir.mkdir(parents=True, exist_ok=True)
    cases = [(name, [cargo, *args], tests) for name, args, tests in groups(baseline, native)]
    if not baseline:
        cases.insert(0, ("python-runner", [sys.executable, str(root / "tools/ci/test_check_inline_pipeline.py")], False))
    if not baseline and wsl_distro:
        cases.append(("native-wsl-conpty", [cargo, "test", "--locked", "-p", "automexia-terminal", "--test", "inline_pipeline_native",
            "inline_pipeline_native_wsl_conpty", "--", "--ignored", "--test-threads=1"], True))
    if not baseline and bench:
        cases.append(("benchmark-smoke", [cargo, "bench", "--locked", "-p", "automexia-terminal", "--bench", "automexia_services",
            "--", "inline_pipeline", "--test"], False))
    if not baseline and ready:
        cases.append(("repository-ready", [cargo, "ready"], False))
    env = dict(os.environ, CARGO_TERM_COLOR="never", NO_COLOR="1",
               PYTHON_COLORS="0", PYTHONIOENCODING="utf-8")
    if wsl_distro:
        env["AUTOMEXIA_TABLE_TEST_WSL_DISTRO"] = wsl_distro
    records = []
    for name, argv, is_test in cases:
        print(f"[inline-pipeline] {name}: {' '.join(argv)}", flush=True)
        started = time.monotonic()
        capture = bytearray()
        path = report_dir / (name + ".log")
        with path.open("wb") as log:
            def consume(chunk):
                if len(capture) + len(chunk) > MAX_LOG:
                    raise ValueError("verification output exceeded its 16 MiB ceiling")
                capture.extend(chunk)
                log.write(chunk)
                log.flush()
            result = owner.run(argv, cwd=root, timeout_seconds=timeout, consume=consume, environment=env)
        output = capture.decode("utf-8", errors="replace")
        reason = result.error
        passed = None
        if result.return_code != 0 or result.timed_out:
            reason = reason or ("timeout" if result.timed_out else f"exit {result.return_code}")
        elif not reason and is_test:
            try:
                passed = test_count(output)
            except ValueError as error:
                reason = str(error)
        elif not reason and name == "python-runner":
            try:
                passed = python_test_count(output)
            except ValueError as error:
                reason = str(error)
        elif not reason and name == "benchmark-smoke":
            # Criterion smoke must actually select our group, not run zero benches.
            if "inline_pipeline" not in output or "Success" not in output:
                reason = "Criterion did not report a selected inline_pipeline smoke success"
        record = dict(name=name, command=argv, passed=passed, ok=not reason,
            reason=reason, elapsed_seconds=round(time.monotonic()-started, 3), log=str(path))
        records.append(record)
        (report_dir / "report.json").write_text(json.dumps({
            "schema": 1, "baseline": baseline, "native_requested": native,
            "wsl_requested": bool(wsl_distro), "native_gui_measured": False,
            "checks": records, "complete": not reason and len(records) == len(cases),
        }, indent=2) + "\n", encoding="utf-8")
        if reason:
            print(output[-8000:], file=sys.stderr)
            print(f"FAILED: {name}: {reason}. Full log: {path}", file=sys.stderr)
            return False
        print(f"  PASS" + (f" ({passed} tests)" if passed is not None else ""), flush=True)
    return True


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--baseline", action="store_true")
    p.add_argument("--no-native", action="store_true", help="explicitly omit native transport evidence")
    p.add_argument("--bench", action="store_true", help="Criterion correctness smoke; not a latency measurement")
    p.add_argument("--ready", action="store_true", help="also run the full repository gate")
    p.add_argument("--timeout", type=float, default=1800)
    p.add_argument("--wsl-distro")
    p.add_argument("--report-dir", type=Path, default=ROOT / "target/inline-pipeline-checks")
    a = p.parse_args()
    try:
        return 0 if verify(ROOT, a.report_dir, baseline=a.baseline, native=not a.no_native,
            bench=a.bench, ready=a.ready, timeout=a.timeout, wsl_distro=a.wsl_distro) else 1
    except (ValueError, RuntimeError, OSError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 2

if __name__ == "__main__":
    raise SystemExit(main())
