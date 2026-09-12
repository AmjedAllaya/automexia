#!/usr/bin/env python3
"""Linux process lease for explicit Windows-backed WSL local-tool commands.

No shell evaluation, installation, persistent state or startup discovery. The
application supplies bounded argv, not executable shell text. Closing its stdin
lease retires the guest process group even if the Windows parent disappears.
"""
import json
import os
import selectors
import shutil
import signal
import subprocess
import sys
import time

PROGRAMS = {"rg", "tldr"}
MAX_REQUEST_BYTES = 16384
MAX_ARGUMENTS = 256


def unique_fields(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate request field")
        result[key] = value
    return result


def validate_request(raw):
    if len(raw.encode("utf-8")) > MAX_REQUEST_BYTES:
        raise ValueError("request limit")
    request = json.loads(raw, object_pairs_hook=unique_fields)
    if not isinstance(request, dict) or set(request) != {"version", "program", "arguments", "timeout_seconds"}:
        raise ValueError("request schema")
    if type(request["version"]) is not int or request["version"] != 1:
        raise ValueError("request version")
    if request["program"] not in PROGRAMS:
        raise ValueError("program rejected")
    args = request["arguments"]
    if not isinstance(args, list) or len(args) > MAX_ARGUMENTS:
        raise ValueError("argument count")
    for arg in args:
        if not isinstance(arg, str) or len(arg.encode("utf-8")) > 4096 or any(ord(c) < 32 or ord(c) == 127 for c in arg):
            raise ValueError("argument rejected")
    if type(request["timeout_seconds"]) is not int or not 1 <= request["timeout_seconds"] <= 60:
        raise ValueError("deadline rejected")
    return request


def resolve_tool(program):
    # An empty or relative PATH component would trust the current project merely
    # because it was opened. Only explicit absolute installed-tool locations apply.
    parts = os.environ.get("PATH", os.defpath).split(os.pathsep)
    if len(parts) > 256 or sum(len(part) for part in parts) > 16384:
        raise ValueError("PATH limit")
    return shutil.which(program, path=os.pathsep.join(part for part in parts if os.path.isabs(part)))


def supervise(request):
    program = resolve_tool(request["program"])
    if not program:
        sys.stderr.write("amx: required Linux tool is missing; install ripgrep (rg) or tealdeer (tldr) explicitly. Nothing was installed.\n")
        return 127
    selector = selectors.DefaultSelector()
    child = None
    try:
        selector.register(sys.stdin, selectors.EVENT_READ)
        # A closed lease must never launch a child in the first place.
        if selector.select(0):
            return 130
        child = subprocess.Popen([program, *request["arguments"]], stdin=subprocess.DEVNULL, start_new_session=True, close_fds=True)
        deadline = time.monotonic() + request["timeout_seconds"]
        while True:
            status = os.waitid(os.P_PID, child.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
            if status is not None:
                # Retain the zombie leader until after retiring its group. Reaping
                # first would permit PID reuse before a later group signal.
                retire(child)
                return child.returncode
            if time.monotonic() >= deadline:
                return 124
            if selector.select(min(0.01, max(0, deadline - time.monotonic()))):
                # The channel is a lease, not an input/evaluation protocol.
                return 130
    finally:
        if child is not None and child.returncode is None:
            retire(child)
        selector.close()


def retire(child):
    try:
        os.killpg(child.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    child.wait(timeout=2)


def main():
    previous = None
    try:
        if sys.platform != "linux" or len(sys.argv) != 2:
            raise ValueError("Linux bridge invocation required")
        previous = signal.signal(signal.SIGTERM, interrupted)
        return supervise(validate_request(sys.argv[1]))
    except (OSError, ValueError, TypeError, KeyError, subprocess.SubprocessError):
        sys.stderr.write("amx: Linux tool bridge failed; check the installed tools and session configuration. No command details were logged.\n")
        return 126
    except KeyboardInterrupt:
        return 130
    finally:
        if previous is not None:
            signal.signal(signal.SIGTERM, previous)


def interrupted(_signal, _frame):
    raise KeyboardInterrupt


if __name__ == "__main__":
    sys.exit(main())
