"""Inspect shipped PE imports without executing the candidate binaries."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import platform
import re
import shutil
import sys

import qa_process


ROOT = Path(__file__).resolve().parents[2]
MAX_OUTPUT_BYTES = 128 * 1024
MAX_BINARY_BYTES = 128 * 1024 * 1024
VC_RUNTIME = re.compile(r"(?:(?:msvcp|msvcr|vcruntime|concrt|vcomp|vcamp|vccorlib)\d[\w.-]*|ucrtbased)\.dll", re.ASCII)


def capture(command: list[str], *, timeout: float = 30) -> str:
    output = bytearray()

    def consume(chunk: bytes) -> None:
        if len(output) + len(chunk) > MAX_OUTPUT_BYTES:
            raise RuntimeError("inspection output limit exceeded")
        output.extend(chunk)

    # DUMPBIN headings must be stable even on non-English developer images.
    environment = dict(os.environ, VSLANG="1033")
    result = qa_process.run(command, cwd=ROOT, timeout_seconds=timeout,
                            consume=consume, environment=environment)
    if result.timed_out or result.error or result.return_code != 0:
        raise RuntimeError("inspection tool failed, timed out, or exceeded its output limit")
    return output.decode("utf-8", errors="replace")


def find_dumpbin(explicit: Path | None = None) -> Path:
    if explicit is not None:
        if not explicit.is_file():
            raise RuntimeError("DUMPBIN executable is missing")
        return explicit.resolve()
    found = shutil.which("dumpbin.exe")
    if found:
        return Path(found).resolve()
    # MSVC is already a Windows packaging prerequisite. Use its installation
    # locator rather than depending on a particular Visual Studio version.
    program_files = os.environ.get("ProgramFiles(x86)") or os.environ.get("ProgramFiles")
    if program_files:
        locator = Path(program_files) / "Microsoft Visual Studio/Installer/vswhere.exe"
        if locator.is_file():
            host = "arm64" if platform.machine().lower() in ("arm64", "aarch64") else "x64"
            pattern = rf"VC\Tools\MSVC\**\bin\Host{host}\{host}\dumpbin.exe"
            output = capture([str(locator), "-latest", "-products", "*", "-find", pattern, "-utf8"])
            for line in output.splitlines()[:32]:
                candidate = Path(line.strip())
                if candidate.is_file():
                    return candidate.resolve()
    raise RuntimeError("DUMPBIN is required: use an MSVC developer shell or pass --dumpbin")


def parse_dependencies(output: str) -> set[str]:
    imports = {name.lower() for name in re.findall(r"^\s+([A-Za-z0-9_.-]+\.dll)\s*$", output, re.MULTILINE | re.IGNORECASE)}
    if (not re.search(r"^File Type: (?:EXECUTABLE IMAGE|DLL)\s*$", output, re.MULTILINE)
            or "Image has the following dependencies:" not in output
            or not re.search(r"^\s+Summary\s*$", output, re.MULTILINE)
            or not imports):
        raise RuntimeError("unrecognized or incomplete DUMPBIN dependency output")
    return imports


def external_crt(imports: set[str]) -> list[str]:
    return sorted(name for name in imports if VC_RUNTIME.fullmatch(name))


def check_binaries(binaries: list[Path], inspector: Path) -> None:
    if not 0 < len(binaries) <= 16:
        raise RuntimeError("invalid Windows binary inventory")
    # Validate the complete inventory before starting tool processes.
    for binary in binaries:
        if not binary.is_file() or not 0 < binary.stat().st_size <= MAX_BINARY_BYTES:
            raise RuntimeError("Windows runtime binary is missing or oversized")
    for binary in binaries:
        output = capture([str(inspector), "/nologo", "/dependents", str(binary.resolve())])
        forbidden = external_crt(parse_dependencies(output))
        if forbidden:
            raise RuntimeError(f"{binary.name}: external Visual C++ runtime imports: {', '.join(forbidden)}; "
                               "rebuild with static CRT before packaging")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dumpbin", type=Path)
    parser.add_argument("binaries", type=Path, nargs="+")
    args = parser.parse_args()
    try:
        check_binaries(args.binaries, find_dumpbin(args.dumpbin))
    except (OSError, RuntimeError, ValueError) as error:
        # OSError can contain private host paths; do not forward it to CI logs.
        detail = str(error) if isinstance(error, RuntimeError) else "could not inspect Windows runtime files"
        print(f"FAIL: {detail}", file=sys.stderr)
        return 1
    print(f"PASS: {len(args.binaries)} Windows binaries have no external Visual C++ runtime imports")
    return 0


if __name__ == "__main__":
    sys.exit(main())
