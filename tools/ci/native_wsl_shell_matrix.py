"""Explicit WSL version/distro/shell checks; run only inside an approved Windows guest.

Uses the existing process owner and shell suites. No distro installation, host
configuration, profile edits, default-distro selection, or silent shell skips.
"""
from __future__ import annotations

import argparse
import base64
import hashlib
import json
import math
import os
from pathlib import Path
import re
import shutil
import statistics
import sys
import tempfile
import time

import qa_process

ROOT = Path(__file__).resolve().parents[2]
SHELLS = ("bash", "zsh", "fish")


class CheckFailure(RuntimeError):
    """A fixed operation label and bounded status, without child output."""

    def __init__(self, message: str, output: bytes):
        super().__init__(message)
        self.output = output  # Inspected for fixed receipts; never published.


def run(argv: list[str], *, cwd: Path = ROOT, environment=None, timeout=120,
        operation="WSL command") -> bytes:
    output = bytearray()

    def consume(chunk: bytes) -> None:
        if len(output) + len(chunk) > 1024 * 1024:
            raise ValueError("WSL check output limit exceeded")
        output.extend(chunk)

    result = qa_process.run(argv, cwd=cwd, environment=environment,
                            timeout_seconds=timeout, consume=consume)
    if result.error or result.timed_out or result.return_code != 0:
        # Keep terminal output, distro names and guest paths out of public errors.
        raise CheckFailure(f"{operation}: exit={result.return_code}, timeout={result.timed_out}, "
                           f"execution_or_cleanup_error={bool(result.error)}", bytes(output))
    return bytes(output)


def decode_inventory(data: bytes) -> str:
    if len(data) > 65536:
        raise ValueError("WSL inventory limit exceeded")
    encoding = "utf-16-le" if b"\0" in data or data.startswith(b"\xff\xfe") else "utf-8-sig"
    return data.decode(encoding).lstrip("\ufeff")


def selected_version(data: bytes, distro: str) -> int:
    # Only the requested name and final numeric version are significant. State
    # and headings are localized by Windows, and distro names may contain spaces.
    pattern = re.compile(r"^\s*\*?\s*" + re.escape(distro) + r"\s+.+?\s+([12])\s*$", re.I)
    matches = [int(match[1]) for line in decode_inventory(data).splitlines()
               if (match := pattern.fullmatch(line))]
    if len(matches) != 1:
        raise ValueError("Selected WSL distro/version is missing or ambiguous")
    return matches[0]


def parse_case(value: str) -> tuple[str, int]:
    distro, separator, version = value.rpartition("=")
    if (not separator or version not in ("1", "2") or not distro
            or len(distro) > 256 or distro.startswith("-")
            or any(ord(char) < 32 for char in distro)):
        raise argparse.ArgumentTypeError("Use an explicit distro=1 or distro=2 case")
    return distro, int(version)


def identity(output: bytes, shell: str, distro: str, user: str = "alice") -> None:
    values = {}
    for name, encoded in re.findall(rb"\x1b\]1337;SetUserVar=(\w+)=([^\x07]*)\x07", output):
        values[name.decode("ascii")] = base64.b64decode(encoded, validate=True).decode("utf-8")
    expected = {"automexia_shell": "1", "automexia_shell_name": shell,
                "automexia_distro": distro, "automexia_shell_user": user,
                "automexia_prompt_active": "1"}
    if any(values.get(key) != value for key, value in expected.items()):
        raise ValueError("WSL prompt identity contract failed")
    if not values.get("automexia_os_name") or not values.get("automexia_shell_path", "").startswith("/"):
        raise ValueError("WSL prompt OS/shell path is missing")
    if b"\x1b]7;file://" not in output or b"\x1b]133;A" not in output:
        raise ValueError("WSL directory or prompt boundary is missing")


def timings(samples: list[float]) -> dict:
    if len(samples) < 30 or any(not math.isfinite(value) or value < 0 for value in samples):
        raise ValueError("At least 30 valid benchmark samples are required")
    return {"samples": len(samples), "median_ms": statistics.median(samples),
            "p95_ms": sorted(samples)[math.ceil(len(samples) * .95) - 1],
            "min_ms": min(samples), "max_ms": max(samples)}


def successful(rows: list[dict]) -> bool:
    return bool(rows) and all(row.get("status") == "passed" for row in rows)


def behavioral_receipt(output: bytes, shell: str) -> bool:
    # Zsh's preexec hook can prefix the receipt with prompt control sequences.
    # Normalize those bytes, then require the complete fixed receipt line.
    plain = re.sub(rb"\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x07\x1b]*\x07)", b"", output)
    receipt = f"PASS: {shell.capitalize()} behavioral assertions completed".encode()
    return receipt in plain.splitlines()


def shell_benchmarks(output: bytes, shell: str) -> dict:
    values = {}
    plain = re.sub(rb"\x1b(?:\[[0-?]*[ -/]*[@-~]|\][^\x07\x1b]*\x07)", b"", output).replace(b"\r\n", b"\n")
    pattern = (rb"^SHELL_BENCHMARK shell=" + shell.encode() +
               rb" operation=(adapter|alias-reload) p95=([0-9]+(?:\.[0-9]+)?) unit=(ms|s)$")
    for operation, number, unit in re.findall(pattern, plain, re.M):
        name = operation.decode()
        value = float(number) * (1000 if unit == b"s" else 1)
        if name in values or not math.isfinite(value):
            raise ValueError("Duplicate or invalid shell benchmark receipt")
        values[name] = {"p95_ms": value, "budget_ms": 50, "samples": 20, "warmups": 5,
                        "status": "passed" if value <= 50 else "failed"}
    if set(values) != {"adapter", "alias-reload"}:
        raise ValueError("Missing shell benchmark receipt")
    return values


def source_digest(root: Path) -> str:
    digest = hashlib.sha256()
    files = sorted(path for relative in ("shell-integration", "tools/ci")
                   for path in (root / relative).rglob("*")
                   if path.is_file() and "__pycache__" not in path.parts)
    if not files or len(files) > 4096:
        raise ValueError("WSL source manifest file bound failed")
    total = 0
    for path in files:
        total += path.stat().st_size
        if path.is_symlink() or total > 64 * 1024 * 1024:
            raise ValueError("WSL source manifest byte/link bound failed")
        digest.update(path.relative_to(root).as_posix().encode() + b"\0")
        digest.update(hashlib.sha256(path.read_bytes()).digest())
    return digest.hexdigest()


def guest(shells: list[str], samples: int) -> dict:
    distro = os.environ.get("WSL_DISTRO_NAME", "")
    if sys.platform != "linux" or not distro:
        raise ValueError("Guest checks require an actual WSL Linux process")
    import pwd
    user = pwd.getpwuid(os.getuid()).pw_name
    if any(shutil.which(tool) is None for tool in ["python3", "git", *shells]):
        raise ValueError("Requested WSL shell or test prerequisite is missing")
    tests = {"bash": "test_shell_integration.sh", "zsh": "test_zsh_integration.zsh",
             "fish": "test_fish_integration.fish"}
    result = {"shells": [], "sources_sha256": source_digest(ROOT),
              "layer": "shell process startup plus first prompt hook; warm filesystem"}
    with tempfile.TemporaryDirectory(prefix="automexia-wsl-matrix-", dir="/tmp") as temporary:
        home = Path(temporary)
        # Stage only the owning sources on the Linux filesystem. Benchmark
        # samples must not measure repeated source reads across /mnt/c.
        native_root = home / "source"
        for relative in ("shell-integration", "tools/ci"):
            source_tree = ROOT / relative
            if any(path.is_symlink() for path in source_tree.rglob("*")):
                raise ValueError("WSL fixture sources must not contain links")
            shutil.copytree(source_tree, native_root / relative,
                            ignore=shutil.ignore_patterns("__pycache__"))
        if str(ROOT).startswith("/mnt/"):
            # Re-enter from the staged tree so the QA supervisor and Python
            # imports also use Linux storage during measured iterations.
            return json.loads(run([
                sys.executable, str(native_root / "tools/ci/native_wsl_shell_matrix.py"),
                "--guest", "--samples", str(samples), "--shells", *shells,
            ], cwd=native_root, timeout=1700, operation="Linux filesystem test driver"))
        environment = dict(os.environ, HOME=str(home), XDG_CONFIG_HOME=str(home / "config"),
                           AUTOMEXIA_CONFIG_HOME=str(home / "config" / "automexia"),
                           USER=user, TERM_PROGRAM="Automexia", AUTOMEXIA_SHELL_INTEGRATION="1",
                           AUTOMEXIA_PLAIN_LS="0", AUTOMEXIA_AMX="0", HISTFILE="/dev/null")
        for shell in shells:
            version_output = run([shell, "--version"], environment=environment, timeout=10)
            version = re.search(rb"\b[0-9]+(?:\.[0-9]+){1,3}\b", version_output)
            if version is None:
                raise ValueError("WSL shell version could not be recorded")
            # Reuse behavioral suites for completion, alias refresh, prompt
            # hooks, command status and hostile metadata; never replace them
            # with a synthetic startup-only compatibility claim.
            row = {"shell": shell, "version": version[0].decode(), "suite": "passed",
                   "behavioral_assertions": "passed", "identity": "not_run"}
            try:
                suite_output = run([shell, str(native_root / "tools/ci" / tests[shell])], cwd=native_root,
                    environment=environment, operation=f"{shell} behavioral suite")
            except CheckFailure as error:
                suite_output = error.output
                row["suite"] = "failed"
                row["behavioral_assertions"] = "passed" if behavioral_receipt(error.output, shell) else "failed"
                row["suite_error"] = str(error)
            try:
                row["benchmarks"] = shell_benchmarks(suite_output, shell)
                if not successful(list(row["benchmarks"].values())):
                    row["suite"] = "failed"
            except ValueError:
                row["suite"] = "failed"
                row["benchmark_error"] = "Missing, duplicate or invalid timing receipt"
            source = str(native_root / f"shell-integration/{shell}/automexia.{shell}")
            if shell == "fish":
                argv = [shell, "--no-config", "-c", "source $argv[1]; __automexia_fish_prompt", source]
            else:
                hook = "__automexia_pre_prompt" if shell == "bash" else "__automexia_precmd"
                flags = ["--noprofile", "--norc"] if shell == "bash" else ["-f"]
                argv = [shell, *flags, "-c", '. "$1"; ' + hook, "automexia-fixture", source]
            durations = []
            try:
                for index in range(samples + 3):
                    start = time.perf_counter()
                    output = run(argv, cwd=home, environment=dict(environment, AUTOMEXIA_PLAIN_LS="1"), timeout=15,
                                 operation=f"{shell} startup and prompt sample")
                    elapsed = (time.perf_counter() - start) * 1000
                    identity(output, shell, distro, user)
                    if index >= 3:
                        durations.append(elapsed)
                row.update(identity="passed", startup_and_hook=timings(durations))
            except (ValueError, CheckFailure):
                row["identity"] = "failed"
            row["status"] = "passed" if row["suite"] == row["identity"] == "passed" else "failed"
            result["shells"].append(row)
    result["status"] = "passed" if successful(result["shells"]) else "failed"
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--case", action="append", type=parse_case, default=[])
    parser.add_argument("--shells", nargs="+", choices=SHELLS, default=list(SHELLS))
    parser.add_argument("--samples", type=int, default=30)
    parser.add_argument("--report", type=Path)
    parser.add_argument("--guest", action="store_true")
    args = parser.parse_args()
    if not 30 <= args.samples <= 200 or len(set(args.shells)) != len(args.shells):
        parser.error("Use 30..200 samples and distinct shells")
    try:
        if args.guest:
            # Internal transport succeeds when it returns a complete report;
            # the Windows caller enforces every shell's status as the CI gate.
            print(json.dumps(guest(args.shells, args.samples)))
            return 0
        if sys.platform != "win32" or not args.case or args.report is None:
            raise ValueError("Windows guest, explicit cases and a report path are required")
        if len(args.case) > 16 or len({name.casefold() for name, _ in args.case}) != len(args.case):
            raise ValueError("Use 1..16 distinct distro cases")
        inventory = run(["wsl.exe", "--list", "--verbose"], timeout=15)
        # Fail before executing any suite if a requested row is absent/wrong.
        for distro, version in args.case:
            if selected_version(inventory, distro) != version:
                raise ValueError("Selected distro uses a different WSL version")
        rows = []
        for number, (distro, version) in enumerate(args.case, 1):
            linux_root = run(["wsl.exe", "--distribution", distro, "--exec", "wslpath", "-a", str(ROOT)], timeout=15).decode().rstrip("\r\n")
            if not linux_root.startswith("/") or any(ord(char) < 32 for char in linux_root):
                raise ValueError("WSL cannot access the staged source")
            payload = run(["wsl.exe", "--distribution", distro, "--exec", "python3",
                           linux_root + "/tools/ci/native_wsl_shell_matrix.py", "--guest",
                           "--samples", str(args.samples), "--shells", *args.shells], timeout=1800)
            rows.append({"case": number, "wsl_version": version, **json.loads(payload)})
        passed = successful(rows)
        report = {"schema_version": 1, "status": "passed" if passed else "failed", "cases": rows,
                  "driver_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                  "limitations": ["No WSLg, GPU, GUI, networking or every-distro certification",
                                  "No baseline comparison; timings include the QA process supervisor"]}
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print(f"{'PASS' if passed else 'FAIL'}: {len(rows)} explicit WSL case(s), {len(args.shells)} shells each; see report")
        return 0 if passed else 1
    except (OSError, ValueError, RuntimeError) as error:
        if args.report:
            args.report.parent.mkdir(parents=True, exist_ok=True)
            args.report.write_text('{"schema_version":1,"status":"failed"}\n', encoding="utf-8")
        detail = str(error) if isinstance(error, CheckFailure) else "prerequisite or contract failed"
        print("FAIL: WSL matrix " + detail, file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
