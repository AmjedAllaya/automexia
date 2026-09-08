#!/usr/bin/env python3
"""Run Phase 0 verification and emit a bounded, redacted evidence bundle."""

from __future__ import annotations

import argparse
import collections
import codecs
import datetime as dt
import hashlib
import html
import io
import json
import os
import pathlib
import platform
import re
import shutil
import stat
import sys
import time
import zipfile
import xml.etree.ElementTree as element_tree

import qa_process

ROOT = pathlib.Path(__file__).resolve().parents[2]
MAX_LOG_BYTES = 2 * 1024 * 1024
MAX_TAIL_CHARS = 4096
MAX_BUNDLE_FILE_BYTES = 16 * 1024 * 1024
MAX_BUNDLE_BYTES = 64 * 1024 * 1024
MAX_JUNIT_BYTES = 8 * 1024 * 1024
MAX_JUNIT_NODES = 100_000
MAX_JUNIT_DEPTH = 8
# Match the build runner's minimum reserve; this is not a cold-build estimate.
MIN_QA_FREE_BYTES = 4 * 1024 * 1024 * 1024
DEFAULT_STEP_TIMEOUT_SECONDS = 30 * 60
STEP_TIMEOUT_SECONDS = {
    "qa-runner-self-tests": 120,
    "python-contract-mutations": 600,
    "github-free-assurance-policy": 120,
    "github-free-assurance-mutations": 120,
    "repository-walker-cache-scope": 120,
    "rustsec-exception-policy": 120,
    "s1-assurance-policy": 120,
    "s1-assurance-mutations": 300,
    "rustfmt": 300,
    "metadata": 300,
    "repository-contracts": 600,
    "repository-formats": 300,
    "shell-contracts": 900,
    "clippy": 3600,
    "nextest": 3000,
    "component-host": 900,
    "benchmark-smoke": 3600,
    "doctests": 1800,
    "resize-stress": 1800,
    "session-clone": 1800,
    "loom-channel": 900,
    "dependency-policy": 900,
    "coverage-build": 5400,
    "coverage-policy": 300,
    "native-windows-gui": 2400,
    "application-verifier": 3600,
    "wpr-native-trace": 3600,
    "benchmark-app": 7200,
    "benchmark-image": 7200,
    "benchmark-vt": 7200,
    "benchmark-channel": 7200,
    "benchmark-pty": 7200,
    "benchmark-ssh-inventory": 7200,
    "benchmark-quick-actions": 7200,
    "benchmark-connection-planning": 7200,
    "benchmark-quick-action-store": 7200,
    "benchmark-keybindings": 7200,
}
TOKEN_PATTERNS = (
    re.compile(r"(?i)(authorization\s*[:=]\s*)(?:bearer\s+)?[^\s]+"),
    re.compile(r"(?i)((?:token|secret|password|passwd|api[_-]?key)\s*[:=]\s*)[^\s]+"),
    re.compile(r"\b(?:ghp|github_pat|sk_live|sk_test)_[A-Za-z0-9_\-]{8,}\b"),
)


def truthy(name: str) -> bool:
    return os.environ.get(name, "").strip().lower() in {"1", "true", "yes", "on"}


def atomic_write(path: pathlib.Path, data: str | bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    if isinstance(data, str):
        temporary.write_text(data, encoding="utf-8", newline="\n")
    else:
        temporary.write_bytes(data)
    os.replace(temporary, path)


def make_redactor():
    replacements: list[tuple[str, str]] = [(str(ROOT), "<WORKSPACE>")]
    home = pathlib.Path.home()
    if str(home) not in {"", str(ROOT)}:
        replacements.append((str(home), "<HOME>"))
    original_replacements = list(replacements)
    replacements.extend(
        (value.replace("\\", "/"), label) for value, label in original_replacements
    )
    # Assertion diagnostics may lowercase or escape complete tracebacks.
    # Match literal roots regardless of case, with the most specific root first.
    # A run of backslashes also covers nested repr/JSON command diagnostics.
    prefix_patterns = [
        (re.compile(re.escape(source).replace(r"\\", r"\\+"), re.IGNORECASE), label)
        for source, label in sorted(
            dict(replacements).items(), key=lambda item: len(item[0]), reverse=True
        )
    ]

    def redact(value: str) -> str:
        for pattern, label in prefix_patterns:
            value = pattern.sub(label, value)
        for pattern in TOKEN_PATTERNS:
            value = pattern.sub(
                lambda match: (match.group(1) if match.lastindex else "") + "<REDACTED>",
                value,
            )
        return value

    return redact


REDACT = make_redactor()


def bounded_capture(command: list[str], timeout_seconds: int = 15) -> str | None:
    output = bytearray()

    def consume(chunk: bytes) -> None:
        if len(output) + len(chunk) > 32768:
            raise ValueError('version output exceeded its byte ceiling')
        output.extend(chunk)

    try:
        result = qa_process.run(
            command, cwd=ROOT, timeout_seconds=timeout_seconds, consume=consume,
        )
        if result.return_code != 0 or result.timed_out or result.error:
            return None
        return REDACT(output.decode('utf-8', errors='replace').replace(chr(0), ""))
    except (OSError, ValueError):
        return None


def safe_version(command: list[str]) -> str:
    output = bounded_capture(command)
    if output is None:
        return "unavailable"
    lines = output.strip().splitlines()
    return lines[0][:512] if lines else "unavailable"


def collect_host_manifest() -> dict[str, object]:
    manifest: dict[str, object] = {
        "os": platform.system(),
        "release": platform.release(),
        "architecture": platform.machine(),
        "python": platform.python_version(),
        "shells": {},
        "wsl": "not-applicable",
        "display": "not-observed",
        "dpi": "not-observed",
        "gpu": [],
        "renderer_backend": "not-observed-by-non-gui-qa",
        "power_profile": "not-observed",
    }
    shells: dict[str, str] = {}
    if os.name == "nt":
        shells["powershell"] = safe_version(
            [
                "powershell",
                "-NoProfile",
                "-Command",
                "$PSVersionTable.PSVersion.ToString()",
            ]
        )
        if shutil.which("pwsh"):
            shells["pwsh"] = safe_version(
                [
                    "pwsh",
                    "-NoProfile",
                    "-Command",
                    "$PSVersionTable.PSVersion.ToString()",
                ]
            )
        shells["cmd"] = safe_version(["cmd", "/D", "/C", "ver"])
        manifest["wsl"] = (
            safe_version(["wsl.exe", "--version"])
            if shutil.which("wsl.exe")
            else "unavailable"
        )
        try:
            import ctypes

            user32 = ctypes.windll.user32
            width = int(user32.GetSystemMetrics(0))
            height = int(user32.GetSystemMetrics(1))
            manifest["display"] = {"primary_pixels": [width, height]}
            get_dpi = getattr(user32, "GetDpiForSystem", None)
            if get_dpi is not None:
                dpi = int(get_dpi())
                manifest["dpi"] = {
                    "system": dpi,
                    "scale": round(dpi / 96.0, 3),
                }
        except (AttributeError, OSError, TypeError, ValueError):
            pass
        gpu_output = bounded_capture(
            [
                "powershell",
                "-NoProfile",
                "-Command",
                "Get-CimInstance Win32_VideoController | "
                "Select-Object Name,DriverVersion,CurrentHorizontalResolution,"
                "CurrentVerticalResolution | ConvertTo-Json -Compress",
            ]
        )
        if gpu_output:
            try:
                parsed = json.loads(gpu_output)
                entries = parsed if isinstance(parsed, list) else [parsed]
                manifest["gpu"] = [
                    {
                        key: str(entry.get(key, ""))[:160]
                        for key in (
                            "Name",
                            "DriverVersion",
                            "CurrentHorizontalResolution",
                            "CurrentVerticalResolution",
                        )
                    }
                    for entry in entries[:8]
                    if isinstance(entry, dict)
                ]
            except (TypeError, ValueError, json.JSONDecodeError):
                pass
        power_output = bounded_capture(["powercfg", "/getactivescheme"])
        if power_output:
            match = re.search(
                r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-"
                r"[0-9a-fA-F]{4}-[0-9a-fA-F]{12}",
                power_output,
            )
            if match:
                manifest["power_profile"] = {
                    "scheme_guid": match.group(0).lower()
                }
    else:
        for shell in ("sh", "bash", "zsh"):
            if shutil.which(shell):
                shells[shell] = safe_version([shell, "--version"])
        session_type = os.environ.get("XDG_SESSION_TYPE", "").strip().lower()
        if session_type in {"x11", "wayland", "tty"}:
            manifest["display"] = {"session_type": session_type}
    manifest["shells"] = shells
    return manifest

def git_value(*args: str) -> str:
    output = bounded_capture(['git', *args])
    return output.strip() if output is not None else 'unavailable'


MAX_SOURCE_STATUS_BYTES = 1024 * 1024
MAX_SOURCE_FILES = 8192
MAX_SOURCE_FILE_BYTES = 16 * 1024 * 1024
MAX_SOURCE_TOTAL_BYTES = 64 * 1024 * 1024
SOURCE_STATUS_TIMEOUT_SECONDS = 20


def source_status_bytes() -> bytes | None:
    """Read raw NUL-delimited status without truncation or disclosure to logs."""
    data = bytearray()

    def consume(chunk: bytes) -> None:
        if len(data) + len(chunk) > MAX_SOURCE_STATUS_BYTES:
            raise ValueError('source inventory exceeds its byte ceiling')
        data.extend(chunk)

    try:
        result = qa_process.run(
            ["git", "--no-pager", "--no-optional-locks", "-c", "core.fsmonitor=false", "status",
             "--porcelain=v1", "-z", "--untracked-files=all"],
            cwd=ROOT, timeout_seconds=SOURCE_STATUS_TIMEOUT_SECONDS,
            consume=consume, merge_stderr=False,
        )
    except (OSError, ValueError):
        return None
    if result.return_code != 0 or result.timed_out or result.error:
        return None
    return bytes(data)


def fingerprint_status_contents(status: bytes, root: pathlib.Path) -> str:
    """Hash dirty content and Git state; never serialize source bytes or paths."""
    if len(status) > MAX_SOURCE_STATUS_BYTES or (status and not status.endswith(b"\0")):
        raise ValueError("invalid source inventory")
    entries = iter(status.split(b"\0")[:-1])
    paths: set[bytes] = set()
    for entry in entries:
        if len(entry) < 4 or entry[2:3] != b" ":
            raise ValueError("invalid source inventory")
        paths.add(entry[3:])
        if b"R" in entry[:2] or b"C" in entry[:2]:
            paths.add(next(entries))
        if len(paths) > MAX_SOURCE_FILES:
            raise ValueError("source inventory exceeds limit")
    digest = hashlib.sha256(b"automexia-qa-content-v1\0" + status)
    total = 0
    for raw_path in sorted(paths):
        relative = pathlib.PurePosixPath(os.fsdecode(raw_path))
        if relative.is_absolute() or not relative.parts or any(part in ("..", ".git") for part in relative.parts) or b"\\" in raw_path or b":" in raw_path:
            raise ValueError("unsafe source inventory path")
        path = root.joinpath(*relative.parts)
        # Do not follow an untracked symlink/junction into private host storage.
        if any(parent.is_symlink() or (hasattr(parent, "is_junction") and parent.is_junction()) for parent in path.parents if parent != root and root in parent.parents):
            raise ValueError("source inventory crosses a link")
        digest.update(len(raw_path).to_bytes(8, "big"))
        digest.update(raw_path)
        try:
            before = path.lstat()
        except FileNotFoundError:
            digest.update(b"missing\0")
            continue
        if stat.S_ISLNK(before.st_mode):
            payload = os.fsencode(os.readlink(path))
            if len(payload) > MAX_SOURCE_FILE_BYTES:
                raise ValueError("source link exceeds limit")
            total += len(payload)
            if total > MAX_SOURCE_TOTAL_BYTES:
                raise ValueError("source contents exceed total limit")
            digest.update(b"link\0" + payload)
            continue
        if not stat.S_ISREG(before.st_mode) or before.st_size > MAX_SOURCE_FILE_BYTES:
            raise ValueError("source file exceeds limit or is not regular")
        total += before.st_size
        if total > MAX_SOURCE_TOTAL_BYTES:
            raise ValueError("source contents exceed total limit")
        digest.update(b"file\0" + before.st_mode.to_bytes(8, "big") + before.st_size.to_bytes(8, "big"))
        with path.open("rb") as source:
            opened = os.fstat(source.fileno())
            if (opened.st_dev, opened.st_ino, opened.st_size) != (before.st_dev, before.st_ino, before.st_size):
                raise ValueError("source changed while opening")
            remaining = before.st_size
            while remaining:
                chunk = source.read(min(remaining, 1024 * 1024))
                if not chunk:
                    raise ValueError("source changed while hashing")
                digest.update(chunk)
                remaining -= len(chunk)
            if source.read(1):
                raise ValueError("source grew while hashing")
        after = path.lstat()
        if (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns) != (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns):
            raise ValueError("source changed while hashing")
    return digest.hexdigest()


def dirty_fingerprint() -> str:
    try:
        status = source_status_bytes()
        if status is None:
            return "unavailable"
        result = fingerprint_status_contents(status, ROOT)
        return result if source_status_bytes() == status else "unavailable"
    except (OSError, ValueError, StopIteration, OverflowError):
        return "unavailable"


def source_identity_stable(before: tuple[str, str], after: tuple[str, str]) -> bool:
    return (
        before == after
        and re.fullmatch(r"(?:[0-9a-f]{40}|[0-9a-f]{64})", before[0]) is not None
        and re.fullmatch(r"[0-9a-f]{64}", before[1]) is not None
    )


def command_label(command: list[str]) -> str:
    return REDACT(" ".join(command))


def run_step(
    run_dir: pathlib.Path,
    index: int,
    name: str,
    command: list[str],
    *,
    env_add: dict[str, str] | None = None,
    required: bool = True,
    timeout_seconds: int | None = None,
) -> dict[str, object]:
    log_path = run_dir / "logs" / f"{index:02d}-{name}.log"
    log_path.parent.mkdir(parents=True, exist_ok=True)
    print(f"==> {name}: {command_label(command)}", flush=True)
    started = time.monotonic()
    started_utc = dt.datetime.now(dt.timezone.utc).isoformat()
    captured = 0
    truncated = False
    timed_out = False
    tail: collections.deque[str] = collections.deque(maxlen=24)
    environment = os.environ.copy()
    if env_add:
        environment.update(env_add)
    error = None
    return_code = None
    if timeout_seconds is None:
        timeout_seconds = STEP_TIMEOUT_SECONDS.get(name, DEFAULT_STEP_TIMEOUT_SECONDS)
    try:
        with log_path.open("w", encoding="utf-8", newline="\n") as log:

            def record_line(raw_line: str) -> None:
                nonlocal captured, truncated
                line = REDACT(raw_line)
                tail_line = line.rstrip()
                if len(tail_line) > MAX_TAIL_CHARS:
                    tail_line = tail_line[:MAX_TAIL_CHARS] + "…<tail-truncated>"
                tail.append(tail_line)
                if truncated:
                    return
                encoded = line.encode("utf-8")
                remaining = MAX_LOG_BYTES - captured
                if len(encoded) <= remaining:
                    log.write(line)
                    captured += len(encoded)
                    return
                marker = (
                    b"\n[log truncated at 2 MiB; subprocess output "
                    b"continued to be drained]\n"
                )
                prefix_budget = max(0, remaining - len(marker))
                prefix = encoded[:prefix_budget].decode("utf-8", errors="ignore")
                marker_budget = remaining - len(prefix.encode("utf-8"))
                log.write(prefix)
                log.write(marker[:marker_budget].decode("ascii", errors="ignore"))
                captured = MAX_LOG_BYTES
                truncated = True

            pending = ""
            suppressing_long_line = False
            decoder = io.IncrementalNewlineDecoder(
                codecs.getincrementaldecoder('utf-8')(errors='replace'), translate=True,
            )

            def consume_text(chunk: str) -> None:
                nonlocal pending, suppressing_long_line
                while chunk:
                    if suppressing_long_line:
                        newline = chunk.find("\n")
                        if newline < 0:
                            chunk = ""
                            continue
                        chunk = chunk[newline + 1 :]
                        suppressing_long_line = False
                        continue
                    newline = chunk.find("\n")
                    if newline >= 0:
                        candidate = pending + chunk[: newline + 1]
                        pending = ""
                        chunk = chunk[newline + 1 :]
                        if len(candidate) > MAX_TAIL_CHARS:
                            record_line("[overlong output line suppressed]\n")
                        else:
                            record_line(candidate)
                        continue
                    pending += chunk
                    chunk = ""
                    if len(pending) > MAX_TAIL_CHARS:
                        record_line("[overlong output line suppressed]\n")
                        pending = ""
                        suppressing_long_line = True

            result = qa_process.run(
                command, cwd=ROOT, environment=environment,
                timeout_seconds=timeout_seconds,
                consume=lambda raw: consume_text(decoder.decode(raw)),
            )
            return_code, timed_out, error = result.return_code, result.timed_out, result.error
            # The native owner has joined the only decoder/log consumer before
            # this final flush, including fragmented UTF-8 and a trailing CR.
            consume_text(decoder.decode(b'', final=True))
            if pending and not suppressing_long_line:
                record_line(pending)
        status = (
            "pass"
            if return_code == 0 and not timed_out and error is None
            else "fail"
        )
    except (OSError, ValueError) as exception:
        status = "fail"
        error = REDACT(f"{type(exception).__name__}: {exception}")
        atomic_write(log_path, error + "\n")
        tail.append(error)
    duration = round(time.monotonic() - started, 3)
    completed_utc = dt.datetime.now(dt.timezone.utc).isoformat()
    print(f"{status.upper()}: {name} ({duration:.3f}s)", flush=True)
    if status == "fail" and tail:
        print("\n".join(list(tail)[-12:]), file=sys.stderr, flush=True)
    return {
        "name": name,
        "command": command_label(command),
        "status": status,
        "required": required,
        "duration_seconds": duration,
        "started_utc": started_utc,
        "completed_utc": completed_utc,
        "timeout_seconds": timeout_seconds,
        "timed_out": timed_out,
        "return_code": return_code,
        "log": log_path.relative_to(run_dir).as_posix(),
        "log_truncated": truncated,
        "error": error,
    }

def read_junit() -> bytes:
    source = ROOT / "target" / "nextest" / "ci" / "junit.xml"
    metadata = source.lstat()
    if not stat.S_ISREG(metadata.st_mode) or getattr(metadata, 'st_file_attributes', 0) & getattr(stat, 'FILE_ATTRIBUTE_REPARSE_POINT', 0):
        raise ValueError('JUnit evidence must be a regular non-link file')
    with source.open('rb') as stream:
        payload = stream.read(MAX_JUNIT_BYTES + 1)
    if len(payload) > MAX_JUNIT_BYTES:
        raise ValueError('JUnit report exceeds the 8 MiB artifact ceiling')
    return payload


def junit_fingerprint() -> str | None:
    try:
        return hashlib.sha256(read_junit()).hexdigest()
    except (OSError, ValueError):
        return None


def validate_junit(payload: str) -> tuple[element_tree.Element, dict[str, int]]:
    # Nextest's Jenkins report is evidence, not merely well-formed XML. Reject
    # entity declarations before parsing and reconcile counts independently.
    if '<!DOCTYPE' in payload or '<!ENTITY' in payload:
        raise ValueError('JUnit declarations are not permitted')
    parser = element_tree.XMLPullParser(events=('start', 'end'))
    depth = nodes = 0
    root = None
    # Incremental events enforce structure limits before building an entire
    # adversarial tree; the byte ceiling alone does not bound XML nesting.
    for offset in range(0, len(payload), 4096):
        parser.feed(payload[offset:offset + 4096])
        for event, element in parser.read_events():
            if event == 'start':
                depth += 1
                nodes += 1
                if root is None:
                    root = element
                if depth > MAX_JUNIT_DEPTH or nodes > MAX_JUNIT_NODES:
                    raise ValueError('JUnit XML structure exceeds its depth/node ceiling')
            else:
                depth -= 1
    parser.close()
    if root is None:
        raise ValueError('JUnit evidence requires a document element')
    if root.tag != 'testsuites' or not len(root):
        raise ValueError('JUnit evidence requires nonempty test suites')
    totals = dict.fromkeys(('tests', 'failures', 'errors', 'skipped'), 0)
    identities: set[tuple[str, str, str]] = set()
    suite_names: set[str] = set()

    def check_counts(element: element_tree.Element, actual: dict[str, int]) -> None:
        for name, count in actual.items():
            value = element.get(name)
            if name == 'skipped' and value is None:
                continue  # Jenkins does not require an aggregate skipped field.
            if value is None or re.fullmatch(r'[0-9]{1,9}', value) is None or int(value) != count:
                raise ValueError('JUnit counts disagree with test evidence')

    for suite in root:
        suite_name = suite.get('name', '')
        if suite.tag != 'testsuite' or not suite_name.strip() or suite_name in suite_names:
            raise ValueError('JUnit suite identity is missing or duplicated')
        suite_names.add(suite_name)
        counts = dict.fromkeys(totals, 0)
        for case in suite.findall('testcase'):
            name, classname = case.get('name', ''), case.get('classname', '')
            identity = (suite_name, classname, name)
            if not name.strip() or not classname.strip() or identity in identities:
                raise ValueError('JUnit test identity is missing or duplicated')
            identities.add(identity)
            counts['tests'] += 1
            outcomes = {key: len(case.findall(tag)) for key, tag in (('failures', 'failure'), ('errors', 'error'), ('skipped', 'skipped'))}
            if sum(outcomes.values()) > 1:
                raise ValueError('JUnit test has contradictory outcomes')
            for key, count in outcomes.items():
                counts[key] += count
        check_counts(suite, counts)
        for key, count in counts.items():
            totals[key] += count
    if totals['tests'] == 0:
        raise ValueError('JUnit evidence contains no tests')
    check_counts(root, totals)
    return root, totals


def collect_junit(run_dir: pathlib.Path, *, previous_digest: str | None = None) -> dict[str, object]:
    destination = run_dir / "artifacts" / "nextest-junit.xml"
    try:
        raw = read_junit()
        if previous_digest is not None and hashlib.sha256(raw).hexdigest() == previous_digest:
            raise ValueError('JUnit report was not replaced by this test run')
        payload = raw.decode('utf-8')
        root, counts = validate_junit(payload)
        # Redact values before XML serialization: raw replacement can introduce
        # angle-bracket markers into attributes/text and corrupt the artifact.
        for element in root.iter():
            for name, value in element.attrib.items():
                element.set(name, REDACT(value))
            if element.text:
                element.text = REDACT(element.text)
            if element.tail:
                element.tail = REDACT(element.tail)
        redacted = element_tree.tostring(root, encoding='unicode')
        if len(redacted.encode('utf-8')) > MAX_JUNIT_BYTES:
            raise ValueError('Redacted JUnit report exceeds the 8 MiB artifact ceiling')
        atomic_write(destination, redacted)
        print("PASS: junit-artifact", flush=True)
        return {
            "name": "junit-artifact",
            "status": "pass",
            "required": True,
            "artifact": destination.relative_to(run_dir).as_posix(),
            "counts": counts,
        }
    except (OSError, ValueError, element_tree.ParseError) as error:
        message = REDACT(f"{type(error).__name__}: {error}")
        print(f"FAIL: junit-artifact — {message}", file=sys.stderr, flush=True)
        return {
            "name": "junit-artifact",
            "status": "fail",
            "required": True,
            "error": message,
        }


def storage_preflight() -> dict[str, object]:
    result: dict[str, object] = {'name': 'storage-preflight', 'required': True, 'minimum_free_bytes': MIN_QA_FREE_BYTES}
    try:
        # The workspace build-dir remains independent of an overridden final
        # target-dir. Measure both, including overrides not created yet, without
        # persisting either location or deleting another task's cache.
        targets = [ROOT / 'target']
        for variable in ('CARGO_TARGET_DIR', 'CARGO_BUILD_BUILD_DIR'):
            value = os.environ.get(variable, '')
            if value:
                value = value.replace('{workspace-root}', str(ROOT))
                if '{' in value or '}' in value:
                    raise ValueError('Unsupported build-directory template')
                target = pathlib.Path(value)
                targets.append(target if target.is_absolute() else ROOT / target)
        free_values = []
        for target in targets:
            while not target.exists() and target != target.parent:
                target = target.parent
            free_values.append(shutil.disk_usage(target).free)
        free = min(free_values)
        result.update(status='pass' if free >= MIN_QA_FREE_BYTES else 'fail', free_bytes=free)
    except (OSError, ValueError):
        result.update(status='fail', error='Build-volume free space could not be measured')
    return result


def skipped(name: str, reason: str, *, external: bool = False) -> dict[str, object]:
    status = "external" if external else "skipped"
    print(f"{status.upper()}: {name} — {reason}", flush=True)
    return {"name": name, "status": status, "required": False, "reason": reason}


def run_benchmark_matrix(
    run_dir: pathlib.Path,
    next_index: int,
    commands: tuple[tuple[str, list[str]], ...],
) -> tuple[list[dict[str, object]], int]:
    benchmark_target = run_dir / "benchmark-target"
    results: list[dict[str, object]] = []
    try:
        for name, command in commands:
            results.append(
                run_step(
                    run_dir,
                    next_index,
                    name,
                    command,
                    env_add={"CARGO_TARGET_DIR": str(benchmark_target)},
                )
            )
            next_index += 1
    finally:
        if benchmark_target.exists():
            metadata = benchmark_target.lstat()
            attributes = getattr(metadata, "st_file_attributes", 0)
            if benchmark_target.is_symlink() or (
                attributes & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)
            ):
                raise RuntimeError("refusing to remove a linked benchmark target")
            shutil.rmtree(benchmark_target)
    return results, next_index


def build_bundle(run_dir: pathlib.Path, bundle_path: pathlib.Path) -> None:
    included: list[pathlib.Path] = []
    excluded: list[dict[str, str]] = []
    total = 0
    for path in sorted(run_dir.rglob("*")):
        if not path.is_file():
            continue
        relative = path.relative_to(run_dir).as_posix()
        suffix = path.suffix.lower()
        if suffix in {".etl", ".png", ".info"}:
            excluded.append({"path": relative, "reason": "private/raw artifact"})
            continue
        size = path.stat().st_size
        if size > MAX_BUNDLE_FILE_BYTES:
            excluded.append({"path": relative, "reason": "per-file ceiling"})
            continue
        if total + size > MAX_BUNDLE_BYTES:
            excluded.append({"path": relative, "reason": "bundle ceiling"})
            continue
        included.append(path)
        total += size
    manifest_path = run_dir / "bundle-manifest.json"
    atomic_write(
        manifest_path,
        json.dumps(
            {
                "included": [
                    path.relative_to(run_dir).as_posix() for path in included
                ],
                "excluded": excluded,
                "uncompressed_bytes": total,
                "limits": {
                    "per_file_bytes": MAX_BUNDLE_FILE_BYTES,
                    "total_bytes": MAX_BUNDLE_BYTES,
                },
            },
            indent=2,
            sort_keys=True,
        )
        + "\n",
    )
    included.append(manifest_path)
    temporary = bundle_path.with_suffix(".zip.tmp")
    with zipfile.ZipFile(
        temporary, "w", compression=zipfile.ZIP_DEFLATED
    ) as archive:
        for path in included:
            archive.write(path, path.relative_to(run_dir.parent))
    os.replace(temporary, bundle_path)

def render_html(report: dict[str, object]) -> str:
    rows = []
    for step in report["steps"]:
        rows.append(
            "<tr>"
            f"<td>{html.escape(str(step['name']))}</td>"
            f"<td>{html.escape(str(step['status']))}</td>"
            f"<td>{html.escape(str(step.get('duration_seconds', '—')))}</td>"
            f"<td><code>{html.escape(str(step.get('command', step.get('reason', ''))))}</code></td>"
            "</tr>"
        )
    return (
        "<!doctype html><html lang=\"en\"><meta charset=\"utf-8\">"
        "<title>Automexia Phase 0 QA evidence</title>"
        "<style>body{font:15px system-ui;margin:2rem;max-width:1100px;color:#d9e7f0;background:#06111d}"
        "table{border-collapse:collapse;width:100%}th,td{padding:.65rem;border-bottom:1px solid #21405a;"
        "text-align:left;vertical-align:top}code{white-space:pre-wrap;color:#8ddcff}h1{font-size:1.5rem}</style>"
        "<h1>Automexia Phase 0 QA evidence</h1>"
        f"<p>Run: <code>{html.escape(str(report['run_id']))}</code></p>"
        f"<p>Overall: <strong>{html.escape(str(report['status']))}</strong></p>"
        "<table><thead><tr><th>Step</th><th>Status</th><th>Seconds</th>"
        "<th>Command / reason</th></tr></thead><tbody>"
        + "".join(rows)
        + "</tbody></table></html>\n"
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--full", action="store_true", required=True)
    parser.add_argument("--bundle", action="store_true")
    args = parser.parse_args()
    requested_run_id = os.environ.get("AUTOMEXIA_QA_RUN_LABEL", "")
    if requested_run_id:
        if len(requested_run_id) > 96 or re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]*", requested_run_id) is None:
            parser.error("AUTOMEXIA_QA_RUN_LABEL must be a bounded portable identifier")
        run_id = requested_run_id
    else:
        run_id = dt.datetime.now(dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ") + f"-{os.getpid()}"
    run_dir = ROOT / "target" / "qa" / run_id
    run_dir.mkdir(parents=True, exist_ok=False)
    source_before = (git_value("rev-parse", "HEAD"), dirty_fingerprint())

    commands: list[tuple[str, list[str], dict[str, str] | None]] = [
        ("qa-runner-self-tests", [sys.executable, "tools/ci/test_qa.py"], None),
        (
            "python-contract-mutations",
            [
                sys.executable,
                "-m",
                "unittest",
                "discover",
                "-s",
                "tools/ci",
                "-p",
                "test_*.py",
                "-v",
            ],
            None,
        ),
        (
            "github-free-assurance-policy",
            [sys.executable, "tools/ci/github_free_assurance.py", "check-policy"],
            None,
        ),
        (
            "github-free-assurance-mutations",
            [sys.executable, "tools/ci/test_github_free_assurance.py"],
            None,
        ),
        (
            "repository-walker-cache-scope",
            [sys.executable, "tools/ci/test_repository_walkers.py"],
            None,
        ),
        (
            "rustsec-exception-policy",
            [sys.executable, "tools/ci/test_rustsec_exceptions.py"],
            None,
        ),
        (
            "feature-test-reinforcement-mutations",
            [sys.executable, "tools/ci/test_feature_test_reinforcement.py"],
            None,
        ),
        (
            "m8-m12-multicloud-contract",
            [sys.executable, "tools/ci/check_m8_m12_multicloud.py"],
            None,
        ),
        (
            "m8-m12-multicloud-mutations",
            [sys.executable, "tools/ci/test_m8_m12_multicloud.py"],
            None,
        ),
        (
            "s1-assurance-policy",
            [sys.executable, "tools/ci/s1_assurance.py", "check-policy"],
            None,
        ),
        (
            "s1-assurance-mutations",
            [sys.executable, "tools/ci/test_s1_assurance.py"],
            None,
        ),
        (
            "performance-assurance-policy",
            [sys.executable, "tools/ci/performance_assurance.py", "check-policy"],
            None,
        ),
        (
            "performance-assurance-mutations",
            [sys.executable, "tools/ci/test_performance_assurance.py"],
            None,
        ),
        (
            "stable-release-policy",
            [sys.executable, "tools/ci/stable_release.py", "check-policy"],
            None,
        ),
        (
            "stable-release-mutations",
            [sys.executable, "tools/ci/test_stable_release.py"],
            None,
        ),
        (
            "public-distribution-policy",
            [sys.executable, "tools/ci/public_distribution.py", "check-policy"],
            None,
        ),
        (
            "public-distribution-mutations",
            [sys.executable, "tools/ci/test_public_distribution.py"],
            None,
        ),
        ("rustfmt", ["cargo", "fmt", "--all", "--", "--check"], None),
        ("metadata", ["cargo", "metadata", "--locked", "--format-version", "1"], None),
        ("repository-contracts", ["cargo", "xtask", "verify", "all"], None),
        ("repository-formats", [sys.executable, "tools/ci/validate_repository.py"], None),
        (
            "repository-protection-mutations",
            [sys.executable, "tools/ci/test_repository_protection.py"],
            None,
        ),
        (
            "cp51-proposal-mutations",
            [sys.executable, "tools/ci/test_command_productivity_cp51.py"],
            None,
        ),
        (
            "cp56-source-mutations",
            [sys.executable, "tools/ci/test_command_productivity_cp56.py"],
            None,
        ),
        (
            "d7-cp6-ecosystem-mutations",
            [sys.executable, "tools/ci/test_ecosystem_d7_cp6.py"],
            None,
        ),
        (
            "ghostty-assurance-mutations",
            [sys.executable, "tools/ci/test_ghostty_compatibility.py"],
            None,
        ),
        (
            "ghostty-native-evidence-mutations",
            [sys.executable, "tools/ci/test_ghostty_native_evidence.py"],
            None,
        ),
        (
            "cp50-research-format",
            [
                "cargo",
                "fmt",
                "--manifest-path",
                "tools/research/cp5-matcher-benchmark/Cargo.toml",
                "--",
                "--check",
            ],
            None,
        ),
        (
            "cp50-research-clippy",
            [
                "cargo",
                "clippy",
                "--manifest-path",
                "tools/research/cp5-matcher-benchmark/Cargo.toml",
                "--all-targets",
                "--locked",
                "--",
                "-D",
                "warnings",
            ],
            None,
        ),
        (
            "cp50-research-tests",
            [
                "cargo",
                "test",
                "--manifest-path",
                "tools/research/cp5-matcher-benchmark/Cargo.toml",
                "--locked",
            ],
            None,
        ),
        (
            "cp50-research-benchmark",
            [
                "cargo",
                "run",
                "--release",
                "--manifest-path",
                "tools/research/cp5-matcher-benchmark/Cargo.toml",
                "--locked",
                "--quiet",
            ],
            None,
        ),
        (
            "clippy",
            ["cargo", "clippy", "--workspace", "--all-targets", "--locked", "--", "-D", "warnings"],
            None,
        ),
        (
            "nextest",
            ["cargo", "nextest", "run", "--workspace", "--locked", "--profile", "ci", "--no-fail-fast"],
            None,
        ),
        ("doctests", ["cargo", "test", "--workspace", "--doc", "--locked"], None),
        # Keep the workspace CI JUnit intact. The no-retry default profile has
        # per-test deadlines but does not overwrite that separate CI report.
        (
            "component-host",
            ["cargo", "nextest", "run", "-p", "automexia-ecosystem-runtime", "--all-features", "--locked", "--profile", "default", "--no-fail-fast"],
            None,
        ),
        # Criterion's test mode executes every enabled benchmark scenario once;
        # it catches broken cases without claiming controlled performance data.
        ("benchmark-smoke", ["cargo", "test", "--workspace", "--all-features", "--benches", "--locked"], None),
        ("resize-stress", ["cargo", "xtask", "test", "resize-stress"], None),
        ("session-clone", ["cargo", "xtask", "test", "session-clone"], None),
        (
            "loom-channel",
            ["cargo", "test", "-p", "corcovado", "--test", "loom_channel_readiness", "--locked"],
            {
                "RUSTFLAGS": (
                    os.environ.get("RUSTFLAGS", "")
                    + " --cfg loom --check-cfg=cfg(loom)"
                ).strip()
            },
        ),
        (
            "dependency-policy",
            ["cargo", "deny", "--locked", "--color", "never", "check", "--hide-inclusion-graph"],
            None,
        ),
    ]
    if os.name == "nt":
        commands.insert(
            4,
            (
                "shell-contracts",
                [
                    "powershell.exe",
                    "-NoLogo",
                    "-NoProfile",
                    "-NonInteractive",
                    "-File",
                    "tools/ci/test_powershell.ps1",
                ],
                None,
            ),
        )
        commands.insert(
            5,
            (
                "cp5-native-powershell-bridge",
                [
                    "pwsh",
                    "-NoLogo",
                    "-NoProfile",
                    "-NonInteractive",
                    "-File",
                    "tools/ci/test_cp5_native_powershell_bridge.ps1",
                ],
                None,
            ),
        )
    else:
        commands.insert(
            4, ("shell-contracts", ["bash", "tools/ci/test_shell_sources.sh"], None)
        )
        commands.insert(
            5,
            (
                "cp5-native-zsh-fish-bridges",
                [sys.executable, "tools/ci/test_cp5_native_shell_adapters.py"],
                None,
            ),
        )
        if platform.system() == "Linux":
            commands.insert(
                5,
                (
                    "cp5-native-bash-bridge",
                    [sys.executable, "tools/ci/test_cp5_native_shell_bridge.py"],
                    None,
                ),
            )

    reserve = storage_preflight()
    print(f"{str(reserve['status']).upper()}: storage-preflight {json.dumps(reserve, sort_keys=True)}", flush=True)
    steps: list[dict[str, object]] = [reserve]
    previous_junit = junit_fingerprint()
    for index, (name, command, env_add) in enumerate(commands, 1):
        if command[0] == 'cargo' and storage_preflight()['status'] != 'pass':
            steps.append({'name': name, 'status': 'fail', 'required': True, 'error': 'Build-volume minimum free-space reserve unavailable; command not started'})
            continue
        steps.append(run_step(run_dir, index, name, command, env_add=env_add))
    steps.append(collect_junit(run_dir, previous_digest=previous_junit))

    next_index = len(steps) + 1
    s1_evidence = os.environ.get("AUTOMEXIA_QA_S1_EVIDENCE", "").strip()
    if s1_evidence:
        steps.append(
            run_step(
                run_dir,
                next_index,
                "s1-release-assurance-evidence",
                [
                    sys.executable,
                    "tools/ci/s1_assurance.py",
                    "validate",
                    "--manifest",
                    s1_evidence,
                    "--expected-commit",
                    git_value("rev-parse", "HEAD"),
                    "--require-complete",
                    "--output",
                    str(run_dir / "artifacts" / "s1-assurance.json"),
                ],
            )
        )
        next_index += 1
    else:
        steps.append(
            skipped(
                "s1-release-assurance-evidence",
                "set AUTOMEXIA_QA_S1_EVIDENCE to the private, redacted, complete controlled-runner manifest",
                external=True,
            )
        )

    if os.environ.get("AUTOMEXIA_QA_NATIVE_OPENSSH_EVIDENCE"):
        steps.append(
            run_step(
                run_dir,
                next_index,
                "native-openssh-release-evidence",
                [
                    sys.executable,
                    "tools/ci/native_openssh_evidence.py",
                    "--validate-environment",
                ],
            )
        )
        next_index += 1
    else:
        steps.append(
            skipped(
                "native-openssh-release-evidence",
                "set AUTOMEXIA_QA_NATIVE_OPENSSH_EVIDENCE to a private redacted manifest on each controlled native runner",
                external=True,
            )
        )

    ghostty_evidence = os.environ.get("AUTOMEXIA_QA_GHOSTTY_EVIDENCE", "").strip()
    if ghostty_evidence:
        steps.append(
            run_step(
                run_dir,
                next_index,
                "ghostty-native-release-evidence",
                [
                    sys.executable,
                    "tools/ci/ghostty_native_evidence.py",
                    "--manifest",
                    ghostty_evidence,
                    "--expected-commit",
                    git_value("rev-parse", "HEAD"),
                    "--require-complete",
                    "--output",
                    str(run_dir / "artifacts" / "ghostty-native-evidence.json"),
                ],
            )
        )
        next_index += 1
    else:
        steps.append(
            skipped(
                "ghostty-native-release-evidence",
                "set AUTOMEXIA_QA_GHOSTTY_EVIDENCE to a private, redacted, complete Windows/Linux/macOS manifest",
                external=True,
            )
        )

    if truthy("AUTOMEXIA_QA_NATIVE") and os.name == "nt":
        steps.append(
            run_step(
                run_dir,
                next_index,
                "native-windows-gui",
                ["cargo", "xtask", "test", "resize-stress", "--native-gui"],
                env_add={
                    "AUTOMEXIA_NATIVE_RESOURCE_REPORT": str(
                        run_dir / "artifacts" / "native-windows-resource.json"
                    )
                },
            )
        )
        next_index += 1
    else:
        steps.append(
            skipped(
                "native-windows-gui",
                "set AUTOMEXIA_QA_NATIVE=1 on a controlled interactive Windows GPU runner",
                external=True,
            )
        )

    if truthy("AUTOMEXIA_QA_APPVERIFIER") and os.name == "nt":
        steps.append(
            run_step(
                run_dir,
                next_index,
                "application-verifier",
                [
                    "powershell.exe",
                    "-NoLogo",
                    "-NoProfile",
                    "-NonInteractive",
                    "-File",
                    "tests/integration/appverifier-windows.ps1",
                    "-OutputDirectory",
                    str(run_dir / "native" / "appverifier"),
                ],
            )
        )
        next_index += 1
    else:
        steps.append(
            skipped(
                "application-verifier",
                "set AUTOMEXIA_QA_APPVERIFIER=1 in an elevated controlled Windows session",
                external=True,
            )
        )

    if truthy("AUTOMEXIA_QA_WPR") and os.name == "nt":
        steps.append(
            run_step(
                run_dir,
                next_index,
                "wpr-native-trace",
                [
                    "powershell.exe",
                    "-NoLogo",
                    "-NoProfile",
                    "-NonInteractive",
                    "-File",
                    "tests/integration/wpr-windows.ps1",
                    "-OutputDirectory",
                    str(run_dir / "native" / "wpr"),
                ],
            )
        )
        next_index += 1
    else:
        steps.append(
            skipped(
                "wpr-native-trace",
                "set AUTOMEXIA_QA_WPR=1 in an elevated controlled Windows session; ETL remains private",
                external=True,
            )
        )

    if truthy("AUTOMEXIA_QA_BENCHMARKS"):
        benchmark_commands = (
            (
                "benchmark-app",
                ["cargo", "bench", "-p", "automexia-terminal", "--bench", "automexia_services", "--locked", "--", "--noplot"],
            ),
            (
                "benchmark-image",
                ["cargo", "bench", "-p", "automexia-terminal", "--bench", "image_preview", "--locked", "--", "--noplot"],
            ),
            (
                "benchmark-vt",
                ["cargo", "bench", "-p", "rio-vt", "--bench", "vt_input", "--locked", "--", "--noplot"],
            ),
            (
                "benchmark-channel",
                ["cargo", "bench", "-p", "corcovado", "--bench", "bench_poll", "--locked", "--", "--noplot"],
            ),
            (
                "benchmark-pty",
                ["cargo", "bench", "-p", "teletypewriter", "--bench", "pty_io", "--locked", "--", "--noplot"],
            ),
            (
                "benchmark-ssh-inventory",
                ["cargo", "bench", "-p", "automexia-devops-ssh", "--bench", "openssh_inventory", "--locked", "--", "--noplot"],
            ),
            (
                "benchmark-quick-actions",
                ["cargo", "bench", "-p", "automexia-command-productivity", "--bench", "quick_actions", "--locked", "--", "--noplot"],
            ),
            (
                "benchmark-connection-planning",
                ["cargo", "bench", "-p", "automexia-connectivity", "--bench", "connection_planning", "--locked", "--", "--noplot"],
            ),
            (
                "benchmark-quick-action-store",
                ["cargo", "bench", "-p", "automexia-terminal", "--bench", "quick_action_store", "--locked", "--", "--noplot"],
            ),
            (
                "benchmark-keybindings",
                ["cargo", "bench", "-p", "automexia-keybindings", "--bench", "registry", "--locked", "--", "--noplot"],
            ),
        )
        benchmark_steps, next_index = run_benchmark_matrix(
            run_dir, next_index, benchmark_commands
        )
        steps.extend(benchmark_steps)
    else:
        steps.append(
            skipped(
                "controlled-benchmarks",
                "set AUTOMEXIA_QA_BENCHMARKS=1 only on named stable hardware",
                external=True,
            )
        )

    if truthy("AUTOMEXIA_QA_COVERAGE") and os.name == "nt":
        coverage_target = (
            ROOT / "target" / f"automexia-qa-coverage-{os.getpid()}"
        )
        coverage_lcov = run_dir / "artifacts" / "lcov.info"
        coverage_summary = (
            run_dir / "artifacts" / "coverage-summary.json"
        )
        try:
            steps.append(
                run_step(
                    run_dir,
                    next_index,
                    "coverage-build",
                    [
                        "cargo",
                        "llvm-cov",
                        "--workspace",
                        "--locked",
                        "--lcov",
                        "--output-path",
                        str(coverage_lcov),
                    ],
                    env_add={"CARGO_TARGET_DIR": str(coverage_target)},
                )
            )
            next_index += 1
            if steps[-1]["status"] == "pass":
                steps.append(
                    run_step(
                        run_dir,
                        next_index,
                        "coverage-policy",
                        [sys.executable, "tools/ci/check_coverage.py"],
                        env_add={
                            "BASE_SHA": "HEAD",
                            "HEAD_SHA": "WORKTREE",
                            "COVERAGE_PLATFORM": "windows-x86_64-msvc",
                            "LCOV_FILE": str(coverage_lcov),
                            "COVERAGE_SUMMARY": str(coverage_summary),
                        },
                    )
                )
                next_index += 1
        finally:
            coverage_lcov.unlink(missing_ok=True)
            shutil.rmtree(coverage_target, ignore_errors=True)
    else:
        reason = (
            "set AUTOMEXIA_QA_COVERAGE=1 on Windows for isolated LLVM coverage"
            if os.name == "nt"
            else "the recorded coverage baseline is Windows x86_64 MSVC"
        )
        steps.append(
            skipped("coverage", reason, external=os.name != "nt")
        )
    steps.append(
        skipped(
            "30-day-performance-baseline",
            "requires 30 consecutive days of named controlled-runner evidence",
            external=True,
        )
    )

    source_after = (git_value("rev-parse", "HEAD"), dirty_fingerprint())
    steps.append({
        "name": "source-identity-stable", "required": True,
        "status": "pass" if source_identity_stable(source_before, source_after) else "fail",
        "reason": "Source commit and bounded content fingerprint must remain unchanged throughout QA.",
    })
    print(f"{steps[-1]['status'].upper()}: source-identity-stable", flush=True)
    required_failures = [
        step["name"]
        for step in steps
        if step.get("required") and step["status"] != "pass"
    ]
    report: dict[str, object] = {
        "schema_version": 2,
        "run_id": run_id,
        "status": "pass" if not required_failures else "fail",
        "required_failures": required_failures,
        "created_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "source": {
            "commit": source_after[0],
            "dirty_fingerprint_sha256": source_after[1],
            "fingerprint_kind": "automexia-qa-content-v1",
            "initial_commit": source_before[0],
            "initial_dirty_fingerprint_sha256": source_before[1],
        },
        "host": collect_host_manifest(),
        "tools": {
            "rustc": safe_version(["rustc", "--version"]),
            "cargo": safe_version(["cargo", "--version"]),
            "nextest": safe_version(["cargo", "nextest", "--version"]),
            "cargo-deny": safe_version(["cargo", "deny", "--version"]),
            "cargo-llvm-cov": safe_version(
                ["cargo", "llvm-cov", "--version"]
            ),
        },
        "limits": {
            "per_log_bytes": MAX_LOG_BYTES,
            "per_bundle_file_bytes": MAX_BUNDLE_FILE_BYTES,
            "bundle_uncompressed_bytes": MAX_BUNDLE_BYTES,
            "environment_dumped": False,
            "user_data_captured": False,
        },
        "steps": steps,
    }
    atomic_write(run_dir / "summary.json", json.dumps(report, indent=2, sort_keys=True) + "\n")
    atomic_write(run_dir / "report.html", render_html(report))
    if args.bundle:
        bundle_path = run_dir.with_suffix(".zip")
        build_bundle(run_dir, bundle_path)
        print(f"Evidence bundle: {bundle_path.relative_to(ROOT).as_posix()}")
    print(f"Evidence report: {(run_dir / 'report.html').relative_to(ROOT).as_posix()}")
    return 0 if not required_failures else 1


if __name__ == "__main__":
    raise SystemExit(main())
