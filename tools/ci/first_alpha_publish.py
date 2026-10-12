#!/usr/bin/env python3
"""Publish the fixed first alpha from a verified CI bundle using owner gh authentication.

No account token is copied to CI. The build/sign jobs stay read-only; the owner
checks repository administration settings immediately before publication.
"""
from __future__ import annotations

import argparse
import base64
import json
import os
from pathlib import Path
import re
import subprocess
import sys

import first_alpha as alpha
from release_downloads import replace_download_guide, upload_argument
from check_public_release_metadata import PUBLIC_KEY

sys.path.insert(0, str(alpha.ROOT / ".github/scripts"))
import build_release_manifest as inventory

REPOSITORY = "AmjedAllaya/automexia"
LEGACY_PUBLISHER = "Publish the immutable first alpha with explicit signing status"
REQUIRED_JOBS = {
    "Authorize exact first-alpha source and branding": 1,
    "Release quality and dependency gate": 1,
    "Build native release binaries": 6,
    "Verify Windows and macOS unsigned runtime reproducibility": 4,
    "Verify Linux release reproducibility": 2,
    "Verify unsigned Windows alpha packages on each native architecture": 2,
    "Verify ad-hoc universal DMG on Intel and Apple Silicon": 2,
    "Verify final Linux packages": 2,
    "Require every first-alpha platform and quality gate": 1,
    "Sign release checksum manifest": 1,
}
NATIVE_JOBS = {
    "Native AX (macos-26-intel)", "Native AX (macos-26)",
    "Native microbenchmarks (macos-26-intel)", "Native microbenchmarks (macos-26)",
    "Controlled raster (macos-26-intel)", "Controlled raster (macos-26)",
    "Controlled raster (ubuntu-24.04)", "Controlled raster (windows-2022)",
}
MAX_API_BYTES = 8 * 1024 * 1024


def checked_process(argv: list[str], *, allow_failure: bool = False, timeout: int = 120) -> subprocess.CompletedProcess:
    env = os.environ.copy()
    env.pop("GH_DEBUG", None)
    if Path(argv[0]).name.casefold() in ("wsl", "wsl.exe", "minisign", "minisign.exe"):
        for key in ("GH_TOKEN", "GITHUB_TOKEN", "GH_ENTERPRISE_TOKEN", "GITHUB_ENTERPRISE_TOKEN",
                    "AUTOMEXIA_RELEASE_MINISIGN_SECRET_KEY", "WSLENV"):
            env.pop(key, None)
    env.update(GH_HOST="github.com", GH_PROMPT_DISABLED="1", GH_PAGER="cat")
    result = subprocess.run(argv, capture_output=True, timeout=timeout, env=env)
    if len(result.stdout) + len(result.stderr) > MAX_API_BYTES:
        raise ValueError("release tool output exceeded its limit")
    if result.returncode and not allow_failure:
        raise ValueError("release tool failed; no further publication steps were performed")
    return result


def api(endpoint: str, *, fields: dict | None = None, missing: bool = False, method: str = "POST", allow_array: bool = False) -> dict | list | None:
    args = ["gh", "api", "--include", endpoint]
    if fields is not None:
        args.extend(["--method", method])
        for key, value in fields.items():
            if isinstance(value, bool):
                args.extend(["-F", f"{key}={str(value).lower()}"])
            else:
                args.extend(["-f", f"{key}={value}"])
    result = checked_process(args, allow_failure=True)
    raw = result.stdout.decode("utf-8").replace("\r\n", "\n")
    headers, separator, body = raw.partition("\n\n")
    status = re.match(r"HTTP/[^ ]+ (\d{3})", headers)
    if not separator or status is None:
        raise ValueError("GitHub API returned an invalid HTTP response")
    code = int(status.group(1))
    if missing and code == 404:
        return None
    if result.returncode or not 200 <= code < 300:
        raise ValueError(f"GitHub API rejected the release operation (HTTP {code})")
    value = json.loads(body, object_pairs_hook=inventory._reject_duplicate_keys)
    if not isinstance(value, dict) and not (allow_array and isinstance(value, list)):
        raise ValueError("GitHub API returned an invalid release object")
    return value


def validate_run(run: dict, jobs: list[dict], commit: str, run_id: int) -> None:
    if (run.get("id") != run_id or run.get("head_sha") != commit
            or run.get("head_branch") != "main" or run.get("event") != "workflow_dispatch"
            or run.get("path") != ".github/workflows/first-alpha.yml"
            or run.get("repository", {}).get("full_name") != REPOSITORY
            or run.get("actor", {}).get("login", "").casefold() != REPOSITORY.split("/")[0].casefold()
            or run.get("status") != "completed" or run.get("conclusion") != "success"):
        raise ValueError("publication requires the completed successful owner-dispatched main run")
    if not jobs or len(jobs) > 100:
        raise ValueError("native job inventory is empty or exceeds its limit")
    seen = set()
    names = set()
    for job in jobs:
        if job.get("id") in seen or job.get("name") in names or job.get("status") != "completed":
            raise ValueError("native job evidence is duplicate or incomplete")
        seen.add(job.get("id"))
        names.add(job.get("name"))
        if job.get("name") == LEGACY_PUBLISHER and job.get("conclusion") == "skipped":
            continue  # The original build-only workflow intentionally defers publication.
        if job.get("conclusion") != "success":
            raise ValueError("a required native job failed, was cancelled, or was skipped")
    for prefix, expected in REQUIRED_JOBS.items():
        matches = [job for job in jobs if job.get("name") == prefix or job.get("name", "").startswith(prefix + " (")]
        if len(matches) != expected:
            raise ValueError(f"native job inventory is incomplete: {prefix}")
    build_names = [job["name"] for job in jobs if job["name"].startswith("Build native release binaries (")]
    if any(sum(target in name for name in build_names) != 1 for target in alpha.TARGETS):
        raise ValueError("each native target must have exactly one successful build")
    prefix = "Native platform accessibility, session and visual QA / "
    native = {job["name"][len(prefix):] for job in jobs if job["name"].startswith(prefix)}
    if native != NATIVE_JOBS:
        raise ValueError("all native accessibility, visual and benchmark jobs must pass")


def verify_signature(bundle: Path, wsl_distro: str | None) -> None:
    paths = [str(bundle / "SHA256SUMS"), str(bundle / "SHA256SUMS.minisig")]
    prefix = ["minisign"]
    if wsl_distro:
        if os.name != "nt" or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.-]{0,63}", wsl_distro):
            raise ValueError("WSL verification requires Windows and an exact distribution name")
        prefix = ["wsl", "-d", wsl_distro, "--exec", "minisign"]
        paths = [checked_process(["wsl", "-d", wsl_distro, "--exec", "wslpath", "-a", path]).stdout.decode("utf-8").strip() for path in paths]
        if any(not path.startswith("/") or "\n" in path for path in paths):
            raise ValueError("invalid WSL verification path")
    checked_process([*prefix, "-V", "-P", PUBLIC_KEY, "-m", paths[0], "-x", paths[1]])


def validate_bundle(bundle: Path, wsl_distro: str | None = None) -> tuple[dict, dict[str, tuple[str, int]]]:
    if bundle.is_symlink() or not bundle.is_dir():
        raise ValueError("bundle must be a real directory")
    paths = list(bundle.iterdir())
    if len(paths) != 16:
        raise ValueError("bundle must contain exactly 11 packages, 2 SBOMs, manifest, checksums and signature")
    names = [path.name for path in paths]
    if len({name.casefold() for name in names}) != len(names) or any(not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.+-]{0,179}", name) for name in names):
        raise ValueError("bundle contains unsafe or ambiguous filenames")
    total = 0
    for path in paths:
        maximum = 4096 if path.name in ("SHA256SUMS", "SHA256SUMS.minisig") else inventory.MAX_ARTIFACT_BYTES
        total += inventory._regular_unlinked(path, "release asset", maximum).st_size
        if total > inventory.MAX_TOTAL_BYTES:
            raise ValueError("release bundle exceeds its total size limit")
    verify_signature(bundle, wsl_distro)
    manifest = json.loads(alpha._bytes(bundle / "release-manifest.json", inventory.MAX_MANIFEST_BYTES), object_pairs_hook=inventory._reject_duplicate_keys)
    alpha.verify_disclosure(manifest)
    if (manifest.get("schema") != 1 or manifest.get("project") != "automexia-terminal"
            or manifest.get("source_repository") != "https://github.com/" + REPOSITORY):
        raise ValueError("release manifest source identity is invalid")
    records = {path.name: inventory.digest(path) for path in paths}
    if sum(size for _, size in records.values()) > inventory.MAX_TOTAL_BYTES:
        raise ValueError("release bundle exceeds its total size limit")
    checksums = {}
    for line in (bundle / "SHA256SUMS").read_text(encoding="ascii").splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  ([A-Za-z0-9][A-Za-z0-9_.+-]{0,179})", line)
        if match is None or match[2] in checksums:
            raise ValueError("invalid or duplicate checksum entry")
        checksums[match[2]] = match[1]
    hashed = set(records) - {"SHA256SUMS", "SHA256SUMS.minisig"}
    if set(checksums) != hashed or any(checksums[name] != records[name][0] for name in hashed):
        raise ValueError("signed checksums do not match the complete bundle")
    entries = manifest.get("artifacts")
    if not isinstance(entries, list) or len(entries) != 13:
        raise ValueError("manifest package/SBOM inventory is incomplete")
    expected = {"release-manifest.json", "SHA256SUMS", "SHA256SUMS.minisig"}
    for entry in entries:
        if not isinstance(entry, dict) or not isinstance(entry.get("file"), str):
            raise ValueError("invalid manifest artifact record")
        name = entry["file"]
        if name in expected or name not in records or (entry.get("sha256"), entry.get("size")) != records[name]:
            raise ValueError("manifest artifact digest or identity mismatch")
        expected.add(name)
    if expected != set(records):
        raise ValueError("release contains unlisted assets")
    packages = [path for path in paths if path.name.endswith(inventory.PACKAGE_SUFFIXES)]
    inventory._validate_exact_packages(packages, alpha.ALPHA_VERSION, public=True)
    inventory._validate_sboms([path for path in paths if path.name in inventory.SBOM_NAMES], alpha.ALPHA_VERSION)
    return manifest, records


def validate_remote(release: dict, records: dict[str, tuple[str, int]], *, notes: str, complete: bool, published: bool = False) -> set[str]:
    if (release.get("tag_name") != alpha.ALPHA_TAG or release.get("prerelease") is not True
            or release.get("draft") is not (not published)
            or not isinstance(release.get("target_commitish"), str) or not release["target_commitish"]
            or release.get("body") != notes or release.get("name") != f"Automexia Terminal {alpha.ALPHA_TAG}"):
        raise ValueError("existing release is published or has a conflicting identity")
    if published and release.get("immutable") is not True:
        raise ValueError("published release is not immutable")
    assets = release.get("assets")
    if not isinstance(assets, list) or len(assets) > len(records):
        raise ValueError("remote asset inventory is invalid")
    seen = set()
    for asset in assets:
        name = asset.get("name")
        if (name not in records or name in seen or asset.get("state") != "uploaded"
                or asset.get("digest") != "sha256:" + records[name][0]
                or asset.get("size") != records[name][1]):
            raise ValueError("remote release asset does not match verified local bytes")
        seen.add(name)
    if complete and seen != set(records):
        raise ValueError("remote release is missing verified assets")
    return seen


def authorize_owner(commit: str, acknowledgement: str) -> None:
    owner = api("user")
    if owner.get("login", "").casefold() != REPOSITORY.split("/")[0].casefold():
        raise ValueError("publication requires the authenticated repository owner")
    if api(f"repos/{REPOSITORY}/immutable-releases").get("enabled") is not True:
        raise ValueError("immutable releases must be enabled")
    current = api(f"repos/{REPOSITORY}/git/ref/heads/main")["object"]["sha"]
    alpha.authorize(commit, current, "refs/heads/main", acknowledgement)


def verify_tag(commit: str, *, allow_missing: bool = False) -> bool:
    # target_commitish is informational once a tag exists; resolve the Git object.
    ref = api(f"repos/{REPOSITORY}/git/ref/tags/{alpha.ALPHA_TAG}", missing=allow_missing)
    if ref is None:
        if allow_missing:
            return False
        raise ValueError("verified release tag is missing")
    target = ref.get("object", {})
    if target.get("type") != "tag" or re.fullmatch(r"[0-9a-f]{40}", target.get("sha", "")) is None:
        raise ValueError("release tag must be annotated")
    annotated = api(f"repos/{REPOSITORY}/git/tags/{target['sha']}").get("object", {})
    if annotated.get("type") != "commit" or annotated.get("sha") != commit:
        raise ValueError("existing release tag conflicts with the verified source")
    return True


def publish(bundle: Path, run_id: int, acknowledgement: str, *, perform: bool, wsl_distro: str | None = None) -> None:
    manifest, records = validate_bundle(bundle, wsl_distro)
    commit = manifest["source_commit"]
    run = api(f"repos/{REPOSITORY}/actions/runs/{run_id}")
    response = api(f"repos/{REPOSITORY}/actions/runs/{run_id}/jobs?per_page=100&filter=latest")
    jobs = response.get("jobs", [])
    if response.get("total_count") != len(jobs):
        raise ValueError("native job inventory was truncated")
    validate_run(run, jobs, commit, run_id)
    authorize_owner(commit, acknowledgement)
    tag = alpha.ALPHA_TAG
    source = api(f"repos/{REPOSITORY}/contents/docs/public-release/FIRST-ALPHA-NOTES.md?ref={commit}")
    if source.get("encoding") != "base64" or not 0 < source.get("size", 0) <= 64 * 1024:
        raise ValueError("release notes are missing or oversized")
    notes_bytes = base64.b64decode(source["content"].replace("\n", ""), validate=True)
    if len(notes_bytes) != source["size"]:
        raise ValueError("release note size mismatch")
    notes = replace_download_guide(notes_bytes.decode("utf-8"), alpha.ALPHA_VERSION)
    releases = api(f"repos/{REPOSITORY}/releases?per_page=100", allow_array=True)
    if not isinstance(releases, list) or len(releases) >= 100:
        raise ValueError("first-alpha release inventory is truncated or invalid")
    matches = [release for release in releases if release.get("tag_name") == tag]
    if len(matches) > 1:
        raise ValueError("ambiguous first-alpha release identity")
    existing = matches[0] if matches else None
    uploaded = validate_remote(existing, records, notes=notes, complete=False) if existing is not None else set()
    tag_exists = verify_tag(commit, allow_missing=True)
    if not perform:
        print(f"Verified first-alpha bundle and native run {run_id}; publication not requested")
        return
    # Recheck local bytes before any remote write. Remote digests are checked again before publishing.
    if any(inventory.digest(bundle / name) != record for name, record in records.items()):
        raise ValueError("bundle changed after verification")
    authorize_owner(commit, acknowledgement)
    if not tag_exists:
        annotated = api(f"repos/{REPOSITORY}/git/tags", fields={"tag": tag, "message": f"Automexia Terminal {tag}", "object": commit, "type": "commit"})
        api(f"repos/{REPOSITORY}/git/refs", fields={"ref": "refs/tags/" + tag, "sha": annotated["sha"]})
    if existing is None:
        existing = api(f"repos/{REPOSITORY}/releases", fields={
            "tag_name": tag, "target_commitish": commit, "draft": True,
            "prerelease": True, "make_latest": "false", "name": f"Automexia Terminal {tag}",
            "body": notes,
        })
    release_id = existing.get("id")
    if not isinstance(release_id, int) or release_id <= 0:
        raise ValueError("invalid draft release identity")
    release_endpoint = f"repos/{REPOSITORY}/releases/{release_id}"
    missing_assets = [upload_argument(bundle / name, alpha.ALPHA_VERSION)
                      for name in sorted(set(records) - uploaded)]
    if missing_assets:
        checked_process(["gh", "release", "upload", tag, "--repo", REPOSITORY, *missing_assets], timeout=900)
    draft = api(release_endpoint)
    validate_remote(draft, records, notes=notes, complete=True)
    authorize_owner(commit, acknowledgement)
    verify_tag(commit)
    api(release_endpoint, method="PATCH", fields={"draft": False, "prerelease": True, "make_latest": "false"})
    result = api(release_endpoint)
    verify_tag(commit)
    validate_remote(result, records, notes=notes, complete=True, published=True)
    checked_process(["gh", "release", "verify", tag, "--repo", REPOSITORY])
    for name in sorted(records):
        checked_process(["gh", "release", "verify-asset", tag, str(bundle / name), "--repo", REPOSITORY])
    print(f"Published and verified https://github.com/{REPOSITORY}/releases/tag/{tag}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundle", type=Path, required=True)
    parser.add_argument("--run-id", type=int, required=True)
    parser.add_argument("--acknowledgement", required=True)
    parser.add_argument("--publish", action="store_true", help="Create or resume the verified draft and publish it")
    parser.add_argument("--wsl-distro", help="Use an existing WSL minisign verifier from Windows")
    args = parser.parse_args()
    if args.run_id <= 0:
        parser.error("--run-id must be positive")
    publish(args.bundle.absolute(), args.run_id, args.acknowledgement, perform=args.publish, wsl_distro=args.wsl_distro)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, KeyError, TypeError, subprocess.TimeoutExpired) as error:
        raise SystemExit(f"First-alpha owner publication stopped: {error}") from error
