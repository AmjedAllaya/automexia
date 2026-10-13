#!/usr/bin/env python3
"""Canonical public tag and native package identity; no publication or network I/O."""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import tomllib
import plistlib
import re
from pathlib import Path

FIRST_ALPHA_VERSION = "0.0.1-alpha"
FIRST_ALPHA_TAG = "0.0.1_alpha"
CURRENT_ALPHA_VERSION = "0.0.3-alpha"
CURRENT_ALPHA_TAG = "v0.0.3-alpha"
ALPHA_VERSIONS = frozenset((FIRST_ALPHA_VERSION, "0.0.2-alpha", CURRENT_ALPHA_VERSION))
STABLE = re.compile(r"(?:0|[1-9][0-9]{0,4})\.(?:0|[1-9][0-9]{0,4})\.(?:0|[1-9][0-9]{0,4})")


def native_version(version: str) -> str:
    numeric = version.removesuffix("-alpha") if version in ALPHA_VERSIONS else version
    if not STABLE.fullmatch(numeric):
        raise ValueError("unsupported release version (expected X.Y.Z or an authorized alpha)")
    major, minor, patch = map(int, numeric.split("."))
    if major > 255 or minor > 255 or patch > 65535:
        raise ValueError("release version exceeds native installer limits")
    return numeric


def release_tag(version: str) -> str:
    native_version(version)
    return FIRST_ALPHA_TAG if version == FIRST_ALPHA_VERSION else "v" + version


def render_macos_plist(template: Path, output: Path, version: str) -> None:
    numeric = native_version(version)
    data = template.read_bytes()
    if len(data) > 64 * 1024:
        raise ValueError("bundle template exceeds its size limit")
    info = plistlib.loads(data)
    info["CFBundleShortVersionString"] = numeric
    info["CFBundleVersion"] = numeric
    info["AutomexiaReleaseVersion"] = version
    output.write_bytes(plistlib.dumps(info, sort_keys=False))


def native_packager_config(root: Path, version: str, binaries: Path, output: Path) -> dict:
    numeric = native_version(version)
    manifest = root / "apps/automexia-terminal/Cargo.toml"
    if manifest.stat().st_size > 128 * 1024:
        raise ValueError("package manifest exceeds its size limit")
    source = tomllib.loads(manifest.read_text(encoding="utf-8"))
    package = source["package"]
    config = dict(package["metadata"]["packager"])
    # These inputs are already reviewed/built/signed. A metadata hook must not
    # silently rebuild or mutate their bytes before native packaging.
    for name in ("before-packaging-command", "before-each-package-command",
                 "beforePackagingCommand", "beforeEachPackageCommand"):
        if config.get(name):
            raise ValueError("native packaging must not execute build hooks")
    config.update(version=numeric, name=package["name"])
    config["binaries-dir"] = str(binaries.resolve())
    config["out-dir"] = str(output.resolve())
    config.setdefault("description", package.get("description", "Automexia Terminal"))
    return config


def package_native(root: Path, version: str, binaries: Path, output: Path,
                   target: str, formats: str) -> None:
    allowed = {"x86_64-pc-windows-msvc": "wix", "x86_64-apple-darwin": "app,dmg",
               "aarch64-apple-darwin": "app,dmg"}
    if allowed.get(target) != formats:
        raise ValueError("unsupported native packager target/format pair")
    config = native_packager_config(root, version, binaries, output)
    # cargo-packager isolates WiX and retains only its namespaced variables.
    # Bind the vendor path to this invocation, never an inherited override.
    env = dict(os.environ, CARGO_PACKAGER_AUTOMEXIA_CONPTY_ROOT=str(binaries.resolve()))
    subprocess.run(["cargo", "packager", "--config", json.dumps(config),
                    "--target", target, "--formats", formats],
                   cwd=root / "apps/automexia-terminal", env=env, check=True, timeout=1800)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("field", choices=("tag", "native", "prerelease", "plist", "package"))
    parser.add_argument("version")
    parser.add_argument("--template", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--binaries", type=Path)
    parser.add_argument("--target")
    parser.add_argument("--formats")
    args = parser.parse_args()
    try:
        numeric = native_version(args.version)
        if args.field == "package":
            if args.binaries is None or args.output is None or args.target is None or args.formats is None:
                parser.error("package requires binaries, output, target and formats")
            package_native(Path(__file__).resolve().parents[2], args.version, args.binaries,
                           args.output, args.target, args.formats)
        elif args.field == "plist":
            if args.template is None or args.output is None:
                parser.error("plist requires --template and --output")
            render_macos_plist(args.template, args.output, args.version)
        else:
            print({"tag": release_tag(args.version), "native": numeric,
                   "prerelease": str(args.version in ALPHA_VERSIONS).lower()}[args.field])
    except (ValueError, OSError, KeyError, TypeError, subprocess.SubprocessError, plistlib.InvalidFileException) as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
