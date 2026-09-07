#!/usr/bin/env python3
"""Build and verify Automexia's bounded public Linux distribution contract."""

from __future__ import annotations

import argparse
import hashlib
import ipaddress
import json
import math
import os
from pathlib import Path
import re
import shutil
import stat
import sys
from typing import NoReturn
from urllib.parse import quote, unquote, urlsplit

import release_trust


PUBLIC_REPOSITORY = "AmjedAllaya/automexia-releases"
SOURCE_REPOSITORY = "AmjedAllaya/automexia-terminal"
RELEASE_OWNER = "AmjedAllaya"
MAX_RELEASE_EVENT_BYTES = 1024 * 1024
ROOT = Path(__file__).resolve().parents[2]
PUBLIC_WORKFLOW = ROOT / ".github/workflows/linux-early-access.yml"
NFPM_CONFIG = ROOT / "packaging/linux/nfpm.yaml"
MANIFEST_NAME = "public-distribution-manifest-v2.json"
DOCUMENT_POLICY = ROOT / "docs/public-release/reviewed-documents.json"
DOCUMENT_SOURCES = {
    "INSTALL.md": "docs/public-release/INSTALL.md",
    "RELEASE-NOTES.md": "docs/public-release/RELEASE-NOTES.md",
    "THIRD_PARTY_NOTICES.md": "docs/public-release/THIRD_PARTY_NOTICES.md",
    "UNINSTALL.md": "docs/public-release/UNINSTALL.md",
}
MAX_ARTIFACTS = 24
MAX_ARTIFACT_BYTES = 512 * 1024 * 1024
MAX_TOTAL_BYTES = 2 * 1024 * 1024 * 1024
MAX_MANIFEST_BYTES = 1024 * 1024
MAX_RELEASE_JSON_BYTES = 4 * 1024 * 1024
MAX_EVIDENCE_BYTES = 128 * 1024 * 1024
MAX_SBOM_BYTES = 16 * 1024 * 1024
MAX_SBOM_NODES = 250000
MAX_SBOM_DEPTH = 48
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
VERSION_RE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+$")
REPOSITORY_RE = re.compile(
    r"^[A-Za-z0-9](?:[A-Za-z0-9-]{0,38})/[A-Za-z0-9._-]{1,100}$"
)
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
CANDIDATE_SUFFIXES = (
    ".deb",
    ".rpm",
    ".tar.gz",
    ".tgz",
    ".AppImage",
    ".msi",
    ".zip",
    ".dmg",
    ".exe",
    ".dll",
    ".pdb",
    ".dwp",
    ".key",
    ".pem",
    ".p12",
    ".pfx",
)
EVIDENCE_NAMES = frozenset(
    {
        "SHA256SUMS",
        "SHA256SUMS.minisig",
        MANIFEST_NAME,
        "automexia-release-key.pub",
        "INSTALL.md",
        "RELEASE-NOTES.md",
        "THIRD_PARTY_NOTICES.md",
        "UNINSTALL.md",
    }
)


class DistributionError(ValueError):
    """A redacted, actionable public-distribution failure."""


def fail(message: str) -> NoReturn:
    raise DistributionError(message)


def load_release_event(path: Path) -> dict[str, object]:
    """Read only a bounded runner event; never echo event fields or host paths."""
    try:
        _regular_unlinked(path, "release event", MAX_RELEASE_EVENT_BYTES)
        with path.open("rb") as source:
            data = source.read(MAX_RELEASE_EVENT_BYTES + 1)
        if len(data) > MAX_RELEASE_EVENT_BYTES:
            fail("release event exceeds its byte limit")
        payload = json.loads(
            data.decode("utf-8"), object_pairs_hook=_reject_duplicate_keys,
            parse_constant=lambda _: fail("non-finite release event value"),
        )
        if not isinstance(payload, dict):
            fail("release event must be an object")
        pending = [(payload, 0)]
        visited = 0
        while pending:
            value, depth = pending.pop()
            visited += 1
            if depth > 32 or visited > 32768:
                fail("release event structure exceeds its limits")
            if isinstance(value, float) and not math.isfinite(value):
                fail("non-finite release event value")
            if isinstance(value, dict):
                children = value.values()
            elif isinstance(value, list):
                children = value
            else:
                children = ()
            if visited + len(pending) + len(children) > 32768:
                fail("release event structure exceeds its limits")
            pending.extend((child, depth + 1) for child in children)
        return payload
    except (OSError, ValueError, RecursionError):
        raise DistributionError(
            "release event is unavailable, malformed or over limit"
        ) from None


def authorize_owner_release(
    payload: object,
    *,
    event_name: str,
    repository: str,
    actor: str,
    triggering_actor: str,
    commit: str,
    current_main: str,
) -> str:
    """Authorize one owner-merged Linux PR, not an arbitrary manual run."""
    def field(*path: str) -> object:
        value = payload
        for part in path:
            if not isinstance(value, dict):
                return None
            value = value.get(part)
        return value

    if event_name != "pull_request" or repository != SOURCE_REPOSITORY:
        fail("release requires a merged pull request in the pinned source repository")
    if actor != RELEASE_OWNER or triggering_actor != RELEASE_OWNER:
        fail("only the release owner may trigger or rerun publication")
    for role in (("sender",), ("pull_request", "user"), ("pull_request", "merged_by")):
        if field(*role, "login") != RELEASE_OWNER or field(*role, "type") != "User":
            fail("release author, merger and sender must be the pinned human owner")
    for path in (
        ("repository",), ("pull_request", "head", "repo"),
        ("pull_request", "base", "repo"),
    ):
        if field(*path, "full_name") != SOURCE_REPOSITORY:
            fail("release pull request must not cross a repository or fork boundary")
    number = field("pull_request", "number")
    if (
        type(number) is not int or number <= 0 or field("action") != "closed"
        or field("pull_request", "state") != "closed"
        or field("pull_request", "merged") is not True
        or field("pull_request", "base", "ref") != "main"
    ):
        fail("release requires a closed and merged pull request into main")
    # Original and rerun events must still refer to the checked-out current main.
    head = field("pull_request", "head", "sha")
    merge = field("pull_request", "merge_commit_sha")
    for identity in (head, merge, commit, current_main):
        if (
            not isinstance(identity, str)
            or COMMIT_RE.fullmatch(identity) is None
            or identity == "0" * 40
        ):
            fail("release commit identity is invalid")
    if merge != commit or current_main != commit:
        fail("release merge, checkout and current main identities must agree")
    branch = field("pull_request", "head", "ref")
    if not isinstance(branch, str) or len(branch) > 64:
        fail("release branch is invalid")
    version = re.fullmatch(
        r"release/linux/((?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*))",
        branch,
    )
    if version is None:
        fail("release branch must be exactly release/linux/X.Y.Z")
    return version.group(1)


def _package_revision() -> str:
    try:
        source = NFPM_CONFIG.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        raise DistributionError(
            "Linux package revision source is unavailable"
        ) from error
    if len(source.encode("utf-8")) > 64 * 1024:
        fail("Linux package revision source exceeds its byte limit")
    revisions = re.findall(r"(?m)^release:\s*([1-9][0-9]*)\s*$", source)
    if len(revisions) != 1:
        fail("Linux package revision source must define one positive release")
    return revisions[0]


def _artifact_contract(version: str) -> dict[str, dict[str, object]]:
    revision = _package_revision()
    rows = (
        (
            "linux-x64-deb",
            "x64",
            ".deb",
            f"automexia-terminal_{version}-{revision}_amd64.deb",
        ),
        (
            "linux-arm64-deb",
            "Arm64",
            ".deb",
            f"automexia-terminal_{version}-{revision}_arm64.deb",
        ),
        (
            "linux-x64-rpm",
            "x64",
            ".rpm",
            f"automexia-terminal-{version}-{revision}.x86_64.rpm",
        ),
        (
            "linux-arm64-rpm",
            "Arm64",
            ".rpm",
            f"automexia-terminal-{version}-{revision}.aarch64.rpm",
        ),
        (
            "linux-x64-portable",
            "x64",
            ".tar.gz",
            f"automexia-terminal-{version}-x86_64-unknown-linux-gnu.tar.gz",
        ),
        (
            "linux-arm64-portable",
            "Arm64",
            ".tar.gz",
            f"automexia-terminal-{version}-aarch64-unknown-linux-gnu.tar.gz",
        ),
    )
    return {
        identifier: {
            "architecture": architecture,
            "file": filename,
            "format": package_format,
            "id": identifier,
            "platform": "linux",
        }
        for identifier, architecture, package_format, filename in rows
    }


def _evidence_contract() -> dict[str, object]:
    return {
        "checksum": "SHA256SUMS",
        "checksum_signature": "SHA256SUMS.minisig",
        "public_key": "automexia-release-key.pub",
        "documents": _reviewed_documents(),
    }


def _reviewed_documents() -> dict[str, str]:
    policy = _load_json(DOCUMENT_POLICY, 16384)
    if set(policy) != {"schema", "documents"} or type(policy["schema"]) is not int or policy["schema"] != 1:
        fail("reviewed public document policy is invalid")
    documents = policy["documents"]
    if not isinstance(documents, dict) or set(documents) != set(DOCUMENT_SOURCES):
        fail("reviewed public document inventory is invalid")
    for digest in documents.values():
        if not isinstance(digest, str) or SHA256_RE.fullmatch(digest) is None:
            fail("reviewed public document digest is invalid")
    return documents


def validate_public_documents(directory: Path) -> None:
    # A valid signature authenticates bytes; it cannot decide whether prose is public.
    # Only documents whose exact canonical bytes were deliberately reviewed may ship.
    for name, expected in _reviewed_documents().items():
        digest, _ = _digest(directory / name, MAX_MANIFEST_BYTES)
        if digest != expected:
            fail("public document differs from the reviewed publication policy")
        try:
            _sbom_structure((directory / name).read_text(encoding="utf-8"), privacy=True)
        except (DistributionError, UnicodeError):
            raise DistributionError("reviewed public document contains unsafe metadata") from None


def _validate_verification_metadata(directory: Path, manifest: dict) -> None:
    try:
        key = (directory / "automexia-release-key.pub").read_text(encoding="utf-8")
        signature = (directory / "SHA256SUMS.minisig").read_text(encoding="utf-8")
    except (OSError, UnicodeError):
        raise DistributionError("public verification metadata is unreadable") from None
    if re.fullmatch(r"untrusted comment: Automexia release verification key\nRW[A-Za-z0-9+/=]{40,100}\n", key) is None:
        fail("public key metadata is outside the approved template")
    trusted = re.escape(f"trusted comment: Automexia Linux Early Access v{manifest['version']} source {manifest['source_commit']}")
    if re.fullmatch(
        r"untrusted comment: signature from minisign secret key(?: [0-9A-Fa-f]{16})?\n"
        r"[A-Za-z0-9+/=]{80,128}\n" + trusted + r"\n[A-Za-z0-9+/=]{80,128}\n",
        signature,
    ) is None:
        fail("public signature metadata is outside the approved template")


def _validate_repository(repository: str) -> None:
    if repository != PUBLIC_REPOSITORY or REPOSITORY_RE.fullmatch(repository) is None:
        fail("public distribution repository identity is invalid")
    if repository.endswith(".git") or ".." in repository:
        fail("public distribution repository identity is invalid")


def _regular_unlinked(path: Path, label: str, maximum: int) -> os.stat_result:
    try:
        metadata = path.lstat()
    except FileNotFoundError as error:
        raise DistributionError(f"{label} is missing") from error
    if path.is_symlink() or not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
        fail(f"{label} must be a regular unlinked file")
    if metadata.st_size <= 0 or metadata.st_size > maximum:
        fail(f"{label} exceeds its byte limit")
    return metadata


def _digest(path: Path, maximum: int = MAX_ARTIFACT_BYTES) -> tuple[str, int]:
    metadata = _regular_unlinked(path, "public release artifact", maximum)
    hasher = hashlib.sha256()
    size = 0
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            size += len(chunk)
            if size > maximum:
                fail("public release artifact grew beyond its byte limit")
            hasher.update(chunk)
    if size != metadata.st_size:
        fail("public release artifact changed while it was hashed")
    return hasher.hexdigest(), size


def _scan_packages(
    source: Path,
    version: str,
    *,
    max_artifacts: int,
    max_artifact_bytes: int,
    max_total_bytes: int,
) -> dict[str, tuple[Path, str, int]]:
    if source.is_symlink() or not source.is_dir():
        fail("public distribution source must be a real directory")
    considered = sorted(
        path
        for path in source.rglob("*")
        if path.is_symlink()
        or (
            path.is_file()
            and (
                path.name.startswith("automexia-terminal")
                or path.name.endswith(CANDIDATE_SUFFIXES)
            )
        )
    )
    if len(considered) > max_artifacts:
        fail("public distribution artifact count exceeds its limit")
    contract = _artifact_contract(version)
    expected_by_name = {row["file"]: identifier for identifier, row in contract.items()}
    slots: dict[str, list[Path]] = {identifier: [] for identifier in contract}
    unexpected: list[str] = []
    total = 0
    records: dict[str, tuple[Path, str, int]] = {}
    for path in considered:
        if path.name == MANIFEST_NAME:
            continue
        if any(ord(character) < 32 for character in path.name):
            fail("public release filename contains control data")
        identifier = expected_by_name.get(path.name)
        if identifier is None:
            unexpected.append(path.name)
            continue
        digest, size = _digest(path, max_artifact_bytes)
        total += size
        if total > max_total_bytes:
            fail("public distribution total byte limit exceeded")
        slots[identifier].append(path)
        records[identifier] = (path, digest, size)
    incomplete = sorted(identifier for identifier, paths in slots.items() if not paths)
    duplicate = sorted(identifier for identifier, paths in slots.items() if len(paths) != 1)
    if incomplete:
        fail("public distribution package allowlist is incomplete: " + ", ".join(incomplete))
    if duplicate:
        fail(
            "every public distribution package slot must contain exactly one artifact: "
            + ", ".join(duplicate)
        )
    if unexpected:
        fail("unexpected or forbidden public distribution artifacts")
    return records


def _atomic_write(path: Path, text: str, maximum: int = MAX_MANIFEST_BYTES) -> None:
    if len(text.encode("utf-8")) > maximum:
        fail("public distribution output exceeds its byte limit")
    if path.exists() or path.is_symlink():
        _regular_unlinked(path, "public distribution manifest", maximum)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    if temporary.exists() or temporary.is_symlink():
        fail("public distribution temporary manifest already exists")
    created = False
    try:
        with temporary.open("x", encoding="utf-8", newline="\n") as output:
            created = True
            output.write(text)
        os.replace(temporary, path)
    finally:
        if created:
            try:
                temporary.unlink()
            except FileNotFoundError:
                pass


def build_public_distribution(
    source: Path,
    output: Path,
    version: str,
    commit: str,
    repository: str = PUBLIC_REPOSITORY,
    *,
    copy_packages: bool = False,
    max_artifacts: int = MAX_ARTIFACTS,
    max_artifact_bytes: int = MAX_ARTIFACT_BYTES,
    max_total_bytes: int = MAX_TOTAL_BYTES,
) -> dict[str, object]:
    if VERSION_RE.fullmatch(version) is None:
        fail("public distribution version must be stable SemVer X.Y.Z")
    if COMMIT_RE.fullmatch(commit) is None:
        fail("public distribution commit must be a lowercase 40-character Git SHA")
    _validate_repository(repository)
    source_resolved = source.resolve()
    output_resolved = output.resolve(strict=False)
    if copy_packages and (
        source_resolved == output_resolved or source_resolved in output_resolved.parents
    ):
        fail("public distribution output must be outside its package source")

    records = _scan_packages(
        source,
        version,
        max_artifacts=max_artifacts,
        max_artifact_bytes=max_artifact_bytes,
        max_total_bytes=max_total_bytes,
    )
    if output.exists() or output.is_symlink():
        if output.is_symlink() or not output.is_dir():
            fail("public distribution output must be a real directory")
    else:
        output.mkdir(parents=True)
    if copy_packages:
        for path, digest, size in records.values():
            destination = output / path.name
            if destination.exists() or destination.is_symlink():
                if _digest(destination, max_artifact_bytes) != (digest, size):
                    fail("public distribution destination conflicts with source identity")
                continue
            temporary = destination.with_name(f".{destination.name}.{os.getpid()}.tmp")
            try:
                shutil.copy2(path, temporary)
                if _digest(temporary, max_artifact_bytes) != (digest, size):
                    fail("copied public distribution artifact identity changed")
                os.replace(temporary, destination)
            finally:
                try:
                    temporary.unlink()
                except FileNotFoundError:
                    pass
        records = _scan_packages(
            output,
            version,
            max_artifacts=max_artifacts,
            max_artifact_bytes=max_artifact_bytes,
            max_total_bytes=max_total_bytes,
        )

    contract = _artifact_contract(version)
    artifacts = []
    for identifier in sorted(contract):
        path, digest, size = records[identifier]
        row = dict(contract[identifier])
        row.update(
            {
                "download_url": (
                    f"https://github.com/{repository}/releases/download/"
                    f"v{version}/{quote(path.name, safe='')}"
                ),
                "friendly_public_path": f"/download/{identifier}",
                "sha256": digest,
                "size": size,
                "versioned_public_path": f"/download/v{version}/{identifier}",
            }
        )
        artifacts.append(row)
    manifest: dict[str, object] = {
        "schema": 2,
        "product": "automexia-terminal",
        "channel": "linux-early-access",
        "provider": "github-releases",
        "repository": repository,
        "version": version,
        "tag": f"v{version}",
        "source_commit": commit,
        "release_page": f"https://github.com/{repository}/releases/tag/v{version}",
        "evidence": _evidence_contract(),
        "artifacts": artifacts,
    }
    _atomic_write(
        output / MANIFEST_NAME,
        json.dumps(manifest, indent=2, sort_keys=True) + "\n",
    )
    return manifest


def _validate_manifest(manifest: dict[str, object]) -> None:
    required = {
        "schema",
        "product",
        "channel",
        "provider",
        "repository",
        "version",
        "tag",
        "source_commit",
        "release_page",
        "evidence",
        "artifacts",
    }
    if set(manifest) != required:
        fail("public distribution manifest keys drifted")
    version = manifest.get("version")
    repository = manifest.get("repository")
    if (
        type(manifest.get("schema")) is not int
        or manifest.get("schema") != 2
        or manifest.get("product") != "automexia-terminal"
        or manifest.get("channel") != "linux-early-access"
        or manifest.get("provider") != "github-releases"
        or not isinstance(version, str)
        or VERSION_RE.fullmatch(version) is None
        or not isinstance(repository, str)
    ):
        fail("public distribution manifest identity is invalid")
    _validate_repository(repository)
    if manifest.get("tag") != f"v{version}":
        fail("public distribution manifest tag is invalid")
    if manifest.get("release_page") != f"https://github.com/{repository}/releases/tag/v{version}":
        fail("public distribution manifest release page is invalid")
    if manifest.get("evidence") != _evidence_contract():
        fail("public distribution manifest evidence contract is invalid")
    if not isinstance(manifest.get("artifacts"), list) or len(manifest["artifacts"]) != 6:
        fail("public distribution manifest artifact inventory is incomplete")
    if not isinstance(manifest.get("source_commit"), str) or COMMIT_RE.fullmatch(manifest["source_commit"]) is None:
        fail("public distribution source identity is invalid")
    contract = _artifact_contract(version)
    seen: set[str] = set()
    for row in manifest["artifacts"]:
        if not isinstance(row, dict) or set(row) != {
            "architecture", "file", "format", "id", "platform", "download_url",
            "friendly_public_path", "sha256", "size", "versioned_public_path",
        }:
            fail("public distribution artifact fields are invalid")
        identifier = row["id"]
        if not isinstance(identifier, str) or identifier not in contract or identifier in seen:
            fail("public distribution artifact identity is invalid or duplicated")
        seen.add(identifier)
        expected = dict(contract[identifier])
        expected.update(
            download_url=f"https://github.com/{repository}/releases/download/v{version}/{expected['file']}",
            friendly_public_path=f"/download/{identifier}",
            versioned_public_path=f"/download/v{version}/{identifier}",
        )
        if any(row[key] != value for key, value in expected.items()):
            fail("public distribution artifact metadata is outside the reviewed contract")
        if (not isinstance(row["sha256"], str) or SHA256_RE.fullmatch(row["sha256"]) is None
                or type(row["size"]) is not int or not 0 < row["size"] <= MAX_ARTIFACT_BYTES):
            fail("public distribution artifact digest or size is invalid")


def required_release_asset_names(manifest: dict[str, object]) -> frozenset[str]:
    _validate_manifest(manifest)
    names = []
    for artifact in manifest["artifacts"]:
        if not isinstance(artifact, dict) or not isinstance(artifact.get("file"), str):
            fail("public distribution manifest contains an invalid artifact")
        names.append(artifact["file"])
    if len(set(names)) != 6:
        fail("public distribution manifest artifact names are not unique")
    return frozenset(names) | EVIDENCE_NAMES


def verify_release_payload(
    manifest: dict[str, object],
    payload: dict[str, object],
    *,
    release_id: int,
    created_url: str | None = None,
    stage: str = "published",
    bundle: Path | None = None,
) -> None:
    _validate_manifest(manifest)
    local_identities: dict[str, tuple[str, int]] | None = None
    if bundle is not None:
        local_manifest, local_identities = _verify_bundle_with_identities(bundle)
        if local_manifest != manifest:
            fail("verified local bundle and release manifest do not match")
    repository = manifest["repository"]
    version = manifest["version"]
    tag = f"v{version}"
    expected_state = {
        "draft": (True, False),
        "published": (False, True),
    }.get(stage)
    if expected_state is None:
        fail("GitHub release verification stage is invalid")
    expected_draft, expected_immutable = expected_state
    if (type(release_id) is not int or not 0 < release_id < 10**20
            or type(payload.get("id")) is not int or payload["id"] != release_id):
        fail("GitHub release numeric identity is invalid or changed")
    # New drafts have no tag yet. Bind their temporary locator to the URL
    # returned by creation and use the same numeric API identity after publish.
    # Temporary locators never relax the final version-pinned public contract.
    locator = tag
    release_url = f"https://github.com/{repository}/releases/tag/{locator}"
    if stage == "draft":
        if not isinstance(created_url, str) or created_url != payload.get("html_url"):
            fail("GitHub draft URL does not match the newly created release")
        if created_url != release_url:
            temporary = re.fullmatch(
                rf"https://github\.com/{re.escape(repository)}/releases/tag/"
                r"(untagged-[0-9a-f]{20})", created_url,
            )
            if temporary is None:
                fail("GitHub draft locator is invalid")
            locator = temporary.group(1)
            release_url = created_url
    elif created_url is not None:
        fail("published verification must not use a draft URL")
    if (
        payload.get("tag_name") != tag
        or payload.get("draft") is not expected_draft
        or payload.get("prerelease") is not True
        or payload.get("immutable") is not expected_immutable
        or payload.get("html_url") != release_url
    ):
        fail("GitHub release state does not match the immutable Early Access contract")
    assets = payload.get("assets")
    if not isinstance(assets, list) or len(assets) > MAX_ARTIFACTS:
        fail("GitHub release asset inventory is invalid")
    expected_names = required_release_asset_names(manifest)
    by_name: dict[str, dict[str, object]] = {}
    for asset in assets:
        if not isinstance(asset, dict):
            fail("GitHub release asset record is invalid")
        name = asset.get("name")
        if not isinstance(name, str) or name in by_name:
            fail("GitHub release asset names are invalid or duplicated")
        by_name[name] = asset
    if set(by_name) != expected_names:
        fail("GitHub release assets do not match the exact public allowlist")

    package_records = {artifact["file"]: artifact for artifact in manifest["artifacts"]}
    for name, asset in by_name.items():
        size = asset.get("size")
        digest = asset.get("digest")
        expected_url = (
            f"https://github.com/{repository}/releases/download/{locator}/"
            f"{quote(name, safe='')}"
        )
        if (
            asset.get("state") != "uploaded"
            or not isinstance(size, int)
            or size <= 0
            or size > (
                MAX_ARTIFACT_BYTES if name in package_records else MAX_EVIDENCE_BYTES
            )
            or not isinstance(digest, str)
            or not digest.startswith("sha256:")
            or SHA256_RE.fullmatch(digest.removeprefix("sha256:")) is None
            or asset.get("browser_download_url") != expected_url
        ):
            fail("GitHub release asset evidence is invalid")
        if name in package_records and (
            size != package_records[name]["size"]
            or digest != f"sha256:{package_records[name]['sha256']}"
        ):
            fail("GitHub release package digest or size changed after upload")
        if local_identities is not None:
            local_digest, local_size = local_identities[name]
            if size != local_size or digest != f"sha256:{local_digest}":
                fail("GitHub release asset changed after local bundle verification")


def _parse_checksum_manifest(data: bytes, expected: frozenset[str]) -> dict[str, str]:
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError as error:
        raise DistributionError("SHA256SUMS is not UTF-8") from error
    if "\r" in text or "\0" in text or not text.endswith("\n"):
        fail("SHA256SUMS must be canonical LF-terminated text")
    records: dict[str, str] = {}
    for line in text.removesuffix("\n").split("\n"):
        match = re.fullmatch(r"([0-9a-f]{64})  ([A-Za-z0-9][A-Za-z0-9._-]*)", line)
        if match is None or match.group(2) in records:
            fail("SHA256SUMS contains an invalid or duplicate record")
        records[match.group(2)] = match.group(1)
    if frozenset(records) != expected:
        fail("SHA256SUMS does not exactly cover the approved public bundle")
    return records


def _sbom_structure(value: object, *, privacy: bool) -> None:
    pending = [(value, 0)]
    visited = 0
    while pending:
        item, depth = pending.pop()
        visited += 1
        if depth > MAX_SBOM_DEPTH or visited > MAX_SBOM_NODES:
            fail("public SBOM structure exceeds its limits")
        if isinstance(item, dict):
            children = [*item.keys(), *item.values()]
        elif isinstance(item, list):
            children = item
        else:
            children = ()
        if visited + len(pending) + len(children) > MAX_SBOM_NODES:
            fail("public SBOM structure exceeds its limits")
        pending.extend((child, depth + 1) for child in children)
        if isinstance(item, float) and not math.isfinite(item):
            fail("public SBOM contains non-finite data")
        if not isinstance(item, str):
            continue
        if len(item.encode("utf-8")) > 256 * 1024:
            fail("public SBOM string exceeds its limit")
        if not privacy:
            continue
        # Inspect keys as well as values, including percent-encoded file URLs.
        # Unknown sensitive fields fail closed; never rewrite license prose.
        decoded = item
        for _ in range(4):
            expanded = unquote(decoded)
            if expanded == decoded:
                break
            decoded = expanded
        else:
            fail("public SBOM contains excessive URL encoding")
        if re.search(
            r"(?:file://|(?<![A-Za-z0-9])[A-Za-z]:[\\/]|\\\\|"
            r"(?:^|[\s'\"(=:])/(?!/)|(?:^|\s)~/|"
            r"[\x00-\x08\x0b\x0c\x0e-\x1f\x7f\u202a-\u202e\u2066-\u2069])",
            decoded, re.IGNORECASE,
        ):
            fail("public SBOM contains local-path or unsafe metadata")
        for url in re.findall(r"https?://[^\s<>\"]+", decoded):
            try:
                parsed = urlsplit(url)
                if parsed.username is not None or parsed.password is not None:
                    fail("public SBOM contains credential-bearing metadata")
                hostname = parsed.hostname or ""
                try:
                    address = ipaddress.ip_address(hostname)
                except ValueError:
                    address = None
                if ((address is not None and not address.is_global) or not hostname
                        or hostname.casefold().endswith((".localhost", ".local", ".internal"))
                        or (address is None and "." not in hostname)):
                    fail("public SBOM contains private-host metadata")
            except ValueError:
                fail("public SBOM contains malformed URL metadata")


def _load_sbom(path: Path) -> dict[str, object]:
    try:
        payload = _load_json(path, MAX_SBOM_BYTES)
        _sbom_structure(payload, privacy=False)
        return payload
    except (ValueError, OSError, RecursionError):
        raise DistributionError("public SBOM is unavailable, malformed or over limit") from None


def _sbom_input_names(version: str) -> frozenset[str]:
    return frozenset({"Cargo.lock", *("packages/" + str(row["file"]) for row in _artifact_contract(version).values())})


def validate_public_sboms(spdx: dict, cdx: dict, version: str) -> None:
    for payload in (spdx, cdx):
        _sbom_structure(payload, privacy=True)
    try:
        release_trust.validate_sboms(spdx, cdx, version, release_trust.load_policy())
        allowed = _sbom_input_names(version)
        if spdx.get("spdxVersion") != "SPDX-2.3" or cdx.get("specVersion") not in {"1.6", "1.7"}:
            fail("public SBOM format requires explicit compatibility review")
        files = spdx.get("files", [])
        if not isinstance(files, list) or not 1 <= len(files) <= len(allowed):
            fail("public SBOM file inventory is invalid")
        spdx_files: dict[str, str] = {}
        spdx_ids = {"SPDXRef-DOCUMENT"}
        for entry in [*spdx["packages"], *files]:
            identity = entry.get("SPDXID")
            if not isinstance(identity, str) or not identity.startswith("SPDXRef-") or identity in spdx_ids:
                fail("public SBOM has a duplicate or missing SPDX identity")
            spdx_ids.add(identity)
        for file in files:
            if not isinstance(file, dict) or file.get("fileName") not in allowed:
                fail("public SBOM file location is outside the approved scan inputs")
            hashes = [h["checksumValue"] for h in file.get("checksums", []) if h.get("algorithm") == "SHA256"]
            if len(hashes) != 1 or not isinstance(hashes[0], str) or SHA256_RE.fullmatch(hashes[0]) is None:
                fail("public SBOM requires one SHA256 per input file")
            if file["fileName"] in spdx_files:
                fail("public SBOM repeats a file location")
            spdx_files[file["fileName"]] = hashes[0]
        cdx_files: dict[str, str] = {}
        cdx_ids: set[str] = set()
        for component in cdx["components"]:
            identity = component.get("bom-ref")
            if not isinstance(identity, str) or not identity or identity in cdx_ids:
                fail("public SBOM has a duplicate or missing CycloneDX identity")
            cdx_ids.add(identity)
            if component.get("type") != "file":
                continue
            if component.get("name") not in allowed or component["name"] in cdx_files:
                fail("public SBOM file location is invalid or repeated")
            hashes = [h["content"] for h in component["hashes"] if h["alg"] == "SHA-256"]
            if len(hashes) != 1:
                fail("public SBOM requires one SHA256 per input file")
            cdx_files[component["name"]] = hashes[0].lower()
        if spdx_files != cdx_files or "Cargo.lock" not in spdx_files:
            fail("public SBOM formats disagree on scan-input file identities")
        for relation in spdx.get("relationships", []):
            if relation.get("spdxElementId") not in spdx_ids or relation.get("relatedSpdxElement") not in spdx_ids:
                fail("public SBOM has a dangling SPDX relationship")
        for relation in cdx.get("dependencies", []):
            if (relation.get("ref") not in cdx_ids or not isinstance(relation.get("dependsOn"), list)
                    or any(ref not in cdx_ids for ref in relation["dependsOn"])):
                fail("public SBOM has a dangling CycloneDX dependency")
    except (release_trust.ReleaseTrustError, TypeError, KeyError, AttributeError):
        raise DistributionError("public SBOM package or file evidence is invalid") from None


def prepare_public_sboms(source: Path, output: Path, scan_root: Path, version: str) -> None:
    """Project only known location fields; keep the scanner's evidence intact."""
    if (VERSION_RE.fullmatch(version) is None or source.is_symlink() or not source.is_dir()
            or output.is_symlink() or not output.is_dir() or scan_root.is_symlink()
            or not scan_root.is_dir() or source.resolve() == output.resolve()):
        fail("public SBOM preparation needs separate real input and output directories")
    names = ("automexia-terminal.spdx.json", "automexia-terminal.cdx.json")
    spdx, cdx = (_load_sbom(source / name) for name in names)
    prefix = scan_root.resolve().as_posix().rstrip("/") + "/"
    allowed = _sbom_input_names(version)

    def relative_location(value: object, *, virtual: bool = False) -> str:
        if not isinstance(value, str):
            fail("public SBOM location must be text")
        normalized = value.replace("\\", "/")
        comparable = normalized.casefold() if os.name == "nt" else normalized
        expected = prefix.casefold() if os.name == "nt" else prefix
        if comparable.startswith(expected):
            normalized = normalized[len(prefix):]
        elif normalized.startswith("./"):
            normalized = normalized[2:]
        elif virtual and normalized.startswith("/") and normalized[1:] in allowed:
            # Syft package locations are rooted in the virtual scan filesystem,
            # unlike its file components, which carry the host's absolute path.
            normalized = normalized[1:]
        if normalized not in allowed:
            fail("public SBOM location is outside the approved scan inputs")
        return normalized

    try:
        for file in spdx.get("files", []):
            file["fileName"] = relative_location(file.get("fileName"))
        for package in spdx.get("packages", []):
            source_info = package.get("sourceInfo")
            if isinstance(source_info, str):
                match = re.fullmatch(
                    r"(acquired package info from (?:rust cargo manifest|DPKG DB|RPM DB): )(.+)",
                    source_info,
                )
                if match is not None:
                    package["sourceInfo"] = match[1] + relative_location(match[2], virtual=True)
        for component in cdx.get("components", []):
            if component.get("type") == "file":
                component["name"] = relative_location(component.get("name"))
            for prop in component.get("properties", []):
                if re.fullmatch(r"syft:location:[0-9]+:path", prop.get("name", "")):
                    prop["value"] = relative_location(prop.get("value"), virtual=True)
        validate_public_sboms(spdx, cdx, version)
    except (TypeError, AttributeError, KeyError):
        raise DistributionError("public SBOM structure is invalid") from None
    # Both documents pass before either write. Replacement is atomic per file;
    # a write failure aborts the workflow before checksum creation or upload.
    encoded = [json.dumps(payload, indent=2, sort_keys=True) + "\n" for payload in (spdx, cdx)]
    if any(len(text.encode("utf-8")) > MAX_SBOM_BYTES for text in encoded):
        fail("public SBOM output exceeds its byte limit")
    for name, text in zip(names, encoded):
        _atomic_write(output / name, text, MAX_SBOM_BYTES)


def _verify_bundle_with_identities(
    directory: Path,
) -> tuple[dict[str, object], dict[str, tuple[str, int]]]:
    if directory.is_symlink() or not directory.is_dir():
        fail("public distribution bundle must be a real directory")
    manifest = _load_json(directory / MANIFEST_NAME, MAX_MANIFEST_BYTES)
    expected = required_release_asset_names(manifest)
    entries = list(directory.iterdir())
    if len(entries) > MAX_ARTIFACTS:
        fail("public distribution bundle exceeds its file-count limit")
    actual: set[str] = set()
    total = 0
    identities: dict[str, tuple[str, int]] = {}
    for path in entries:
        if path.name in actual:
            fail("public distribution bundle contains duplicate names")
        maximum = MAX_ARTIFACT_BYTES if path.name not in EVIDENCE_NAMES else MAX_MANIFEST_BYTES
        if path.name in {"automexia-release-key.pub", "SHA256SUMS.minisig"}:
            maximum = 4096
        digest, size = _digest(path, maximum)
        actual.add(path.name)
        identities[path.name] = (digest, size)
        total += size
        if total > MAX_TOTAL_BYTES:
            fail("public distribution bundle exceeds its total byte limit")
    if actual != expected:
        fail("public distribution bundle does not match the exact release allowlist")
    covered = expected - {"SHA256SUMS", "SHA256SUMS.minisig"}
    checksums = _parse_checksum_manifest((directory / "SHA256SUMS").read_bytes(), covered)
    for name in covered:
        if checksums[name] != identities[name][0]:
            fail(f"public distribution checksum mismatch for {name}")
    package_records = {item["file"]: item for item in manifest["artifacts"]}
    for name, record in package_records.items():
        digest, size = identities[name]
        if record.get("sha256") != digest or record.get("size") != size:
            fail("public distribution manifest package identity drifted")
    key_lines = (directory / "automexia-release-key.pub").read_text(encoding="utf-8").splitlines()
    if len([line for line in key_lines if re.fullmatch(r"RW[A-Za-z0-9+/=]+", line)]) != 1:
        fail("public distribution key file has no unique minisign public key")
    validate_public_documents(directory)
    _validate_verification_metadata(directory, manifest)
    return manifest, identities


def verify_bundle(directory: Path) -> dict[str, object]:
    return _verify_bundle_with_identities(directory)[0]


def write_activation_handoff(manifest_path: Path, output: Path) -> dict[str, object]:
    manifest = _load_json(manifest_path, MAX_MANIFEST_BYTES)
    _validate_manifest(manifest)
    digest, _ = _digest(manifest_path, MAX_MANIFEST_BYTES)
    handoff = {
        "schemaVersion": 1,
        "status": "verified",
        "repository": manifest["repository"],
        "releaseTag": manifest["tag"],
        "sourceCommit": manifest["source_commit"],
        "manifestSha256": digest,
    }
    _atomic_write(output, json.dumps(handoff, indent=2, sort_keys=True) + "\n")
    return handoff


def ruleset_bypass_query() -> str:
    owner, name = PUBLIC_REPOSITORY.split("/")
    return (
        '{repository(owner:"' + owner + '",name:"' + name + '"){'
        'nameWithOwner rulesets(first:100){pageInfo{hasNextPage} nodes{'
        'id databaseId name enforcement bypassActors(first:1){'
        'totalCount nodes{bypassMode} pageInfo{hasNextPage}}}}}}'
    )


def _validate_ruleset_bypass_evidence(
    evidence: object, rulesets: tuple[dict[str, object], ...]
) -> None:
    # REST intentionally omits bypass_actors without ruleset-write permission.
    # GraphQL exposes the count even when individual actor identities are null;
    # require three agreeing empty signals, never interpret redaction as zero.
    if not isinstance(evidence, dict) or "errors" in evidence:
        fail("ruleset bypass evidence is unavailable or contains GraphQL errors")
    data = evidence.get("data")
    repository = data.get("repository") if isinstance(data, dict) else None
    if not isinstance(repository, dict) or repository.get("nameWithOwner") != PUBLIC_REPOSITORY:
        fail("ruleset bypass evidence has the wrong repository identity")
    def complete_page(page: object) -> bool:
        return (isinstance(page, dict) and set(page) == {"hasNextPage"}
                and page["hasNextPage"] is False)

    connection = repository.get("rulesets")
    if not isinstance(connection, dict) or not complete_page(connection.get("pageInfo")):
        fail("ruleset bypass evidence is incomplete")
    nodes = connection.get("nodes")
    if not isinstance(nodes, list) or not 2 <= len(nodes) <= 100:
        fail("ruleset bypass inventory exceeds its bounds or is incomplete")
    by_id: dict[int, dict[str, object]] = {}
    seen_nodes: set[str] = set()
    for node in nodes:
        if not isinstance(node, dict):
            fail("ruleset bypass inventory contains a redacted rule")
        identity, node_id = node.get("databaseId"), node.get("id")
        if (type(identity) is not int or identity <= 0 or identity in by_id
                or not isinstance(node_id, str) or not 1 <= len(node_id) <= 128
                or node_id in seen_nodes):
            fail("ruleset bypass inventory contains invalid or duplicate identities")
        by_id[identity] = node
        seen_nodes.add(node_id)
    for ruleset in rulesets:
        identity = ruleset.get("id")
        if type(identity) is not int:
            fail("REST ruleset identity is unavailable for independent verification")
        node = by_id.get(identity)
        if (node is None or node.get("id") != ruleset.get("node_id")
                or node.get("name") != ruleset.get("name")
                or node.get("enforcement") != "ACTIVE"):
            fail("REST and GraphQL ruleset identities or enforcement disagree")
        actors = node.get("bypassActors")
        if (not isinstance(actors, dict) or type(actors.get("totalCount")) is not int
                or actors.get("totalCount") != 0 or actors.get("nodes") != []
                or not complete_page(actors.get("pageInfo"))):
            fail("ruleset bypass policy is nonempty, redacted, or incomplete")


def validate_public_repository_governance(
    repository: dict[str, object],
    immutable: dict[str, object],
    main_ruleset: dict[str, object],
    tag_ruleset: dict[str, object],
    *,
    bypass_evidence: object = None,
) -> None:
    expected_repository = {
        "visibility": "public",
        "has_issues": False,
        "has_projects": False,
        "has_wiki": False,
        "allow_squash_merge": True,
        "allow_merge_commit": False,
        "allow_rebase_merge": False,
    }
    for field, expected in expected_repository.items():
        if repository.get(field) != expected:
            fail(f"public release repository has unsafe {field} configuration")
    if immutable.get("enabled") is not True:
        fail("public release repository must enforce immutable releases")

    if bypass_evidence is not None:
        _validate_ruleset_bypass_evidence(bypass_evidence, (main_ruleset, tag_ruleset))

    def rules_by_type(
        ruleset: dict[str, object], name: str, target: str, include: list[str]
    ) -> dict[str, dict[str, object]]:
        if (
            ruleset.get("name") != name
            or ruleset.get("target") != target
            or ruleset.get("enforcement") != "active"
            or ("bypass_actors" in ruleset and ruleset["bypass_actors"] != [])
            or ("bypass_actors" not in ruleset and bypass_evidence is None)
        ):
            fail(f"{name} ruleset identity, enforcement, or bypass policy drifted")
        conditions = ruleset.get("conditions")
        ref_name = conditions.get("ref_name") if isinstance(conditions, dict) else None
        if not isinstance(ref_name, dict) or ref_name.get("include") != include or ref_name.get("exclude") != []:
            fail(f"{name} ruleset reference scope drifted")
        raw_rules = ruleset.get("rules")
        if not isinstance(raw_rules, list):
            fail(f"{name} ruleset has no rule list")
        indexed: dict[str, dict[str, object]] = {}
        for rule in raw_rules:
            if not isinstance(rule, dict) or not isinstance(rule.get("type"), str):
                fail(f"{name} ruleset contains an invalid rule")
            rule_type = rule["type"]
            if rule_type in indexed:
                fail(f"{name} ruleset repeats {rule_type}")
            indexed[rule_type] = rule
        return indexed

    main_rules = rules_by_type(main_ruleset, "Protect main", "branch", ["~DEFAULT_BRANCH"])
    for required in (
        "deletion",
        "non_fast_forward",
        "required_linear_history",
        "required_signatures",
        "pull_request",
    ):
        if required not in main_rules:
            fail(f"Protect main ruleset is missing {required}")
    review = main_rules["pull_request"].get("parameters")
    if not isinstance(review, dict):
        fail("Protect main pull-request rule has no parameters")
    approvals = review.get("required_approving_review_count")
    if not isinstance(approvals, int) or isinstance(approvals, bool) or approvals < 1:
        fail("Protect main must require at least one approval")
    for flag in (
        "dismiss_stale_reviews_on_push",
        "require_code_owner_review",
        "require_last_push_approval",
        "required_review_thread_resolution",
    ):
        if review.get(flag) is not True:
            fail(f"Protect main pull-request rule must enable {flag}")
    if review.get("allowed_merge_methods") != ["squash"]:
        fail("Protect main must allow squash merging only")

    tag_rules = rules_by_type(tag_ruleset, "Protect release tags", "tag", ["refs/tags/v*"])
    for required in ("deletion", "update", "non_fast_forward"):
        if required not in tag_rules:
            fail(f"Protect release tags ruleset is missing {required}")


def validate_workflow(path: Path = PUBLIC_WORKFLOW) -> None:
    try:
        workflow = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        raise DistributionError("public Linux release workflow is unavailable") from error
    if len(workflow.encode("utf-8")) > 256 * 1024:
        fail("public Linux release workflow exceeds its byte limit")
    required = {
        "manual rehearsal trigger": "workflow_dispatch:",
        "rehearsal-only dispatch mode": "publish=false",
        "internal merged PR": "github.event.pull_request.head.repo.full_name == github.repository",
        "owner release authorization": "tools/ci/public_distribution.py authorize-release",
        "manifest builder": "tools/ci/public_distribution.py build",
        "bundle verifier": "tools/ci/public_distribution.py verify-bundle",
        "draft verifier": "--stage draft",
        "published verifier": "--stage published",
        "full upload verification": "--bundle release-bundle",
        "immutable repository audit": "repos/$PUBLIC_REPOSITORY/immutable-releases",
        "repository governance audit": "tools/ci/public_distribution.py verify-repository",
        "protected metadata branch": "Protect main",
        "protected release tags": "Protect release tags",
        "create-once publication": "gh release create",
        "no replacement": "A release or tag already exists in the public repository",
        "scoped repository": "repositories: automexia-releases",
        "scoped contents permission": "permission-contents: write",
        "scoped administration permission": "permission-administration: read",
        "short-lived GitHub App": "actions/create-github-app-token@bcd2ba49218906704ab6c1aa796996da409d3eb1",
        "activation handoff": "website-activation.json",
        "rehearsal warning": "REHEARSAL-NOT-A-PUBLIC-RELEASE.txt",
        "current GitHub API contract": "X-GitHub-Api-Version: 2026-03-10",
        "release attestation verification": 'gh release verify "$tag"',
        "asset attestation verification": 'gh release verify-asset "$tag" "$asset"',
        "release signature": "minisign -S -W",
        "signature verification": "minisign -V -P",
        "release quality source cache": "actions/cache@55cc8345863c7cc4c66a329aec7e433d2d1c52a9",
        "reviewed compiler cache": (
            "mozilla-actions/sccache-action@"
            "fc920bf0ec8de6ee65d409111f7ec508035751ba"
        ),
        "pinned compiler cache release": "version: v0.16.0",
        "release quality lint cleanup": (
            "Reclaim release lint artifacts before the all-feature test build"
        ),
    }
    for label, token in required.items():
        if token not in workflow:
            fail(f"public Linux release workflow is missing {label}")
    if re.search(r"(?m)^\s*environment\s*:", workflow):
        fail("public Linux release workflow must not require paid private environments")
    if "--clobber" in workflow or workflow.count("permission-contents: write") != 1:
        fail("public Linux release workflow weakens create-once least privilege")
    if workflow.count("--bundle release-bundle") != 2:
        fail("draft and published release checks must verify every uploaded bundle file")
    if workflow.count("AUTOMEXIA_DISTRIBUTION_APP_PRIVATE_KEY") != 1:
        fail("public Linux release workflow must expose the App private key to one step only")

    def job_body(name: str) -> str:
        match = re.search(
            rf"(?ms)^  {re.escape(name)}:\n(?P<body>.*?)(?=^  [A-Za-z0-9_-]+:\n|\Z)",
            workflow,
        )
        if match is None:
            fail(f"public Linux release workflow is missing {name} job")
        return match.group("body")

    authorize = job_body("authorize")
    quality = job_body("quality")
    package = job_body("package")
    rehearsal = job_body("rehearsal")
    assemble = job_body("assemble")
    publish = job_body("publish")
    # Keep scanner output private until preparation passes, before any signing
    # secret is exposed. Action defaults must not upload raw metadata elsewhere.
    for format_name, suffix in (("spdx-json", "spdx"), ("cyclonedx-json", "cdx")):
        expected = (
            f"          format: {format_name}\n"
            "          syft-version: v1.51.1\n"
            f"          output-file: sbom-private/automexia-terminal.{suffix}.json\n"
            "          upload-artifact: false\n"
            "          upload-release-assets: false\n"
            "          dependency-snapshot: false\n"
        )
        if assemble.count(expected) != 1:
            fail("public SBOM scanner output must stay private with uploads disabled")
    preparation = ('python3 tools/ci/public_distribution.py prepare-sboms --source sbom-private '
                   '--output sbom-reviewed --scan-root sbom-input --version "$RELEASE_VERSION"')
    preparation_block = (
        '      - name: Validate private build inventories\n'
        '        shell: bash\n'
        '        env:\n'
        '          RELEASE_VERSION: ${{ needs.authorize.outputs.version }}\n'
        '        run: |\n'
        '          set -euo pipefail\n'
        f'          {preparation}\n'
        '          python3 tools/ci/public_distribution.py verify-documents --directory release-bundle\n'
    )
    if (assemble.count(preparation_block) != 1 or assemble.count(preparation) != 1
            or not 0 <= assemble.find("format: cyclonedx-json") < assemble.find(preparation)
            < assemble.find("- name: Create and verify the detached checksum signature")):
        fail("public SBOM privacy verification must precede signing and upload")
    if ("      - name: Remove private scanner scratch\n        if: always()\n" not in assemble
            or assemble.count("rm -rf -- sbom-private sbom-input sbom-reviewed") != 1):
        fail("private SBOM scanner scratch must have unconditional bounded cleanup")
    private_retention = (
        '      - name: Retain inventories only in the private source repository\n'
        '        if: github.event.repository.private == true\n'
        '        uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a\n'
        '        with:\n'
        '          name: private-build-inventories\n'
        '          path: sbom-reviewed\n'
        '          if-no-files-found: error\n'
        '          retention-days: 7\n'
        '          compression-level: 0\n'
    )
    if assemble.count(private_retention) != 1:
        fail("build inventory retention must be bounded and private")
    if ('          cp docs/public-release/THIRD_PARTY_NOTICES.md release-bundle/THIRD_PARTY_NOTICES.md' not in assemble
            or '          test "${#assets[@]}" -eq 14' not in publish):
        fail("minimal publication documents and asset count must match the reviewed contract")
    if '          rm -rf -- sbom-private sbom-input sbom-reviewed\n          compression-level:' in assemble:
        fail("artifact compression options must not execute as cleanup commands")
    dispatch = re.search(
        r"(?ms)if \[\[ \"\$EVENT_NAME\" == 'workflow_dispatch' \]\]; then"
        r"(?P<rehearsal>.*?)^\s*else\s*$"
        r"(?P<release>.*?)^\s*fi\s*$",
        authorize,
    )
    if dispatch is None:
        fail("authorize job must separate manual rehearsal from public release")
    if "publish=false" not in dispatch.group("rehearsal"):
        fail("manual dispatch must select non-public rehearsal mode")
    if "publish=true" not in dispatch.group("release"):
        fail("only the reviewed merge path may select public release mode")
    owner_lines = [line.strip() for line in dispatch.group("release").strip().splitlines()]
    expected_owner_lines = [
        'current_main="$(gh api "repos/$GITHUB_REPOSITORY/git/ref/heads/main" --jq .object.sha)"',
        'version="$(python3 tools/ci/public_distribution.py authorize-release \\',
        '--event "$GITHUB_EVENT_PATH" --event-name "$EVENT_NAME" \\',
        '--repository "$GITHUB_REPOSITORY" --actor "$GITHUB_ACTOR" \\',
        '--triggering-actor "$GITHUB_TRIGGERING_ACTOR" \\',
        '--commit "$EVENT_COMMIT" --current-main "$current_main")"',
        'publish=true',
    ]
    if (
        owner_lines != expected_owner_lines
        or 'set -euo pipefail' not in authorize
        or 'set +' in authorize
        or authorize.count('publish=true') != 1
        or 'continue-on-error:' in authorize
    ):
        fail("owner authorization must succeed with exact runner contexts before publication")
    resource_contract = {
        "single build job": "CARGO_BUILD_JOBS: '1'",
        "disabled development debug info": "CARGO_PROFILE_DEV_DEBUG: '0'",
        "disabled test debug info": "CARGO_PROFILE_TEST_DEBUG: '0'",
        "single test thread": "NEXTEST_TEST_THREADS: '1'",
        "compiler wrapper": "RUSTC_WRAPPER: sccache",
        "GitHub cache backend": "SCCACHE_GHA_ENABLED: 'true'",
        "versioned compiler cache": "SCCACHE_GHA_VERSION: automexia-rust-1.96.1-v2",
    }
    for label, token in resource_contract.items():
        if quality.count(token) != 1:
            fail(f"release quality job must enforce {label}")
    package_resource_contract = {
        "single build job": "CARGO_BUILD_JOBS: '1'",
        "disabled release debug info": "CARGO_PROFILE_RELEASE_DEBUG: '0'",
    }
    for label, token in package_resource_contract.items():
        if package.count(token) != 1:
            fail(f"native package job must enforce {label}")
    source_cache_contract = {
        "Cargo registry source cache": "~/.cargo/registry",
        "Cargo Git source cache": "~/.cargo/git",
        "Cargo.lock cache identity": "hashFiles('Cargo.lock')",
        "versioned shared cache key": (
            "cargo-sources-v1-${{ runner.os }}-${{ hashFiles('Cargo.lock') }}"
        ),
    }
    for job_name, body in (("release quality", quality), ("native package", package)):
        for label, token in source_cache_contract.items():
            if body.count(token) != 1:
                fail(f"{job_name} job must enforce {label}")
        if body.count(
            "actions/cache@55cc8345863c7cc4c66a329aec7e433d2d1c52a9"
        ) != 1:
            fail(f"{job_name} job must use the reviewed Cargo source cache Action")
        if re.search(r"(?m)^\s+target(?:/.*)?\s*$", body):
            fail(f"{job_name} cache must not retain target build artifacts")
    compiler_cache_contract = {
        "reviewed Action": (
            "mozilla-actions/sccache-action@"
            "fc920bf0ec8de6ee65d409111f7ec508035751ba"
        ),
        "pinned release": "version: v0.16.0",
        "step identity": "id: sccache",
        "statistics": "run: sccache --show-stats",
    }
    for label, token in compiler_cache_contract.items():
        if quality.count(token) != 1:
            fail(f"release quality compiler cache must enforce {label}")
    if "sccache" in package.casefold() or "RUSTC_WRAPPER" in package:
        fail("native package builds must stay cold and reject compiler cache inputs")
    clippy = quality.find(
        "cargo clippy --workspace --all-targets --all-features --locked -- -D warnings"
    )
    compiler_cache = quality.find(
        "mozilla-actions/sccache-action@"
        "fc920bf0ec8de6ee65d409111f7ec508035751ba"
    )
    clean = quality.find("run: cargo clean")
    nextest = quality.find(
        "cargo nextest run --workspace --all-features --locked --profile ci"
    )
    if quality.count("run: cargo clean") != 1 or not (
        0 <= compiler_cache < clippy < clean < nextest
    ):
        fail(
            "release quality job must clean lint artifacts between Clippy and "
            "the all-feature test build"
        )
    if quality.find("run: sccache --show-stats") < nextest:
        fail("release quality compiler-cache statistics must follow the build gates")
    if re.search(r"(?m)^    needs: authorize$", package) is None:
        fail("native package builds must start beside quality after authorization")
    joined_dependencies = "needs: [authorize, quality, package]"
    for name, body in (("rehearsal", rehearsal), ("assemble", assemble)):
        if body.count(joined_dependencies) != 1:
            fail(f"{name} must join successful quality and native package evidence")
    nfpm_contract = {
        "pinned nFPM release": "NFPM_VERSION: '2.43.4'",
        "x64 nFPM archive mapping": (
            "runner_arch: X64\n"
            "            nfpm_arch: x86_64\n"
            "            nfpm_sha256: "
            "cafb544650cb0305d1b164fc0ab261eb77a81af324e18011282d326b326d20fb"
        ),
        "Arm64 nFPM archive mapping": (
            "runner_arch: ARM64\n"
            "            nfpm_arch: arm64\n"
            "            nfpm_sha256: "
            "e4365707dedfda6e089f597dcdab9497beea80accb2c2704be18981e4a4d9b9b"
        ),
        "native nFPM archive": (
            "nfpm_${NFPM_VERSION}_Linux_${{ matrix.nfpm_arch }}.tar.gz"
        ),
        "upstream nFPM release URL": (
            "https://github.com/goreleaser/nfpm/releases/download/"
            "v${NFPM_VERSION}/$nfpm_archive"
        ),
        "bounded HTTPS nFPM download": (
            "curl --fail --location --proto '=https' --tlsv1.2 --retry 3 "
            "--retry-all-errors"
        ),
        "nFPM checksum verification": "sha256sum --check",
        "single-member nFPM extraction": (
            'tar --extract --gzip --file "$nfpm_path" '
            '--directory "$RUNNER_TEMP/nfpm-bin" nfpm'
        ),
    }
    for label, token in nfpm_contract.items():
        if package.count(token) != 1:
            fail(f"native package job must enforce {label}")
    if "go install github.com/goreleaser/nfpm" in package:
        fail("native package job must not compile nFPM from source")
    download = package.find("curl --fail --location")
    checksum = package.find("sha256sum --check")
    extraction = package.find("tar --extract --gzip")
    version_smoke = package.find('"$RUNNER_TEMP/nfpm-bin/nfpm" --version')
    if not 0 <= download < checksum < extraction < version_smoke:
        fail("native package job must verify nFPM before extraction and use")
    if "if: needs.authorize.outputs.publish == 'false'" not in rehearsal:
        fail("credential-free rehearsal must be restricted to non-public dispatches")
    for name, body in (("assemble", assemble), ("publish", publish)):
        if "if: needs.authorize.outputs.publish == 'true'" not in body:
            fail(f"{name} job must be restricted to an authorized public release")
    if "X-GitHub-Api-Version: 2026-03-10" not in publish:
        fail("publication must use the current pinned GitHub API contract")
    bypass_fetch = (
        'timeout 30s gh api graphql -f query="$(python3 tools/ci/public_distribution.py '
        'ruleset-bypass-query)" > "$governance/ruleset-bypass.json"'
    )
    bypass_argument = '--ruleset-bypass-json "$governance/ruleset-bypass.json"'
    bypass_block = "\n" + "\n".join((
        bypass_fetch,
        'python3 tools/ci/public_distribution.py verify-repository \\',
        '--repository-json "$governance/repository.json" \\',
        '--immutable-json "$governance/immutable.json" \\',
        '--main-ruleset-json "$governance/main-ruleset.json" \\',
        '--tag-ruleset-json "$governance/tag-ruleset.json" \\',
        bypass_argument,
        '',
    ))
    normalized_publish = "\n" + "\n".join(line.strip() for line in publish.splitlines()) + "\n"
    if (publish.count(bypass_fetch) != 1 or publish.count(bypass_argument) != 1
            or normalized_publish.count(bypass_block) != 1
            or not publish.find(bypass_fetch) < publish.find(bypass_argument) < publish.find('gh release create')
            or 'permission-administration: write' in publish or 'set +' in publish
            or 'continue-on-error:' in publish):
        fail("publication requires independent read-only bypass evidence before creation")
    for forbidden in (
        "secrets.",
        "create-github-app-token",
        "gh release create",
        "minisign -S",
        "activation-handoff",
    ):
        if forbidden in rehearsal:
            fail(f"credential-free rehearsal unexpectedly contains {forbidden}")
    # The tag endpoint describes published releases, not newly created drafts.
    # Bind creation, lookup, verification and the sole publication write as one
    # fail-fast block; mutations must not skip or reorder any authority check.
    draft_publication = (
        'created_release_url="$(gh release create "$tag" "${assets[@]}" --repo "$PUBLIC_REPOSITORY" --draft --prerelease --title "Automexia Terminal $tag — Linux Early Access" --notes-file release-bundle/RELEASE-NOTES.md)"',
        'release_id="$(gh release view "$tag" --repo "$PUBLIC_REPOSITORY" --json databaseId --jq \'.databaseId\')"',
        '[[ "$release_id" =~ ^[1-9][0-9]{0,19}$ ]]',
        'gh api -H "$api_header" "repos/$PUBLIC_REPOSITORY/releases/$release_id" > "$RUNNER_TEMP/release-draft.json"',
        'python3 tools/ci/public_distribution.py verify-release --manifest release-bundle/public-distribution-manifest-v2.json --release-json "$RUNNER_TEMP/release-draft.json" --release-id "$release_id" --created-url "$created_release_url" --bundle release-bundle --stage draft',
        'gh api -H "$api_header" --method PATCH "repos/$PUBLIC_REPOSITORY/releases/$release_id" -F draft=false -F prerelease=true -f make_latest=false >/dev/null',
        'gh api -H "$api_header" "repos/$PUBLIC_REPOSITORY/releases/$release_id" > "$RUNNER_TEMP/release-published.json"',
        'python3 tools/ci/public_distribution.py verify-release --manifest release-bundle/public-distribution-manifest-v2.json --release-json "$RUNNER_TEMP/release-published.json" --release-id "$release_id" --bundle release-bundle --stage published',
    )
    if (normalized_publish.count("\n" + "\n".join(draft_publication) + "\n") != 1
            or any(publish.count(command) != 1 for command in draft_publication)
            or publish.count('gh release create') != 1
            or publish.count('--method PATCH') != 1):
        fail("draft creation, exact release identity and verified publication order are required")
    published = workflow.find("--stage published")
    release_attestation = workflow.find('gh release verify "$tag"')
    asset_attestation = workflow.find('gh release verify-asset "$tag" "$asset"')
    activation = workflow.find("activation-handoff")
    if not published < release_attestation < asset_attestation < activation:
        fail("release and asset attestations must be verified before website activation")
    if re.search(
        r'(?ms)for asset in "\$\{assets\[@\]\}"; do\s+'
        r'gh release verify-asset "\$tag" "\$asset" '
        r'--repo "\$PUBLIC_REPOSITORY" --format json >/dev/null\s+done',
        publish,
    ) is None:
        fail("every uploaded release asset must receive attestation verification")


def _reject_duplicate_keys(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            fail("public distribution JSON contains duplicate keys")
        result[key] = value
    return result


def _load_json(path: Path, maximum: int) -> dict[str, object]:
    metadata = _regular_unlinked(path, "public distribution JSON", maximum)
    if metadata.st_size > maximum:
        fail("public distribution JSON exceeds its byte limit")
    try:
        with path.open("rb") as source:
            opened = os.fstat(source.fileno())
            if (opened.st_dev, opened.st_ino, opened.st_nlink) != (metadata.st_dev, metadata.st_ino, 1):
                fail("public distribution JSON changed before reading")
            data = source.read(maximum + 1)
        final = path.lstat()
        if (len(data) != metadata.st_size or len(data) > maximum
                or (metadata.st_dev, metadata.st_ino, metadata.st_size, metadata.st_mtime_ns, metadata.st_ctime_ns)
                != (final.st_dev, final.st_ino, final.st_size, final.st_mtime_ns, final.st_ctime_ns)):
            fail("public distribution JSON changed while reading")
        loaded = json.loads(data.decode("utf-8"), object_pairs_hook=_reject_duplicate_keys,
                            parse_constant=lambda _: fail("public distribution JSON contains non-finite data"))
    except (UnicodeDecodeError, json.JSONDecodeError, RecursionError):
        raise DistributionError("public distribution JSON is invalid") from None
    if not isinstance(loaded, dict):
        fail("public distribution JSON root must be an object")
    return loaded


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    authorize = commands.add_parser("authorize-release")
    authorize.add_argument("--event", type=Path, required=True)
    for name in ("event-name", "repository", "actor", "triggering-actor", "commit", "current-main"):
        authorize.add_argument(f"--{name}", required=True)
    build = commands.add_parser("build")
    build.add_argument("--source", type=Path, required=True)
    build.add_argument("--output", type=Path, required=True)
    build.add_argument("--version", required=True)
    build.add_argument("--commit", required=True)
    build.add_argument("--repository", default=PUBLIC_REPOSITORY)
    build.add_argument("--copy-packages", action="store_true")
    verify = commands.add_parser("verify-release")
    verify.add_argument("--manifest", type=Path, required=True)
    verify.add_argument("--release-id", type=int, required=True)
    verify.add_argument("--created-url")
    verify.add_argument("--release-json", type=Path, required=True)
    verify.add_argument("--stage", choices=("draft", "published"), default="published")
    verify.add_argument("--bundle", type=Path, required=True)
    governance = commands.add_parser("verify-repository")
    governance.add_argument("--repository-json", type=Path, required=True)
    governance.add_argument("--immutable-json", type=Path, required=True)
    governance.add_argument("--main-ruleset-json", type=Path, required=True)
    governance.add_argument("--tag-ruleset-json", type=Path, required=True)
    governance.add_argument("--ruleset-bypass-json", type=Path, required=True)
    commands.add_parser("ruleset-bypass-query")
    documents = commands.add_parser("verify-documents")
    documents.add_argument("--directory", type=Path, required=True)
    sboms = commands.add_parser("prepare-sboms")
    sboms.add_argument("--source", type=Path, required=True)
    sboms.add_argument("--output", type=Path, required=True)
    sboms.add_argument("--scan-root", type=Path, required=True)
    sboms.add_argument("--version", required=True)
    bundle = commands.add_parser("verify-bundle")
    bundle.add_argument("--directory", type=Path, required=True)
    activation = commands.add_parser("activation-handoff")
    activation.add_argument("--manifest", type=Path, required=True)
    activation.add_argument("--output", type=Path, required=True)
    commands.add_parser("check-policy")
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    try:
        if args.command == "authorize-release":
            version = authorize_owner_release(
                load_release_event(args.event), event_name=args.event_name,
                repository=args.repository, actor=args.actor,
                triggering_actor=args.triggering_actor, commit=args.commit,
                current_main=args.current_main,
            )
            print(version)
        elif args.command == "build":
            manifest = build_public_distribution(
                args.source,
                args.output,
                args.version,
                args.commit,
                args.repository,
                copy_packages=args.copy_packages,
            )
            print(
                f"Built {manifest['channel']} manifest with "
                f"{len(manifest['artifacts'])} exact packages"
            )
        elif args.command == "verify-release":
            manifest = _load_json(args.manifest, MAX_MANIFEST_BYTES)
            payload = _load_json(args.release_json, MAX_RELEASE_JSON_BYTES)
            verify_release_payload(
                manifest, payload, stage=args.stage, bundle=args.bundle,
                release_id=args.release_id, created_url=args.created_url,
            )
            print(f"Verified GitHub Early Access {args.stage} release evidence")
        elif args.command == "verify-documents":
            validate_public_documents(args.directory)
            print("Verified reviewed public documents")
        elif args.command == "prepare-sboms":
            prepare_public_sboms(args.source, args.output, args.scan_root, args.version)
            print("Validated private build inventories")
        elif args.command == "verify-bundle":
            manifest = verify_bundle(args.directory)
            print(f"Verified exact public bundle for v{manifest['version']}")
        elif args.command == "verify-repository":
            validate_public_repository_governance(
                _load_json(args.repository_json, MAX_RELEASE_JSON_BYTES),
                _load_json(args.immutable_json, MAX_RELEASE_JSON_BYTES),
                _load_json(args.main_ruleset_json, MAX_RELEASE_JSON_BYTES),
                _load_json(args.tag_ruleset_json, MAX_RELEASE_JSON_BYTES),
                bypass_evidence=load_release_event(args.ruleset_bypass_json),
            )
            print("Public release repository governance passed")
        elif args.command == "ruleset-bypass-query":
            print(ruleset_bypass_query())
        elif args.command == "activation-handoff":
            handoff = write_activation_handoff(args.manifest, args.output)
            print(f"Prepared website activation handoff for {handoff['releaseTag']}")
        else:
            validate_workflow()
            validate_public_documents(ROOT / "docs/public-release")
            print("Public Linux release workflow policy passed")
    except (OSError, UnicodeError):
        print("public distribution gate failed: input or output is unavailable", file=sys.stderr)
        return 1
    except DistributionError as error:
        print(f"public distribution gate failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
