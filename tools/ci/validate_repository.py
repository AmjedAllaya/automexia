#!/usr/bin/env python3
"""Validate repository-owned structured text without changing the checkout."""

from __future__ import annotations

import configparser
import hashlib
import json
import os
from pathlib import Path
import struct
import sys
import tomllib
import xml.etree.ElementTree as element_tree

import yaml


ROOT = Path(__file__).resolve().parents[2]
EXCLUDED_PARTS = {".git", ".cargo-packager", "target"}


def files_with_suffixes(*suffixes: str) -> list[Path]:
    matches: list[Path] = []
    for directory, child_directories, filenames in os.walk(ROOT):
        child_directories[:] = [
            name for name in child_directories if name not in EXCLUDED_PARTS
        ]
        base = Path(directory)
        matches.extend(
            base / filename
            for filename in filenames
            if Path(filename).suffix.lower() in suffixes
        )
    return sorted(matches)


def relative(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def png_metadata(path: Path) -> tuple[int, int, int]:
    data = path.read_bytes()
    if len(data) < 26 or data[:8] != b"\x89PNG\r\n\x1a\n" or data[12:16] != b"IHDR":
        raise ValueError(f"{relative(path)} is not a valid PNG")
    width, height = struct.unpack(">II", data[16:24])
    return width, height, data[25]


def validate_brand_assets() -> None:
    manifest_path = ROOT / "assets/brand/ASSET-MANIFEST.toml"
    with manifest_path.open("rb") as source_file:
        manifest = tomllib.load(source_file)

    source = manifest["source"]
    source_path = ROOT / source["path"]
    source_digest = hashlib.sha256(source_path.read_bytes()).hexdigest()
    if source_digest != source["sha256"]:
        raise ValueError(
            f"{relative(source_path)} SHA-256 does not match ASSET-MANIFEST.toml"
        )
    width, height, color_type = png_metadata(source_path)
    if (width, height) != (512, 512) or color_type not in {4, 6}:
        raise ValueError(
            f"{relative(source_path)} must be a 512x512 PNG with an alpha channel"
        )

    png_root = ROOT / "assets/brand/png"
    for size in (16, 32, 48, 64, 128, 256, 512, 1024):
        path = png_root / f"automexia-terminal-{size}.png"
        dimensions = png_metadata(path)[:2]
        if dimensions != (size, size):
            raise ValueError(
                f"{relative(path)} is {dimensions[0]}x{dimensions[1]}; "
                f"expected {size}x{size}"
            )
    master = ROOT / "assets/brand/automexia-terminal-1024.png"
    if png_metadata(master)[:2] != (1024, 1024):
        raise ValueError(f"{relative(master)} must be 1024x1024")

    ico_path = ROOT / "assets/brand/automexia-terminal.ico"
    ico = ico_path.read_bytes()
    if len(ico) < 6 or ico[:4] != b"\x00\x00\x01\x00":
        raise ValueError(f"{relative(ico_path)} has an invalid ICO header")
    count = struct.unpack("<H", ico[4:6])[0]
    if len(ico) < 6 + count * 16:
        raise ValueError(f"{relative(ico_path)} has a truncated ICO directory")
    ico_sizes: set[int] = set()
    for index in range(count):
        offset = 6 + index * 16
        width_byte, height_byte = ico[offset : offset + 2]
        width = width_byte or 256
        height = height_byte or 256
        if width != height:
            raise ValueError(f"{relative(ico_path)} contains a non-square icon")
        ico_sizes.add(width)
    expected_ico = {16, 32, 48, 64, 128, 256}
    if not expected_ico.issubset(ico_sizes):
        raise ValueError(
            f"{relative(ico_path)} has sizes {sorted(ico_sizes)}; "
            f"expected at least {sorted(expected_ico)}"
        )

    icns_path = ROOT / "assets/brand/automexia-terminal.icns"
    icns = icns_path.read_bytes()
    if len(icns) < 8 or icns[:4] != b"icns":
        raise ValueError(f"{relative(icns_path)} has an invalid ICNS header")
    if struct.unpack(">I", icns[4:8])[0] != len(icns):
        raise ValueError(f"{relative(icns_path)} has an invalid declared length")
    expected_types = {
        b"icp4",
        b"icp5",
        b"icp6",
        b"ic07",
        b"ic08",
        b"ic09",
        b"ic10",
    }
    found_types: set[bytes] = set()
    offset = 8
    while offset < len(icns):
        if offset + 8 > len(icns):
            raise ValueError(f"{relative(icns_path)} has a truncated entry")
        entry_type = icns[offset : offset + 4]
        entry_length = struct.unpack(">I", icns[offset + 4 : offset + 8])[0]
        if entry_length < 8 or offset + entry_length > len(icns):
            raise ValueError(f"{relative(icns_path)} has an invalid entry length")
        found_types.add(entry_type)
        offset += entry_length
    if not expected_types.issubset(found_types):
        raise ValueError(
            f"{relative(icns_path)} is missing standard icon entries"
        )


def validate() -> None:
    counts: dict[str, int] = {}

    toml_files = files_with_suffixes(".toml") + [ROOT / "Cargo.lock"]
    for path in toml_files:
        with path.open("rb") as source:
            tomllib.load(source)
    counts["TOML"] = len(toml_files)

    yaml_files = files_with_suffixes(".yaml", ".yml")
    for path in yaml_files:
        with path.open("r", encoding="utf-8") as source:
            yaml.safe_load(source)
    counts["YAML"] = len(yaml_files)

    json_files = files_with_suffixes(".json")
    for path in json_files:
        with path.open("r", encoding="utf-8") as source:
            json.load(source)
    counts["JSON"] = len(json_files)

    xml_files = files_with_suffixes(".xml", ".plist", ".wxs")
    for path in xml_files:
        element_tree.parse(path)
    counts["XML"] = len(xml_files)

    desktop_files = files_with_suffixes(".desktop")
    for path in desktop_files:
        parser = configparser.ConfigParser(interpolation=None, strict=True)
        parser.optionxform = str
        with path.open("r", encoding="utf-8") as source:
            parser.read_file(source)
        if "Desktop Entry" not in parser:
            raise ValueError(f"{relative(path)} has no [Desktop Entry]")
        entry = parser["Desktop Entry"]
        expected = {
            "Type": "Application",
            "Name": "Automexia Terminal",
            "Exec": "automexia",
            "Icon": "automexia-terminal",
            "StartupWMClass": "AutomexiaTerminal",
        }
        for key, value in expected.items():
            if entry.get(key) != value:
                raise ValueError(
                    f"{relative(path)} has {key}={entry.get(key)!r}; expected {value!r}"
                )
        if "x-scheme-handler/automexia;" not in entry.get("MimeType", ""):
            raise ValueError(f"{relative(path)} does not register automexia://")
    counts["desktop"] = len(desktop_files)

    validate_brand_assets()
    counts["brand assets"] = 1

    rendered = ", ".join(f"{kind}={count}" for kind, count in counts.items())
    print(f"PASS: repository structured formats are valid ({rendered})")


if __name__ == "__main__":
    try:
        validate()
    except Exception as error:  # noqa: BLE001 - report the parser's exact error.
        print(f"repository format validation failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
