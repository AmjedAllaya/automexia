#!/usr/bin/env python3
"""Independent native client fixture; no network or user files are accessed."""
import json
import os
from pathlib import Path
import subprocess
import sys
import time

if Path(sys.argv[0]).name == "tldr":
    mode_file = Path(__file__).with_name("mode")
    mode = mode_file.read_text() if mode_file.exists() else "normal"
    if mode != "normal":
        with Path(__file__).with_name("calls.jsonl").open("a") as calls:
            calls.write(json.dumps(sys.argv[1:]) + "\n")
    if sys.argv[1:] == ["--version"]:
        print("unsupported-client 1.0" if mode == "unsupported" else "tealdeer 1.8.0")
    elif sys.argv[1:] == ["--no-auto-update", "--raw", "--color", "never", "--", "tar"]:
        if mode == "missing-cache":
            print("\x1b]52;fixture-client-diagnostic", file=sys.stderr)
            raise SystemExit(3)
        if mode == "invalid-text":
            sys.stdout.buffer.write(b"\xff")
        elif mode == "controls":
            print("touch amx-example-must-not-run\n\x1b]52;\u202eexample")
        else:
            print("# tar\nExample only: tar -tf {{archive.tar}}")
    else:
        raise SystemExit(8)
elif sys.argv[1] == "lease":
    marker = Path(sys.argv[2])
    pending = marker.with_suffix(".pending")
    pending.write_text(json.dumps([os.getpid()]))
    pending.replace(marker)
    time.sleep(30)
elif sys.argv[1] == "descendant":
    child = subprocess.Popen([sys.executable, __file__, "lease", sys.argv[2]])
    deadline = time.monotonic() + 5
    while not Path(sys.argv[2]).with_suffix(".ack").exists():
        if time.monotonic() >= deadline:
            raise SystemExit(9)
        time.sleep(0.005)
else:
    raise SystemExit(7)
