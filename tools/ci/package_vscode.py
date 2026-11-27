#!/usr/bin/env python3
"""Assemble a platform-specific VSIX from a prebuilt companion and approved desktop staging."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import stat
import struct
import subprocess
import tempfile
from urllib.parse import quote
import xml.etree.ElementTree as ET

import reproducible_archive as archive
import windows_conpty_runtime
import windows_powershell_editor


ROOT = Path(__file__).resolve().parents[2]
EXTENSION = ROOT / "apps" / "automexia-vscode"
TARGETS = tuple(f"{platform}-{arch}" for platform in ("win32", "linux", "darwin")
                for arch in ("x64", "arm64"))
APP = "Automexia Terminal.app"
COMPANION = "automexia-vscode-host"
RUNTIMES = ("automexia", "amx", "automexia-suggestion-helper", "automexia-ssh-helper")
DOCUMENTS = ("LICENSE", "NOTICE.md", "THIRD_PARTY_NOTICES.md", "README.md")
EDITOR_FILES = {
    "extension.js": "extension.js",
    "docked-view.js": "docked-view.js",
    "docked-context.js": "docked-context.js",
    "desktop-catalog.js": "desktop-catalog.js",
    "native-runtime.js": "native-runtime.js",
    "media/icon.png": "media/icon.png",
    "media/panel.html": "media/panel.html",
    "media/panel.css": "media/panel.css",
    "media/panel.js": "media/panel.js",
    "README.md": "readme.md",
    "LICENSE": "LICENSE.txt",
}
MIME_TYPES = {
    "css": "text/css",
    "html": "text/html",
    "js": "application/javascript",
    "json": "application/json",
    "md": "text/markdown",
    "png": "image/png",
    "txt": "text/plain",
    "vsixmanifest": "text/xml",
    "xml": "text/xml",
}
VSIX_NAMESPACE = "http://schemas.microsoft.com/developer/vsx-schema/2011"
CONTENT_NAMESPACE = "http://schemas.openxmlformats.org/package/2006/content-types"


def plain_ancestors(path: Path) -> None:
    for part in (path, *path.parents):
        try:
            metadata = part.lstat()
        except FileNotFoundError:
            continue
        if linked(metadata):
            raise ValueError("source and output paths must not contain links or reparse points")


def linked(metadata: os.stat_result) -> bool:
    return stat.S_ISLNK(metadata.st_mode) or bool(
        getattr(metadata, "st_file_attributes", 0) & stat.FILE_ATTRIBUTE_REPARSE_POINT
    )


def ordinary(path: Path, maximum: int = archive.MAX_FILE_BYTES) -> os.stat_result:
    metadata = path.lstat()
    if linked(metadata) or not stat.S_ISREG(metadata.st_mode) or not 0 < metadata.st_size <= maximum:
        raise ValueError("packaged inputs must be ordinary nonempty bounded files")
    return metadata


def copy_file(path: Path, destination: Path, metadata: os.stat_result, unix: bool) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    with path.open("rb") as source, destination.open("xb") as target:
        remaining = metadata.st_size
        while remaining:
            chunk = source.read(min(remaining, 1024 * 1024))
            if not chunk:
                raise ValueError("packaged input was truncated")
            target.write(chunk)
            remaining -= len(chunk)
        if source.read(1):
            raise ValueError("packaged input grew during staging")
    archive.unchanged(path, metadata)
    destination.chmod(0o755 if unix and metadata.st_mode & stat.S_IXUSR else 0o644)


def mach_executable_cpu(stream, offset: int) -> int:
    stream.seek(offset)
    header = stream.read(32)
    if len(header) != 32 or header[:4] != b"\xcf\xfa\xed\xfe":
        raise ValueError("Mach-O executable has an invalid header")
    cpu, _, file_type = struct.unpack_from("<III", header, 4)
    if file_type != 2:
        raise ValueError("Mach-O file is not an executable image")
    return cpu


def mach_architectures(stream, size: int) -> set[int]:
    magic = stream.read(4)
    if magic == b"\xcf\xfa\xed\xfe":
        return {mach_executable_cpu(stream, 0)}
    if magic not in (b"\xca\xfe\xba\xbe", b"\xca\xfe\xba\xbf"):
        raise ValueError("Mach-O executable has an invalid universal header")
    count_bytes = stream.read(4)
    if len(count_bytes) != 4:
        raise ValueError("Mach-O universal header is truncated")
    count = int.from_bytes(count_bytes, "big")
    if not 1 <= count <= 2:
        raise ValueError("Mach-O executable has an unsupported slice count")
    table_format = ">IIIII" if magic.endswith(b"\xbe") else ">IIQQII"
    entry_size = struct.calcsize(table_format)
    table = stream.read(count * entry_size)
    if len(table) != count * entry_size:
        raise ValueError("Mach-O universal slice table is truncated")
    result, ranges = set(), []
    for cpu, _, offset, length, *_ in struct.iter_unpack(table_format, table):
        end = offset + length
        if offset < 8 + len(table) or length < 32 or end > size:
            raise ValueError("Mach-O slice extends beyond the executable")
        if cpu in result or any(offset < stop and start < end for start, stop in ranges):
            raise ValueError("Mach-O slices overlap or repeat an architecture")
        if mach_executable_cpu(stream, offset) != cpu:
            raise ValueError("Mach-O slice disagrees with the universal header")
        result.add(cpu)
        ranges.append((offset, end))
    return result


def verify_executable(path: Path, metadata: os.stat_result, platform: str, architecture: str) -> None:
    if platform == "win32":
        if windows_conpty_runtime.pe_architecture(path) != architecture:
            raise ValueError("Windows executable architecture differs from the VSIX target")
        return
    if not metadata.st_mode & stat.S_IXUSR:
        raise ValueError("Unix executable lacks the owner execute bit")
    with path.open("rb") as stream:
        if platform == "darwin":
            expected = 0x01000007 if architecture == "x64" else 0x0100000C
            if expected not in mach_architectures(stream, metadata.st_size):
                raise ValueError("Mach-O executable does not contain the VSIX target architecture")
        else:
            header = stream.read(64)
            expected = 62 if architecture == "x64" else 183
            if len(header) != 64 or header[:7] != b"\x7fELF\x02\x01\x01":
                raise ValueError("Linux executable lacks a little-endian ELF64 header")
            file_type, machine = struct.unpack_from("<HH", header, 16)
            if file_type not in (2, 3) or machine != expected:
                raise ValueError("Linux executable architecture differs from the VSIX target")


def tracked_shell_paths() -> set[Path]:
    source = Path("shell-integration")
    tracked = subprocess.run(
        ["git", "ls-files", "-z", "--", source.as_posix()],
        cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, check=True, timeout=30,
    ).stdout
    if not tracked or len(tracked) > 64 * 1024 or not tracked.endswith(b"\0"):
        raise ValueError("tracked shell integration inventory is missing or exceeds its limit")
    paths = set()
    for value in tracked[:-1].decode("utf-8").split("\0"):
        relative = Path(value)
        if relative.is_absolute() or ".." in relative.parts or not relative.is_relative_to(source):
            raise ValueError("tracked shell integration inventory contains an invalid path")
        path = ROOT / relative
        plain_ancestors(path)
        try:
            ordinary(path, 32 * 1024 * 1024)
        except FileNotFoundError:
            # The current checkout may have removed a formerly tracked script.
            continue
        paths.add(relative)
    if not paths or len(paths) > 128:
        raise ValueError("tracked shell integration inventory has an unsupported file count")
    return paths


def runtime_files(desktop: Path, platform: str, architecture: str):
    if platform == "darwin":
        if desktop.name != APP:
            raise ValueError("macOS desktop input must be the staged Automexia Terminal.app")
        binary_root = Path("Contents/MacOS")
        resource_root = Path("Contents/Resources")
        required = {Path("Contents/Info.plist"), resource_root / "automexia-terminal.icns"}
        optional = {Path("Contents/PkgInfo"), Path("Contents/_CodeSignature/CodeResources")}
    else:
        binary_root = resource_root = Path()
        required, optional = set(), set()
    extension = ".exe" if platform == "win32" else ""
    executables = {binary_root / f"{name}{extension}" for name in RUNTIMES}
    required.update(executables)
    required.update(resource_root / name for name in DOCUMENTS)
    required.update(resource_root / path for path in tracked_shell_paths())
    if platform == "win32":
        required.update(Path(item["path"]) for item in windows_conpty_runtime.runtime_files(architecture))
        required.update(windows_powershell_editor.MODULE_PATH / path for path in windows_powershell_editor.FILES)

    # Only the known portable/app layout may enter the archive. Check
    # directory metadata before walking so Windows reparse points are never
    # followed, including under Python versions without Path.is_junction.
    allowed = required | optional
    directories = {parent for path in allowed for parent in path.parents}
    pending, files = [desktop], {}
    while pending:
        directory = pending.pop()
        with os.scandir(directory) as children:
            for child in children:
                path = Path(child.path)
                relative = path.relative_to(desktop)
                metadata = child.stat(follow_symlinks=False)
                if linked(metadata):
                    raise ValueError("desktop staging contains a link or reparse point")
                if relative in directories and stat.S_ISDIR(metadata.st_mode):
                    pending.append(path)
                elif relative in allowed and stat.S_ISREG(metadata.st_mode):
                    files[path] = ordinary(path)
                else:
                    raise ValueError("desktop staging includes an unexpected entry")
    if not required.issubset({path.relative_to(desktop) for path in files}):
        raise ValueError("desktop staging is missing required runtime files")
    if sum(metadata.st_size for metadata in files.values()) > archive.MAX_TOTAL_BYTES:
        raise ValueError("desktop staging exceeds the bundle size limit")
    for relative in executables:
        path = desktop / relative
        verify_executable(path, files[path], platform, architecture)
    conpty = ()
    if platform == "win32":
        conpty = windows_conpty_runtime.verify_runtime(desktop, architecture)
        windows_powershell_editor.verify_runtime(desktop)
    return files, conpty


def write_metadata(stage: Path, manifest: dict, target: str) -> None:
    package = ET.Element("PackageManifest", Version="2.0.0", xmlns=VSIX_NAMESPACE)
    metadata = ET.SubElement(package, "Metadata")
    ET.SubElement(metadata, "Identity", Language="en-US", Id=manifest["name"],
                  Publisher=manifest["publisher"], Version=manifest["version"], TargetPlatform=target)
    ET.SubElement(metadata, "DisplayName").text = manifest["displayName"]
    ET.SubElement(metadata, "Description").text = manifest["description"]
    ET.SubElement(metadata, "GalleryFlags").text = "Private"
    properties = ET.SubElement(metadata, "Properties")
    for name, value in {
        "Engine": manifest["engines"]["vscode"],
        "ExtensionKind": "workspace",
        "ExecutesCode": "true",
    }.items():
        ET.SubElement(properties, "Property", Id=f"Microsoft.VisualStudio.Code.{name}", Value=value)
    ET.SubElement(metadata, "License").text = "extension/LICENSE.txt"
    ET.SubElement(metadata, "Icon").text = "extension/media/icon.png"
    installation = ET.SubElement(package, "Installation")
    ET.SubElement(installation, "InstallationTarget", Id="Microsoft.VisualStudio.Code")
    ET.SubElement(package, "Dependencies")
    assets = ET.SubElement(package, "Assets")
    for kind, path in {
        "Microsoft.VisualStudio.Code.Manifest": "package.json",
        "Microsoft.VisualStudio.Services.Content.Details": "readme.md",
        "Microsoft.VisualStudio.Services.Content.License": "LICENSE.txt",
        "Microsoft.VisualStudio.Services.Icons.Default": "media/icon.png",
    }.items():
        ET.SubElement(assets, "Asset", Type=kind, Path=f"extension/{path}", Addressable="true")
    ET.ElementTree(package).write(stage / "extension.vsixmanifest", encoding="utf-8", xml_declaration=True)

    paths = [path.relative_to(stage) for path, _ in archive.inventory(stage)]
    extensions = {path.suffix[1:].lower() for path in paths if path.suffix}
    types = ET.Element("Types", xmlns=CONTENT_NAMESPACE)
    for extension in sorted(extensions):
        ET.SubElement(types, "Default", Extension=extension,
                      ContentType=MIME_TYPES.get(extension, "application/octet-stream"))
    for path in (path for path in paths if not path.suffix):
        mime = "text/plain" if path.name in ("LICENSE", ".zshenv") else "application/octet-stream"
        ET.SubElement(types, "Override", PartName=f"/{quote(path.as_posix(), safe='/')}", ContentType=mime)
    ET.ElementTree(types).write(stage / "[Content_Types].xml", encoding="utf-8", xml_declaration=True)


def assemble(target: str, desktop: Path, companion: Path, output: Path, epoch: int) -> None:
    if target not in TARGETS:
        raise ValueError("unsupported VSIX target")
    platform, architecture = target.split("-")
    desktop, companion, output = (path.absolute() for path in (desktop, companion, output))
    for path in (desktop, companion, output, EXTENSION):
        plain_ancestors(path)
    desktop = desktop.resolve(strict=True)
    companion = companion.resolve(strict=True)
    output = output.resolve()
    editor_source = EXTENSION.resolve(strict=True)
    expected_name = f"{COMPANION}.exe" if platform == "win32" else COMPANION
    if companion.name != expected_name:
        raise ValueError("companion input has an unexpected executable name")
    if output.suffix.lower() != ".vsix" or output.exists() or any(
        output.is_relative_to(source) for source in (desktop, editor_source)
    ):
        raise ValueError("VSIX output must be a new .vsix file outside the source trees")
    if not desktop.is_dir():
        raise ValueError("desktop input must be a complete staged runtime directory")

    companion_info = ordinary(companion)
    verify_executable(companion, companion_info, platform, architecture)
    files, conpty = runtime_files(desktop, platform, architecture)
    manifest_path = editor_source / "package.json"
    manifest_info = ordinary(manifest_path, 64 * 1024)
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    archive.unchanged(manifest_path, manifest_info)
    if (manifest["name"], manifest["publisher"], manifest["extensionKind"], manifest["main"]) != (
        "automexia-wsl", "automexia", ["workspace"], "./extension.js",
    ):
        raise ValueError("unexpected editor extension identity")
    manifest["files"] = [*dict.fromkeys([*manifest["files"], "runtime"])]
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".vscode-package-", dir=output.parent) as scratch:
        temp = Path(scratch)
        stage = temp / "stage"
        extension = stage / "extension"
        extension.mkdir(parents=True)
        (extension / "package.json").write_bytes(
            (json.dumps(manifest, ensure_ascii=False, indent=2) + "\n").encode("utf-8")
        )
        for relative, destination in EDITOR_FILES.items():
            source = editor_source / relative
            plain_ancestors(source)
            copy_file(source, extension / destination, ordinary(source, 128 * 1024), unix=False)

        runtime = extension / "runtime" / target
        desktop_root = runtime / "desktop"
        if platform == "darwin":
            desktop_root /= APP
        companion_root = runtime / "companion"
        for source, metadata in files.items():
            copy_file(source, desktop_root / source.relative_to(desktop), metadata, platform != "win32")
        copy_file(companion, companion_root / expected_name, companion_info, platform != "win32")
        for source in conpty:
            copy_file(source, companion_root / source.relative_to(desktop), files[source], unix=False)

        write_metadata(stage, manifest, target)
        candidate = temp / "candidate.vsix"
        archive.build(stage, candidate, "zip", epoch)
        # Atomically publish a new name on the same filesystem. Unlike replace,
        # link refuses to overwrite a caller's artifact even across contenders.
        os.link(candidate, output)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=TARGETS, required=True)
    parser.add_argument("--desktop", type=Path, required=True)
    parser.add_argument("--companion", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--epoch", type=int, help="Archive timestamp, defaults to SOURCE_DATE_EPOCH or HEAD")
    args = parser.parse_args()
    try:
        epoch = archive.source_epoch() if args.epoch is None else args.epoch
        assemble(args.target, args.desktop, args.companion, args.out, epoch)
    except (OSError, KeyError, TypeError, subprocess.SubprocessError):
        parser.exit(1, "VSIX assembly failed: unavailable or malformed staging, manifest, or output\n")
    except ValueError as error:
        # Do not echo names or values from malformed JSON or native paths.
        message = "malformed input" if isinstance(error, (json.JSONDecodeError, UnicodeError)) else str(error)
        parser.exit(1, f"VSIX assembly failed: {message}\n")


if __name__ == "__main__":
    main()
