#!/usr/bin/env python3
"""Validate repository-owned structured text without changing the checkout."""

from __future__ import annotations

import configparser
import json
import os
from pathlib import Path
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

    rendered = ", ".join(f"{kind}={count}" for kind, count in counts.items())
    print(f"PASS: repository structured formats are valid ({rendered})")


if __name__ == "__main__":
    try:
        validate()
    except Exception as error:  # noqa: BLE001 - report the parser's exact error.
        print(f"repository format validation failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
