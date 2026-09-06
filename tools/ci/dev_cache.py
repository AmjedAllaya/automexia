#!/usr/bin/env python3
"""Inspect and safely reclaim Automexia contributor build storage."""

from __future__ import annotations

import argparse
import contextlib
from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import stat
import subprocess
import sys
import time
from typing import Iterator, Sequence


ROOT = Path(__file__).resolve().parents[2]
CACHE_DIRECTORY_NAME = ".automexia-cache"
LEGACY_TOOL_DIRECTORY_NAME = ".automexia-tools"
VERIFICATION_TARGET_PREFIX = "automexia-verification-v1-"
LEASE_DIRECTORY_NAME = ".automexia-leases"
ACTIVE_MARKER_NAME = ".automexia-active"
TOOLSET_SCHEMA = 1
DEFAULT_GRACE_HOURS = 72
DEFAULT_WARNING_GIB = 40
GIB = 1024 * 1024 * 1024
MAX_FILES = 2_000_000
MAX_DIRECTORIES = 200_000
MAX_MANIFEST_BYTES = 64 * 1024
SCOPES = ("automatic", "tools", "worktrees", "all")


class CacheError(ValueError):
    """The cache contract is unsafe, malformed, or unavailable."""


@dataclass(frozen=True)
class Measurement:
    bytes: int
    newest_mtime: float
    files: int
    directories: int


@dataclass(frozen=True)
class Candidate:
    label: str
    category: str
    path: Path
    measurement: Measurement
    current: bool = False
    clean: bool = True
    leased: bool = False
    current_toolset: bool = False

    def age_hours(self, now: float) -> float:
        return max(0.0, (now - self.measurement.newest_mtime) / 3600.0)


def _run_git(arguments: Sequence[str], *, root: Path = ROOT) -> bytes:
    try:
        completed = subprocess.run(
            ["git", *arguments],
            cwd=root,
            check=False,
            capture_output=True,
            timeout=30,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise CacheError("could not inspect Git worktree ownership") from error
    if completed.returncode != 0:
        raise CacheError("Git worktree ownership inspection failed")
    return completed.stdout


def _is_reparse_or_link(path: Path) -> bool:
    try:
        metadata = path.lstat()
    except FileNotFoundError:
        return False
    except OSError as error:
        raise CacheError("could not inspect a cache path") from error
    if stat.S_ISLNK(metadata.st_mode):
        return True
    attributes = getattr(metadata, "st_file_attributes", 0)
    return bool(attributes & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0))


def git_common_directory(*, root: Path = ROOT) -> Path:
    output = _run_git(
        ["rev-parse", "--path-format=absolute", "--git-common-dir"], root=root
    ).decode("utf-8", "strict").strip()
    if not output:
        raise CacheError("Git common directory is empty")
    path = Path(output)
    if not path.is_absolute():
        raise CacheError("Git common directory must be absolute")
    return path.resolve(strict=True)


def cache_root(*, root: Path = ROOT) -> Path:
    override = os.environ.get("AUTOMEXIA_DEV_CACHE_DIR", "").strip()
    if override:
        path = Path(override)
        if not path.is_absolute():
            raise CacheError("AUTOMEXIA_DEV_CACHE_DIR must be absolute")
        path = Path(os.path.abspath(path))
        if path.exists() and _is_reparse_or_link(path):
            raise CacheError("development cache root cannot be a link or reparse point")
        return path
    common = git_common_directory(root=root)
    if common.name != ".git" or not common.is_dir():
        raise CacheError("Git common directory is not a supported non-bare checkout")
    # Windows toolchains still encounter components without extended-path
    # support, so generated cache paths stay at the repository drive root.
    path = (
        Path(common.anchor) / CACHE_DIRECTORY_NAME
        if os.name == "nt"
        else common.parent / CACHE_DIRECTORY_NAME
    )
    if path.exists() and _is_reparse_or_link(path):
        raise CacheError("development cache root cannot be a link or reparse point")
    return path


def ensure_cache_directory(path: Path) -> None:
    try:
        path.mkdir(parents=True, exist_ok=True)
    except OSError as error:
        raise CacheError("could not create the development cache") from error
    if _is_reparse_or_link(path):
        raise CacheError("development cache directories cannot be links or reparse points")


def toolsets_root(*, root: Path = ROOT) -> Path:
    return cache_root(root=root) / "toolsets"


def mutable_root(*, root: Path = ROOT) -> Path:
    return cache_root(root=root) / "mutable"


def runtime_root(*, root: Path = ROOT) -> Path:
    return mutable_root(root=root) / "runtime"


def downloads_root(*, root: Path = ROOT) -> Path:
    return mutable_root(root=root) / "downloads"


def staging_root(*, root: Path = ROOT) -> Path:
    return cache_root(root=root) / "staging"


def temporary_root(*, root: Path = ROOT) -> Path:
    return cache_root(root=root) / "temporary"


def leases_root(*, root: Path = ROOT) -> Path:
    return cache_root(root=root) / LEASE_DIRECTORY_NAME


def normalized_platform() -> str:
    return {
        "win32": "windows",
        "darwin": "macos",
        "linux": "linux",
    }.get(sys.platform, sys.platform)


def normalized_architecture() -> str:
    machine = os.environ.get("PROCESSOR_ARCHITECTURE", "") if os.name == "nt" else ""
    value = (machine or platform.machine()).lower()
    return {
        "amd64": "x86_64",
        "x86_64": "x86_64",
        "arm64": "aarch64",
        "aarch64": "aarch64",
    }.get(value, value)


def toolset_descriptor(tools: dict[str, str]) -> dict[str, object]:
    if not tools or any(not name or not version for name, version in tools.items()):
        raise CacheError("toolset inventory must contain named pinned versions")
    return {
        "schema": TOOLSET_SCHEMA,
        "platform": normalized_platform(),
        "architecture": normalized_architecture(),
        "python": {
            "implementation": sys.implementation.name,
            "major": sys.version_info.major,
            "minor": sys.version_info.minor,
        },
        "tools": dict(sorted(tools.items())),
    }


def toolset_id(tools: dict[str, str]) -> str:
    payload = json.dumps(
        toolset_descriptor(tools), sort_keys=True, separators=(",", ":")
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def toolset_root(tools: dict[str, str], *, root: Path = ROOT) -> Path:
    return toolsets_root(root=root) / toolset_id(tools)


def parse_worktree_porcelain(payload: bytes) -> list[Path]:
    paths: list[Path] = []
    for field in payload.split(b"\0"):
        if field.startswith(b"worktree "):
            value = os.fsdecode(field[len(b"worktree ") :])
            path = Path(value)
            if not path.is_absolute():
                raise CacheError("Git returned a non-absolute worktree path")
            paths.append(Path(os.path.abspath(path)))
    if not paths:
        raise CacheError("Git returned no worktrees")
    return paths


def worktree_paths(*, root: Path = ROOT) -> list[Path]:
    return parse_worktree_porcelain(
        _run_git(["worktree", "list", "--porcelain", "-z"], root=root)
    )


def worktree_is_clean(path: Path) -> bool:
    try:
        completed = subprocess.run(
            ["git", "status", "--porcelain=v1", "--untracked-files=normal"],
            cwd=path,
            check=False,
            capture_output=True,
            timeout=30,
        )
    except (OSError, subprocess.TimeoutExpired):
        return False
    return completed.returncode == 0 and not completed.stdout.strip()


def measure_tree(path: Path) -> Measurement:
    if not path.exists():
        return Measurement(0, 0.0, 0, 0)
    if _is_reparse_or_link(path):
        raise CacheError("refusing to traverse a linked or reparse-point cache")
    total = 0
    files = 0
    directories = 0
    try:
        newest = path.stat().st_mtime
    except OSError as error:
        raise CacheError("could not inspect a cache directory") from error
    pending = [path]
    while pending:
        directory = pending.pop()
        directories += 1
        if directories > MAX_DIRECTORIES:
            raise CacheError("cache directory count exceeds its safety ceiling")
        try:
            entries = list(os.scandir(directory))
        except OSError as error:
            raise CacheError("could not read a cache directory") from error
        for entry in entries:
            try:
                metadata = entry.stat(follow_symlinks=False)
            except OSError as error:
                raise CacheError("could not inspect a cache entry") from error
            newest = max(newest, metadata.st_mtime)
            if entry.is_symlink() or (
                getattr(metadata, "st_file_attributes", 0)
                & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)
            ):
                raise CacheError("cache trees cannot contain links or reparse points")
            if entry.is_dir(follow_symlinks=False):
                pending.append(Path(entry.path))
            elif entry.is_file(follow_symlinks=False):
                files += 1
                if files > MAX_FILES:
                    raise CacheError("cache file count exceeds its safety ceiling")
                total += metadata.st_size
            else:
                raise CacheError("cache trees can contain only files and directories")
    return Measurement(total, newest, files, directories)


def _direct_directories(path: Path, *, allow_ordinary_files: bool = False) -> list[Path]:
    if not path.exists():
        return []
    if _is_reparse_or_link(path):
        raise CacheError("cache collection root cannot be a link or reparse point")
    try:
        entries = list(path.iterdir())
    except OSError as error:
        raise CacheError("could not enumerate a cache collection") from error
    if len(entries) > MAX_DIRECTORIES:
        raise CacheError("cache collection exceeds its directory ceiling")
    directories = []
    for entry in entries:
        if _is_reparse_or_link(entry):
            raise CacheError("cache collections cannot contain links or reparse points")
        if entry.is_dir():
            directories.append(entry)
        elif entry.is_file() and allow_ordinary_files:
            continue
        elif entry.is_file():
            raise CacheError("cache collections can contain only directories")
        else:
            raise CacheError("cache collections contain an unsupported entry")
    return sorted(directories, key=lambda item: item.name)


def _lock(handle: object, *, nonblocking: bool) -> None:
    if os.name == "nt":
        import msvcrt

        mode = msvcrt.LK_NBLCK if nonblocking else msvcrt.LK_LOCK
        handle.seek(0)
        msvcrt.locking(handle.fileno(), mode, 1)
    else:
        import fcntl

        mode = fcntl.LOCK_EX | (fcntl.LOCK_NB if nonblocking else 0)
        fcntl.flock(handle.fileno(), mode)


def _unlock(handle: object) -> None:
    if os.name == "nt":
        import msvcrt

        handle.seek(0)
        msvcrt.locking(handle.fileno(), msvcrt.LK_UNLCK, 1)
    else:
        import fcntl

        fcntl.flock(handle.fileno(), fcntl.LOCK_UN)


@contextlib.contextmanager
def cache_lease(name: str, *, root: Path = ROOT) -> Iterator[None]:
    if not name or any(character not in "abcdefghijklmnopqrstuvwxyz0123456789-" for character in name):
        raise CacheError("cache lease name must be a bounded lowercase identifier")
    directory = leases_root(root=root)
    ensure_cache_directory(directory)
    path = directory / f"{name}.lock"
    try:
        with path.open("a+b") as handle:
            if path.stat().st_size == 0:
                handle.write(b"0")
                handle.flush()
            _lock(handle, nonblocking=False)
            try:
                yield
            finally:
                _unlock(handle)
    except OSError as error:
        raise CacheError("could not acquire the development-cache lease") from error


def shared_cache_is_leased(*, root: Path = ROOT) -> bool:
    directory = leases_root(root=root)
    if not directory.exists():
        return False
    if _is_reparse_or_link(directory):
        raise CacheError("cache lease directory cannot be a link or reparse point")
    try:
        entries = list(directory.glob("*.lock"))
    except OSError as error:
        raise CacheError("could not inspect development-cache leases") from error
    if len(entries) > MAX_DIRECTORIES:
        raise CacheError("cache lease count exceeds its safety ceiling")
    for path in entries:
        if _is_reparse_or_link(path) or not path.is_file():
            raise CacheError("cache leases must be ordinary files")
        try:
            with path.open("r+b") as handle:
                try:
                    _lock(handle, nonblocking=True)
                except OSError:
                    return True
                else:
                    _unlock(handle)
        except OSError:
            return True
    return False


def process_is_active(process_id: int) -> bool:
    if process_id <= 0:
        return False
    if process_id == os.getpid():
        return True
    if os.name == "nt":
        import ctypes

        process_query_limited_information = 0x1000
        still_active = 259
        kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel32.OpenProcess.argtypes = [ctypes.c_ulong, ctypes.c_int, ctypes.c_ulong]
        kernel32.OpenProcess.restype = ctypes.c_void_p
        kernel32.GetExitCodeProcess.argtypes = [
            ctypes.c_void_p,
            ctypes.POINTER(ctypes.c_ulong),
        ]
        kernel32.GetExitCodeProcess.restype = ctypes.c_int
        kernel32.CloseHandle.argtypes = [ctypes.c_void_p]
        kernel32.CloseHandle.restype = ctypes.c_int
        handle = kernel32.OpenProcess(
            process_query_limited_information, False, process_id
        )
        if not handle:
            # Access denied can describe a real protected process. Fail safe by
            # preserving its cache; an unknown or absent PID reports invalid.
            return ctypes.get_last_error() == 5
        try:
            exit_code = ctypes.c_ulong()
            if not kernel32.GetExitCodeProcess(handle, ctypes.byref(exit_code)):
                return True
            return exit_code.value == still_active
        finally:
            kernel32.CloseHandle(handle)
    try:
        os.kill(process_id, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    except OSError:
        return True
    return True


def _active_marker(path: Path) -> bool:
    marker = path / ACTIVE_MARKER_NAME
    if not marker.exists():
        return _is_reparse_or_link(marker)
    if _is_reparse_or_link(marker):
        return True
    try:
        if not marker.is_file() or marker.stat().st_size > 32:
            return True
        raw_process_id = marker.read_text(encoding="ascii").strip()
        if not raw_process_id.isdigit():
            return True
        return process_is_active(int(raw_process_id))
    except (OSError, UnicodeError, ValueError):
        return True


def _current_toolset_id(*, root: Path = ROOT) -> str | None:
    policy = root / "tests" / "assurance" / "github-free-assurance-policy-v1.json"
    if not policy.is_file():
        return None
    try:
        if policy.stat().st_size > MAX_MANIFEST_BYTES:
            raise CacheError("assurance policy exceeds its cache-parser ceiling")
        value = json.loads(policy.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise CacheError("could not read the assurance toolset policy") from error
    tools = value.get("tools") if isinstance(value, dict) else None
    if not isinstance(tools, dict) or any(
        not isinstance(name, str) or not isinstance(version, str)
        for name, version in tools.items()
    ):
        raise CacheError("assurance policy has no pinned tool inventory")
    return toolset_id(tools)


def _candidate(
    label: str,
    category: str,
    path: Path,
    **state: bool,
) -> Candidate:
    return Candidate(label, category, path, measure_tree(path), **state)


def inventory(
    *,
    root: Path = ROOT,
    scope: str = "all",
) -> list[Candidate]:
    if scope not in SCOPES:
        raise CacheError("unknown cache scope")
    current = Path(os.path.abspath(root))
    shared_leased = shared_cache_is_leased(root=root)
    candidates: list[Candidate] = []
    worktrees = worktree_paths(root=root)
    for index, worktree in enumerate(worktrees, start=1):
        if not worktree.exists():
            continue
        worktree = Path(os.path.abspath(worktree))
        is_current = worktree == current
        target = worktree / "target"
        if scope != "automatic" and target.is_dir():
            candidates.append(
                _candidate(
                    "current/target" if is_current else f"worktree-{index}/target",
                    "current-target" if is_current else "other-target",
                    target,
                    current=is_current,
                    clean=is_current or worktree_is_clean(worktree),
                    leased=_active_marker(target),
                )
            )
        if target.is_dir():
            for child in _direct_directories(target, allow_ordinary_files=True):
                if child.name.startswith(VERIFICATION_TARGET_PREFIX):
                    candidates.append(
                        _candidate(
                            f"worktree-{index}/verification",
                            "stale-verification",
                            child,
                            leased=_active_marker(child),
                        )
                    )
        legacy = worktree / LEGACY_TOOL_DIRECTORY_NAME
        if legacy.is_dir() and scope in {"tools", "all"}:
            candidates.append(
                _candidate(
                    f"worktree-{index}/legacy-tools",
                    "legacy-tool",
                    legacy,
                    leased=_active_marker(legacy),
                )
            )

    current_toolset = _current_toolset_id(root=root)
    for index, path in enumerate(_direct_directories(toolsets_root(root=root)), start=1):
        candidates.append(
            _candidate(
                f"toolset-{index}",
                "current-toolset" if path.name == current_toolset else "old-toolset",
                path,
                leased=shared_leased,
                current_toolset=path.name == current_toolset,
            )
        )
    for category, parent in (
        ("staging", staging_root(root=root)),
        ("temporary", temporary_root(root=root)),
    ):
        for index, path in enumerate(_direct_directories(parent), start=1):
            candidates.append(
                _candidate(
                    f"{category}-{index}",
                    category,
                    path,
                    leased=shared_leased,
                )
            )
    for label, category, path in (
        ("shared/downloads", "downloads", downloads_root(root=root)),
        ("shared/runtime", "runtime", runtime_root(root=root)),
    ):
        if path.is_dir() and scope == "all":
            candidates.append(
                _candidate(label, category, path, leased=shared_leased)
            )
    return candidates


def _eligible(candidate: Candidate, scope: str, grace_hours: int, now: float) -> bool:
    if candidate.current or candidate.leased or candidate.current_toolset:
        return False
    if candidate.age_hours(now) < grace_hours:
        return False
    if scope == "automatic":
        return candidate.category in {
            "stale-verification",
            "old-toolset",
            "staging",
            "temporary",
        }
    if scope == "tools":
        return candidate.category in {"legacy-tool", "old-toolset"}
    if scope == "worktrees":
        return candidate.category == "other-target" and candidate.clean
    return (
        candidate.category
        in {
            "stale-verification",
            "staging",
            "temporary",
            "legacy-tool",
            "old-toolset",
            "downloads",
            "runtime",
        }
        or (candidate.category == "other-target" and candidate.clean)
    )


def reclaimable(
    candidates: Sequence[Candidate],
    *,
    scope: str,
    grace_hours: int,
    now: float | None = None,
) -> list[Candidate]:
    if scope not in SCOPES:
        raise CacheError("unknown cache scope")
    if grace_hours < 0 or grace_hours > 24 * 365:
        raise CacheError("cache grace hours must be within 0..8760")
    instant = time.time() if now is None else now
    selected = [
        candidate
        for candidate in candidates
        if _eligible(candidate, scope, grace_hours, instant)
    ]
    # An all-scope target can contain a separately listed verification target.
    # Keep only the outermost exact target so accounting and removal are single.
    result: list[Candidate] = []
    for candidate in sorted(selected, key=lambda item: len(item.path.parts)):
        if any(candidate.path.is_relative_to(existing.path) for existing in result):
            continue
        result.append(candidate)
    return result


def _allowed(candidate: Candidate, *, root: Path) -> bool:
    worktrees = worktree_paths(root=root)
    exact = {worktree / "target" for worktree in worktrees}
    exact.update(worktree / LEGACY_TOOL_DIRECTORY_NAME for worktree in worktrees)
    if candidate.path in exact:
        return True
    if candidate.category == "stale-verification":
        return (
            candidate.path.parent in {worktree / "target" for worktree in worktrees}
            and candidate.path.name.startswith(VERIFICATION_TARGET_PREFIX)
        )
    if candidate.category in {
        "current-toolset",
        "old-toolset",
        "staging",
        "temporary",
    }:
        parents = {
            "current-toolset": toolsets_root(root=root),
            "old-toolset": toolsets_root(root=root),
            "staging": staging_root(root=root),
            "temporary": temporary_root(root=root),
        }
        return candidate.path.parent == parents[candidate.category]
    if candidate.category == "downloads":
        return candidate.path == downloads_root(root=root)
    if candidate.category == "runtime":
        return candidate.path == runtime_root(root=root)
    return False


def _retry_readonly_removal(function: object, path: str, error_info: tuple[object, BaseException, object]) -> None:
    error = error_info[1]
    if not isinstance(error, PermissionError):
        raise error
    os.chmod(path, stat.S_IWRITE)
    function(path)  # type: ignore[operator]


def remove_candidate(candidate: Candidate, *, root: Path = ROOT) -> None:
    if not _allowed(candidate, root=root):
        raise CacheError("refusing to remove a path outside the cache contract")
    if not candidate.path.exists():
        return
    if _is_reparse_or_link(candidate.path):
        raise CacheError("refusing to remove a linked or reparse-point cache")
    try:
        shutil.rmtree(candidate.path, onerror=_retry_readonly_removal)
    except OSError as error:
        raise CacheError("could not remove an eligible cache directory") from error


def format_bytes(value: int) -> str:
    if value >= GIB:
        return f"{value / GIB:.2f} GiB"
    if value >= 1024 * 1024:
        return f"{value / (1024 * 1024):.2f} MiB"
    if value >= 1024:
        return f"{value / 1024:.2f} KiB"
    return f"{value} B"


def _distinct_bytes(candidates: Sequence[Candidate]) -> int:
    total = 0
    roots: list[Path] = []
    for candidate in sorted(candidates, key=lambda item: len(item.path.parts)):
        if any(candidate.path.is_relative_to(parent) for parent in roots):
            continue
        roots.append(candidate.path)
        total += candidate.measurement.bytes
    return total


def print_status(candidates: Sequence[Candidate], *, warn_gib: int) -> None:
    if warn_gib < 1 or warn_gib > 1024:
        raise CacheError("cache warning must be within 1..1024 GiB")
    print("Automexia development storage")
    for candidate in candidates:
        flags = []
        if candidate.current:
            flags.append("current")
        if not candidate.clean:
            flags.append("dirty")
        if candidate.leased:
            flags.append("leased")
        if candidate.current_toolset:
            flags.append("required")
        state = ",".join(flags) if flags else "inactive"
        print(
            f"{candidate.label:<28} {format_bytes(candidate.measurement.bytes):>12}  {state}"
        )
    used = _distinct_bytes(candidates)
    print(f"{'distinct used':<28} {format_bytes(used):>12}")
    if used >= warn_gib * GIB:
        print(
            "WARNING: development storage exceeds its budget; run "
            "`cargo xtask cache gc` to preview safe cleanup"
        )
    else:
        print("PASS: development storage is below its warning budget")


def run_gc(
    *,
    root: Path,
    scope: str,
    grace_hours: int,
    apply: bool,
) -> int:
    candidates = inventory(root=root, scope=scope)
    selected = reclaimable(
        candidates, scope=scope, grace_hours=grace_hours
    )
    total = sum(candidate.measurement.bytes for candidate in selected)
    action = "REMOVE" if apply else "WOULD REMOVE"
    for candidate in selected:
        print(f"{action}: {candidate.label} ({format_bytes(candidate.measurement.bytes)})")
    if apply:
        for candidate in selected:
            remove_candidate(candidate, root=root)
        print(f"PASS: removed {len(selected)} cache directories ({format_bytes(total)})")
    else:
        print(
            f"DRY RUN: {len(selected)} cache directories are reclaimable "
            f"({format_bytes(total)}); add --apply to remove them"
        )
    return total


def main() -> int:
    parser = argparse.ArgumentParser()
    subcommands = parser.add_subparsers(dest="command", required=True)
    status = subcommands.add_parser("status")
    status.add_argument("--warn-gib", type=int, default=DEFAULT_WARNING_GIB)
    gc = subcommands.add_parser("gc")
    gc.add_argument("--scope", choices=SCOPES, default="automatic")
    gc.add_argument("--grace-hours", type=int, default=DEFAULT_GRACE_HOURS)
    gc.add_argument("--apply", action="store_true")
    arguments = parser.parse_args()
    try:
        if arguments.command == "status":
            print_status(inventory(root=ROOT), warn_gib=arguments.warn_gib)
        else:
            run_gc(
                root=ROOT,
                scope=arguments.scope,
                grace_hours=arguments.grace_hours,
                apply=arguments.apply,
            )
    except CacheError as error:
        print(f"cache: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
