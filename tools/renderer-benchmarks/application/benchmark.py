#!/usr/bin/env python3
"""Application benchmark evidence CLI. Raw data and summaries remain separate."""

import argparse
import sys

from benchmark_model import BenchmarkError, compare, load, summarize, write


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    run = commands.add_parser("run", help="collect an isolated native application campaign")
    run.add_argument("--binary", required=True)
    run.add_argument("--profile", required=True)
    run.add_argument("--font-file", action="append", required=True)
    run.add_argument("--work-dir", required=True, help="new private campaign directory")
    run.add_argument("--output", required=True)
    run.add_argument("--runs", type=int, choices=range(5, 31), default=5)
    run.add_argument("--scenario", action="append", choices=("idle", "input", "resize", "search", "four-pane", "output", "kitty", "iterm2", "sixel"))
    run.add_argument("--desktop", action="store_true", help="authorize owned-window keyboard/resize probes on an idle desktop")
    run.add_argument("--diagnostic", action="store_true", help="debug validation only; evidence cannot pass comparison")
    identity = commands.add_parser("fingerprints", help="print current configuration/workload/font hashes")
    identity.add_argument("--font-file", action="append", required=True)
    summary = commands.add_parser("summarize")
    summary.add_argument("--input", required=True)
    summary.add_argument("--output", required=True)
    comparison = commands.add_parser("compare")
    comparison.add_argument("--baseline", required=True)
    comparison.add_argument("--candidate", required=True)
    comparison.add_argument("--output", required=True)
    args = parser.parse_args(argv)
    try:
        if args.command == "run":
            from collector import collect
            return collect(args)
        if args.command == "fingerprints":
            from collector import fingerprints
            import json
            print(json.dumps(fingerprints(args.font_file), sort_keys=True))
            return 0
        if args.command == "compare":
            report = compare(load(args.baseline), load(args.candidate))
            write(args.output, report)
            print("Application benchmark comparison: " + report["status"])
            return 0 if report["status"] == "pass" else 1 if report["status"] == "regression" else 2
        document = load(args.input)
        write(args.output, {"schema": 1, "kind": "application-summary",
                            "commit": document["commit"], "metrics": summarize(document)})
        return 0
    except BenchmarkError as error:
        print(f"Application benchmark error: {error}", file=sys.stderr)
        return 2
    except (OSError, ValueError) as error:
        print(f"Application benchmark operation failed ({type(error).__name__}).", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
