#!/usr/bin/env python3
"""Regression tests for the bounded Linux Early Access distribution contract."""

from __future__ import annotations

import copy
import contextlib
import importlib.util
import hashlib
import io
import json
import os
import shutil
from pathlib import Path
import tempfile
import subprocess
import sys
import unittest
from unittest import mock


MODULE_PATH = Path(__file__).with_name("public_distribution.py")
SPEC = importlib.util.spec_from_file_location("automexia_public_distribution", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load tools/ci/public_distribution.py")
DISTRIBUTION = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DISTRIBUTION)

VERSION = "1.2.3"
COMMIT = "0123456789abcdef0123456789abcdef01234567"
REPOSITORY = "AmjedAllaya/automexia-releases"


def owner_release_event() -> dict:
    # Public project identities are intentional; no live account/event is copied.
    owner = {"login": "AmjedAllaya", "type": "User"}
    repo = {"full_name": "AmjedAllaya/automexia-terminal"}
    return {
        "action": "closed", "repository": copy.deepcopy(repo),
        "sender": copy.deepcopy(owner),
        "pull_request": {
            "number": 1, "state": "closed", "merged": True,
            "user": copy.deepcopy(owner), "merged_by": copy.deepcopy(owner),
            "merge_commit_sha": COMMIT,
            "head": {"ref": "release/linux/1.2.3", "sha": "b" * 40,
                     "repo": copy.deepcopy(repo)},
            "base": {"ref": "main", "repo": copy.deepcopy(repo)},
        },
    }


class OwnerReleaseAuthorizationTests(unittest.TestCase):
    def authorize(self, payload: object, **overrides: str) -> str:
        arguments = {
            "event_name": "pull_request", "repository": "AmjedAllaya/automexia-terminal",
            "actor": "AmjedAllaya", "triggering_actor": "AmjedAllaya",
            "commit": COMMIT, "current_main": COMMIT,
        }
        arguments.update(overrides)
        return DISTRIBUTION.authorize_owner_release(payload, **arguments)

    def test_owner_can_author_and_merge_without_a_self_review(self) -> None:
        self.assertEqual(self.authorize(owner_release_event()), "1.2.3")

    def test_every_actor_and_context_must_be_the_owner_and_exact_source(self) -> None:
        for name, values in {
            "actor": ("alice", "AmjedAllaya[bot]", ""),
            "triggering_actor": ("alice", "AmjedAllaya[bot]", ""),
            "repository": ("alice/automexia-terminal", "AmjedAllaya/automexia-releases", ""),
            "event_name": ("workflow_dispatch", "pull_request_target", "push", ""),
            "commit": ("a" * 40, "0" * 40, "bad", ""),
            "current_main": ("a" * 40, "0" * 40, "bad", ""),
        }.items():
            for value in values:
                with self.subTest(field=name, value=value):
                    with self.assertRaises(DISTRIBUTION.DistributionError):
                        self.authorize(owner_release_event(), **{name: value})

    def test_untrusted_event_mutations_never_authorize(self) -> None:
        mutations = {
            ("action",): ("opened", "synchronize", None),
            ("repository", "full_name"): ("alice/automexia-terminal", None),
            ("sender", "login"): ("alice", "", "AmjedAllaya\n"),
            ("sender", "type"): ("Bot", None),
            ("pull_request", "user", "login"): ("alice", None),
            ("pull_request", "user", "type"): ("Bot", None),
            ("pull_request", "merged_by", "login"): ("alice", None),
            ("pull_request", "merged_by", "type"): ("Bot", None),
            ("pull_request", "number"): (0, -1, True, "1", None),
            ("pull_request", "state"): ("open", None),
            ("pull_request", "merged"): (False, 1, "true", None),
            ("pull_request", "merge_commit_sha"): ("a" * 40, "0" * 40, COMMIT + "\n", None),
            ("pull_request", "head", "sha"): ("", "0" * 40, "b" * 39, "b" * 41, None),
            ("pull_request", "base", "ref"): ("release", "main\n", None),
            ("pull_request", "head", "repo", "full_name"): ("alice/automexia-terminal", None),
            ("pull_request", "base", "repo", "full_name"): ("alice/automexia-terminal", None),
            ("pull_request", "head", "ref"): (
                "release/1.2.3", "release/linux/1.2.3-rc.1", "release/linux/01.2.3",
                "release/linux/1.2.3\necho bad", "release/linux/" + "1" * 100, None,
            ),
        }
        for path, values in mutations.items():
            for value in values:
                with self.subTest(path=path, value=value):
                    payload = owner_release_event()
                    target = payload
                    for part in path[:-1]:
                        target = target[part]
                    target[path[-1]] = value
                    with self.assertRaises(DISTRIBUTION.DistributionError):
                        self.authorize(payload)
        for payload in (None, [], {}, "private-canary", {"pull_request": []}):
            with self.subTest(payload=payload):
                with self.assertRaises(DISTRIBUTION.DistributionError):
                    self.authorize(payload)

    def test_event_reader_is_bounded_strict_redacted_and_read_only(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            event = Path(temporary) / "event.json"
            maximum = 1024 * 1024
            valid = json.dumps(owner_release_event()).encode()
            event.write_bytes(valid + b" " * (maximum - len(valid)))
            self.assertEqual(DISTRIBUTION.load_release_event(event), owner_release_event())
            # Each rejected payload contains private-like data that must not enter logs.
            for data in (
                b" " * (maximum + 1), b'{"private-canary":1,"private-canary":2}',
                b'{"private-canary":', b'"private-canary"', b'\xff',
                b'{"value":NaN}', b'{"value":Infinity}', b'{"value":1e999}',
                b'{"items":[' + b'0,' * 32768 + b'0]}',
                b'{"nested":' + b'[' * 2000 + b'0' + b']' * 2000 + b'}',
            ):
                event.write_bytes(data)
                with self.subTest(kind=data[:20]):
                    with self.assertRaises(DISTRIBUTION.DistributionError) as raised:
                        DISTRIBUTION.load_release_event(event)
                    self.assertNotIn("private-canary", str(raised.exception))
                    self.assertNotIn(temporary, str(raised.exception))
                    self.assertEqual(event.read_bytes(), data)
                    self.assertEqual([p.name for p in Path(temporary).iterdir()], ["event.json"])

    def test_event_reader_depth_boundary_and_missing_file(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            event = Path(temporary) / "event.json"
            with self.assertRaises(DISTRIBUTION.DistributionError):
                DISTRIBUTION.load_release_event(event)
            for depth in (31, 32, 33):
                event.write_bytes(b'{"item":' * depth + b'0' + b'}' * depth)
                if depth <= 32:
                    self.assertIsInstance(DISTRIBUTION.load_release_event(event), dict)
                else:
                    with self.assertRaises(DISTRIBUTION.DistributionError):
                        DISTRIBUTION.load_release_event(event)

    def test_event_node_boundary_and_hardlink_rejection(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            event = Path(temporary) / "event.json"
            for count in (32765, 32766, 32767):
                event.write_text(json.dumps({"items": [0] * count}), encoding="utf-8")
                if count <= 32766:
                    self.assertEqual(len(DISTRIBUTION.load_release_event(event)["items"]), count)
                else:
                    with self.assertRaises(DISTRIBUTION.DistributionError):
                        DISTRIBUTION.load_release_event(event)
            event.write_text(json.dumps(owner_release_event()), encoding="utf-8")
            os.link(event, Path(temporary) / "linked.json")
            with self.assertRaises(DISTRIBUTION.DistributionError):
                DISTRIBUTION.load_release_event(event)

    def test_workflow_cannot_bypass_or_substitute_owner_authorization(self) -> None:
        workflow = DISTRIBUTION.PUBLIC_WORKFLOW.read_text(encoding="utf-8")
        invocation = 'version="$(python3 tools/ci/public_distribution.py authorize-release'
        mutations = {
            "delete checker": workflow.replace(invocation, 'version="$(echo 1.2.3'),
            "ignore failure": workflow.replace('--current-main "$current_main")"', '--current-main "$current_main" || true)"'),
            "duplicate publication": workflow.replace('publish=true', 'publish=true\n            publish=true'),
            "activate before validation": workflow.replace(invocation, 'publish=true\n            ' + invocation),
            "rerun substitution": workflow.replace('--triggering-actor "$GITHUB_TRIGGERING_ACTOR"', '--triggering-actor "$GITHUB_ACTOR"'),
            "stale main": workflow.replace('--current-main "$current_main"', '--current-main "$EVENT_COMMIT"'),
            "event substitution": workflow.replace('--event "$GITHUB_EVENT_PATH"', '--event event.json'),
            "wrong source": workflow.replace('--repository "$GITHUB_REPOSITORY"', '--repository alice/automexia-terminal'),
            "remove fail fast": workflow.replace('set -euo pipefail', 'set -uo pipefail', 1),
            "disable fail fast": workflow.replace('set -euo pipefail', 'set -euo pipefail\n          set +e', 1),
            "continue after failure": workflow.replace('        id: authorize', '        continue-on-error: true\n        id: authorize', 1),
        }
        with tempfile.TemporaryDirectory() as temporary:
            candidate = Path(temporary) / "workflow.yml"
            for label, mutated in mutations.items():
                with self.subTest(mutation=label):
                    self.assertNotEqual(mutated, workflow)
                    candidate.write_text(mutated, encoding="utf-8", newline="\n")
                    with self.assertRaises(DISTRIBUTION.DistributionError):
                        DISTRIBUTION.validate_workflow(candidate)

    def test_real_cli_returns_only_validated_version_and_rejects_bad_event(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            event = Path(temporary) / "event.json"
            event.write_text(json.dumps(owner_release_event()), encoding="utf-8")
            argv = ["authorize-release", "--event", str(event), "--event-name", "pull_request",
                    "--repository", "AmjedAllaya/automexia-terminal", "--actor", "AmjedAllaya",
                    "--triggering-actor", "AmjedAllaya", "--commit", COMMIT, "--current-main", COMMIT]
            stdout, stderr = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                self.assertEqual(DISTRIBUTION.main(argv), 0)
            self.assertEqual(stdout.getvalue(), "1.2.3\n")
            self.assertEqual(stderr.getvalue(), "")
            process = subprocess.run(
                [sys.executable, str(MODULE_PATH), *argv],
                capture_output=True, text=True, timeout=15, check=False,
            )
            self.assertEqual((process.returncode, process.stdout, process.stderr), (0, "1.2.3\n", ""))
            event.write_text('{"private-canary":', encoding="utf-8")
            stdout, stderr = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
                self.assertEqual(DISTRIBUTION.main(argv), 1)
            self.assertEqual(stdout.getvalue(), "")
            self.assertNotIn("private-canary", stderr.getvalue())
            self.assertNotIn(temporary, stderr.getvalue())


def package_names() -> tuple[str, ...]:
    # nFPM emits the Debian revision from the repository-owned version `1`.
    # This fixture mirrors the real native package job so the post-download
    # aggregator cannot silently drift from package production again.
    return (
        f"automexia-terminal_{VERSION}-1_amd64.deb",
        f"automexia-terminal_{VERSION}-1_arm64.deb",
        f"automexia-terminal-{VERSION}-1.x86_64.rpm",
        f"automexia-terminal-{VERSION}-1.aarch64.rpm",
        f"automexia-terminal-{VERSION}-x86_64-unknown-linux-gnu.tar.gz",
        f"automexia-terminal-{VERSION}-aarch64-unknown-linux-gnu.tar.gz",
    )


def write_packages(root: Path) -> None:
    for index, name in enumerate(package_names()):
        path = root / f"native-{index}" / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(f"fixture-{index}\n".encode())


def release_payload(
    manifest: dict[str, object], bundle: Path | None = None
) -> dict[str, object]:
    assets = []
    records = {
        artifact["file"]: artifact for artifact in manifest["artifacts"]
    }
    for name in DISTRIBUTION.required_release_asset_names(manifest):
        record = records.get(name)
        if bundle is not None:
            data = (bundle / name).read_bytes()
            digest = hashlib.sha256(data).hexdigest()
            size = len(data)
        else:
            digest = record["sha256"] if record else "a" * 64
            size = record["size"] if record else 12
        assets.append(
            {
                "name": name,
                "size": size,
                "digest": f"sha256:{digest}",
                "state": "uploaded",
                "browser_download_url": (
                    f"https://github.com/{REPOSITORY}/releases/download/"
                    f"v{VERSION}/{name}"
                ),
            }
        )
    return {
        "id": 7,
        "tag_name": f"v{VERSION}",
        "draft": False,
        "prerelease": True,
        "immutable": True,
        "html_url": f"https://github.com/{REPOSITORY}/releases/tag/v{VERSION}",
        "assets": assets,
    }


def sbom_documents() -> tuple[dict, dict]:
    # A real package graph plus a versionless, hashed file reproduces Syft output;
    # header-only JSON cannot prove that release dependencies were inventoried.
    packages = [("automexia-terminal", VERSION), *[(f"dependency-{i}", "1.0.0") for i in range(9)]]
    spdx = {
        "spdxVersion": "SPDX-2.3", "SPDXID": "SPDXRef-DOCUMENT",
        "dataLicense": "CC0-1.0", "name": "sbom-input",
        "documentNamespace": "https://example.invalid/sbom/fixture",
        "creationInfo": {"created": "2026-01-01T00:00:00Z", "creators": ["Tool: syft-fixture"]},
        "packages": [
            {"SPDXID": f"SPDXRef-Package-{i}", "name": name, "versionInfo": version,
             "downloadLocation": "NOASSERTION", "filesAnalyzed": False,
             "externalRefs": [{"referenceCategory": "PACKAGE-MANAGER", "referenceType": "purl",
                               "referenceLocator": f"pkg:cargo/{name}@{version}"}]}
            for i, (name, version) in enumerate(packages)
        ],
        "files": [{"SPDXID": "SPDXRef-File-lock", "fileName": "Cargo.lock",
                   "checksums": [{"algorithm": "SHA256", "checksumValue": "a" * 64}]}],
        "relationships": [{"spdxElementId": "SPDXRef-Package-0", "relationshipType": "CONTAINS",
                           "relatedSpdxElement": "SPDXRef-File-lock"}],
    }
    cdx = {
        "bomFormat": "CycloneDX", "specVersion": "1.7", "version": 1,
        "serialNumber": "urn:uuid:12345678-1234-1234-1234-123456789abc",
        "metadata": {"component": {"name": "sbom-input", "type": "file", "bom-ref": "scan-root"}},
        "components": [
            {"type": "library", "bom-ref": f"package-{i}", "name": name, "version": version,
             "purl": f"pkg:cargo/{name}@{version}", "licenses": [{"license": {"id": "MIT"}}]}
            for i, (name, version) in enumerate(packages)
        ] + [{"type": "file", "bom-ref": "file-lock", "name": "Cargo.lock",
              "hashes": [{"alg": "SHA-256", "content": "a" * 64}]}],
        "dependencies": [{"ref": "package-0", "dependsOn": ["package-1"]}],
    }
    return spdx, cdx


def rewrite_checksums(bundle: Path) -> None:
    covered = sorted(path for path in bundle.iterdir() if path.name not in {"SHA256SUMS", "SHA256SUMS.minisig"})
    lines = [f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}" for path in covered]
    (bundle / "SHA256SUMS").write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")


class PublicSbomPreparationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.source, self.output, self.scan = (self.root / name for name in ("private", "public", "scan"))
        for directory in (self.source, self.output, self.scan):
            directory.mkdir()

    def write(self, spdx: dict, cdx: dict) -> None:
        for name, payload in (("spdx", spdx), ("cdx", cdx)):
            (self.source / f"automexia-terminal.{name}.json").write_text(json.dumps(payload), encoding="utf-8")

    def prepare(self) -> None:
        DISTRIBUTION.prepare_public_sboms(self.source, self.output, self.scan, VERSION)

    def test_real_cli_preserves_graph_hashes_and_licenses_and_never_copies_raw_paths(self) -> None:
        spdx, cdx = sbom_documents()
        spdx["packages"].append({"SPDXID": "SPDXRef-DocumentRoot-Directory-fixture", "name": "sbom-input",
                                  "filesAnalyzed": False, "primaryPackagePurpose": "FILE"})
        spdx["packages"][0]["sourceInfo"] = "acquired package info from rust cargo manifest: /Cargo.lock"
        cdx["components"][0]["properties"] = [{"name": "syft:location:0:path", "value": "/Cargo.lock"}]
        expected_spdx, expected_cdx = copy.deepcopy(spdx), copy.deepcopy(cdx)
        expected_spdx["packages"][0]["sourceInfo"] = "acquired package info from rust cargo manifest: Cargo.lock"
        expected_cdx["components"][0]["properties"][0]["value"] = "Cargo.lock"
        spdx["files"][0]["fileName"] = "./Cargo.lock"
        cdx["components"][-1]["name"] = str(self.scan / "Cargo.lock")
        self.write(spdx, cdx)
        before = {p.name: p.read_bytes() for p in self.source.iterdir()}
        completed = subprocess.run(
            [sys.executable, str(MODULE_PATH), "prepare-sboms", "--source", str(self.source),
             "--output", str(self.output), "--scan-root", str(self.scan), "--version", VERSION],
            capture_output=True, text=True, timeout=10,
        )
        self.assertEqual(completed.returncode, 0, "SBOM preparation CLI failed")
        self.assertEqual(completed.stdout.strip(), "Validated private build inventories")
        for suffix, expected in (("spdx", expected_spdx), ("cdx", expected_cdx)):
            data = (self.output / f"automexia-terminal.{suffix}.json").read_bytes()
            self.assertEqual(json.loads(data), expected)
            self.assertNotIn(str(self.root).encode(), data)
        self.assertEqual(before, {p.name: p.read_bytes() for p in self.source.iterdir()})
        first = {p.name: p.read_bytes() for p in self.output.iterdir()}
        self.prepare()
        self.assertEqual(first, {p.name: p.read_bytes() for p in self.output.iterdir()})

    def test_rejected_paths_or_unknown_metadata_leave_both_outputs_untouched(self) -> None:
        self.write(*sbom_documents())
        self.prepare()
        before = {p.name: p.read_bytes() for p in self.output.iterdir()}
        for value in (str(self.root / "other" / "Cargo.lock"), str(self.scan / ".." / "Cargo.lock"),
                      "../Cargo.lock", "packages/unapproved.deb", "file:///Cargo.lock", "%2FCargo.lock"):
            with self.subTest(case="rejected location"):
                spdx, cdx = sbom_documents()
                cdx["components"][-1]["name"] = value
                self.write(spdx, cdx)
                with self.assertRaises(DISTRIBUTION.DistributionError) as raised:
                    self.prepare()
                self.assertNotIn(value, str(raised.exception))
                self.assertEqual(before, {p.name: p.read_bytes() for p in self.output.iterdir()})
        for value in (str(self.scan / "Cargo.lock"), "file%253A%252F%252F%252FCargo.lock",
                      "https://alice:private-canary@example.invalid/package", "unsafe\u202epath",
                      "https://localhost/package", "https://devbox.local/package", "https://127.0.0.1/package",
                      "https://[::1]/package"):
            spdx, cdx = sbom_documents()
            cdx["metadata"]["unknown"] = {"value": value}
            self.write(spdx, cdx)
            with self.assertRaises(DISTRIBUTION.DistributionError):
                self.prepare()
            self.assertEqual(before, {p.name: p.read_bytes() for p in self.output.iterdir()})

    def test_malformed_bounded_input_and_file_identity_fail_without_output(self) -> None:
        for data in (b'{"private-canary":1,"private-canary":2}', b'{"value":1e999}',
                     b'{"value":NaN}', b'\xff', b'{"a":' * 1000 + b'0' + b'}' * 1000):
            self.write(*sbom_documents())
            (self.source / "automexia-terminal.cdx.json").write_bytes(data)
            with self.assertRaises(DISTRIBUTION.DistributionError) as raised:
                self.prepare()
            self.assertNotIn("private-canary", str(raised.exception))
            self.assertEqual(list(self.output.iterdir()), [])
        for field in ("hashes", "bom-ref", "name"):
            spdx, cdx = sbom_documents()
            del cdx["components"][-1][field]
            self.write(spdx, cdx)
            with self.assertRaises(DISTRIBUTION.DistributionError):
                self.prepare()
        self.write(*sbom_documents())
        for limit, size in (("MAX_SBOM_BYTES", 64), ("MAX_SBOM_NODES", 10), ("MAX_SBOM_DEPTH", 2)):
            with mock.patch.object(DISTRIBUTION, limit, size):
                with self.assertRaises(DISTRIBUTION.DistributionError):
                    self.prepare()
        self.assertEqual(list(self.output.iterdir()), [])

    def test_privacy_guard_and_shape_limits_have_exact_boundaries(self) -> None:
        for value in ("https://example.invalid/license", "pkg:cargo/fixture@1.2.3", "MIT AND Apache-2.0"):
            DISTRIBUTION._sbom_structure(value, privacy=True)
        with mock.patch.object(DISTRIBUTION, "MAX_SBOM_DEPTH", 2):
            DISTRIBUTION._sbom_structure([[0]], privacy=True)
            with self.assertRaises(DISTRIBUTION.DistributionError):
                DISTRIBUTION._sbom_structure([[[0]]], privacy=True)
        with mock.patch.object(DISTRIBUTION, "MAX_SBOM_NODES", 3):
            DISTRIBUTION._sbom_structure([0, 1], privacy=True)
            with self.assertRaises(DISTRIBUTION.DistributionError):
                DISTRIBUTION._sbom_structure([0, 1, 2], privacy=True)

    def test_public_graph_file_checksums_and_package_count_cannot_be_fabricated(self) -> None:
        mutations = (
            lambda spdx, cdx: spdx.update(files=[]),
            lambda spdx, cdx: cdx["components"].pop(),
            lambda spdx, cdx: cdx["components"][-1]["hashes"][0].update(content="b" * 64),
            lambda spdx, cdx: cdx["components"][-1].update(hashes=[{"alg": "SHA-1", "content": "a" * 40}]),
            lambda spdx, cdx: spdx["files"][0].update(checksums=[]),
            lambda spdx, cdx: spdx["packages"][1].update(SPDXID=spdx["packages"][0]["SPDXID"]),
            lambda spdx, cdx: cdx["components"][1].update({"bom-ref": cdx["components"][0]["bom-ref"]}),
            lambda spdx, cdx: cdx["dependencies"][0].update(dependsOn=["unknown"]),
            lambda spdx, cdx: spdx["relationships"][0].update(relatedSpdxElement="unknown"),
            lambda spdx, cdx: cdx.update(version=True),
            lambda spdx, cdx: cdx.update(components=[cdx["components"][0], *[dict(cdx["components"][-1], **{"bom-ref": str(i)}) for i in range(20)]]),
        )
        for index, mutation in enumerate(mutations):
            with self.subTest(mutation=index):
                spdx, cdx = sbom_documents()
                mutation(spdx, cdx)
                with self.assertRaises(DISTRIBUTION.DistributionError):
                    DISTRIBUTION.validate_public_sboms(spdx, cdx, VERSION)

    def test_atomic_sbom_output_preserves_previous_bytes_and_cleans_owned_temporary(self) -> None:
        self.write(*sbom_documents())
        self.prepare()
        before = {p.name: p.read_bytes() for p in self.output.iterdir()}
        with mock.patch.object(DISTRIBUTION.os, "replace", side_effect=PermissionError("private-canary")):
            with self.assertRaises(OSError):
                self.prepare()
        self.assertEqual(before, {p.name: p.read_bytes() for p in self.output.iterdir()})
        scratch = self.output / f".automexia-terminal.spdx.json.{os.getpid()}.tmp"
        scratch.write_text("unrelated sentinel", encoding="utf-8")
        with self.assertRaises(DISTRIBUTION.DistributionError):
            self.prepare()
        self.assertEqual(scratch.read_text(encoding="utf-8"), "unrelated sentinel")

    def test_workflow_cannot_skip_duplicate_move_or_upload_raw_sboms(self) -> None:
        workflow = DISTRIBUTION.PUBLIC_WORKFLOW.read_text(encoding="utf-8")
        command = ('python3 tools/ci/public_distribution.py prepare-sboms --source sbom-private '
                   '--output sbom-reviewed --scan-root sbom-input --version "$RELEASE_VERSION"')
        candidates = [workflow.replace(command, replacement, 1) for replacement in (
            "true", "true || " + command, command + " || true", command + "\n          " + command,
            command.replace("--output sbom-reviewed", "--output release-bundle"),
        )]
        candidates += [workflow.replace(original, replacement, 1) for original, replacement in (
            ("syft-version: v1.51.1", "syft-version: latest"),
            ("output-file: sbom-private/", "output-file: release-bundle/"),
            ("upload-release-assets: false", "upload-release-assets: true"),
            ("upload-artifact: false", "upload-artifact: true"),
            ("dependency-snapshot: false", "dependency-snapshot: true"),
            ("if: github.event.repository.private == true", "if: always()"),
            ("name: private-build-inventories", "name: public-linux-release-bundle"),
            ("path: sbom-reviewed", "path: sbom-private"),
            ("--directory release-bundle\n      - name: Retain", "--directory ignored\n      - name: Retain"),
            ('test "${#assets[@]}" -eq 14', 'test "${#assets[@]}" -eq 16'),
            ("rm -rf -- sbom-private sbom-input sbom-reviewed", "rm -rf -- sbom-private sbom-input sbom-reviewed\n          compression-level: 0"),
            ("        if: always()\n        shell: bash\n        run: |\n          set -euo pipefail\n          # Only job-owned", "        shell: bash\n        run: |\n          set -euo pipefail\n          # Only job-owned"),
            ("rm -rf -- sbom-private sbom-input sbom-reviewed", "rm -rf -- release-bundle"),
        )]
        start = workflow.index("      - name: Validate private build inventories")
        end = workflow.index("      - name: Create and verify", start)
        block = workflow[start:end]
        candidates.append(workflow[:start] + workflow[end:] + block)
        candidate = self.root / "workflow.yml"
        for mutated in candidates:
            self.assertNotEqual(mutated, workflow)
            candidate.write_text(mutated, encoding="utf-8")
            with self.assertRaises(DISTRIBUTION.DistributionError):
                DISTRIBUTION.validate_workflow(candidate)


class MinimalPublicationTests(unittest.TestCase):
    def test_forbidden_asset_names_are_redacted_from_diagnostics(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / 'source'
            source.mkdir()
            write_packages(source)
            (source / 'private-project-canary.pdb').write_bytes(b'fixture')
            with self.assertRaises(DISTRIBUTION.DistributionError) as raised:
                DISTRIBUTION.build_public_distribution(source, root / 'bundle', VERSION, COMMIT)
            self.assertNotIn('private-project-canary', str(raised.exception))
            self.assertFalse((root / 'bundle').exists())

    def test_repinning_a_document_does_not_allow_local_path_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_bundle(root)
            bundle = root / 'bundle'
            policy = root / 'review.json'
            approved = json.loads(DISTRIBUTION.DOCUMENT_POLICY.read_text(encoding='utf-8'))
            original = (bundle / 'INSTALL.md').read_bytes()
            for suffix in (b'file:///home/alice/build', b'https://devbox.internal/build',
                           b'C:\\Users\\alice\\build', b'file%3A%2F%2F%2Fworkspace%2Fprivate', b'\xe2\x80\xaeprivate'):
                modified = original + suffix
                (bundle / 'INSTALL.md').write_bytes(modified)
                approved['documents']['INSTALL.md'] = hashlib.sha256(modified).hexdigest()
                policy.write_text(json.dumps(approved), encoding='utf-8')
                with mock.patch.object(DISTRIBUTION, 'DOCUMENT_POLICY', policy):
                    with self.assertRaises(DISTRIBUTION.DistributionError):
                        DISTRIBUTION.validate_public_documents(bundle)

    @unittest.skipUnless(shutil.which('minisign'), 'Native Minisign unavailable; Linux release runner required')
    def test_real_minisign_output_passes_template_and_tampering_fails_crypto(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = write_bundle(root)
            bundle = root / 'bundle'

            def run(*arguments: str) -> subprocess.CompletedProcess:
                return subprocess.run(['minisign', *arguments], cwd=root, capture_output=True,
                                      timeout=15, check=False)

            # Ephemeral test key only; never access a contributor's release credentials.
            self.assertEqual(run('-G', '-W', '-s', 'test.sec', '-p', 'test.pub').returncode, 0)
            public = (root / 'test.pub').read_text(encoding='utf-8').splitlines()[-1]
            (bundle / 'automexia-release-key.pub').write_text(
                f'untrusted comment: Automexia release verification key\n{public}\n', encoding='utf-8')
            rewrite_checksums(bundle)
            comment = f'Automexia Linux Early Access v{VERSION} source {COMMIT}'
            self.assertEqual(run('-S', '-W', '-s', 'test.sec', '-m', 'bundle/SHA256SUMS',
                                 '-x', 'bundle/SHA256SUMS.minisig', '-t', comment).returncode, 0)
            self.assertEqual(DISTRIBUTION.verify_bundle(bundle), manifest)
            self.assertEqual(run('-V', '-P', public, '-m', 'bundle/SHA256SUMS').returncode, 0)
            (bundle / 'SHA256SUMS').write_bytes(b'tampered\n')
            self.assertNotEqual(run('-V', '-P', public, '-m', 'bundle/SHA256SUMS').returncode, 0)

    def test_verification_comments_cannot_publish_private_identifiers(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_bundle(root)
            bundle = root / 'bundle'
            for name in ('automexia-release-key.pub', 'SHA256SUMS.minisig'):
                original = (bundle / name).read_bytes()
                for suffix in (b'private-account-canary\n', b'/home/alice/build\n', b'A' * 4097):
                    (bundle / name).write_bytes(original + suffix)
                    rewrite_checksums(bundle)
                    with self.assertRaises(DISTRIBUTION.DistributionError):
                        DISTRIBUTION.verify_bundle(bundle)
                (bundle / name).write_bytes(original)

    def test_document_review_policy_cannot_drop_expand_or_corrupt_approval(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            policy = Path(temporary) / 'review.json'
            original = json.loads(DISTRIBUTION.DOCUMENT_POLICY.read_text(encoding='utf-8'))
            for mutate in (
                lambda p: p.update(schema=True),
                lambda p: p.update(internal='private-project-canary'),
                lambda p: p['documents'].pop('THIRD_PARTY_NOTICES.md'),
                lambda p: p['documents'].update({'build.json': 'a' * 64}),
                lambda p: p['documents'].update({'INSTALL.md': '/home/alice/build'}),
            ):
                candidate = copy.deepcopy(original)
                mutate(candidate)
                policy.write_text(json.dumps(candidate), encoding='utf-8')
                with mock.patch.object(DISTRIBUTION, 'DOCUMENT_POLICY', policy):
                    with self.assertRaises(DISTRIBUTION.DistributionError):
                        DISTRIBUTION._reviewed_documents()

    def test_new_bundle_contains_only_packages_and_reviewed_user_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = write_bundle(root)
            self.assertEqual(manifest['schema'], 2)
            self.assertEqual(len(DISTRIBUTION.required_release_asset_names(manifest)), 14)
            self.assertFalse(any('spdx' in name or 'cdx' in name
                                 for name in DISTRIBUTION.required_release_asset_names(manifest)))
            self.assertEqual(DISTRIBUTION.verify_bundle(root / 'bundle'), manifest)

    def test_unknown_manifest_fields_cannot_smuggle_internal_details(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            original = write_bundle(root)
            bundle = root / 'bundle'
            # Recompute all checksums: integrity alone must not authorize disclosure.
            for section in ('root', 'evidence', 'artifact'):
                for value in ('private-project-canary', '/home/alice/build', 'file:///workspace/build',
                              {'account': 'private-account-canary'}, ['internal-module-canary']):
                    manifest = copy.deepcopy(original)
                    target = manifest if section == 'root' else (
                        manifest['evidence'] if section == 'evidence' else manifest['artifacts'][0])
                    target['internal'] = value
                    (bundle / DISTRIBUTION.MANIFEST_NAME).write_text(json.dumps(manifest), encoding='utf-8')
                    rewrite_checksums(bundle)
                    with self.subTest(section=section), self.assertRaises(DISTRIBUTION.DistributionError):
                        DISTRIBUTION.verify_bundle(bundle)

    def test_rehashed_document_leaks_and_notice_removal_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_bundle(root)
            bundle = root / 'bundle'
            for name in ('INSTALL.md', 'RELEASE-NOTES.md', 'THIRD_PARTY_NOTICES.md', 'UNINSTALL.md'):
                original = (bundle / name).read_bytes()
                for suffix in (b'private-project-canary', b'/home/alice/build', b'C:\\Users\\alice\\build',
                               b'file%3A%2F%2F%2Fworkspace%2Fprivate', b'\xe2\x80\xaeprivate'):
                    (bundle / name).write_bytes(original + suffix)
                    rewrite_checksums(bundle)
                    with self.subTest(name=name), self.assertRaises(DISTRIBUTION.DistributionError) as raised:
                        DISTRIBUTION.verify_bundle(bundle)
                    self.assertNotIn('private-project-canary', str(raised.exception))
                (bundle / name).write_bytes(b'Notices removed.\n')
                rewrite_checksums(bundle)
                with self.assertRaises(DISTRIBUTION.DistributionError):
                    DISTRIBUTION.verify_bundle(bundle)
                (bundle / name).write_bytes(original)


def write_bundle(root: Path) -> dict[str, object]:
    source = root / "source"
    bundle = root / "bundle"
    source.mkdir()
    write_packages(source)
    manifest = DISTRIBUTION.build_public_distribution(
        source, bundle, VERSION, COMMIT, REPOSITORY, copy_packages=True
    )
    evidence = {
        **{name: (DISTRIBUTION.ROOT / source).read_bytes()
           for name, source in DISTRIBUTION.DOCUMENT_SOURCES.items()},
        "automexia-release-key.pub": b"untrusted comment: Automexia release verification key\nRWAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\n",
    }
    for name, data in evidence.items():
        (bundle / name).write_bytes(data)
    rewrite_checksums(bundle)
    (bundle / "SHA256SUMS.minisig").write_text(
        'untrusted comment: signature from minisign secret key\n' + 'A' * 88 + '\n'
        + f'trusted comment: Automexia Linux Early Access v{VERSION} source {COMMIT}\n'
        + 'A' * 88 + '\n', encoding='utf-8')
    return manifest


def repository_governance() -> dict[str, dict[str, object]]:
    return {
        "repository": {
            "visibility": "public",
            "has_issues": False,
            "has_projects": False,
            "has_wiki": False,
            "allow_squash_merge": True,
            "allow_merge_commit": False,
            "allow_rebase_merge": False,
        },
        "immutable": {"enabled": True},
        "main": {
            "id": 101, "node_id": "RRS_fixture_main",
            "name": "Protect main",
            "target": "branch",
            "enforcement": "active",
            "bypass_actors": [],
            "conditions": {"ref_name": {"include": ["~DEFAULT_BRANCH"], "exclude": []}},
            "rules": [
                {"type": "deletion"},
                {"type": "non_fast_forward"},
                {"type": "required_linear_history"},
                {
                    "type": "pull_request",
                    "parameters": {
                        "required_approving_review_count": 1,
                        "dismiss_stale_reviews_on_push": True,
                        "require_code_owner_review": True,
                        "require_last_push_approval": True,
                        "required_review_thread_resolution": True,
                        "allowed_merge_methods": ["squash"],
                    },
                },
                {"type": "required_signatures"},
            ],
        },
        "tag": {
            "id": 102, "node_id": "RRS_fixture_tag",
            "name": "Protect release tags",
            "target": "tag",
            "enforcement": "active",
            "bypass_actors": [],
            "conditions": {"ref_name": {"include": ["refs/tags/v*"], "exclude": []}},
            "rules": [
                {"type": "deletion"},
                {"type": "update"},
                {"type": "non_fast_forward"},
            ],
        },
    }


def graphql_governance() -> dict:
    return {"data": {"repository": {
        "nameWithOwner": REPOSITORY,
        "rulesets": {
            "pageInfo": {"hasNextPage": False},
            "nodes": [
                {"id": node, "databaseId": identity, "name": name,
                 "enforcement": "ACTIVE", "bypassActors": {
                     "totalCount": 0, "nodes": [],
                     "pageInfo": {"hasNextPage": False}}}
                for identity, node, name in (
                    (101, "RRS_fixture_main", "Protect main"),
                    (102, "RRS_fixture_tag", "Protect release tags"),
                )
            ],
        },
    }}}


class ReadOnlyGovernanceTests(unittest.TestCase):
    def verify(self, evidence: object, rest: dict | None = None) -> None:
        state = rest or repository_governance()
        if rest is None:
            for key in ("main", "tag"):
                state[key].pop("bypass_actors")
        DISTRIBUTION.validate_public_repository_governance(
            state["repository"], state["immutable"], state["main"], state["tag"],
            bypass_evidence=evidence,
        )

    def test_read_only_rest_omission_requires_independent_complete_graphql(self) -> None:
        # The actual App token omits REST bypass_actors, while GraphQL reports
        # real counts even when an individual actor's identity is redacted.
        self.verify(graphql_governance())
        with self.assertRaises(DISTRIBUTION.DistributionError):
            self.verify(None)

    def test_graphql_never_treats_redacted_or_inconsistent_actor_as_empty(self) -> None:
        for index in (0, 1):
            for count in (1, 2, 100, -1, True, False, "0", None):
                data = graphql_governance()
                actors = data["data"]["repository"]["rulesets"]["nodes"][index]["bypassActors"]
                actors.update(totalCount=count, nodes=[None])
                with self.subTest(index=index, count=count):
                    with self.assertRaises(DISTRIBUTION.DistributionError):
                        self.verify(data)
            for mutation in ({"nodes": [None]}, {"pageInfo": {"hasNextPage": True}},
                             {"nodes": None}, {"totalCount": False}):
                data = graphql_governance()
                data["data"]["repository"]["rulesets"]["nodes"][index]["bypassActors"].update(mutation)
                with self.assertRaises(DISTRIBUTION.DistributionError):
                    self.verify(data)

    def test_graphql_errors_identity_substitution_truncation_and_duplicates_fail(self) -> None:
        mutations = [
            lambda d: d.update(errors=[{"message": "private-canary"}]),
            lambda d: d.update(errors=[]),
            lambda d: d.update(data=None),
            lambda d: d["data"].update(repository=None),
            lambda d: d["data"]["repository"].update(nameWithOwner="alice/archive"),
            lambda d: d["data"]["repository"]["rulesets"]["pageInfo"].update(hasNextPage=True),
            lambda d: d["data"]["repository"]["rulesets"].update(nodes=[]),
            lambda d: d["data"]["repository"]["rulesets"]["nodes"].pop(),
            lambda d: d["data"]["repository"]["rulesets"]["nodes"].append(None),
            lambda d: d["data"]["repository"]["rulesets"]["nodes"].append(d["data"]["repository"]["rulesets"]["nodes"][0]),
        ]
        for field, value in (("id", "wrong"), ("databaseId", True), ("databaseId", 999),
                             ("name", "Wrong rule"), ("enforcement", "DISABLED"),
                             ("bypassActors", None)):
            mutations.append(lambda d, f=field, v=value: d["data"]["repository"]["rulesets"]["nodes"][0].update({f: v}))
        for mutate in mutations:
            data = graphql_governance()
            mutate(data)
            with self.assertRaises(DISTRIBUTION.DistributionError) as failure:
                self.verify(data)
            self.assertNotIn("private-canary", str(failure.exception))

    def test_rest_conflicts_cannot_be_overridden_by_graphql(self) -> None:
        for key in ("main", "tag"):
            for value in (None, [{"actor_type": "RepositoryRole"}], False):
                state = repository_governance()
                state[key]["bypass_actors"] = value
                with self.assertRaises(DISTRIBUTION.DistributionError):
                    self.verify(graphql_governance(), state)

    def test_bounded_complete_inventory_and_independent_rule_identities(self) -> None:
        data = graphql_governance()
        nodes = data["data"]["repository"]["rulesets"]["nodes"]
        for identity in range(103, 201):
            nodes.append({"databaseId": identity, "id": f"RRS_fixture_{identity}"})
        self.verify(data)
        nodes.append({"databaseId": 201, "id": "RRS_fixture_201"})
        with self.assertRaises(DISTRIBUTION.DistributionError):
            self.verify(data)
        for index in (0, 1):
            for field in ("id", "databaseId", "name", "enforcement", "bypassActors"):
                broken = graphql_governance()
                broken["data"]["repository"]["rulesets"]["nodes"][index].pop(field)
                with self.assertRaises(DISTRIBUTION.DistributionError):
                    self.verify(broken)

    def test_pagination_requires_explicit_boolean_false_not_numeric_zero(self) -> None:
        for value in (0, 1, None, "false", [], True):
            for actors in (False, True):
                evidence = graphql_governance()
                connection = evidence["data"]["repository"]["rulesets"]
                if actors:
                    connection = connection["nodes"][0]["bypassActors"]
                connection["pageInfo"]["hasNextPage"] = value
                with self.subTest(value=value, actors=actors):
                    with self.assertRaises(DISTRIBUTION.DistributionError):
                        self.verify(evidence)

    def test_real_governance_cli_rejects_missing_malformed_and_private_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            state = repository_governance()
            arguments = [sys.executable, str(MODULE_PATH), "verify-repository"]
            for key, flag in (("repository", "repository"), ("immutable", "immutable"),
                              ("main", "main-ruleset"), ("tag", "tag-ruleset")):
                value = state[key]
                if key in ("main", "tag"):
                    value.pop("bypass_actors")
                path = root / f"{key}.json"
                path.write_text(json.dumps(value), encoding="utf-8")
                arguments.extend([f"--{flag}-json", str(path)])
            evidence = root / "bypass.json"
            arguments.extend(["--ruleset-bypass-json", str(evidence)])
            evidence.write_text(json.dumps(graphql_governance()), encoding="utf-8")
            ok = subprocess.run(arguments, capture_output=True, text=True, timeout=10)
            self.assertEqual(ok.returncode, 0, ok.stderr)
            self.assertEqual(ok.stdout.strip(), "Public release repository governance passed")
            for content in (b'{"private-canary":1,"private-canary":2}', b'\xff',
                            b'{"data":NaN}', b' ' * (1024 * 1024 + 1),
                            json.dumps({"errors": [{"message": "private-canary"}]}).encode()):
                evidence.write_bytes(content)
                failed = subprocess.run(arguments, capture_output=True, text=True, timeout=10)
                self.assertEqual(failed.returncode, 1)
                self.assertNotIn("private-canary", failed.stdout + failed.stderr)
                self.assertNotIn(temporary, failed.stdout + failed.stderr)
                self.assertEqual(evidence.read_bytes(), content)
            evidence.unlink()
            failed = subprocess.run(arguments, capture_output=True, text=True, timeout=10)
            self.assertEqual(failed.returncode, 1)
            self.assertNotIn(temporary, failed.stderr)

    def test_workflow_cannot_skip_weaken_fake_or_move_bypass_evidence(self) -> None:
        text = DISTRIBUTION.PUBLIC_WORKFLOW.read_text(encoding="utf-8")
        fetch = 'timeout 30s gh api graphql -f query="$(python3 tools/ci/public_distribution.py ruleset-bypass-query)" > "$governance/ruleset-bypass.json"'
        argument = '--ruleset-bypass-json "$governance/ruleset-bypass.json"'
        mutations = [text.replace(fetch, replacement, 1) for replacement in (
            'echo bypass-not-checked', '# ' + fetch, 'true || ' + fetch,
            fetch + ' || true', fetch.replace('30s', '300s'),
            fetch.replace('gh api graphql', 'echo gh api graphql'),
            fetch + '\n          ' + fetch,
        )]
        mutations.extend(text.replace(argument, replacement, 1) for replacement in (
            '', argument + ' || true', '--ruleset-bypass-json other.json', '# ' + argument,
        ))
        mutations.append(text.replace('permission-administration: read', 'permission-administration: write', 1))
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'workflow.yml'
            for mutation in mutations:
                self.assertNotEqual(mutation, text)
                path.write_text(mutation, encoding='utf-8')
                with self.assertRaises(DISTRIBUTION.DistributionError):
                    DISTRIBUTION.validate_workflow(path)


class PublicDistributionTests(unittest.TestCase):
    def test_native_nfpm_revision_drives_exact_deb_and_rpm_names(self) -> None:
        contract = DISTRIBUTION._artifact_contract(VERSION)
        self.assertEqual(
            contract["linux-x64-deb"]["file"],
            f"automexia-terminal_{VERSION}-1_amd64.deb",
        )
        self.assertEqual(
            contract["linux-arm64-rpm"]["file"],
            f"automexia-terminal-{VERSION}-1.aarch64.rpm",
        )

        with tempfile.TemporaryDirectory() as temporary:
            config = Path(temporary) / "nfpm.yaml"
            config.write_text("name: automexia-terminal\nrelease: 7\n", encoding="utf-8")
            with mock.patch.object(DISTRIBUTION, "NFPM_CONFIG", config):
                changed = DISTRIBUTION._artifact_contract(VERSION)
            self.assertEqual(
                changed["linux-x64-deb"]["file"],
                f"automexia-terminal_{VERSION}-7_amd64.deb",
            )
            self.assertEqual(
                changed["linux-arm64-rpm"]["file"],
                f"automexia-terminal-{VERSION}-7.aarch64.rpm",
            )

            config.write_text("release: 1\nrelease: 2\n", encoding="utf-8")
            with mock.patch.object(DISTRIBUTION, "NFPM_CONFIG", config):
                with self.assertRaisesRegex(
                    DISTRIBUTION.DistributionError, "one positive"
                ):
                    DISTRIBUTION._artifact_contract(VERSION)

            config.write_text("release: 0\n", encoding="utf-8")
            with mock.patch.object(DISTRIBUTION, "NFPM_CONFIG", config):
                with self.assertRaisesRegex(
                    DISTRIBUTION.DistributionError, "one positive"
                ):
                    DISTRIBUTION._artifact_contract(VERSION)

            config.write_text("#" * (64 * 1024 + 1), encoding="utf-8")
            with mock.patch.object(DISTRIBUTION, "NFPM_CONFIG", config):
                with self.assertRaisesRegex(
                    DISTRIBUTION.DistributionError, "byte limit"
                ):
                    DISTRIBUTION._artifact_contract(VERSION)

            missing = Path(temporary) / "missing.yaml"
            with mock.patch.object(DISTRIBUTION, "NFPM_CONFIG", missing):
                with self.assertRaisesRegex(
                    DISTRIBUTION.DistributionError, "unavailable"
                ):
                    DISTRIBUTION._artifact_contract(VERSION)

    def test_public_repository_governance_fails_closed_on_every_release_control(self) -> None:
        baseline = repository_governance()
        DISTRIBUTION.validate_public_repository_governance(
            baseline["repository"], baseline["immutable"], baseline["main"], baseline["tag"]
        )
        mutations = {
            "private archive": lambda state: state["repository"].__setitem__("visibility", "private"),
            "interactive issues": lambda state: state["repository"].__setitem__("has_issues", True),
            "mutable releases": lambda state: state["immutable"].__setitem__("enabled", False),
            "main bypass": lambda state: state["main"].__setitem__(
                "bypass_actors", [{"actor_type": "OrganizationAdmin"}]
            ),
            "main scope": lambda state: state["main"]["conditions"]["ref_name"].__setitem__(
                "include", ["refs/heads/release"]
            ),
            "no approval": lambda state: state["main"]["rules"][3]["parameters"].__setitem__(
                "required_approving_review_count", 0
            ),
            "no code owner": lambda state: state["main"]["rules"][3]["parameters"].__setitem__(
                "require_code_owner_review", False
            ),
            "merge commit": lambda state: state["main"]["rules"][3]["parameters"].__setitem__(
                "allowed_merge_methods", ["squash", "merge"]
            ),
            "unsigned main": lambda state: state["main"]["rules"].pop(4),
            "tag scope": lambda state: state["tag"]["conditions"]["ref_name"].__setitem__(
                "include", ["refs/tags/latest"]
            ),
            "rewritable tag": lambda state: state["tag"]["rules"].pop(1),
        }
        for label, mutate in mutations.items():
            with self.subTest(label=label):
                state = copy.deepcopy(baseline)
                mutate(state)
                with self.assertRaises(DISTRIBUTION.DistributionError):
                    DISTRIBUTION.validate_public_repository_governance(
                        state["repository"], state["immutable"], state["main"], state["tag"]
                    )

    def test_exact_six_package_manifest_is_deterministic(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            output = root / "output"
            source.mkdir()
            write_packages(source)

            first = DISTRIBUTION.build_public_distribution(
                source,
                output,
                VERSION,
                COMMIT,
                REPOSITORY,
                copy_packages=True,
            )
            second = DISTRIBUTION.build_public_distribution(
                output,
                output,
                VERSION,
                COMMIT,
                REPOSITORY,
            )
            self.assertEqual(first, second)
            self.assertEqual(len(first["artifacts"]), 6)
            self.assertEqual(
                [artifact["id"] for artifact in first["artifacts"]],
                [
                    "linux-arm64-deb",
                    "linux-arm64-portable",
                    "linux-arm64-rpm",
                    "linux-x64-deb",
                    "linux-x64-portable",
                    "linux-x64-rpm",
                ],
            )
            for artifact in first["artifacts"]:
                self.assertEqual(
                    artifact["download_url"],
                    (
                        f"https://github.com/{REPOSITORY}/releases/download/"
                        f"v{VERSION}/{artifact['file']}"
                    ),
                )
                self.assertEqual(
                    artifact["versioned_public_path"],
                    f"/download/v{VERSION}/{artifact['id']}",
                )
            self.assertEqual(
                json.loads(
                    (output / "public-distribution-manifest-v2.json").read_text(
                        encoding="utf-8"
                    )
                ),
                first,
            )

    def test_missing_duplicate_unowned_and_forbidden_files_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            source.mkdir()
            write_packages(source)
            next(source.rglob("*arm64.deb")).unlink()
            with self.assertRaisesRegex(DISTRIBUTION.DistributionError, "incomplete"):
                DISTRIBUTION.build_public_distribution(
                    source, root / "out", VERSION, COMMIT, REPOSITORY, copy_packages=True
                )

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            source.mkdir()
            write_packages(source)
            duplicate = (
                source / "duplicate" / f"automexia-terminal_{VERSION}-1_amd64.deb"
            )
            duplicate.parent.mkdir()
            duplicate.write_bytes(b"duplicate\n")
            with self.assertRaisesRegex(DISTRIBUTION.DistributionError, "exactly one"):
                DISTRIBUTION.build_public_distribution(
                    source, root / "out", VERSION, COMMIT, REPOSITORY, copy_packages=True
                )

        for name in (
            f"automexia-terminal-{VERSION}-x86_64.AppImage",
            f"automexia-terminal-{VERSION}-debug.pdb",
            f"automexia-terminal-{VERSION}-x86_64-pc-windows-msvc.zip",
        ):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                source = root / "source"
                source.mkdir()
                write_packages(source)
                (source / name).write_bytes(b"unexpected\n")
                with self.assertRaisesRegex(
                    DISTRIBUTION.DistributionError, "unexpected|forbidden"
                ):
                    DISTRIBUTION.build_public_distribution(
                        source,
                        root / "out",
                        VERSION,
                        COMMIT,
                        REPOSITORY,
                        copy_packages=True,
                    )

    def test_repository_tag_links_and_resource_limits_are_strict(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_packages(root)
            for repository in (
                "other/repository",
                "AmjedAllaya/automexia-releases.git",
                "AmjedAllaya/../release",
            ):
                with self.subTest(repository=repository), self.assertRaisesRegex(
                    DISTRIBUTION.DistributionError, "repository"
                ):
                    DISTRIBUTION.build_public_distribution(
                        root, root / "out", VERSION, COMMIT, repository
                    )
            with self.assertRaisesRegex(DISTRIBUTION.DistributionError, "total byte"):
                DISTRIBUTION.build_public_distribution(
                    root,
                    root / "out",
                    VERSION,
                    COMMIT,
                    REPOSITORY,
                    max_total_bytes=1,
                )

    def test_linked_package_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_packages(root)
            original = next(root.rglob("*.deb"))
            linked = original.with_name("linked-" + original.name)
            try:
                os.link(original, linked)
            except OSError:
                self.skipTest("hard links are unavailable on this host")
            with self.assertRaisesRegex(
                DISTRIBUTION.DistributionError, "regular unlinked file"
            ):
                DISTRIBUTION.build_public_distribution(
                    root, root / "out", VERSION, COMMIT, REPOSITORY
                )

    def test_release_payload_requires_immutable_exact_assets_and_digests(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = write_bundle(root)
            bundle = root / "bundle"
            DISTRIBUTION.verify_release_payload(
                manifest, release_payload(manifest, bundle), bundle=bundle, release_id=7
            )

            mutations = {
                "mutable": lambda payload: payload.update(immutable=False),
                "draft": lambda payload: payload.update(draft=True),
                "stable": lambda payload: payload.update(prerelease=False),
                "wrong tag": lambda payload: payload.update(tag_name="v9.9.9"),
                "extra executable": lambda payload: payload["assets"].append(
                    {
                        "name": "debug.exe",
                        "size": 1,
                        "digest": f"sha256:{'b' * 64}",
                        "state": "uploaded",
                        "browser_download_url": (
                            f"https://github.com/{REPOSITORY}/releases/download/"
                            f"v{VERSION}/debug.exe"
                        ),
                    }
                ),
                "wrong digest": lambda payload: next(
                    asset
                    for asset in payload["assets"]
                    if asset["name"].endswith("_amd64.deb")
                ).update(digest=f"sha256:{'c' * 64}"),
                "wrong evidence digest": lambda payload: next(
                    asset
                    for asset in payload["assets"]
                    if asset["name"] == "INSTALL.md"
                ).update(digest=f"sha256:{'d' * 64}"),
            }
            for label, mutate in mutations.items():
                with self.subTest(label=label):
                    payload = release_payload(manifest, bundle)
                    mutate(payload)
                    with self.assertRaises(DISTRIBUTION.DistributionError):
                        DISTRIBUTION.verify_release_payload(
                            manifest, payload, bundle=bundle, release_id=7
                        )

            draft = release_payload(manifest, bundle)
            draft.update(draft=True, immutable=False)
            DISTRIBUTION.verify_release_payload(
                manifest, draft, stage="draft", bundle=bundle, release_id=7,
                created_url=draft["html_url"],
            )

            malformed = json.loads(json.dumps(manifest))
            malformed["evidence"]["checksum"] = "checksums.txt"
            with self.assertRaisesRegex(
                DISTRIBUTION.DistributionError, "evidence contract"
            ):
                DISTRIBUTION.verify_release_payload(
                    malformed, release_payload(manifest, bundle), bundle=bundle, release_id=7
                )

    def test_real_draft_locator_and_same_identity_publication_transition(self) -> None:
        # GitHub assigns this bounded temporary locator before a new tag exists;
        # neither the tag endpoint nor final asset URLs describe that draft.
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = write_bundle(root)
            bundle = root / "bundle"
            published = release_payload(manifest, bundle)
            draft = copy.deepcopy(published)
            locator = "untagged-0123456789abcdef0123"
            created_url = f"https://github.com/{REPOSITORY}/releases/tag/{locator}"
            draft.update(draft=True, immutable=False, html_url=created_url)
            for asset in draft["assets"]:
                asset["browser_download_url"] = (
                    f"https://github.com/{REPOSITORY}/releases/download/{locator}/{asset['name']}"
                )
            DISTRIBUTION.verify_release_payload(
                manifest, draft, bundle=bundle, stage="draft", release_id=7,
                created_url=created_url,
            )
            DISTRIBUTION.verify_release_payload(
                manifest, published, bundle=bundle, release_id=7,
            )
            faults = {
                "different release": lambda item: item.update(id=8),
                "boolean identity": lambda item: item.update(id=True),
                "numeric string identity": lambda item: item.update(id="7"),
                "missing identity": lambda item: item.pop("id"),
                "different tag": lambda item: item.update(tag_name="v9.9.9"),
                "different draft locator": lambda item: item.update(html_url=created_url + "0"),
                "different repository": lambda item: item.update(html_url=created_url.replace(REPOSITORY, "example/releases")),
                "unbound asset locator": lambda item: item["assets"][0].update(browser_download_url=published["assets"][0]["browser_download_url"]),
                "asset digest changed": lambda item: item["assets"][0].update(digest="sha256:" + "e" * 64),
                "published too soon": lambda item: item.update(draft=False, immutable=True),
            }
            for label, mutate in faults.items():
                with self.subTest(label=label):
                    broken = copy.deepcopy(draft)
                    mutate(broken)
                    with self.assertRaises(DISTRIBUTION.DistributionError):
                        DISTRIBUTION.verify_release_payload(
                            manifest, broken, bundle=bundle, stage="draft", release_id=7,
                            created_url=created_url,
                        )
            for identity in (None, True, 0, -1, "7", 10**20):
                with self.subTest(identity=identity):
                    with self.assertRaises(DISTRIBUTION.DistributionError):
                        DISTRIBUTION.verify_release_payload(
                            manifest, draft, bundle=bundle, stage="draft", release_id=identity,
                            created_url=created_url,
                        )
            for created in (None, "", created_url + "?redirect=1", created_url.replace("https:", "http:")):
                with self.subTest(created=created):
                    with self.assertRaises(DISTRIBUTION.DistributionError):
                        DISTRIBUTION.verify_release_payload(
                            manifest, draft, bundle=bundle, stage="draft", release_id=7,
                            created_url=created,
                        )
            # A mutable draft locator can never escape into public availability.
            for stage in ("published", "invalid"):
                with self.assertRaises(DISTRIBUTION.DistributionError):
                    DISTRIBUTION.verify_release_payload(
                        manifest, draft, bundle=bundle, stage=stage, release_id=7,
                    )
            for suffix in ("", "0" * 19, "0" * 21, "g" * 20, "0" * 20 + "?q=1", "../v1.2.3"):
                broken = copy.deepcopy(draft)
                created = f"https://github.com/{REPOSITORY}/releases/tag/untagged-{suffix}"
                broken["html_url"] = created
                with self.assertRaises(DISTRIBUTION.DistributionError):
                    DISTRIBUTION.verify_release_payload(
                        manifest, broken, bundle=bundle, stage="draft", release_id=7,
                        created_url=created,
                    )

    def test_bundle_checksums_and_activation_handoff_are_exact(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = write_bundle(root)
            self.assertEqual(DISTRIBUTION.verify_bundle(root / "bundle"), manifest)
            handoff_path = root / "website-activation.json"
            handoff = DISTRIBUTION.write_activation_handoff(
                root / "bundle" / DISTRIBUTION.MANIFEST_NAME, handoff_path
            )
            self.assertEqual(handoff["status"], "verified")
            self.assertEqual(handoff["sourceCommit"], COMMIT)
            self.assertRegex(handoff["manifestSha256"], r"^[0-9a-f]{64}$")
            self.assertEqual(json.loads(handoff_path.read_text(encoding="utf-8")), handoff)

            # Changing one covered byte while leaving SHA256SUMS untouched must fail.
            (root / "bundle" / "INSTALL.md").write_bytes(b"changed\n")
            with self.assertRaisesRegex(DISTRIBUTION.DistributionError, "checksum mismatch"):
                DISTRIBUTION.verify_bundle(root / "bundle")

    def test_bundle_rejects_local_metadata_even_when_checksums_match(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_bundle(root)
            bundle = root / "bundle"
            original = json.dumps(sbom_documents()[1]).encode()
            for location in ("file", "nested", "key"):
                with self.subTest(location=location):
                    cdx = json.loads(original)
                    # Runtime-created private-like paths must not be printed or retained.
                    canary = str(root / "scan" / "Cargo.lock")
                    if location == "file":
                        cdx["components"][-1]["name"] = canary
                    elif location == "key":
                        cdx["metadata"][canary] = "hidden"
                    else:
                        cdx["components"][0]["properties"] = [{"name": "unexpected", "value": canary}]
                    (bundle / "automexia-terminal.cdx.json").write_text(json.dumps(cdx), encoding="utf-8")
                    rewrite_checksums(bundle)
                    with self.assertRaisesRegex(DISTRIBUTION.DistributionError, "allowlist") as raised:
                        DISTRIBUTION.verify_bundle(bundle)
                    self.assertNotIn(canary, str(raised.exception))

    def test_bundle_rejects_header_only_or_wrong_product_sboms(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_bundle(root)
            bundle = root / "bundle"
            for cdx in ({"bomFormat": "CycloneDX"}, sbom_documents()[1]):
                if "components" in cdx:
                    cdx["components"][0]["version"] = "9.9.9"
                (bundle / "automexia-terminal.cdx.json").write_text(json.dumps(cdx), encoding="utf-8")
                rewrite_checksums(bundle)
                with self.assertRaisesRegex(DISTRIBUTION.DistributionError, "allowlist"):
                    DISTRIBUTION.verify_bundle(bundle)

    def test_bundle_rejects_extra_symbols_and_checksum_path_records(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_bundle(root)
            (root / "bundle" / "debug.pdb").write_bytes(b"symbols\n")
            with self.assertRaisesRegex(DISTRIBUTION.DistributionError, "allowlist"):
                DISTRIBUTION.verify_bundle(root / "bundle")

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write_bundle(root)
            checksum = root / "bundle" / "SHA256SUMS"
            checksum.write_text(
                "a" * 64 + "  ../INSTALL.md\n", encoding="utf-8", newline="\n"
            )
            with self.assertRaisesRegex(DISTRIBUTION.DistributionError, "invalid|cover|canonical"):
                DISTRIBUTION.verify_bundle(root / "bundle")

    def test_draft_cli_requires_created_identity_and_preserves_files_on_failure(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            manifest = write_bundle(root)
            bundle = root / "bundle"
            payload = release_payload(manifest, bundle)
            payload.update(draft=True, immutable=False)
            payload_file = root / "draft.json"
            payload_file.write_text(json.dumps(payload), encoding="utf-8")
            arguments = [sys.executable, str(MODULE_PATH), "verify-release", "--manifest",
                         str(bundle / DISTRIBUTION.MANIFEST_NAME), "--release-json", str(payload_file),
                         "--bundle", str(bundle), "--stage", "draft", "--release-id", "7",
                         "--created-url", payload["html_url"]]
            passed = subprocess.run(arguments, capture_output=True, text=True, timeout=10)
            self.assertEqual(passed.returncode, 0, passed.stderr)
            before = {path.relative_to(root): path.read_bytes() for path in root.rglob("*") if path.is_file()}
            for label, change in {
                "missing creation URL": lambda argv: argv[:-2],
                "different release ID": lambda argv: [*argv[:-3], "8", *argv[-2:]],
                "missing release ID": lambda argv: [*argv[:-4], *argv[-2:]],
            }.items():
                with self.subTest(label=label):
                    failed = subprocess.run(change(arguments), capture_output=True, text=True, timeout=10)
                    self.assertNotEqual(failed.returncode, 0)
                    self.assertEqual(before, {path.relative_to(root): path.read_bytes() for path in root.rglob("*") if path.is_file()})

    def test_workflow_binds_new_draft_and_publication_to_one_numeric_identity(self) -> None:
        workflow = DISTRIBUTION.PUBLIC_WORKFLOW.read_text(encoding="utf-8")
        mutations = {
            "tag endpoint cannot find new draft": ('releases/$release_id" > "$RUNNER_TEMP/release-draft.json"', 'releases/tags/$tag" > "$RUNNER_TEMP/release-draft.json"'),
            "unbounded numeric ID": ('^[1-9][0-9]{0,19}$', '^[0-9]+$'),
            "unbound draft identity": ('--release-id "$release_id" --created-url', '--release-id 7 --created-url'),
            "unbound creation result": ('--created-url "$created_release_url"', '--created-url "$tag"'),
            "publish by unrelated identity": ('--method PATCH "repos/$PUBLIC_REPOSITORY/releases/$release_id"', '--method PATCH "repos/$PUBLIC_REPOSITORY/releases/7"'),
            "still a draft": ('-F draft=false', '-F draft=true'),
            "stable rather than prerelease": ('-F prerelease=true', '-F prerelease=false'),
            "unexpected latest channel": ('-f make_latest=false', '-f make_latest=true'),
            "skip lookup": ('release_id="$(gh release view', 'true || release_id="$(gh release view'),
            "skip creation": ('created_release_url="$(gh release create', 'true || created_release_url="$(gh release create'),
            "masked publish failure": ('gh api -H "$api_header" --method PATCH', 'true || gh api -H "$api_header" --method PATCH'),
        }
        with tempfile.TemporaryDirectory() as temporary:
            candidate = Path(temporary) / "workflow.yml"
            for label, (original, replacement) in mutations.items():
                with self.subTest(label=label):
                    self.assertIn(original, workflow)
                    candidate.write_text(workflow.replace(original, replacement, 1), encoding="utf-8")
                    with self.assertRaises(DISTRIBUTION.DistributionError):
                        DISTRIBUTION.validate_workflow(candidate)
            for owner in ('release_id="$(gh release view', '[[ "$release_id"', '--stage draft', '--method PATCH', 'releases/$release_id" > "$RUNNER_TEMP/release-published.json"'):
                line = next(line for line in workflow.splitlines(keepends=True) if owner in line)
                for replacement in ("", line + line, line.replace(line.lstrip(), "# " + line.lstrip())):
                    candidate.write_text(workflow.replace(line, replacement, 1), encoding="utf-8")
                    with self.assertRaises(DISTRIBUTION.DistributionError):
                        DISTRIBUTION.validate_workflow(candidate)
            draft_line = next(line for line in workflow.splitlines(keepends=True) if '--stage draft' in line)
            publish_line = next(line for line in workflow.splitlines(keepends=True) if '--method PATCH' in line)
            candidate.write_text(workflow.replace(draft_line + publish_line, publish_line + draft_line), encoding="utf-8")
            with self.assertRaises(DISTRIBUTION.DistributionError):
                DISTRIBUTION.validate_workflow(candidate)

    def test_workflow_policy_rejects_publication_gate_removal(self) -> None:
        DISTRIBUTION.validate_workflow()
        workflow = DISTRIBUTION.PUBLIC_WORKFLOW.read_text(encoding="utf-8")
        package_start = workflow.index('\n  package:\n')
        package_end = workflow.index('\n  rehearsal:\n', package_start)
        package = workflow[package_start:package_end]
        self.assertEqual(package.count('            ~/.cargo/git\n'), 1)
        # Target the owning cache, not whichever setup step happens to follow it.
        cached_package = package.replace('            ~/.cargo/git\n',
                                         '            ~/.cargo/git\n            target\n', 1)
        mutations = {
            "immutable repository audit": workflow.replace(
                "repos/$PUBLIC_REPOSITORY/immutable-releases", "repos/$PUBLIC_REPOSITORY", 1
            ),
            "draft verification": workflow.replace("--stage draft", "--stage published", 1),
            "repository scope": workflow.replace(
                "repositories: automexia-releases", "repositories: automexia-terminal", 1
            ),
            "repository governance": workflow.replace(
                "tools/ci/public_distribution.py verify-repository",
                "echo 'governance not checked'",
                1,
            ),
            "create once": workflow.replace(
                "A release or tag already exists in the public repository",
                "existing state ignored",
                1,
            ),
            "manual rehearsal": workflow.replace("workflow_dispatch:", "manual_rehearsal:", 1),
            "manual dispatch mode": workflow.replace("publish=false", "publish=true", 1),
            "rehearsal publication isolation": workflow.replace(
                "needs.authorize.outputs.publish == 'false'",
                "needs.authorize.outputs.publish == 'true'",
                1,
            ),
            "assemble publication gate": workflow.replace(
                "  assemble:\n    name: Assemble and sign public Linux bundle\n"
                "    needs: [authorize, quality, package]\n"
                "    if: needs.authorize.outputs.publish == 'true'",
                "  assemble:\n    name: Assemble and sign public Linux bundle\n"
                "    needs: [authorize, quality, package]\n    if: always()",
                1,
            ),
            "publish publication gate": workflow.replace(
                "  publish:\n    name: Publish immutable public Linux prerelease\n"
                "    needs: [authorize, assemble]\n"
                "    if: needs.authorize.outputs.publish == 'true'",
                "  publish:\n    name: Publish immutable public Linux prerelease\n"
                "    needs: [authorize, assemble]\n    if: always()",
                1,
            ),
            "rehearsal warning": workflow.replace(
                "REHEARSAL-NOT-A-PUBLIC-RELEASE.txt",
                "rehearsal.txt",
            ),
            "API version": workflow.replace(
                "X-GitHub-Api-Version: 2026-03-10",
                "X-GitHub-Api-Version: 2022-11-28",
                1,
            ),
            "release attestation": workflow.replace(
                'gh release verify "$tag"',
                'echo "release not verified"',
                1,
            ),
            "asset attestation": workflow.replace(
                'gh release verify-asset "$tag" "$asset"',
                'echo "asset not verified"',
                1,
            ),
            "partial asset attestation": workflow.replace(
                'for asset in "${assets[@]}"; do',
                'for asset in "${assets[0]}"; do',
                1,
            ),
            "release quality build parallelism": workflow.replace(
                "CARGO_BUILD_JOBS: '1'",
                "CARGO_BUILD_JOBS: '2'",
                1,
            ),
            "release quality development debug info": workflow.replace(
                "CARGO_PROFILE_DEV_DEBUG: '0'",
                "CARGO_PROFILE_DEV_DEBUG: '1'",
                1,
            ),
            "release quality test debug info": workflow.replace(
                "CARGO_PROFILE_TEST_DEBUG: '0'",
                "CARGO_PROFILE_TEST_DEBUG: '1'",
                1,
            ),
            "release quality test parallelism": workflow.replace(
                "NEXTEST_TEST_THREADS: '1'",
                "NEXTEST_TEST_THREADS: '2'",
                1,
            ),
            "release quality lint cleanup": workflow.replace(
                "        run: cargo clean\n",
                "        run: cargo metadata --locked --format-version 1\n",
                1,
            ),
            "release quality source cache": workflow.replace(
                "actions/cache@55cc8345863c7cc4c66a329aec7e433d2d1c52a9",
                "actions/cache@1111111111111111111111111111111111111111",
                1,
            ),
            "release quality target cache": workflow.replace(
                "            ~/.cargo/git\n",
                "            ~/.cargo/git\n            target\n",
                1,
            ),
            "release quality stale cache identity": workflow.replace(
                "${{ hashFiles('Cargo.lock') }}",
                "static-lock-identity",
                1,
            ),
            "release quality cleanup ordering": workflow.replace(
                "      - name: Reclaim release lint artifacts before the all-feature "
                "test build\n"
                "        run: cargo clean\n"
                "      - name: Workspace unit and integration tests\n"
                "        run: cargo nextest run --workspace --all-features --locked "
                "--profile ci\n",
                "      - name: Workspace unit and integration tests\n"
                "        run: cargo nextest run --workspace --all-features --locked "
                "--profile ci\n"
                "      - name: Reclaim release lint artifacts after the all-feature "
                "test build\n"
                "        run: cargo clean\n",
                1,
            ),
            "release quality compiler cache pin": workflow.replace(
                "fc920bf0ec8de6ee65d409111f7ec508035751ba",
                "1111111111111111111111111111111111111111",
                1,
            ),
            "release quality compiler cache version": workflow.replace(
                "version: v0.16.0", "version: v0.15.0", 1
            ),
            "release quality compiler cache backend": workflow.replace(
                "SCCACHE_GHA_ENABLED: 'true'", "SCCACHE_GHA_ENABLED: 'false'", 1
            ),
            "release quality compiler cache namespace": workflow.replace(
                "SCCACHE_GHA_VERSION: automexia-rust-1.96.1-v2",
                "SCCACHE_GHA_VERSION: unversioned",
                1,
            ),
            "release quality compiler cache stats": workflow.replace(
                "run: sccache --show-stats", "run: echo stats-skipped", 1
            ),
            "serialized native package build": workflow.replace(
                "  package:\n    name: Native Linux package build\n    needs: authorize",
                "  package:\n    name: Native Linux package build\n"
                "    needs: [authorize, quality]",
                1,
            ),
            "rehearsal missing quality join": workflow.replace(
                "needs: [authorize, quality, package]",
                "needs: [authorize, package]",
                1,
            ),
            "assemble missing quality join": workflow.replace(
                "  assemble:\n    name: Assemble and sign public Linux bundle\n"
                "    needs: [authorize, quality, package]",
                "  assemble:\n    name: Assemble and sign public Linux bundle\n"
                "    needs: [authorize, package]",
                1,
            ),
            "native package compiler cache": workflow.replace(
                "      NFPM_VERSION: '2.43.4'\n",
                "      NFPM_VERSION: '2.43.4'\n      RUSTC_WRAPPER: sccache\n",
                1,
            ),
            "native package nFPM digest": workflow.replace(
                "cafb544650cb0305d1b164fc0ab261eb77a81af324e18011282d326b326d20fb",
                "1" * 64,
                1,
            ),
            "native package Arm64 nFPM digest": workflow.replace(
                "e4365707dedfda6e089f597dcdab9497beea80accb2c2704be18981e4a4d9b9b",
                "2" * 64,
                1,
            ),
            "native package nFPM architecture": workflow.replace(
                "nfpm_arch: arm64", "nfpm_arch: x86_64", 1
            ),
            "native package nFPM checksum": workflow.replace(
                "sha256sum --check", "echo checksum-skipped", 1
            ),
            "native package source cache Action": workflow.replace(
                "          ref: ${{ needs.authorize.outputs.commit }}\n"
                "      - name: Cache Cargo registry and Git sources\n"
                "        uses: actions/cache@55cc8345863c7cc4c66a329aec7e433d2d1c52a9",
                "          ref: ${{ needs.authorize.outputs.commit }}\n"
                "      - name: Cache Cargo registry and Git sources\n"
                "        uses: actions/cache@1111111111111111111111111111111111111111",
                1,
            ),
            "native package target cache": workflow.replace(
                package, cached_package, 1,
            ),
            "native package nFPM download origin": workflow.replace(
                "https://github.com/goreleaser/nfpm/releases/download/",
                "https://example.invalid/nfpm/",
                1,
            ),
            "native package nFPM extraction scope": workflow.replace(
                ' --directory "$RUNNER_TEMP/nfpm-bin" nfpm',
                ' --directory "$RUNNER_TEMP/nfpm-bin"',
                1,
            ),
            "native package build parallelism": workflow.replace(
                "      AUTOMEXIA_VERSION: ${{ needs.authorize.outputs.version }}\n"
                "      CARGO_BUILD_JOBS: '1'\n",
                "      AUTOMEXIA_VERSION: ${{ needs.authorize.outputs.version }}\n"
                "      CARGO_BUILD_JOBS: '2'\n",
                1,
            ),
            "native package release debug info": workflow.replace(
                "      CARGO_PROFILE_RELEASE_DEBUG: '0'\n",
                "      CARGO_PROFILE_RELEASE_DEBUG: '1'\n",
                1,
            ),
            "credential leak into rehearsal": workflow.replace(
                "\n  assemble:",
                "\n      - run: echo '${{ secrets.TEST_PRIVATE_KEY }}'\n\n  assemble:",
                1,
            ),
        }
        for label, mutated in mutations.items():
            with self.subTest(label=label), tempfile.TemporaryDirectory() as temporary:
                self.assertNotEqual(
                    workflow,
                    mutated,
                    f"{label} mutation did not alter the workflow fixture",
                )
                path = Path(temporary) / "workflow.yml"
                path.write_text(mutated, encoding="utf-8")
                if label == 'native package target cache':
                    with self.assertRaisesRegex(DISTRIBUTION.DistributionError,
                                                'native package cache must not retain target build artifacts'):
                        DISTRIBUTION.validate_workflow(path)
                with self.assertRaises(DISTRIBUTION.DistributionError):
                    DISTRIBUTION.validate_workflow(path)

    def test_workflow_rehearses_without_credentials_and_verifies_attestations(self) -> None:
        workflow = DISTRIBUTION.PUBLIC_WORKFLOW.read_text(encoding="utf-8")
        for required in (
            "workflow_dispatch:",
            "needs.authorize.outputs.publish == 'false'",
            "REHEARSAL-NOT-A-PUBLIC-RELEASE.txt",
            'gh release verify "$tag"',
            'gh release verify-asset "$tag" "$asset"',
            'X-GitHub-Api-Version: 2026-03-10',
            "CARGO_BUILD_JOBS: '1'",
            "CARGO_PROFILE_DEV_DEBUG: '0'",
            "CARGO_PROFILE_TEST_DEBUG: '0'",
            "NEXTEST_TEST_THREADS: '1'",
            "Reclaim release lint artifacts before the all-feature test build",
            "actions/cache@55cc8345863c7cc4c66a329aec7e433d2d1c52a9",
            "~/.cargo/registry",
            "~/.cargo/git",
            "hashFiles('Cargo.lock')",
        ):
            with self.subTest(required=required):
                self.assertIn(required, workflow)
        self.assertLess(
            workflow.index("--stage published"),
            workflow.index('gh release verify "$tag"'),
        )
        self.assertLess(
            workflow.index('gh release verify "$tag"'),
            workflow.index("activation-handoff"),
        )
        self.assertEqual(workflow.count("CARGO_BUILD_JOBS: '1'"), 2)
        self.assertEqual(workflow.count("CARGO_PROFILE_RELEASE_DEBUG: '0'"), 1)

    def test_workflow_parallelizes_cold_packages_without_caching_shipped_objects(self) -> None:
        workflow = DISTRIBUTION.PUBLIC_WORKFLOW.read_text(encoding="utf-8")
        quality = workflow.split("  quality:\n", 1)[1].split("  package:\n", 1)[0]
        package = workflow.split("  package:\n", 1)[1].split("  rehearsal:\n", 1)[0]
        rehearsal = workflow.split("  rehearsal:\n", 1)[1].split("  assemble:\n", 1)[0]
        assemble = workflow.split("  assemble:\n", 1)[1].split("  publish:\n", 1)[0]

        for token in (
            "mozilla-actions/sccache-action@fc920bf0ec8de6ee65d409111f7ec508035751ba",
            "version: v0.16.0",
            "SCCACHE_GHA_ENABLED: 'true'",
            "SCCACHE_GHA_VERSION: automexia-rust-1.96.1-v2",
            "RUSTC_WRAPPER: sccache",
            "sccache --show-stats",
        ):
            with self.subTest(quality_token=token):
                self.assertEqual(quality.count(token), 1)

        self.assertIn("    needs: authorize\n", package)
        self.assertNotIn("needs: [authorize, quality]", package)
        self.assertIn("needs: [authorize, quality, package]", rehearsal)
        self.assertIn("needs: [authorize, quality, package]", assemble)
        self.assertNotIn("sccache", package.casefold())
        self.assertNotIn("RUSTC_WRAPPER", package)
        self.assertNotRegex(package, r"(?m)^\s+target(?:/.*)?\s*$")

        shared_source_key = (
            "cargo-sources-v1-${{ runner.os }}-${{ hashFiles('Cargo.lock') }}"
        )
        self.assertEqual(quality.count(shared_source_key), 1)
        self.assertEqual(package.count(shared_source_key), 1)
        self.assertNotIn("go install github.com/goreleaser/nfpm", package)
        for digest in (
            "cafb544650cb0305d1b164fc0ab261eb77a81af324e18011282d326b326d20fb",
            "e4365707dedfda6e089f597dcdab9497beea80accb2c2704be18981e4a4d9b9b",
        ):
            self.assertEqual(package.count(digest), 1)
        self.assertIn("nfpm_${NFPM_VERSION}_Linux_${{ matrix.nfpm_arch }}.tar.gz", package)
        self.assertIn("sha256sum --check", package)


if __name__ == "__main__":
    unittest.main(verbosity=2)
