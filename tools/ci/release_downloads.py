#!/usr/bin/env python3
"""User-facing filenames, asset labels and download guidance for release packages."""

from __future__ import annotations

import argparse
from pathlib import Path

from release_identity import release_tag


# Slots match the manifest owner's eleven-package matrix. Native build filenames
# stay internal; public names are assigned before SBOM/checksum generation.
DOWNLOADS = (
    ("Windows x64 MSI", "windows-x64-installer.msi", "Windows - Intel/AMD 64-bit (x64) - Installer (.msi)"),
    ("Windows x64 ZIP", "windows-x64-portable.zip", "Windows - Intel/AMD 64-bit (x64) - Portable (.zip)"),
    ("Windows ARM64 MSI", "windows-arm64-installer.msi", "Windows - ARM64 - Installer (.msi)"),
    ("Windows ARM64 ZIP", "windows-arm64-portable.zip", "Windows - ARM64 - Portable (.zip)"),
    ("macOS Universal DMG", "macos-apple-silicon-and-intel.dmg", "macOS - Apple Silicon and Intel - Installer (.dmg)"),
    ("Linux x64 DEB", "linux-debian-ubuntu-x64.deb", "Linux - Debian/Ubuntu family - Intel/AMD 64-bit (.deb)"),
    ("Linux x64 RPM", "linux-fedora-opensuse-x64.rpm", "Linux - Fedora/openSUSE family - Intel/AMD 64-bit (.rpm)"),
    ("Linux x64 tarball", "linux-x64-portable.tar.gz", "Linux - Intel/AMD 64-bit (x64) - Portable (.tar.gz)"),
    ("Linux ARM64 DEB", "linux-debian-ubuntu-arm64.deb", "Linux - Debian/Ubuntu family - ARM64 (.deb)"),
    ("Linux ARM64 RPM", "linux-fedora-opensuse-arm64.rpm", "Linux - Fedora/openSUSE family - ARM64 (.rpm)"),
    ("Linux ARM64 tarball", "linux-arm64-portable.tar.gz", "Linux - ARM64 - Portable (.tar.gz)"),
)

METADATA_LABELS = {
    "SHA256SUMS": "Verification - SHA-256 checksums (SHA256SUMS)",
    "SHA256SUMS.minisig": "Verification - Checksum signature (SHA256SUMS.minisig)",
    "release-manifest.json": "Verification - Package inventory and source commit (.json)",
    "automexia-terminal.spdx.json": "Dependency inventory - SPDX (.json)",
    "automexia-terminal.cdx.json": "Dependency inventory - CycloneDX (.json)",
}


def package_names(version: str) -> dict[str, str]:
    release_tag(version)  # Reuse the release owner's exact supported-version contract.
    return {slot: f"automexia-{version}-{suffix}" for slot, suffix, _ in DOWNLOADS}


def asset_label(filename: str, version: str) -> str | None:
    names = package_names(version)
    return next((label for slot, _, label in DOWNLOADS if names[slot] == filename),
                METADATA_LABELS.get(filename))


def upload_argument(path: Path, version: str) -> str:
    label = asset_label(path.name, version)
    return f"{path}#{label}" if label else str(path)


def is_windows_package(filename: str) -> bool:
    suffixes = tuple("-" + suffix for slot, suffix, _ in DOWNLOADS if slot.startswith("Windows "))
    return filename.endswith((*suffixes, "-pc-windows-msvc.zip", "-pc-windows-msvc.msi"))


def download_guide(version: str) -> str:
    tag = release_tag(version)
    names = package_names(version)
    base = f"https://github.com/AmjedAllaya/automexia/releases/download/{tag}/"
    rows = ["## Choose your download", "", "| Download | Use this when |", "|---|---|"]
    descriptions = {
        "Windows x64 MSI": "Install on an Intel or AMD 64-bit Windows PC (most PCs).",
        "Windows x64 ZIP": "Extract and run on an Intel or AMD 64-bit Windows PC, without installing.",
        "Windows ARM64 MSI": "Install on an ARM-based Windows PC, such as a Snapdragon PC.",
        "Windows ARM64 ZIP": "Extract and run on an ARM-based Windows PC, without installing.",
        "macOS Universal DMG": "Works on both Apple Silicon (M-series) and Intel Macs. Drag the app to Applications.",
        "Linux x64 DEB": "Use a DEB package manager on an Intel or AMD 64-bit PC.",
        "Linux x64 RPM": "Use an RPM package manager on an Intel or AMD 64-bit PC.",
        "Linux x64 tarball": "Extract manually on a compatible Intel or AMD 64-bit GNU/Linux system.",
        "Linux ARM64 DEB": "Use a DEB package manager on a 64-bit ARM device.",
        "Linux ARM64 RPM": "Use an RPM package manager on a 64-bit ARM device.",
        "Linux ARM64 tarball": "Extract manually on a compatible 64-bit ARM GNU/Linux system.",
    }
    rows.extend(f"| [{label}]({base}{names[slot]}) | {descriptions[slot]} |" for slot, _, label in DOWNLOADS)
    rows.extend([
        "", "**Check your processor:** Windows: Settings > System > About > System type. "
        "Linux: run `uname -m`; `x86_64` means x64, and `aarch64` or `arm64` means ARM64. "
        "macOS: the same DMG works on either chip type.",
        "", "**Choose one package.** Installer and portable downloads contain the same application. "
        "Use the installer for a normal installation; choose portable for manual extraction. "
        "Portable packages still use normal application settings and require system libraries.",
        "", "**Linux compatibility:** DEB is the package format used by Debian, Ubuntu and Linux Mint; "
        "RPM is used by Fedora and openSUSE. These are format guides, not certification of every distro or version. "
        "All Linux packages require GNU libc (glibc) 2.35 or newer and the required desktop, font and audio libraries. "
        "The portable archive does not bypass those requirements. Alpine/musl is not supported by these builds.",
        "", "**Other files:** `SHA256SUMS`, its `.minisig` signature and `release-manifest.json` "
        "are for verification. The SPDX and CycloneDX JSON files list dependencies. "
        "GitHub's Source code ZIP/tar.gz files contain source, not a ready-to-run app.", "",
    ])
    return "\n".join(rows)


def replace_download_guide(notes: str, version: str) -> str:
    """Replace only the download section; retain every release disclosure."""
    before, heading, remainder = notes.partition("## Choose your download\n")
    _, next_heading, after = remainder.partition("\n## ")
    if not heading or not next_heading:
        raise ValueError("release notes require a bounded download section")
    return before + download_guide(version) + "\n## " + after


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("guide", "upload-arguments"))
    parser.add_argument("--version", required=True)
    parser.add_argument("--directory", type=Path)
    args = parser.parse_args()
    if args.action == "guide":
        print(download_guide(args.version), end="")
    else:
        if args.directory is None:
            parser.error("upload-arguments requires --directory")
        names = set(package_names(args.version).values()) | set(METADATA_LABELS)
        paths = sorted(args.directory.iterdir())
        if not paths or len(paths) > 32:
            parser.error("publication directory must contain 1 to 32 assets")
        for path in paths:
            if (path.name not in names or path.is_symlink() or not path.is_file()
                    or any(c in str(path) for c in "\r\n#")):
                parser.error("upload arguments require regular files and unambiguous paths")
        for path in paths:
            print(upload_argument(path, args.version))


if __name__ == "__main__":
    main()
