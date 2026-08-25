#!/usr/bin/env python3
"""Validate repository-owned structured text without changing the checkout."""

from __future__ import annotations

import configparser
import hashlib
import json
import os
from pathlib import Path
import re
import struct
import sys
import tomllib
from urllib.parse import unquote
import xml.etree.ElementTree as element_tree

import yaml

from check_command_productivity import (
    validate_repository as validate_command_productivity,
)
from check_command_productivity_cp1 import (
    validate_repository as validate_command_productivity_cp1,
)
from check_command_productivity_cp22 import (
    validate_repository as validate_command_productivity_cp22,
)
from check_command_productivity_cp30 import (
    validate_repository as validate_command_productivity_cp30,
)
from check_command_productivity_cp31 import (
    validate_repository as validate_command_productivity_cp31,
)
from check_command_productivity_cp32 import (
    validate_repository as validate_command_productivity_cp32,
)
from check_command_productivity_cp33 import (
    validate_repository as validate_command_productivity_cp33,
)
from check_command_productivity_cp50 import (
    validate_repository as validate_command_productivity_cp50,
)
from check_command_productivity_cp51 import (
    validate_repository as validate_command_productivity_cp51,
)
from check_command_productivity_cp56 import (
    validate_repository as validate_command_productivity_cp56,
)
from check_ecosystem_d7_cp6 import (
    validate_repository as validate_ecosystem_d7_cp6,
)
from check_session_launch_d0 import validate_repository as validate_session_launch_d0
from check_provider_auth_m7 import validate_repository as validate_provider_auth_m7
from check_provider_quick_actions_cp4 import (
    validate_repository as validate_provider_quick_actions_cp4,
)
from check_documentation_coverage import validate as validate_documentation_coverage
from check_documentation_hygiene import validate as validate_documentation_hygiene
from check_devops_alias_spec import validate_repository as validate_devops_alias_spec
from check_feature_assurance import load_and_validate as validate_feature_assurance
from check_feature_test_reinforcement import (
    load_and_validate as validate_feature_test_reinforcement,
)
from check_phase_implementation_audit import validate as validate_phase_audit
from check_repository_aligned_docs import validate as validate_repository_aligned_docs
from check_platform_coverage import validate_repository_workflows
from repository_protection import validate_repository as validate_repository_protection
from release_trust import load_policy as validate_release_trust_policy


ROOT = Path(__file__).resolve().parents[2]
EXCLUDED_PARTS = {".git", ".cargo-packager", "target"}
MARKDOWN_LINK = re.compile(r"!?\[[^\]]*\]\(([^)]+)\)")
ACTION_USE = re.compile(r"^\s*-\s+uses:\s*([^\s#]+)", re.MULTILINE)


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


def markdown_anchors(path: Path) -> set[str]:
    anchors: set[str] = set()
    occurrences: dict[str, int] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        match = re.match(r"^#{1,6}\s+(.+?)\s*#*\s*$", line)
        if not match:
            continue
        heading = re.sub(r"<[^>]+>", "", match.group(1)).strip().lower()
        slug = re.sub(r"[^\w\- ]", "", heading, flags=re.UNICODE)
        slug = re.sub(r"[\s-]+", "-", slug).strip("-")
        duplicate = occurrences.get(slug, 0)
        occurrences[slug] = duplicate + 1
        anchors.add(slug if duplicate == 0 else f"{slug}-{duplicate}")
    return anchors


def validate_markdown_links() -> int:
    markdown_files = files_with_suffixes(".md")
    anchor_cache: dict[Path, set[str]] = {}
    for source in markdown_files:
        content = source.read_text(encoding="utf-8")
        for match in MARKDOWN_LINK.finditer(content):
            raw_target = match.group(1).strip()
            if raw_target.startswith("<") and raw_target.endswith(">"):
                raw_target = raw_target[1:-1]
            if re.match(r"^(?:https?|mailto):", raw_target, re.IGNORECASE):
                continue
            target_text, separator, anchor = raw_target.partition("#")
            target_text = unquote(target_text)
            target = source if not target_text else (source.parent / target_text).resolve()
            try:
                target.relative_to(ROOT)
            except ValueError as error:
                raise ValueError(
                    f"{relative(source)} links outside the repository: {raw_target}"
                ) from error
            if not target.exists():
                raise ValueError(
                    f"{relative(source)} has a missing local link: {raw_target}"
                )
            if separator and anchor and target.suffix.lower() == ".md":
                anchors = anchor_cache.setdefault(target, markdown_anchors(target))
                if unquote(anchor).lower() not in anchors:
                    raise ValueError(
                        f"{relative(source)} has a missing Markdown anchor: {raw_target}"
                    )
    return len(markdown_files)


def validate_action_pins() -> int:
    workflows = sorted((ROOT / ".github/workflows").glob("*.y*ml"))
    uses = 0
    for path in workflows:
        content = path.read_text(encoding="utf-8")
        for match in ACTION_USE.finditer(content):
            action = match.group(1).strip("'\"")
            if action.startswith("./"):
                continue
            _, separator, revision = action.rpartition("@")
            if not separator or not re.fullmatch(r"[0-9a-f]{40}", revision):
                raise ValueError(
                    f"{relative(path)} uses an action without a full commit pin: {action}"
                )
            uses += 1
    return uses


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

    xml_files = files_with_suffixes(".xml", ".plist", ".wxs", ".ps1xml")
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

    counts["Markdown"] = validate_markdown_links()
    counts["Markdown hygiene"] = validate_documentation_hygiene()
    counts["aligned documentation pack"] = validate_repository_aligned_docs()
    counts["pinned Actions"] = validate_action_pins()

    validate_repository_workflows()
    counts["platform workflow matrix"] = 1

    protection_counts = validate_repository_protection()
    counts["repository protection rulesets"] = protection_counts["rulesets"]
    counts["repository protection checks"] = protection_counts["required_checks"]

    validate_release_trust_policy()
    counts["release trust policy"] = 1

    feature_counts = validate_feature_assurance()
    counts["feature assurance entries"] = feature_counts["features"]
    reinforcement_counts = validate_feature_test_reinforcement()
    counts["feature reinforcement entries"] = reinforcement_counts["features"]
    counts["feature reinforcement scenarios"] = reinforcement_counts["needed_tests"]

    documentation_counts = validate_documentation_coverage()
    counts["documented source entries"] = sum(
        count
        for kind, count in documentation_counts.items()
        if kind != "pages"
    )

    phase_audit_counts = validate_phase_audit()
    counts["phase audit entries"] = phase_audit_counts["phase_sections"]

    command_productivity_counts = validate_command_productivity()
    counts["command productivity CP0"] = command_productivity_counts["threats"]
    command_productivity_cp1_counts = validate_command_productivity_cp1()
    counts["command productivity CP1"] = command_productivity_cp1_counts["providers"]
    command_productivity_cp22_counts = validate_command_productivity_cp22()
    counts["command productivity CP2.2"] = command_productivity_cp22_counts["tests"]
    command_productivity_cp30_counts = validate_command_productivity_cp30()
    counts["command productivity CP3.0"] = command_productivity_cp30_counts["tests"]
    command_productivity_cp31_counts = validate_command_productivity_cp31()
    counts["command productivity CP3.1"] = command_productivity_cp31_counts["tests"]
    command_productivity_cp32_counts = validate_command_productivity_cp32()
    counts["command productivity CP3.2"] = command_productivity_cp32_counts["tests"]
    command_productivity_cp33_counts = validate_command_productivity_cp33()
    counts["command productivity CP3.3"] = command_productivity_cp33_counts["tests"]
    command_productivity_cp50_counts = validate_command_productivity_cp50()
    counts["command productivity CP5.0"] = command_productivity_cp50_counts["shells"]
    command_productivity_cp51_counts = validate_command_productivity_cp51()
    counts["command productivity CP5.1 accepted contract"] = command_productivity_cp51_counts[
        "threats"
    ]
    command_productivity_cp56_counts = validate_command_productivity_cp56()
    counts["command productivity CP5.1-CP5.6 source"] = (
        command_productivity_cp56_counts["source_files"]
    )
    ecosystem_d7_cp6_counts = validate_ecosystem_d7_cp6()
    counts["ecosystem D7/CP6 proposal"] = ecosystem_d7_cp6_counts["threats"]
    session_launch_d0_counts = validate_session_launch_d0()
    counts["session launch D0/D3"] = session_launch_d0_counts["scenarios"]
    provider_auth_m7_counts = validate_provider_auth_m7()
    counts["provider auth M7/D6.0"] = provider_auth_m7_counts["tests"]
    provider_cp4_counts = validate_provider_quick_actions_cp4()
    counts["provider Quick Actions M13/CP4"] = provider_cp4_counts["tests"]

    alias_spec_counts = validate_devops_alias_spec()
    counts["planned CP2/CP3 alias assurance"] = alias_spec_counts[
        "verification_domains"
    ]

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
