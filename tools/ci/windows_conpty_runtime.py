"""Prepare the pinned Microsoft transport; never used by terminal hot paths.

The development-cache lease owns download/extraction/staging. Installed code
continues using the existing sibling DLL loader and performs no downloads.
"""

from __future__ import annotations

import argparse
import contextlib
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import struct
import sys
import tempfile
import time
import urllib.error
import urllib.request
import zipfile

import dev_cache
import qa_process


ROOT = Path(__file__).resolve().parents[2]
RECIPE = ROOT / "packaging/windows/conpty-runtime.json"
MAX_ARCHIVE_BYTES = 8 * 1024 * 1024
MAX_MEMBER_BYTES = 4 * 1024 * 1024
MAX_ARCHIVE_MEMBERS = 64
MAX_EXPANDED_BYTES = 24 * 1024 * 1024
DOWNLOAD_SECONDS = 120
ARCHITECTURES = {"x64": 0x8664, "arm64": 0xAA64}
_preparation_quarantine: tuple[Path, object] | None = None


class RuntimeError(ValueError):
    """A bounded preparation failure, with no secret-bearing path/output."""


class _DownloadCleanupIncomplete(RuntimeError):
    def __init__(self, staging: Path):
        super().__init__("ConPTY download cleanup remains unresolved; further preparation is blocked")
        self.staging = staging


@contextlib.contextmanager
def _preparation_lease():
    global _preparation_quarantine
    if _preparation_quarantine is not None:
        raise RuntimeError("ConPTY download cleanup remains unresolved; further preparation is blocked")
    lease = dev_cache.cache_lease("windows-conpty-runtime", root=ROOT)
    lease.__enter__()
    retained = False
    try:
        yield
    except _DownloadCleanupIncomplete as error:
        # QA retains the native process owner. Keep its staging and GC lease
        # alive too, with at most one unresolved preparation in this process.
        _preparation_quarantine = (error.staging, lease)
        retained = True
        raise
    finally:
        if not retained:
            lease.__exit__(*sys.exc_info())


def normalize_architecture(architecture: str) -> str:
    aliases = {
        "x64": "x64", "x86_64": "x64", "x86_64-pc-windows-msvc": "x64",
        "arm64": "arm64", "aarch64": "arm64", "aarch64-pc-windows-msvc": "arm64",
    }
    try:
        return aliases[architecture]
    except KeyError as error:
        raise RuntimeError("ConPTY requires a supported Windows x64 or ARM64 target") from error


def _relative(value: str) -> bool:
    return (isinstance(value, str) and 0 < len(value) <= 200 and "\\" not in value
            and ":" not in value and not value.startswith("/")
            and all(part not in {"", ".", ".."} for part in value.split("/")))


def load_recipe() -> dict:
    try:
        if RECIPE.stat().st_size > 16 * 1024:
            raise RuntimeError("ConPTY recipe exceeds its size bound")
        recipe = json.loads(RECIPE.read_text(encoding="utf-8"))
        package = recipe["package"]
        files = recipe["files"]
        version = package["version"]
        expected_url = ("https://api.nuget.org/v3-flatcontainer/"
                        f"microsoft.windows.console.conpty/{version}/"
                        f"microsoft.windows.console.conpty.{version}.nupkg")
        valid = (recipe["schema_version"] == 1
                 and package["id"] == "Microsoft.Windows.Console.ConPTY"
                 and package["publisher"] == "Microsoft Corporation"
                 and package["license"] == "MIT"
                 and package["source"] == "https://github.com/microsoft/terminal"
                 and re.fullmatch(r"[0-9]+(?:\.[0-9]+){2}", version)
                 and package["url"] == expected_url
                 and re.fullmatch(r"[0-9a-f]{64}", package["sha256"])
                 and len(files) == 4)
        expected = {("conpty.dll", "x64", ARCHITECTURES["x64"]),
                    ("conpty.dll", "arm64", ARCHITECTURES["arm64"]),
                    ("x64/OpenConsole.exe", "any", ARCHITECTURES["x64"]),
                    ("arm64/OpenConsole.exe", "any", ARCHITECTURES["arm64"])}
        actual = {(f["path"], f["architecture"], f["machine"]) for f in files}
        valid = valid and actual == expected and len({f["member"] for f in files}) == 4
        valid = valid and all(_relative(f["member"]) and _relative(f["path"])
                              and type(f["size"]) is int and 0 < f["size"] <= MAX_MEMBER_BYTES
                              and re.fullmatch(r"[0-9a-f]{64}", f["sha256"]) for f in files)
        if not valid:
            raise RuntimeError("ConPTY recipe violates the pinned runtime contract")
        return recipe
    except (OSError, ValueError, KeyError, TypeError) as error:
        raise RuntimeError("ConPTY recipe cannot be read or is malformed") from error


def runtime_files(architecture: str) -> tuple[dict, ...]:
    architecture = normalize_architecture(architecture)
    return tuple(dict(f) for f in load_recipe()["files"]
                 if f["architecture"] in {architecture, "any"})


def runtime_version() -> str:
    return load_recipe()["package"]["version"]


def _safe_path(path: Path) -> None:
    # Check every existing ancestor before mkdir/open so linked parents cannot
    # redirect generated copies, even when the final child does not yet exist.
    for candidate in (path, *path.parents):
        try:
            metadata = candidate.lstat()
        except FileNotFoundError:
            continue
        if stat.S_ISLNK(metadata.st_mode) or getattr(metadata, "st_file_attributes", 0) & 0x400:
            raise RuntimeError("ConPTY paths cannot contain links or reparse points")


def _directory(path: Path) -> None:
    _safe_path(path)
    dev_cache.ensure_cache_directory(path)


def _digest(path: Path, maximum: int) -> tuple[int, str]:
    _safe_path(path)
    if not path.is_file() or path.stat().st_size > maximum:
        raise RuntimeError("ConPTY asset is not an ordinary bounded file")
    digest = hashlib.sha256()
    total = 0
    with path.open("rb") as source:
        while chunk := source.read(1024 * 1024):
            total += len(chunk)
            if total > maximum:
                raise RuntimeError("ConPTY asset exceeds its byte ceiling")
            digest.update(chunk)
    return total, digest.hexdigest()


def pe_architecture(path: Path) -> str:
    """Read only bounded DOS/COFF headers; never load or execute the image."""
    _safe_path(path)
    with path.open("rb") as source:
        header = source.read(64)
        if len(header) != 64 or header[:2] != b"MZ":
            raise RuntimeError("ConPTY executable has an invalid DOS header")
        offset = struct.unpack_from("<I", header, 60)[0]
        if not 64 <= offset <= 1024 * 1024:
            raise RuntimeError("ConPTY executable PE offset exceeds its bound")
        source.seek(offset)
        coff = source.read(6)
        if len(coff) != 6 or coff[:4] != b"PE\0\0":
            raise RuntimeError("ConPTY executable has an invalid PE header")
        machine = struct.unpack_from("<H", coff, 4)[0]
    for architecture, expected in ARCHITECTURES.items():
        if machine == expected:
            return architecture
    raise RuntimeError("ConPTY executable has an unsupported machine architecture")


def _verify_file(path: Path, item: dict) -> None:
    size, digest = _digest(path, item["size"])
    if size != item["size"] or digest != item["sha256"]:
        raise RuntimeError("ConPTY asset differs from the pinned recipe")
    if ARCHITECTURES[pe_architecture(path)] != item["machine"]:
        raise RuntimeError("ConPTY asset architecture differs from the pinned recipe")


def verify_runtime(destination: Path, architecture: str) -> tuple[Path, ...]:
    files = runtime_files(architecture)
    _safe_path(destination)
    # The vendor loader prioritizes a root host over kernel-specific hosts.
    _safe_path(destination / "OpenConsole.exe")
    if (destination / "OpenConsole.exe").exists():
        raise RuntimeError("Root OpenConsole.exe overrides kernel architecture; use a fresh destination")
    paths = tuple(destination / f["path"] for f in files)
    for path, item in zip(paths, files):
        _verify_file(path, item)
    return paths


def _download(url: str, destination: Path) -> None:
    if url != load_recipe()["package"]["url"]:
        raise RuntimeError("ConPTY download must use the pinned package origin")
    try:
        result = qa_process.run(
            [sys.executable, str(Path(__file__).resolve()), "--download-worker", str(destination)],
            cwd=ROOT, timeout_seconds=DOWNLOAD_SECONDS, consume=lambda _: None,
        )
    finally:
        # The shared owner deliberately retains its native identities on
        # incomplete cleanup. Do not delete files it may still be writing.
        if qa_process._quarantine is not None:
            raise _DownloadCleanupIncomplete(destination.parent)
    if result.timed_out or result.error or result.return_code != 0:
        raise RuntimeError("Could not download pinned ConPTY within its deadline; retry or supply --archive")


def _download_worker(destination: Path) -> None:
    staging = Path(os.path.abspath(dev_cache.staging_root(root=ROOT)))
    destination = Path(os.path.abspath(destination))
    _safe_path(destination)
    if (destination.name != "runtime.nupkg" or destination.parent.parent != staging
            or not re.fullmatch(r"conpty-[a-z0-9_]{8}", destination.parent.name)
            or not destination.parent.is_dir()):
        raise RuntimeError("ConPTY download requires its owned staging directory")
    _download_stream(load_recipe()["package"]["url"], destination)


def _download_stream(url: str, destination: Path) -> None:
    # Unlike assurance download_binary, this preserves a NuGet archive with
    # several verified members, rather than installing/executing a single tool.
    deadline = time.monotonic() + DOWNLOAD_SECONDS
    try:
        with urllib.request.urlopen(url, timeout=30) as response, destination.open("xb") as output:
            if response.geturl() != url:
                raise RuntimeError("ConPTY download redirected away from its pinned origin")
            length = response.headers.get("Content-Length")
            if length is not None and (not length.isdigit() or int(length) > MAX_ARCHIVE_BYTES):
                raise RuntimeError("ConPTY download exceeds its byte ceiling")
            total = 0
            while chunk := response.read(64 * 1024):
                total += len(chunk)
                if total > MAX_ARCHIVE_BYTES or time.monotonic() > deadline:
                    raise RuntimeError("ConPTY download exceeds its byte or time ceiling")
                output.write(chunk)
    except (OSError, urllib.error.URLError) as error:
        raise RuntimeError("Could not download pinned ConPTY; retry online or supply --archive") from error


def _extract(archive: Path, destination: Path, recipe: dict) -> None:
    try:
        with zipfile.ZipFile(archive) as source:
            members = source.infolist()
            if len(members) > MAX_ARCHIVE_MEMBERS:
                raise RuntimeError("ConPTY archive has too many members")
            names: set[str] = set()
            expanded = 0
            for item in members:
                name = item.filename.rstrip("/") if item.is_dir() else item.filename
                key = name.casefold()
                mode = item.external_attr >> 16
                expanded += item.file_size
                if (not _relative(name) or key in names or stat.S_ISLNK(mode)
                        or item.flag_bits & 1 or item.file_size > MAX_MEMBER_BYTES
                        or expanded > MAX_EXPANDED_BYTES):
                    raise RuntimeError("ConPTY archive contains an unsafe or oversized member")
                names.add(key)
            for item in recipe["files"]:
                member = source.getinfo(item["member"])
                if member.is_dir() or member.file_size != item["size"]:
                    raise RuntimeError("ConPTY archive member size differs from the recipe")
                target = destination / item["member"]
                _directory(target.parent)
                with source.open(member) as input_file, target.open("xb") as output:
                    shutil.copyfileobj(input_file, output, length=64 * 1024)
                _verify_file(target, item)
    except (OSError, KeyError, zipfile.BadZipFile, RuntimeError) as error:
        raise RuntimeError("ConPTY archive could not be safely verified and extracted") from error


def _cached_members(recipe: dict, *, offline: bool, archive: Path | None) -> Path:
    package = recipe["package"]
    runtime_root = dev_cache.runtime_root(root=ROOT) / "conpty"
    cache = runtime_root / package["sha256"]
    _directory(runtime_root)
    if cache.exists():
        for item in recipe["files"]:
            _verify_file(cache / item["member"], item)
        return cache
    downloads = dev_cache.downloads_root(root=ROOT) / "conpty"
    staging = dev_cache.staging_root(root=ROOT)
    _directory(downloads)
    _directory(staging)
    cached_archive = downloads / (package["sha256"] + ".nupkg")
    temporary = Path(tempfile.mkdtemp(prefix="conpty-", dir=staging))
    retained = False
    try:
        source = archive or cached_archive
        if archive is None and not source.exists():
            if offline:
                raise RuntimeError("Pinned ConPTY is not cached; prepare once online or supply --archive")
            source = temporary / "runtime.nupkg"
            _download(package["url"], source)
        size, digest = _digest(source, MAX_ARCHIVE_BYTES)
        if not size or digest != package["sha256"]:
            raise RuntimeError("ConPTY archive checksum differs from the pinned recipe")
        members = temporary / "members"
        _directory(members)
        _extract(source, members, recipe)
        # Publish only a fully verified cache; an interrupted extraction cannot
        # become a cache hit. Downloads and runtime data retain existing GC ownership.
        os.replace(members, cache)
        if not cached_archive.exists():
            copy = temporary / "verified.nupkg"
            shutil.copyfile(source, copy)
            os.replace(copy, cached_archive)
    except _DownloadCleanupIncomplete:
        retained = True
        raise
    finally:
        if not retained:
            shutil.rmtree(temporary)
    return cache


def prepare_runtime(destination: Path, architecture: str, *, offline: bool = False,
                    archive: Path | None = None) -> tuple[Path, ...]:
    files = runtime_files(architecture)
    recipe = load_recipe()
    destination = Path(os.path.abspath(destination))
    with _preparation_lease():
        _directory(destination)
        _safe_path(destination / "OpenConsole.exe")
        if (destination / "OpenConsole.exe").exists():
            raise RuntimeError("Root OpenConsole.exe overrides kernel architecture; use a fresh destination")
        # Preflight every destination before altering any existing file. Equal
        # files are left untouched, including DLLs held open by running sessions.
        missing = []
        for item in files:
            path = destination / item["path"]
            _safe_path(path)
            if path.exists():
                try:
                    _verify_file(path, item)
                except RuntimeError as error:
                    raise RuntimeError("Conflicting ConPTY asset; preserve it and use a fresh destination") from error
            else:
                missing.append(item)
        if not missing:
            return verify_runtime(destination, architecture)
        cache = _cached_members(recipe, offline=offline, archive=archive)
        with tempfile.TemporaryDirectory(prefix=".conpty-stage-", dir=destination) as temporary_name:
            temporary = Path(temporary_name)
            for item in missing:
                staged = temporary / item["path"]
                _directory(staged.parent)
                shutil.copyfile(cache / item["member"], staged)
                _verify_file(staged, item)
            # Exact flat loader layout cannot be swapped as one directory.
            # Publish both hosts first and the DLL last, after all staged hashes
            # pass. Existing unknown assets are never replaced or removed.
            for item in sorted(missing, key=lambda item: item["path"] == "conpty.dll"):
                target = destination / item["path"]
                _directory(target.parent)
                if target.exists():
                    _verify_file(target, item)
                else:
                    # A hard-link publication is same-volume and no-clobber.
                    try:
                        os.link(temporary / item["path"], target)
                    except OSError as error:
                        raise RuntimeError("Could not publish ConPTY asset; use an ordinary writable directory") from error
        return verify_runtime(destination, architecture)


def main(argv: list[str] | None = None) -> int:
    argv = sys.argv[1:] if argv is None else argv
    if argv and argv[0] == "--download-worker":
        if len(argv) != 2:
            return 2
        try:
            _download_worker(Path(argv[1]))
            return 0
        except (RuntimeError, dev_cache.CacheError, OSError):
            return 1
    parser = argparse.ArgumentParser(description="Prepare the pinned Windows PTY runtime")
    parser.add_argument("action", choices=("prepare", "verify"))
    parser.add_argument("--destination", required=True, type=Path)
    parser.add_argument("--architecture", required=True)
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--archive", type=Path)
    args = parser.parse_args(argv)
    try:
        if args.action == "prepare":
            prepare_runtime(args.destination, args.architecture, offline=args.offline, archive=args.archive)
        else:
            verify_runtime(args.destination, args.architecture)
        print("PASS: pinned ConPTY DLL and both kernel hosts verified")
        return 0
    except (RuntimeError, dev_cache.CacheError, OSError) as error:
        detail = str(error) if isinstance(error, (RuntimeError, dev_cache.CacheError)) else "ConPTY filesystem operation failed"
        print(f"ERROR: {detail}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
