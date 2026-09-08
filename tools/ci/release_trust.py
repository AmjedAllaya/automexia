#!/usr/bin/env python3
"""Validate and inventory only the final, bounded Automexia release artifacts."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import sys
import time
import tempfile
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
POLICY_PATH = ROOT / "tests/assurance/release-trust-policy-v1.json"
VERSION = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?$")
POLICY_KEYS = {
    "schema",
    "product",
    "repository",
    "package_prefix",
    "chunk_bytes",
    "sbom",
    "artifacts",
    "forbidden_suffixes",
    "final_metadata",
}
METADATA_FORMATS = {
    "sha256sums",
    "cyclonedx-json",
    "spdx-json",
    "release-manifest-v1",
    "release-benchmark-v1",
    "windows-trust-v1",
}


class ReleaseTrustError(ValueError):
    """The release artifact or policy violates the fail-closed trust contract."""


def load_policy(path: Path = POLICY_PATH) -> dict[str, Any]:
    policy = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(policy, dict) or set(policy) != POLICY_KEYS:
        raise ReleaseTrustError(
            f"release trust policy must define exactly {sorted(POLICY_KEYS)}"
        )
    if policy.get("schema") != 1:
        raise ReleaseTrustError("release trust policy schema must be 1")
    for key in ("product", "repository", "package_prefix"):
        if not isinstance(policy.get(key), str) or not policy[key].strip():
            raise ReleaseTrustError(f"release trust {key} must be a non-empty string")
    if not isinstance(policy.get("artifacts"), list) or not policy["artifacts"]:
        raise ReleaseTrustError("release trust policy has no artifact rules")
    if not isinstance(policy.get("chunk_bytes"), int) or policy["chunk_bytes"] <= 0:
        raise ReleaseTrustError("release trust chunk_bytes must be positive")
    sbom = policy.get("sbom")
    if not isinstance(sbom, dict) or set(sbom) != {
        "minimum_components",
        "required_purl_prefixes",
    }:
        raise ReleaseTrustError(
            "release trust sbom must define minimum_components and required_purl_prefixes"
        )
    if (
        not isinstance(sbom["minimum_components"], int)
        or sbom["minimum_components"] < 2
    ):
        raise ReleaseTrustError("SBOM minimum_components must be at least 2")
    prefixes = sbom["required_purl_prefixes"]
    if (
        not isinstance(prefixes, list)
        or not prefixes
        or any(not isinstance(prefix, str) or not prefix.startswith("pkg:") for prefix in prefixes)
        or len(set(prefixes)) != len(prefixes)
    ):
        raise ReleaseTrustError("SBOM required_purl_prefixes must be unique purl prefixes")
    ids: set[str] = set()
    suffixes: set[str] = set()
    for rule in policy["artifacts"]:
        required = {
            "id",
            "suffix",
            "count",
            "max_bytes",
            "required_architecture_tokens",
        }
        if not isinstance(rule, dict) or set(rule) != required:
            raise ReleaseTrustError(f"artifact rule must define exactly {sorted(required)}")
        if rule["id"] in ids or rule["suffix"].casefold() in suffixes:
            raise ReleaseTrustError("release artifact ids and suffixes must be unique")
        ids.add(rule["id"])
        suffixes.add(rule["suffix"].casefold())
        if not isinstance(rule["count"], int) or rule["count"] <= 0:
            raise ReleaseTrustError(f"{rule['id']} count must be positive")
        if not isinstance(rule["max_bytes"], int) or rule["max_bytes"] <= 0:
            raise ReleaseTrustError(f"{rule['id']} max_bytes must be positive")
        groups = rule["required_architecture_tokens"]
        if not isinstance(groups, list) or len(groups) != rule["count"]:
            raise ReleaseTrustError(
                f"{rule['id']} must declare one architecture token group per artifact"
            )
        if any(
            not isinstance(group, list)
            or not group
            or any(not isinstance(token, str) or not token for token in group)
            for group in groups
        ):
            raise ReleaseTrustError(f"{rule['id']} has invalid architecture tokens")
    forbidden = policy.get("forbidden_suffixes")
    if (
        not isinstance(forbidden, list)
        or not forbidden
        or any(not isinstance(value, str) or not value.startswith(".") for value in forbidden)
        or len({value.casefold() for value in forbidden}) != len(forbidden)
    ):
        raise ReleaseTrustError("forbidden suffixes must be unique dotted strings")
    metadata = policy.get("final_metadata")
    if not isinstance(metadata, list) or not metadata:
        raise ReleaseTrustError("final metadata rules must be a non-empty list")
    metadata_names: set[str] = set()
    for rule in metadata:
        if not isinstance(rule, dict) or set(rule) != {"name", "format", "max_bytes"}:
            raise ReleaseTrustError(
                "final metadata rule must define exactly name, format, and max_bytes"
            )
        name = rule["name"]
        if (
            not isinstance(name, str)
            or not name
            or Path(name).name != name
            or name.casefold() in metadata_names
        ):
            raise ReleaseTrustError("final metadata names must be unique flat filenames")
        metadata_names.add(name.casefold())
        if rule["format"] not in METADATA_FORMATS:
            raise ReleaseTrustError(f"unsupported final metadata format {rule['format']!r}")
        if not isinstance(rule["max_bytes"], int) or rule["max_bytes"] <= 0:
            raise ReleaseTrustError(f"final metadata {name} max_bytes must be positive")
    if {rule["format"] for rule in metadata} != METADATA_FORMATS:
        raise ReleaseTrustError("final metadata must declare every required format exactly once")
    return policy


def metadata_rules(policy: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return {rule["name"]: rule for rule in policy["final_metadata"]}


def classify(name: str, policy: dict[str, Any]) -> dict[str, Any] | None:
    folded = name.casefold()
    matches = [
        rule
        for rule in policy["artifacts"]
        if folded.endswith(str(rule["suffix"]).casefold())
    ]
    if len(matches) > 1:
        raise ReleaseTrustError(f"artifact {name!r} matches multiple policy rules")
    return matches[0] if matches else None


def package_files(directory: Path, version: str, policy: dict[str, Any]) -> list[tuple[Path, dict[str, Any]]]:
    if not VERSION.fullmatch(version):
        raise ReleaseTrustError(f"invalid release version {version!r}")
    if not directory.is_dir():
        raise ReleaseTrustError(f"artifact directory is missing: {directory}")
    entries = sorted(directory.iterdir(), key=lambda path: path.name.casefold())
    if not entries:
        raise ReleaseTrustError("artifact directory is empty")
    classified: list[tuple[Path, dict[str, Any]]] = []
    forbidden = tuple(str(value).casefold() for value in policy["forbidden_suffixes"])
    prefix = str(policy["package_prefix"]).casefold()
    for path in entries:
        if path.is_symlink():
            raise ReleaseTrustError(f"release artifacts must not be symlinks: {path.name}")
        if not path.is_file():
            raise ReleaseTrustError(f"release package directory must be flat: {path.name}")
        folded = path.name.casefold()
        if folded.endswith(forbidden):
            raise ReleaseTrustError(f"forbidden raw or secret release file: {path.name}")
        rule = classify(path.name, policy)
        if rule is None:
            raise ReleaseTrustError(f"unallowlisted release package: {path.name}")
        version_tokens = (f"-{version.casefold()}", f"_{version.casefold()}")
        if not folded.startswith(prefix) or not any(token in folded for token in version_tokens):
            raise ReleaseTrustError(
                f"release package must contain canonical prefix and version: {path.name}"
            )
        size = path.stat().st_size
        if size <= 0 or size > rule["max_bytes"]:
            raise ReleaseTrustError(
                f"{path.name} size {size} is outside 1..{rule['max_bytes']} bytes"
            )
        classified.append((path, rule))

    for rule in policy["artifacts"]:
        matches = [path for path, owner in classified if owner["id"] == rule["id"]]
        if len(matches) != rule["count"]:
            raise ReleaseTrustError(
                f"{rule['id']} requires {rule['count']} artifacts; found {len(matches)}"
            )
        observed_groups: set[int] = set()
        for path in matches:
            folded_name = path.name.casefold()
            group_matches = [
                index
                for index, group in enumerate(rule["required_architecture_tokens"])
                if any(token.casefold() in folded_name for token in group)
            ]
            if len(group_matches) != 1:
                raise ReleaseTrustError(
                    f"{rule['id']} artifact {path.name} must match exactly one architecture token group"
                )
            observed_groups.add(group_matches[0])
        expected_groups = set(range(len(rule["required_architecture_tokens"])))
        if observed_groups != expected_groups:
            raise ReleaseTrustError(f"{rule['id']} is missing an architecture token group")
    return classified


def digest(path: Path, chunk_bytes: int) -> str:
    value = hashlib.sha256()
    with path.open("rb") as source:
        while chunk := source.read(chunk_bytes):
            value.update(chunk)
    return value.hexdigest()


def write_json_atomic(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    try:
        temporary.write_text(
            json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def inventory(
    directory: Path,
    version: str,
    manifest_path: Path,
    benchmark_path: Path,
    policy: dict[str, Any],
) -> dict[str, Any]:
    artifacts = package_files(directory, version, policy)
    started = time.perf_counter()
    entries = []
    total_bytes = 0
    for path, rule in artifacts:
        size = path.stat().st_size
        total_bytes += size
        entries.append(
            {
                "kind": rule["id"],
                "name": path.name,
                "sha256": digest(path, policy["chunk_bytes"]),
                "size": size,
            }
        )
    elapsed = max(time.perf_counter() - started, 0.000001)
    manifest = {
        "schema": 1,
        "product": policy["product"],
        "repository": policy["repository"],
        "version": version,
        "artifacts": entries,
    }
    benchmark = {
        "schema": 1,
        "operation": "streaming-sha256-release-inventory",
        "artifact_count": len(entries),
        "total_bytes": total_bytes,
        "elapsed_milliseconds": round(elapsed * 1000, 3),
        "throughput_mib_per_second": round(total_bytes / elapsed / 1048576, 3),
        "classification": "informational-release-pipeline-overhead",
    }
    write_json_atomic(manifest_path, manifest)
    write_json_atomic(benchmark_path, benchmark)
    return benchmark


def read_bounded(path: Path, max_bytes: int) -> bytes:
    size = path.stat().st_size
    if size <= 0 or size > max_bytes:
        raise ReleaseTrustError(
            f"{path.name} size {size} is outside 1..{max_bytes} bytes"
        )
    with path.open("rb") as source:
        value = source.read(max_bytes + 1)
    if len(value) != size:
        raise ReleaseTrustError(f"{path.name} changed while it was being validated")
    return value


def read_json_bounded(path: Path, max_bytes: int) -> Any:
    try:
        return json.loads(read_bounded(path, max_bytes).decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ReleaseTrustError(f"{path.name} is not valid UTF-8 JSON: {error}") from error


def validate_sboms(
    spdx: Any,
    cyclonedx: Any,
    version: str,
    policy: dict[str, Any],
) -> None:
    minimum = policy["sbom"]["minimum_components"]
    prefixes = tuple(policy["sbom"]["required_purl_prefixes"])
    product_name = policy["package_prefix"].casefold()

    if (
        not isinstance(spdx, dict)
        or not str(spdx.get("spdxVersion", "")).startswith("SPDX-")
        or spdx.get("SPDXID") != "SPDXRef-DOCUMENT"
        or spdx.get("dataLicense") != "CC0-1.0"
        or not str(spdx.get("documentNamespace", "")).startswith(("https://", "http://"))
    ):
        raise ReleaseTrustError("automexia-terminal.spdx.json is not a complete SPDX document")
    creation = spdx.get("creationInfo")
    if (
        not isinstance(creation, dict)
        or not isinstance(creation.get("created"), str)
        or not isinstance(creation.get("creators"), list)
        or not creation["creators"]
    ):
        raise ReleaseTrustError("SPDX creationInfo is incomplete")
    spdx_packages = spdx.get("packages")
    if not isinstance(spdx_packages, list) or len(spdx_packages) < minimum:
        raise ReleaseTrustError(f"SPDX packages must contain at least {minimum} components")

    spdx_identities: set[tuple[str, str]] = set()
    spdx_purls: set[str] = set()
    for package in spdx_packages:
        if not isinstance(package, dict):
            raise ReleaseTrustError("SPDX packages must be objects")
        name = package.get("name")
        package_version = package.get("versionInfo")
        package_id = package.get("SPDXID")
        if (package.get("primaryPackagePurpose") == "FILE"
                and isinstance(package_id, str) and package_id.startswith("SPDXRef-DocumentRoot-")
                and isinstance(name, str) and name and package.get("filesAnalyzed") is False):
            # The directory being scanned is not a versioned dependency and
            # cannot contribute to the minimum shared package inventory.
            continue
        if (
            not isinstance(name, str)
            or not name
            or not isinstance(package_version, str)
            or not package_version
            or not isinstance(package_id, str)
            or not package_id.startswith("SPDXRef-")
        ):
            raise ReleaseTrustError("SPDX package identity is incomplete")
        spdx_identities.add((name.casefold(), package_version))
        references = package.get("externalRefs", [])
        if not isinstance(references, list):
            raise ReleaseTrustError("SPDX externalRefs must be a list")
        for reference in references:
            if (
                isinstance(reference, dict)
                and str(reference.get("referenceType", "")).casefold() == "purl"
                and isinstance(reference.get("referenceLocator"), str)
            ):
                spdx_purls.add(reference["referenceLocator"])

    if (product_name, version) not in spdx_identities:
        raise ReleaseTrustError("SPDX does not identify the released Automexia version")
    for prefix in prefixes:
        if not any(purl.startswith(prefix) for purl in spdx_purls):
            raise ReleaseTrustError(f"SPDX has no required {prefix} purl")

    if (
        not isinstance(cyclonedx, dict)
        or cyclonedx.get("bomFormat") != "CycloneDX"
        or not isinstance(cyclonedx.get("specVersion"), str)
        or not str(cyclonedx.get("serialNumber", "")).startswith("urn:uuid:")
        or type(cyclonedx.get("version")) is not int
        or cyclonedx["version"] < 1
        or not isinstance(cyclonedx.get("metadata"), dict)
    ):
        raise ReleaseTrustError(
            "automexia-terminal.cdx.json is not a complete CycloneDX document"
        )
    components = cyclonedx.get("components")
    if not isinstance(components, list) or len(components) < minimum:
        raise ReleaseTrustError(f"CycloneDX components must contain at least {minimum} entries")

    cyclonedx_identities: set[tuple[str, str]] = set()
    cyclonedx_purls: set[str] = set()
    for component in components:
        if not isinstance(component, dict):
            raise ReleaseTrustError("CycloneDX components must be objects")
        name = component.get("name")
        component_version = component.get("version")
        component_type = component.get("type")
        if component_type == "file":
            # Syft files have content hashes and references, not package versions.
            # They do not count toward the required cross-format package graph.
            hashes = component.get("hashes")
            hash_lengths = {"SHA-1": 40, "SHA-256": 64, "SHA-512": 128}
            if (not isinstance(name, str) or not name
                    or not isinstance(component.get("bom-ref"), str) or not component["bom-ref"]
                    or not isinstance(hashes, list) or not hashes
                    or any(not isinstance(item, dict) or item.get("alg") not in hash_lengths
                           or not isinstance(item.get("content"), str)
                           or re.fullmatch(r"[0-9a-fA-F]{" + str(hash_lengths[item["alg"]]) + r"}", item["content"]) is None
                           for item in hashes)
                    or not any(item["alg"] in {"SHA-256", "SHA-512"} for item in hashes)):
                raise ReleaseTrustError("CycloneDX file identity or checksum is incomplete")
            continue
        if (
            not isinstance(name, str)
            or not name
            or not isinstance(component_version, str)
            or not component_version
            or not isinstance(component_type, str)
            or not component_type
        ):
            raise ReleaseTrustError("CycloneDX component identity is incomplete")
        cyclonedx_identities.add((name.casefold(), component_version))
        purl = component.get("purl")
        if isinstance(purl, str):
            cyclonedx_purls.add(purl)

    if (product_name, version) not in cyclonedx_identities:
        raise ReleaseTrustError("CycloneDX does not identify the released Automexia version")
    for prefix in prefixes:
        if not any(purl.startswith(prefix) for purl in cyclonedx_purls):
            raise ReleaseTrustError(f"CycloneDX has no required {prefix} purl")
    if len(spdx_identities & cyclonedx_identities) < minimum:
        raise ReleaseTrustError(
            f"SPDX and CycloneDX must agree on at least {minimum} component identities"
        )


def verify_metadata(
    directory: Path,
    version: str,
    policy: dict[str, Any],
    artifacts: list[tuple[Path, dict[str, Any]]],
    expected_windows_publisher: str,
) -> None:
    rules = metadata_rules(policy)
    parsed: dict[str, Any] = {}
    for name, rule in rules.items():
        path = directory / name
        if rule["format"] == "sha256sums":
            read_bounded(path, rule["max_bytes"])
        else:
            parsed[name] = read_json_bounded(path, rule["max_bytes"])

    expected_manifest = {
        "schema": 1,
        "product": policy["product"],
        "repository": policy["repository"],
        "version": version,
        "artifacts": [
            {
                "kind": rule["id"],
                "name": path.name,
                "sha256": digest(path, policy["chunk_bytes"]),
                "size": path.stat().st_size,
            }
            for path, rule in artifacts
        ],
    }
    if parsed["release-manifest.json"] != expected_manifest:
        raise ReleaseTrustError("release-manifest.json does not describe the final packages")

    benchmark = parsed["release-trust-benchmark.json"]
    expected_bytes = sum(path.stat().st_size for path, _ in artifacts)
    required_benchmark = {
        "schema": 1,
        "operation": "streaming-sha256-release-inventory",
        "artifact_count": len(artifacts),
        "total_bytes": expected_bytes,
        "classification": "informational-release-pipeline-overhead",
    }
    if not isinstance(benchmark, dict) or any(
        benchmark.get(key) != value for key, value in required_benchmark.items()
    ):
        raise ReleaseTrustError("release-trust-benchmark.json has inconsistent totals")
    for key in ("elapsed_milliseconds", "throughput_mib_per_second"):
        value = benchmark.get(key)
        if not isinstance(value, (int, float)) or isinstance(value, bool) or not math.isfinite(value) or value < 0:
            raise ReleaseTrustError(f"release benchmark {key} must be finite and non-negative")

    validate_sboms(
        parsed["automexia-terminal.spdx.json"],
        parsed["automexia-terminal.cdx.json"],
        version,
        policy,
    )

    windows = parsed["release-trust-windows.json"]
    windows_packages = sorted(
        (path for path, _ in artifacts if path.suffix.casefold() in {".msi", ".zip"}),
        key=lambda path: path.name.casefold(),
    )
    windows_package_bytes = sum(path.stat().st_size for path in windows_packages)
    expected_windows = {
        "schema": 1,
        "version": version,
        "scanner": "Microsoft Defender Antivirus",
        "artifact_count": 4,
        "artifact_bytes": windows_package_bytes,
        "artifacts": [
            {
                "name": path.name,
                "size": path.stat().st_size,
                "sha256": digest(path, policy["chunk_bytes"]),
            }
            for path in windows_packages
        ],
        "signature_count": 4,
        "embedded_script_signature_count": 16,
        "publisher": expected_windows_publisher,
        "result": "pass",
    }
    expected_windows_fields = {
        *expected_windows,
        "scanner_version",
        "security_intelligence_version",
        "security_intelligence_updated_utc",
        "scan_milliseconds",
        "scan_timeout_seconds",
    }
    if (
        not isinstance(windows, dict)
        or set(windows) != expected_windows_fields
        or any(windows.get(key) != value for key, value in expected_windows.items())
    ):
        raise ReleaseTrustError("release-trust-windows.json is not exact passing trust evidence")
    for key in (
        "scanner_version",
        "security_intelligence_version",
        "security_intelligence_updated_utc",
    ):
        if not isinstance(windows.get(key), str) or not windows[key].strip():
            raise ReleaseTrustError(f"Windows trust evidence is missing {key}")
    scan_milliseconds = windows.get("scan_milliseconds")
    scan_timeout_seconds = windows.get("scan_timeout_seconds")
    if (
        not isinstance(scan_milliseconds, int)
        or isinstance(scan_milliseconds, bool)
        or scan_milliseconds < 0
    ):
        raise ReleaseTrustError("Windows trust evidence scan_milliseconds must be non-negative")
    if (
        not isinstance(scan_timeout_seconds, int)
        or isinstance(scan_timeout_seconds, bool)
        or not 60 <= scan_timeout_seconds <= 3600
    ):
        raise ReleaseTrustError("Windows trust evidence scan_timeout_seconds is outside 60..3600")
    if scan_milliseconds > scan_timeout_seconds * 1000 + 10_000:
        raise ReleaseTrustError("Windows trust scan duration exceeds its timeout contract")


def verify_checksums(directory: Path, max_bytes: int) -> None:
    checksum_path = directory / "SHA256SUMS"
    try:
        lines = read_bounded(checksum_path, max_bytes).decode("utf-8").splitlines()
    except UnicodeDecodeError as error:
        raise ReleaseTrustError("SHA256SUMS is not valid UTF-8") from error
    expected_names = {path.name for path in directory.iterdir() if path.name != "SHA256SUMS"}
    observed: dict[str, str] = {}
    for line in lines:
        match = re.fullmatch(r"([0-9a-f]{64})  (.+)", line)
        if not match or Path(match.group(2)).name != match.group(2):
            raise ReleaseTrustError(f"invalid SHA256SUMS entry: {line!r}")
        if match.group(2) in observed:
            raise ReleaseTrustError(f"duplicate SHA256SUMS entry: {match.group(2)}")
        observed[match.group(2)] = match.group(1)
    if set(observed) != expected_names:
        raise ReleaseTrustError("SHA256SUMS does not cover every final release asset exactly once")
    for name, expected in observed.items():
        actual = digest(directory / name, 1048576)
        if actual != expected:
            raise ReleaseTrustError(f"checksum mismatch for {name}")


def verify_final(
    directory: Path,
    version: str,
    policy: dict[str, Any],
    expected_windows_publisher: str,
) -> None:
    if (
        not isinstance(expected_windows_publisher, str)
        or not expected_windows_publisher.strip()
    ):
        raise ReleaseTrustError("expected Windows publisher must not be empty")
    rules = metadata_rules(policy)
    expected_metadata = set(rules)
    entries = list(directory.iterdir()) if directory.is_dir() else []
    if any(path.is_symlink() or not path.is_file() for path in entries):
        raise ReleaseTrustError("final release directory must be flat and contain no symlinks")
    package_names = {
        path.name for path in entries if classify(path.name, policy) is not None
    }
    metadata_names = {path.name for path in entries if path.name not in package_names}
    if metadata_names != expected_metadata:
        raise ReleaseTrustError(
            f"final release metadata mismatch: expected {sorted(expected_metadata)}, found {sorted(metadata_names)}"
        )
    for name, rule in rules.items():
        read_bounded(directory / name, rule["max_bytes"])
    with tempfile.TemporaryDirectory(
        prefix=f".{directory.name}-packages-", dir=directory.parent
    ) as temporary_name:
        temporary = Path(temporary_name)
        for name in package_names:
            os.link(directory / name, temporary / name)
        artifacts = package_files(temporary, version, policy)
        verify_metadata(directory, version, policy, artifacts, expected_windows_publisher)
    verify_checksums(directory, rules["SHA256SUMS"]["max_bytes"])


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check-policy", action="store_true")
    parser.add_argument("--artifacts", type=Path)
    parser.add_argument("--version")
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--benchmark", type=Path)
    parser.add_argument("--verify-final", type=Path)
    parser.add_argument("--expected-windows-publisher")
    arguments = parser.parse_args()
    try:
        policy = load_policy()
        if arguments.check_policy:
            if any(
                value is not None
                for value in (
                    arguments.artifacts,
                    arguments.version,
                    arguments.manifest,
                    arguments.benchmark,
                    arguments.verify_final,
                    arguments.expected_windows_publisher,
                )
            ):
                raise ReleaseTrustError("--check-policy cannot be combined with artifact modes")
            print("PASS: release trust policy is structurally valid")
            return 0
        if arguments.verify_final is not None:
            if not arguments.version or not arguments.expected_windows_publisher:
                raise ReleaseTrustError(
                    "--verify-final requires --version and --expected-windows-publisher"
                )
            verify_final(
                arguments.verify_final,
                arguments.version,
                policy,
                arguments.expected_windows_publisher,
            )
            print("PASS: final release assets, metadata allowlist, and checksums are valid")
            return 0
        if arguments.expected_windows_publisher is not None:
            raise ReleaseTrustError(
                "--expected-windows-publisher is valid only with --verify-final"
            )
        if not all(
            value is not None
            for value in (
                arguments.artifacts,
                arguments.version,
                arguments.manifest,
                arguments.benchmark,
            )
        ):
            raise ReleaseTrustError(
                "inventory mode requires --artifacts, --version, --manifest, and --benchmark"
            )
        result = inventory(
            arguments.artifacts,
            arguments.version,
            arguments.manifest,
            arguments.benchmark,
            policy,
        )
        print(
            "PASS: release package allowlist and streaming inventory are valid "
            f"({result['artifact_count']} artifacts, {result['total_bytes']} bytes)"
        )
        return 0
    except (OSError, json.JSONDecodeError, ReleaseTrustError) as error:
        print(f"release trust validation failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
