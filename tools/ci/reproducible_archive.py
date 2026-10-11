#!/usr/bin/env python3
"""Bounded deterministic portable archives from already verified staging inputs."""
from __future__ import annotations

import argparse
import datetime
import gzip
import os
from pathlib import Path
import stat
import subprocess
import tarfile
import tempfile
import zipfile

MAX_FILES = 512
MAX_FILE_BYTES = 512 * 1024 * 1024
MAX_TOTAL_BYTES = 2 * 1024 * 1024 * 1024


def source_epoch() -> int:
    value = os.environ.get("SOURCE_DATE_EPOCH")
    if value is None:
        value = subprocess.run(["git", "show", "-s", "--format=%ct", "HEAD"],
                               check=True, text=True, capture_output=True, timeout=30).stdout.strip()
    if not value.isascii() or not value.isdigit() or len(value) > 10:
        raise ValueError("SOURCE_DATE_EPOCH must be a bounded Unix timestamp")
    epoch = int(value)
    if not 0 <= epoch <= 4354819198:
        raise ValueError("archive epoch is outside the supported timestamp range")
    return epoch


def inventory(source: Path) -> list[tuple[Path, os.stat_result]]:
    if source.is_symlink() or not source.is_dir():
        raise ValueError("archive source must be a real directory")
    files, pending, visited, total = [], [source], 0, 0
    while pending:
        with os.scandir(pending.pop()) as entries:
            for entry in entries:
                visited += 1
                if visited > MAX_FILES * 2:
                    raise ValueError("archive tree exceeds its entry bound")
                path = Path(entry.path)
                metadata = path.lstat()
                if path.is_symlink() or getattr(path, "is_junction", lambda: False)():
                    raise ValueError("archive inputs must not contain linked paths")
                if stat.S_ISDIR(metadata.st_mode):
                    pending.append(path)
                elif stat.S_ISREG(metadata.st_mode) and metadata.st_nlink == 1:
                    total += metadata.st_size
                    files.append((path, metadata))
                    if len(files) > MAX_FILES or metadata.st_size > MAX_FILE_BYTES or total > MAX_TOTAL_BYTES:
                        raise ValueError("archive payload exceeds its resource bounds")
                else:
                    raise ValueError("archive inputs must be regular unlinked files")
    if not files:
        raise ValueError("archive payload is empty")
    return sorted(files, key=lambda item: item[0].relative_to(source).as_posix())


def unchanged(path: Path, before: os.stat_result) -> None:
    after = path.lstat()
    if (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns) != (
            after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns):
        raise ValueError("archive input changed during construction")


def build(source: Path, output: Path, kind: str, epoch: int) -> None:
    if kind not in ("zip", "tar.gz") or not 0 <= epoch <= 4354819198:
        raise ValueError("invalid archive format or timestamp")
    source = source.absolute()
    if output.resolve() == source.resolve() or source.resolve() in output.resolve().parents:
        raise ValueError("archive output must be outside its source")
    files = inventory(source)
    output.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=".archive-", dir=output.parent)
    temporary = Path(temporary_name)
    try:
        with os.fdopen(descriptor, "wb") as raw:
            if kind == "tar.gz":
                with gzip.GzipFile(filename="", fileobj=raw, mode="wb", mtime=0, compresslevel=9) as compressed:
                    with tarfile.open(fileobj=compressed, mode="w|", format=tarfile.PAX_FORMAT) as archive:
                        for path, metadata in files:
                            entry = tarfile.TarInfo(path.relative_to(source).as_posix())
                            entry.size, entry.mtime = metadata.st_size, epoch
                            entry.mode = 0o755 if metadata.st_mode & 0o111 else 0o644
                            with path.open("rb") as payload:
                                archive.addfile(entry, payload)
                            unchanged(path, metadata)
            else:
                timestamp = datetime.datetime.fromtimestamp(max(epoch, 315532800), datetime.timezone.utc).timetuple()[:6]
                with zipfile.ZipFile(raw, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
                    for path, metadata in files:
                        entry = zipfile.ZipInfo(path.relative_to(source).as_posix(), timestamp)
                        entry.create_system = 3
                        entry.external_attr = (stat.S_IFREG | (0o755 if metadata.st_mode & 0o111 else 0o644)) << 16
                        entry.compress_type = zipfile.ZIP_DEFLATED
                        with path.open("rb") as payload, archive.open(entry, "w") as destination:
                            remaining = metadata.st_size
                            while remaining:
                                chunk = payload.read(min(remaining, 1024 * 1024))
                                if not chunk:
                                    raise ValueError("archive input was truncated")
                                destination.write(chunk)
                                remaining -= len(chunk)
                            if payload.read(1):
                                raise ValueError("archive input grew during construction")
                        unchanged(path, metadata)
            raw.flush()
            os.fsync(raw.fileno())
        os.replace(temporary, output)
    finally:
        temporary.unlink(missing_ok=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--format", choices=("zip", "tar.gz"), required=True)
    args = parser.parse_args()
    try:
        build(args.source, args.output, args.format, source_epoch())
    except (OSError, ValueError, subprocess.SubprocessError):
        parser.exit(1, "portable archive failed: invalid, changed or unavailable input\n")


if __name__ == "__main__":
    main()
