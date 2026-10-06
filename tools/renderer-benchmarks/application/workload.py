#!/usr/bin/env python3
"""Fixed synthetic PTY workloads. This is an exact executable, never a shell script.

No user files, history, credentials, commands or network services are consulted.
Output goes only to the owned benchmark PTY; timing metadata stays numeric.
"""

import argparse
import base64
import json
import os
import re
import struct
import sys
import threading
import time
import zlib

LINES = 1_000_000
IMAGE_SIDE = 256
PIXEL = bytes((64, 128, 192, 255))
SEARCH_QUERY = "searchneedle"


def output_chunks():
    """Exactly 1M 64-byte lines; bounded 64 KiB writes, no 1M-line allocation."""
    line = b"Automexia throughput fixture ".ljust(62, b".") + b"\r\n"
    assert len(line) == 64
    block = line * 1024
    for _ in range(LINES // 1024):
        yield block
    yield line * (LINES % 1024)


def png():
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    scanline = b"\x00" + PIXEL * IMAGE_SIDE
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", IMAGE_SIDE, IMAGE_SIDE, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(scanline * IMAGE_SIDE)) + chunk(b"IEND", b""))


def image_chunks(protocol):
    if protocol == "kitty":
        encoded = base64.b64encode(PIXEL * IMAGE_SIDE * IMAGE_SIDE)
        for offset in range(0, len(encoded), 4096):
            more = int(offset + 4096 < len(encoded))
            header = f"a=T,f=32,s={IMAGE_SIDE},v={IMAGE_SIDE},i=1,q=2," if offset == 0 else ""
            yield f"\x1b_G{header}m={more};".encode() + encoded[offset:offset + 4096] + b"\x1b\\"
    elif protocol == "iterm2":
        data = png()
        yield (f"\x1b]1337;File=inline=1;size={len(data)};width={IMAGE_SIDE}px;height={IMAGE_SIDE}px;preserveAspectRatio=1:".encode()
               + base64.b64encode(data) + b"\x07")
    elif protocol == "sixel":
        yield f'\x1bPq"1;1;{IMAGE_SIDE};{IMAGE_SIDE}#0;2;25;50;75'.encode()
        for y in range(0, IMAGE_SIDE, 6):
            bits = (1 << min(6, IMAGE_SIDE - y)) - 1
            yield b"#0!256" + bytes((63 + bits,)) + (b"-" if y + 6 < IMAGE_SIDE else b"")
        yield b"\x1b\\"
    else:
        raise ValueError("unknown fixed image workload")


def interactive(seconds):
    # A safety deadline belongs only to this disposable fixture process. It
    # cannot leave a blocked console read behind after a failed native driver.
    timer = threading.Timer(seconds, lambda: os._exit(0))
    timer.daemon = True
    timer.start()
    if os.name == "nt":
        import msvcrt
        read = msvcrt.getwch
        restore = lambda: None
    else:
        import termios
        import tty
        old = termios.tcgetattr(sys.stdin.fileno())
        tty.setraw(sys.stdin.fileno())
        read = lambda: os.read(sys.stdin.fileno(), 1).decode("ascii", errors="ignore")
        restore = lambda: termios.tcsetattr(sys.stdin.fileno(), termios.TCSANOW, old)
    try:
        while True:
            char = read()
            if char in ("q", "\x03", ""):
                return
            # Echo only the fixed input probe. Function/modifier keys do not
            # cause a command or an accidental child process.
            if char == "x":
                sys.stdout.buffer.write(b"x")
                sys.stdout.buffer.flush()
    finally:
        timer.cancel()
        restore()


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=("idle", "interactive", "output", "kitty", "iterm2", "sixel"))
    parser.add_argument("--seconds", type=int, default=30)
    args = parser.parse_args(argv)
    if not 5 <= args.seconds <= 120:
        parser.error("fixture lifetime must be 5..120 seconds")
    token = os.environ.get("AUTOMEXIA_BENCHMARK_TOKEN", "")
    destination = os.environ.get("AUTOMEXIA_BENCHMARK_FIXTURE_OUTPUT", "")
    if re.fullmatch(r"[0-9a-f]{32}", token) is None or not destination:
        parser.error("explicit benchmark fixture environment is required")
    # A single open owner avoids clobbering another pane's timing record.
    try:
        output = open(destination, "x", encoding="utf-8", newline="\n")
    except FileExistsError:
        if args.mode != "interactive":
            raise
        output = None
    with_context = {"schema": 1, "mode": args.mode}
    sys.stdout.buffer.write(b"\x1b[2J\x1b[H")
    if args.mode == "interactive":
        for index in range(1000):
            sys.stdout.buffer.write(f"benchmark row {index:04d} {SEARCH_QUERY}\r\n".encode())
    else:
        sys.stdout.buffer.write(b"Automexia fixed benchmark fixture\r\n")
    sys.stdout.buffer.flush()
    time.sleep(3)
    start_wall = time.time_ns()
    start = time.monotonic_ns()
    if args.mode in ("output", "kitty", "iterm2", "sixel"):
        chunks = output_chunks() if args.mode == "output" else image_chunks(args.mode)
        for chunk in chunks:
            sys.stdout.buffer.write(chunk)
        sys.stdout.buffer.write(f"\r\nAMX_BENCH_{token}_DONE_{args.mode.upper()}\r\n".encode())
        sys.stdout.buffer.flush()
        with_context.update(start_wall_ns=start_wall, write_duration_ns=time.monotonic_ns() - start,
                            lines=LINES if args.mode == "output" else 0,
                            image_side=IMAGE_SIDE if args.mode != "output" else 0)
        # The terminal must consume, decode and present before the child exits;
        # collection rejects the run when the sentinel never reached a frame.
        time.sleep(3)
    elif args.mode == "idle":
        time.sleep(args.seconds)
    else:
        interactive(args.seconds)
    with_context.update(wall_start_ns=start_wall, wall_end_ns=time.time_ns(),
                        duration_ns=time.monotonic_ns() - start)
    if output is not None:
        json.dump(with_context, output, separators=(",", ":"))
        output.write("\n")
        output.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
