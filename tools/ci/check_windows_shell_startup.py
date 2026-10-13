#!/usr/bin/env python3
"""Exercise native startup with the exact ConPTY runtime shipped in packages."""
import argparse
import json
import os
from pathlib import Path
import sys

import qa_process
from check_inline_pipeline import NativeTestStage
from check_ssh_library import compiled_executable, test_count

ROOT = Path(__file__).resolve().parents[2]
MAX_LOG = 16 * 1024 * 1024


def run(command, report, label, timeout):
    output = bytearray()
    with (report / (label + ".log")).open("wb") as log:
        def consume(chunk):
            if len(output) + len(chunk) > MAX_LOG:
                raise ValueError("native startup log exceeded its limit")
            output.extend(chunk)
            log.write(chunk)
        result = qa_process.run(command, cwd=ROOT, timeout_seconds=timeout,
                                consume=consume, environment=dict(os.environ))
    return result, bytes(output)


def verify(report, compare_inbox):
    if os.name != "nt":
        raise ValueError("native Windows is required")
    report.mkdir(parents=True, exist_ok=True)
    result, output = run([
        "cargo", "test", "-p", "automexia-terminal", "--test", "powershell_startup",
        "--features", "visual-test-hooks", "--locked", "--no-run", "--message-format=json",
    ], report, "build", 1800)
    if result.return_code != 0 or result.error or result.timed_out:
        raise ValueError("native startup fixture build failed; inspect the bounded build log")
    executable = Path(compiled_executable(output, "powershell_startup", "test"))
    if not executable.is_file() or executable.is_symlink():
        raise ValueError("native startup fixture must be a regular file")
    stage = NativeTestStage(report)
    records = []
    try:
        stage.prepare(executable, offline=False)
        candidates = [("bundled", stage.executable)]
        if compare_inbox:
            if (executable.parent / "conpty.dll").exists():
                raise ValueError("inbox control contains a sibling ConPTY DLL")
            candidates.insert(0, ("inbox", executable))
        for label, binary in candidates:
            result, output = run([str(binary), "--include-ignored", "--test-threads=1"],
                                 report, label, 180)
            if result.error:
                stage.cleanup_safe = False
            passed = result.return_code == 0 and not result.error and not result.timed_out
            if passed:
                passed = test_count(output, "rust") == 3
            records.append({"runtime": label, "passed": passed,
                            "exit": result.return_code, "timed_out": result.timed_out,
                            "conpty_version": stage.version if label == "bundled" else None})
            sys.stdout.buffer.write(output[-12000:])
            sys.stdout.flush()
        (report / "summary.json").write_text(json.dumps(records, indent=2) + "\n", encoding="utf-8")
        return all(record["passed"] for record in records)
    finally:
        stage.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compare-inbox", action="store_true")
    parser.add_argument("--report-dir", type=Path, default=ROOT / "artifacts/windows-shell-startup")
    args = parser.parse_args()
    try:
        raise SystemExit(0 if verify(args.report_dir.resolve(), args.compare_inbox) else 1)
    except (OSError, ValueError, RuntimeError) as error:
        print(f"Native startup validation failed ({type(error).__name__}); inspect the bounded local report.", file=sys.stderr)
        raise SystemExit(1)
