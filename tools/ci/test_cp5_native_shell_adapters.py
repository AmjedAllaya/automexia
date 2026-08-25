#!/usr/bin/env python3
"""Exercise real Zsh and Fish editor replacement APIs through CP5 pipes."""

from __future__ import annotations

import os
from pathlib import Path
import select
import shutil
import subprocess
import struct
import sys
import time

if os.name != "nt":
    import fcntl
    import termios

from test_cp5_native_shell_bridge import decode_request, shlex_quote


ROOT = Path(__file__).resolve().parents[2]
ZSH_ADAPTER = ROOT / "shell-integration/suggestions/zsh/automexia-suggestions.zsh"
FISH_ADAPTER = ROOT / "shell-integration/suggestions/fish/automexia-suggestions.fish"
IO_TIMEOUT_SECONDS = 10.0


def wait_readable(fd: int) -> None:
    readable, _, _ = select.select([fd], [], [], IO_TIMEOUT_SECONDS)
    if not readable:
        raise AssertionError("native shell did not publish bounded bridge output")


def run_zsh_case(response: str, expected_line: str, expected_reason: str) -> None:
    original_line = "echo café ca"
    original_cursor = len(original_line)
    expected_start = len("echo café ".encode("utf-8"))
    request_read, request_write = os.pipe()
    response_read, response_write = os.pipe()
    env = os.environ.copy()
    env["AUTOMEXIA_SUGGESTION_REQUEST_FD"] = str(request_write)
    env["AUTOMEXIA_SUGGESTION_RESPONSE_FD"] = str(response_read)
    env["AUTOMEXIA_SUGGESTION_PREVIEW"] = "1"
    script = f"""
source {shlex_quote(str(ZSH_ADAPTER))}
function zle {{ return 0 }}
automexia_suggestions_enable || exit 70
[[ $__automexia_suggestion_state == ready-unbound && $__automexia_suggestion_reason == none ]] || exit 71
__automexia_suggestion_state=ready
BUFFER={shlex_quote(original_line)}
CURSOR={original_cursor}
__automexia_suggestions_request
printf '%s\\t%s\\t%s\\n' "$BUFFER" "$CURSOR" "$__automexia_suggestion_reason"
"""
    process = subprocess.Popen(
        ["zsh", "-f", "-c", script],
        cwd=ROOT,
        env=env,
        pass_fds=(request_write, response_read),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    os.close(request_write)
    os.close(response_read)
    wait_readable(request_read)
    request = decode_request(request_read)
    os.close(request_read)
    assert request["buffer"] == original_line
    assert request["cursor"] == len(original_line.encode("utf-8")), request
    assert request["start"] == expected_start
    assert request["end"] == request["cursor"]
    rendered = response.format(
        generation=request["generation"],
        start=request["start"],
        end=request["end"],
    )
    os.write(response_write, rendered.encode("ascii"))
    os.close(response_write)
    stdout, stderr = process.communicate(timeout=IO_TIMEOUT_SECONDS)
    assert process.returncode == 0, stderr
    line, cursor, reason = stdout.rstrip("\n").split("\t")
    assert line == expected_line, (line, cursor, reason, stderr)
    assert cursor == str(len(expected_line) if expected_reason == "accepted" else original_cursor)
    assert reason == expected_reason


def configure_fish_child(
    slave_fd: int,
    request_write: int,
    response_read: int,
    report_write: int,
) -> None:
    os.setsid()
    fcntl.ioctl(slave_fd, termios.TIOCSCTTY, 0)
    os.dup2(request_write, 3)
    os.dup2(response_read, 4)
    os.set_inheritable(3, True)
    os.set_inheritable(4, True)
    os.set_inheritable(report_write, True)


def read_report(fd: int) -> str:
    wait_readable(fd)
    output = bytearray()
    deadline = time.monotonic() + IO_TIMEOUT_SECONDS
    while b"\n" not in output:
        if time.monotonic() >= deadline:
            raise AssertionError("Fish report was not newline terminated")
        output.extend(os.read(fd, 4096))
    return bytes(output[: output.index(b"\n")]).decode("utf-8", "strict")


def queued_bytes(fd: int) -> int:
    operation = getattr(termios, "FIONREAD", None)
    if operation is None:
        return -1
    try:
        result = fcntl.ioctl(fd, operation, struct.pack("I", 0))
    except OSError:
        return -1
    return struct.unpack("I", result)[0]

def run_fish_case(response: str, expected_line: str, expected_reason: str) -> None:
    original_line = "echo café ca"
    original_cursor = len(original_line)
    request_read, request_write = os.pipe()
    response_read, response_write = os.pipe()
    response_probe = os.dup(response_read)
    report_read, report_write = os.pipe()
    master, slave = os.openpty()
    env = os.environ.copy()
    env.update(
        {
            "TERM": "xterm-256color",
            "AUTOMEXIA_SUGGESTION_REQUEST_FD": "3",
            "AUTOMEXIA_SUGGESTION_RESPONSE_FD": "4",
            "AUTOMEXIA_SUGGESTION_PREVIEW": "1",
            "AUTOMEXIA_TEST_REPORT_FD": str(report_write),
        }
    )
    process = subprocess.Popen(
        ["fish", "--no-config", "--interactive"],
        cwd=ROOT,
        env=env,
        stdin=slave,
        stdout=slave,
        stderr=slave,
        preexec_fn=lambda: configure_fish_child(
            slave, request_write, response_read, report_write
        ),
        close_fds=False,
    )
    os.close(slave)
    os.close(request_write)
    os.close(response_read)
    os.close(report_write)
    setup = "\n".join(
        [
            "function fish_prompt; end",
            f"source {shlex_quote(str(FISH_ADAPTER))}",
            "automexia_suggestions_enable",
            "or begin; printf 'ENABLE_FAILED\\n' >&$AUTOMEXIA_TEST_REPORT_FD; exit 70; end",
            'test "$__automexia_suggestion_state" = ready-unbound -a "$__automexia_suggestion_reason" = none',
            "or begin; printf 'ENABLE_STATE_FAILED\\n' >&$AUTOMEXIA_TEST_REPORT_FD; exit 71; end",
            "set -g __automexia_suggestion_state ready",
            "function __automexia_test_request",
            "printf 'ENTER\\n' >&$AUTOMEXIA_TEST_REPORT_FD",
            "__automexia_suggestions_request",
            "printf 'RETURNED\\n' >&$AUTOMEXIA_TEST_REPORT_FD",
            "printf '%s\\t%s\\t%s\\n' (commandline) (commandline --cursor) \"$__automexia_suggestion_reason\" >&$AUTOMEXIA_TEST_REPORT_FD",
            "exit",
            "end",
            "bind (printf '\\e[99~') __automexia_test_request",
            "printf 'READY\\n' >&$AUTOMEXIA_TEST_REPORT_FD",
            "",
        ]
    )
    try:
        os.write(master, setup.encode("utf-8"))
        assert read_report(report_read) == "READY"
        child_descriptor = Path(f"/proc/{process.pid}/fd/4")
        if child_descriptor.exists():
            child_response = os.readlink(child_descriptor)
            parent_response = os.readlink(f"/proc/self/fd/{response_write}")
            assert child_response == parent_response, "Fish response descriptor mismatch"
        os.write(master, original_line.encode("utf-8") + b"\x1b[99~")
        assert read_report(report_read) == "ENTER"
        wait_readable(request_read)
        request = decode_request(request_read)
        assert request["buffer"] == original_line
        assert request["cursor"] == len(original_line.encode("utf-8"))
        assert request["start"] == len("echo café ".encode("utf-8"))
        rendered = response.format(
            generation=request["generation"],
            start=request["start"],
            end=request["end"],
        )
        os.write(response_write, rendered.encode("ascii"))
        os.close(response_write)
        response_write = -1
        try:
            assert read_report(report_read) == "RETURNED"
            report = read_report(report_read)
        except AssertionError as error:
            terminal_bytes = bytearray()
            while select.select([master], [], [], 0)[0]:
                terminal_bytes.extend(os.read(master, 4096))
            pending = queued_bytes(response_probe)
            request_pending = queued_bytes(request_read)
            lowered = bytes(terminal_bytes).lower()
            markers = [
                marker.decode("ascii")
                for marker in (b"commandline:", b"read:", b"read>", b"math:", b"redirect", b"error")
                if marker in lowered
            ]
            raise AssertionError(
                f"Fish report missing; return={process.poll()}, terminal-bytes="
                f"{len(terminal_bytes)}, pending-reply-bytes={pending}, "
                f"pending-request-bytes={request_pending}, redacted-markers={markers}"
            ) from error
        line, cursor, reason = report.split("\t")
        assert line == expected_line, (line, cursor, reason)
        assert cursor == str(len(expected_line) if expected_reason == "accepted" else original_cursor)
        assert reason == expected_reason
        process.wait(timeout=IO_TIMEOUT_SECONDS)
        assert process.returncode == 0
    finally:
        for fd in (master, request_read, response_write, response_probe, report_read):
            try:
                os.close(fd)
            except OSError:
                pass
        if process.poll() is None:
            process.terminate()
            process.wait(timeout=IO_TIMEOUT_SECONDS)


def main() -> int:
    if os.name == "nt":
        print("EXTERNAL: native Zsh/Fish editor APIs are unavailable on Windows")
        return 0
    missing = [shell for shell in ("zsh", "fish") if shutil.which(shell) is None]
    if missing:
        print(f"EXTERNAL: native shell runtime missing: {', '.join(missing)}")
        return 0

    insertion_text = "café-€-🚀"
    insertion = insertion_text.encode("utf-8").hex().upper()
    replacement = (
        f"AXSR1\tR\t{{generation}}\t1\t{{start}}\t{{end}}\t{insertion}\n"
    )
    run_zsh_case(replacement, f"echo café {insertion_text}", "accepted")
    run_zsh_case("AXSR1\tS\t2\n", "echo café ca", "no-replacement")
    run_fish_case("AXSR1\tS\t2\n", "echo café ca", "no-replacement")
    run_fish_case(replacement, f"echo café {insertion_text}", "accepted")
    for runner in (run_zsh_case, run_fish_case):
        for hostile_hex in (
            "80",
            "C0AF",
            "C2",
            "E08080",
            "E282",
            "EDA080",
            "F0808080",
            "F09F92",
            "F4908080",
            "FF",
            "09",
            "C280",
            "D89C",
            "E280AE",
        ):
            runner(
                f"AXSR1\tR\t{{generation}}\t1\t{{start}}\t{{end}}\t{hostile_hex}\n",
                "echo café ca",
                "helper-disconnected",
            )
        runner("AXSR1\tS\t9\n", "echo café ca", "helper-disconnected")
        runner("AXSR1\tS\t2\textra\n", "echo café ca", "helper-disconnected")
        runner("AXSR1\tS\t" + "2" * 2176 + "\n", "echo café ca", "helper-disconnected")
    print("PASS: native Zsh and Fish request/replacement/status editor paths")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
