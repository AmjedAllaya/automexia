#!/usr/bin/env python3
"""Exercise the real Bash CP5 adapter against independent pipe/protocol oracles."""

from __future__ import annotations

import os
from pathlib import Path
import select
import shutil
import struct
import subprocess
import time
import sys


ROOT = Path(__file__).resolve().parents[2]
ADAPTER = ROOT / "shell-integration/suggestions/bash/automexia-suggestions.bash"


def read_exact(stream: int, count: int) -> bytes:
    output = bytearray()
    deadline = time.monotonic() + 10.0
    while len(output) < count:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise AssertionError("adapter did not finish a bounded request")
        readable, _, _ = select.select([stream], [], [], remaining)
        if not readable:
            shape = describe_partial_request(bytes(output), count)
            raise AssertionError(
                f"adapter request readiness timed out: {len(output)}/{count} bytes; {shape}"
            )
        chunk = os.read(stream, count - len(output))
        if not chunk:
            raise AssertionError("adapter closed a fragmented request")
        output.extend(chunk)
    return bytes(output)


def describe_partial_request(payload: bytes, declared: int) -> str:
    """Return only structural lengths; never include private editor bytes."""
    try:
        view = memoryview(payload)
        buffer_length = struct.unpack_from("<I", view, 0)[0]
        offset = 4 + buffer_length
        cursor = struct.unpack_from("<I", view, offset)[0]
        offset += 4
        generation = struct.unpack_from("<Q", view, offset)[0]
        offset += 8
        start = struct.unpack_from("<I", view, offset)[0]
        end = struct.unpack_from("<I", view, offset + 4)[0]
        offset += 10
        candidate_count = struct.unpack_from("<I", view, offset)[0]
        offset += 4
    except (IndexError, struct.error):
        return "fixed structural fields incomplete and private bytes redacted"
    prefix = (
        f"declared={declared}, buffer={buffer_length}, cursor={cursor}, "
        f"generation={generation}, span={start}:{end}, candidates={candidate_count}"
    )
    candidate_lengths = []
    for index in range(candidate_count):
        if offset + 4 > len(view):
            return f"{prefix}, missing length prefix for candidate {index + 1}"
        length = struct.unpack_from("<I", view, offset)[0]
        candidate_lengths.append(length)
        offset += 4
        available = len(view) - offset
        if available < length:
            return (
                f"{prefix}, lengths={candidate_lengths}, candidate {index + 1} "
                f"has {available}/{length} bytes"
            )
        offset += length
    return f"{prefix}, lengths={candidate_lengths}, structural-end={offset}"
def u32(payload: memoryview, offset: int) -> tuple[int, int]:
    return struct.unpack_from("<I", payload, offset)[0], offset + 4


def decode_request(stream: int) -> dict[str, object]:
    header = read_exact(stream, 11)
    assert header[:4] == b"AXSH"
    assert struct.unpack_from("<H", header, 4)[0] == 1
    assert header[6] == 1
    declared = struct.unpack_from("<I", header, 7)[0]
    assert declared <= 512 * 1024
    payload = memoryview(read_exact(stream, declared))
    length, offset = u32(payload, 0)
    buffer = bytes(payload[offset : offset + length]).decode("utf-8", "strict")
    offset += length
    cursor, offset = u32(payload, offset)
    generation = struct.unpack_from("<Q", payload, offset)[0]
    offset += 8
    start, offset = u32(payload, offset)
    end, offset = u32(payload, offset)
    assert payload[offset] == 0
    assert payload[offset + 1] == 5
    offset += 2
    count, offset = u32(payload, offset)
    candidates: list[str] = []
    for _ in range(count):
        candidate_length, offset = u32(payload, offset)
        candidate = bytes(payload[offset : offset + candidate_length]).decode(
            "utf-8", "strict"
        )
        offset += candidate_length
        candidates.append(candidate)
    assert offset == len(payload)
    return {
        "buffer": buffer,
        "cursor": cursor,
        "generation": generation,
        "start": start,
        "end": end,
        "candidates": candidates,
    }


def run_bash_case(
    response: str,
    expected_line: str,
    expected_reason: str,
    original_line: str = "git ca",
    original_point: int = 6,
    expected_start: int = 4,
) -> None:
    request_read, request_write = os.pipe()
    response_read, response_write = os.pipe()
    env = os.environ.copy()
    env["AUTOMEXIA_SUGGESTION_REQUEST_FD"] = str(request_write)
    env["AUTOMEXIA_SUGGESTION_RESPONSE_FD"] = str(response_read)
    env["AUTOMEXIA_SUGGESTION_PREVIEW"] = "1"
    script = f"""
source {shlex_quote(str(ADAPTER))}
automexia_suggestions_enable || exit 70
[[ $__automexia_suggestion_state == ready-unbound && $__automexia_suggestion_reason == none ]] || exit 71
__automexia_suggestion_state=ready
READLINE_LINE={shlex_quote(original_line)}
READLINE_POINT={original_point}
__automexia_suggestions_request
printf '%s\\t%s\\t%s\\n' "$READLINE_LINE" "$READLINE_POINT" "$__automexia_suggestion_reason"
"""
    process = subprocess.Popen(
        ["bash", "--noprofile", "--norc", "-c", script],
        cwd=ROOT,
        env=env,
        pass_fds=(request_write, response_read),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    os.close(request_write)
    os.close(response_read)
    request = decode_request(request_read)
    os.close(request_read)
    assert request == {
        "buffer": original_line,
        "cursor": original_point,
        "generation": 1,
        "start": expected_start,
        "end": original_point,
        "candidates": [],
    }
    rendered = response.format(
        generation=request["generation"],
        start=request["start"],
        end=request["end"],
    )
    os.write(response_write, rendered.encode("ascii"))
    os.close(response_write)
    stdout, stderr = process.communicate(timeout=10)
    assert process.returncode == 0, stderr
    line, point, reason = stdout.rstrip("\n").split("\t")
    assert line == expected_line, (line, expected_line, point, reason, stderr)
    assert reason == expected_reason
    if expected_reason == "accepted":
        assert point == str(len(expected_line.encode("utf-8")))
    else:
        assert point == str(original_point)


def shlex_quote(value: str) -> str:
    return "'" + value.replace("'", "'\\''") + "'"


def main() -> int:
    if os.name == "nt" or shutil.which("bash") is None:
        print("EXTERNAL: native POSIX Bash pipe inheritance is unavailable on this host")
        return 0
    insertion_text = "café-€-🚀"
    insertion = insertion_text.encode("utf-8").hex().upper()
    run_bash_case(
        f"AXSR1\tR\t{{generation}}\t1\t{{start}}\t{{end}}\t{insertion}\n",
        f"git {insertion_text}",
        "accepted",
    )
    run_bash_case(
        f"AXSR1\tR\t{{generation}}\t1\t{{start}}\t{{end}}\t{insertion}\n",
        f"echo café {insertion_text}",
        "accepted",
        original_line="echo café ca",
        original_point=len("echo café ca".encode("utf-8")),
        expected_start=len("echo café ".encode("utf-8")),
    )
    run_bash_case(
        "AXSR1\tR\t{generation}\t1\t0\t{end}\t63616665\n",
        "git ca",
        "helper-disconnected",
    )
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
        run_bash_case(
            f"AXSR1\tR\t{{generation}}\t1\t{{start}}\t{{end}}\t{hostile_hex}\n",
            "git ca",
            "helper-disconnected",
        )
    run_bash_case("AXSR1\tS\t2\n", "git ca", "no-replacement")
    run_bash_case("AXSR1\tS\t9\n", "git ca", "helper-disconnected")
    run_bash_case("AXSR1\tS\t2\textra\n", "git ca", "helper-disconnected")
    run_bash_case("AXSR1\tS\t" + "2" * 2176 + "\n", "git ca", "helper-disconnected")
    print("PASS: native Bash helper request/replacement/stale/status paths")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
