#!/usr/bin/env python3
"""Validate the documentation boundary, including the retired research pack."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import sys
from typing import Any, Iterator

from check_documentation_hygiene import (
    DocumentationHygieneError,
    validate_markdown_payload,
)


ROOT = Path(__file__).resolve().parents[2]
PACK_PATH = "automexia_docs_repository_aligned"
PRIVATE_PATH = ".automexia-private"
POLICY_PATH = Path("docs/PRIVATE-DOCUMENTATION-POLICY.md")
MANIFEST_NAME = "MANIFEST.json"
MAX_MANIFEST_BYTES = 4 * 1024 * 1024
MAX_PACK_FILE_BYTES = 4 * 1024 * 1024
REQUIRED_BASELINE_DOCUMENTS = (
    "README.md",
    "repository_snapshot/CURRENT_REPOSITORY_STATE_2026-08-23.md",
    "research_proposals/ghostty/GHOSTTY_IMPLEMENTATION_STATUS_2026-08-23.md",
)
FORBIDDEN_NONHISTORICAL_TEXT = (
    "terminal/ptty",
    "behavioral oracle",
    "until the separate lifecycle adr is accepted",
    "recommended authoritative direction",
    "default to an automexia-managed verified ffmpeg/ffprobe runtime.",
    "sandboxed extensions use wasmtime and versioned wit interfaces.",
    "use a supported tokio lts minor for core control-plane code",
)
PRIVATE_LINK = re.compile(
    r"\]\([^\n)]*\.automexia-private(?:[/\\]|%2f)", re.IGNORECASE
)
REQUIRED_POLICY_HEADINGS = (
    "## Public documentation",
    "## Local-only documentation",
    "## Never document in the repository",
    "## Publication review",
)
PUBLIC_FUTURE_PLANNING = re.compile(
    r"^#{1,6}\s+(?:direction after|future features?)\b"
    r"|^Status:\s*(?:planned public|proposed public)\b"
    r"|\b(?:high-level direction only|separate (?:optional )?later direction only"
    r"|preserved as separate later extensions)\b",
    re.IGNORECASE | re.MULTILINE,
)


class DocumentationPackError(ValueError):
    """The public/private documentation boundary is inconsistent."""


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    document: dict[str, Any] = {}
    for key, value in pairs:
        if key in document:
            raise DocumentationPackError(f"manifest contains duplicate key {key!r}")
        document[key] = value
    return document


def load_manifest(pack: Path) -> dict[str, Any]:
    path = pack / MANIFEST_NAME
    if path.is_symlink() or not path.is_file():
        raise DocumentationPackError("documentation manifest is missing or linked")
    payload = path.read_bytes()
    if len(payload) > MAX_MANIFEST_BYTES:
        raise DocumentationPackError("documentation manifest exceeds its byte limit")
    try:
        document = json.loads(
            payload.decode("utf-8"), object_pairs_hook=reject_duplicate_keys
        )
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise DocumentationPackError("documentation manifest is not strict UTF-8 JSON") from error
    if not isinstance(document, dict):
        raise DocumentationPackError("documentation manifest must be an object")
    return document


def validate_relative_path(raw: Any) -> str:
    if not isinstance(raw, str) or not raw:
        raise DocumentationPackError("manifest entry path must be a non-empty string")
    if "\\" in raw:
        raise DocumentationPackError(f"manifest path must use forward slashes: {raw!r}")
    candidate = PurePosixPath(raw)
    if candidate.is_absolute() or ".." in candidate.parts or str(candidate) != raw:
        raise DocumentationPackError(f"manifest path is not canonical: {raw!r}")
    return raw


def actual_inventory(pack: Path) -> set[str]:
    result: set[str] = set()
    for path in pack.rglob("*"):
        if path.is_symlink():
            raise DocumentationPackError(
                f"documentation pack contains a symbolic link: {path.relative_to(pack)}"
            )
        if path.is_file() and path.name != MANIFEST_NAME:
            result.add(path.relative_to(pack).as_posix())
    return result


def validate_metadata(manifest: dict[str, Any]) -> str:
    if manifest.get("schema") != 1:
        raise DocumentationPackError("documentation manifest schema must be 1")
    if manifest.get("pack_status") != "repository-aligned research/proposal pack":
        raise DocumentationPackError("documentation manifest pack status is invalid")
    if manifest.get("project_authority") is not False:
        raise DocumentationPackError("research pack must explicitly deny project authority")
    if manifest.get("blocking_documentation_issues") != []:
        raise DocumentationPackError(
            "manifest may claim no blocking issues only after every check passes"
        )
    baseline = manifest.get("source_audit_head")
    if not isinstance(baseline, str) or re.fullmatch(r"[0-9a-f]{40}", baseline) is None:
        raise DocumentationPackError("source audit baseline must be a full commit SHA")
    if manifest.get("source_audit_kind") != (
        "committed implementation baseline; not moving HEAD"
    ):
        raise DocumentationPackError("source audit kind must describe a frozen baseline")
    if manifest.get("current_rust") != "1.96.1" or manifest.get("current_edition") != "2021":
        raise DocumentationPackError("current Rust baseline does not match the repository")
    owners = manifest.get("real_project_adr_owners")
    expected_owners = {
        "cp5": "0025",
        "llm_orchestration": "0033",
        "top_level_parked_tab_history": "0028",
        "public_ecosystem": "0029",
    }
    if owners != expected_owners:
        raise DocumentationPackError(
            f"real ADR ownership must be {expected_owners!r}, found {owners!r}"
        )
    return baseline


def validate_entries(pack: Path, manifest: dict[str, Any]) -> tuple[int, int, int]:
    entries = manifest.get("files")
    if not isinstance(entries, list) or not entries:
        raise DocumentationPackError("documentation manifest requires file entries")
    expected_inventory = actual_inventory(pack)
    declared_inventory: set[str] = set()
    nonhistorical_hashes: dict[str, str] = {}
    historical_count = 0

    for entry in entries:
        if not isinstance(entry, dict):
            raise DocumentationPackError("manifest file entry must be an object")
        if set(entry) != {"path", "bytes", "sha256", "historical"}:
            raise DocumentationPackError("manifest file entry has unexpected keys")
        path_text = validate_relative_path(entry.get("path"))
        if path_text in declared_inventory:
            raise DocumentationPackError(f"duplicate manifest path {path_text!r}")
        declared_inventory.add(path_text)
        target = pack / Path(*PurePosixPath(path_text).parts)
        if target.is_symlink() or not target.is_file():
            raise DocumentationPackError(f"manifest target is missing or linked: {path_text}")
        payload = target.read_bytes()
        if len(payload) > MAX_PACK_FILE_BYTES:
            raise DocumentationPackError(f"documentation file exceeds byte limit: {path_text}")
        expected_bytes = entry.get("bytes")
        expected_hash = entry.get("sha256")
        if not isinstance(expected_bytes, int) or expected_bytes < 0:
            raise DocumentationPackError(f"manifest byte count is invalid for {path_text}")
        if not isinstance(expected_hash, str) or re.fullmatch(r"[0-9a-f]{64}", expected_hash) is None:
            raise DocumentationPackError(f"manifest hash is invalid for {path_text}")
        actual_hash = hashlib.sha256(payload).hexdigest()
        if expected_bytes != len(payload):
            raise DocumentationPackError(f"manifest size mismatch for {path_text}")
        if expected_hash != actual_hash:
            raise DocumentationPackError(f"manifest hash mismatch for {path_text}")
        historical = entry.get("historical")
        if not isinstance(historical, bool):
            raise DocumentationPackError(f"historical flag must be boolean for {path_text}")
        if historical:
            historical_count += 1
        else:
            previous = nonhistorical_hashes.get(actual_hash)
            if previous is not None:
                raise DocumentationPackError(
                    f"nonhistorical duplicate content: {previous} and {path_text}"
                )
            nonhistorical_hashes[actual_hash] = path_text
            if target.suffix.lower() == ".md":
                try:
                    validate_markdown_payload(
                        path_text,
                        payload,
                        require_single_h1=True,
                        require_heading_order=True,
                    )
                    text = payload.decode("utf-8").casefold()
                except (DocumentationHygieneError, UnicodeDecodeError) as error:
                    raise DocumentationPackError(str(error)) from error
                for forbidden in FORBIDDEN_NONHISTORICAL_TEXT:
                    if forbidden in text:
                        raise DocumentationPackError(
                            f"forbidden stale or contradictory claim in {path_text}: {forbidden!r}"
                        )

    if declared_inventory != expected_inventory:
        missing = sorted(expected_inventory - declared_inventory)
        extra = sorted(declared_inventory - expected_inventory)
        raise DocumentationPackError(
            f"manifest inventory mismatch (unlisted={missing}, missing={extra})"
        )
    return len(entries), historical_count, 0


def validate_baseline_documents(pack: Path, baseline: str) -> None:
    for relative in REQUIRED_BASELINE_DOCUMENTS:
        path = pack / relative
        if path.is_symlink() or not path.is_file():
            raise DocumentationPackError(f"required baseline document is missing: {relative}")
        text = path.read_text(encoding="utf-8")
        if baseline not in text:
            raise DocumentationPackError(
                f"required baseline document does not cite {baseline}: {relative}"
            )

    readme = (pack / "README.md").read_text(encoding="utf-8").casefold()
    required_readme_terms = (
        "adr 0025",
        "adr 0028",
        "adr 0029",
        "adr 0033",
        "v0.4 release closure",
        "v0.5 activation hardening",
    )
    missing = [term for term in required_readme_terms if term not in readme]
    if missing:
        raise DocumentationPackError(f"pack README is missing reconciled terms: {missing}")


def private_boundary_is_enabled(root: Path) -> bool:
    private = root / PRIVATE_PATH
    policy = root / POLICY_PATH
    ignore = root / ".gitignore"
    if private.exists() or private.is_symlink() or policy.exists() or policy.is_symlink():
        return True
    if ignore.is_file() and not ignore.is_symlink():
        try:
            return PRIVATE_PATH in ignore.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            return True
    return False


def public_markdown_paths(root: Path) -> Iterator[Path]:
    excluded_roots = {PRIVATE_PATH, ".automexia-tools", ".git", "target", "artifacts"}

    def unreadable_public(error: OSError) -> None:
        raise DocumentationPackError("cannot read a public documentation directory") from error

    for directory, children, filenames in os.walk(root, topdown=True, followlinks=False, onerror=unreadable_public):
        base = Path(directory)
        # This publication policy excludes root-owned private/generated data.
        # Nested source directories remain in scope, unlike benchmark caches.
        if base == root:
            children[:] = [name for name in children if name not in excluded_roots]
        for name in filenames:
            if name.endswith(".md"):
                yield base / name


def validate_private_boundary(root: Path) -> dict[str, int]:
    legacy = root / PACK_PATH
    if legacy.exists() or legacy.is_symlink():
        raise DocumentationPackError(
            f"legacy public research pack must remain removed: {PACK_PATH}"
        )

    private = root / PRIVATE_PATH
    if private.is_symlink():
        raise DocumentationPackError(
            f"private documentation workspace must not be a symbolic link: {PRIVATE_PATH}"
        )

    ignore = root / ".gitignore"
    if ignore.is_symlink() or not ignore.is_file():
        raise DocumentationPackError(".gitignore is missing or linked")
    try:
        rules = [
            line.strip()
            for line in ignore.read_text(encoding="utf-8").splitlines()
            if line.strip()
        ]
    except UnicodeDecodeError as error:
        raise DocumentationPackError(".gitignore is not strict UTF-8") from error
    if f"/{PRIVATE_PATH}/" not in rules:
        raise DocumentationPackError(
            f".gitignore must contain the exact local-only rule /{PRIVATE_PATH}/"
        )
    negations = [
        rule
        for rule in rules
        if rule.startswith("!") and PRIVATE_PATH.casefold() in rule.casefold()
    ]
    if negations:
        raise DocumentationPackError(
            f"private documentation ignore rule is negated: {negations}"
        )

    policy = root / POLICY_PATH
    if policy.is_symlink() or not policy.is_file():
        raise DocumentationPackError(f"public boundary policy is missing: {POLICY_PATH}")
    try:
        policy_text = policy.read_text(encoding="utf-8")
    except UnicodeDecodeError as error:
        raise DocumentationPackError("public boundary policy is not strict UTF-8") from error
    missing = [heading for heading in REQUIRED_POLICY_HEADINGS if heading not in policy_text]
    if missing:
        raise DocumentationPackError(
            f"public/private documentation policy is missing headings: {missing}"
        )

    public_count = 0
    for path in public_markdown_paths(root):
        relative = path.relative_to(root)
        if path.is_symlink():
            raise DocumentationPackError(
                f"public Markdown must not be a symbolic link: {relative.as_posix()}"
            )
        if not path.is_file():
            continue
        payload = path.read_bytes()
        if len(payload) > MAX_PACK_FILE_BYTES:
            raise DocumentationPackError(
                f"public documentation file exceeds byte limit: {relative.as_posix()}"
            )
        try:
            text = payload.decode("utf-8")
        except UnicodeDecodeError as error:
            raise DocumentationPackError(
                f"public documentation is not strict UTF-8: {relative.as_posix()}"
            ) from error
        if PACK_PATH.casefold() in text.casefold():
            raise DocumentationPackError(
                "public documentation references removed internal research pack: "
                f"{relative.as_posix()}"
            )
        if PRIVATE_LINK.search(text):
            raise DocumentationPackError(
                "public documentation links into the ignored private workspace: "
                f"{relative.as_posix()}"
            )
        # Diagnose the owning page only: repeating matched planning text in CI
        # would disclose the material this boundary is intended to protect.
        if PUBLIC_FUTURE_PLANNING.search(text):
            raise DocumentationPackError(
                f"public documentation exposes future planning: {relative.as_posix()}"
            )
        public_count += 1
    return {"files": public_count, "historical": 0, "duplicates": 0}


def validate(root: Path = ROOT) -> dict[str, int]:
    if private_boundary_is_enabled(root):
        return validate_private_boundary(root)
    pack = root / PACK_PATH
    if pack.is_symlink() or not pack.is_dir():
        raise DocumentationPackError(f"missing documentation pack {PACK_PATH}")
    manifest = load_manifest(pack)
    baseline = validate_metadata(manifest)
    files, historical, duplicates = validate_entries(pack, manifest)
    validate_baseline_documents(pack, baseline)
    return {"files": files, "historical": historical, "duplicates": duplicates}


def update_manifest(root: Path = ROOT) -> int:
    pack = root / PACK_PATH
    manifest = load_manifest(pack)
    previous_flags = {
        entry["path"]: bool(entry.get("historical"))
        for entry in manifest.get("files", [])
        if isinstance(entry, dict) and isinstance(entry.get("path"), str)
    }
    entries: list[dict[str, Any]] = []
    for relative in sorted(actual_inventory(pack)):
        path = pack / Path(*PurePosixPath(relative).parts)
        payload = path.read_bytes()
        historical = previous_flags.get(
            relative,
            relative.startswith("historical/")
            or relative == "repository_snapshot/SUPPLIED_AGENT_REPOSITORY_AUDIT_2026-08-23.md",
        )
        entries.append(
            {
                "path": relative,
                "bytes": len(payload),
                "sha256": hashlib.sha256(payload).hexdigest(),
                "historical": historical,
            }
        )
    manifest["files"] = entries
    destination = pack / MANIFEST_NAME
    temporary = destination.with_suffix(".json.tmp")
    temporary.write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    temporary.replace(destination)
    return len(entries)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--update-manifest", action="store_true")
    arguments = parser.parse_args()
    if arguments.update_manifest:
        count = update_manifest()
        print(f"updated repository-aligned documentation manifest ({count} files)")
        return 0
    counts = validate()
    print(
        "PASS: documentation boundary is coherent "
        f"(files={counts['files']}, historical={counts['historical']}, "
        f"duplicates={counts['duplicates']})"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except DocumentationPackError as error:
        print(f"documentation boundary validation failed: {error}", file=sys.stderr)
        raise SystemExit(1) from error
