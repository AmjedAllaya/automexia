#!/usr/bin/env python3
"""Regression tests for the bounded Linux Early Access distribution contract."""

from __future__ import annotations

import copy
import importlib.util
import hashlib
import json
import os
from pathlib import Path
import tempfile
import unittest


MODULE_PATH = Path(__file__).with_name("public_distribution.py")
SPEC = importlib.util.spec_from_file_location("automexia_public_distribution", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load tools/ci/public_distribution.py")
DISTRIBUTION = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DISTRIBUTION)

VERSION = "1.2.3"
COMMIT = "0123456789abcdef0123456789abcdef01234567"
REPOSITORY = "AmjedAllaya/automexia-releases"


def package_names() -> tuple[str, ...]:
    return (
        f"automexia-terminal_{VERSION}_amd64.deb",
        f"automexia-terminal_{VERSION}_arm64.deb",
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
        "tag_name": f"v{VERSION}",
        "draft": False,
        "prerelease": True,
        "immutable": True,
        "html_url": f"https://github.com/{REPOSITORY}/releases/tag/v{VERSION}",
        "assets": assets,
    }


def write_bundle(root: Path) -> dict[str, object]:
    source = root / "source"
    bundle = root / "bundle"
    source.mkdir()
    write_packages(source)
    manifest = DISTRIBUTION.build_public_distribution(
        source, bundle, VERSION, COMMIT, REPOSITORY, copy_packages=True
    )
    evidence = {
        "INSTALL.md": b"Install fixture.\n",
        "RELEASE-NOTES.md": b"Release fixture.\n",
        "THIRD_PARTY_NOTICES.md": b"Notices fixture.\n",
        "UNINSTALL.md": b"Uninstall fixture.\n",
        "automexia-release-key.pub": b"untrusted comment: fixture\nRWAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\n",
        "automexia-terminal.cdx.json": b'{"bomFormat":"CycloneDX"}\n',
        "automexia-terminal.spdx.json": b'{"spdxVersion":"SPDX-2.3"}\n',
    }
    for name, data in evidence.items():
        (bundle / name).write_bytes(data)
    covered = sorted(
        path for path in bundle.iterdir() if path.name not in {"SHA256SUMS", "SHA256SUMS.minisig"}
    )
    lines = [f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}" for path in covered]
    (bundle / "SHA256SUMS").write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    (bundle / "SHA256SUMS.minisig").write_text("fixture signature\n", encoding="utf-8")
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


class PublicDistributionTests(unittest.TestCase):
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
                    (output / "public-distribution-manifest-v1.json").read_text(
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
            duplicate = source / "duplicate" / f"automexia-terminal_{VERSION}_amd64.deb"
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
                manifest, release_payload(manifest, bundle), bundle=bundle
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
                            manifest, payload, bundle=bundle
                        )

            draft = release_payload(manifest, bundle)
            draft.update(draft=True, immutable=False)
            DISTRIBUTION.verify_release_payload(
                manifest, draft, stage="draft", bundle=bundle
            )

            malformed = json.loads(json.dumps(manifest))
            malformed["evidence"]["checksum"] = "checksums.txt"
            with self.assertRaisesRegex(
                DISTRIBUTION.DistributionError, "evidence contract"
            ):
                DISTRIBUTION.verify_release_payload(
                    malformed, release_payload(manifest, bundle), bundle=bundle
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

    def test_workflow_policy_rejects_publication_gate_removal(self) -> None:
        DISTRIBUTION.validate_workflow()
        workflow = DISTRIBUTION.PUBLIC_WORKFLOW.read_text(encoding="utf-8")
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
                "    needs: [authorize, package]\n"
                "    if: needs.authorize.outputs.publish == 'true'",
                "  assemble:\n    name: Assemble and sign public Linux bundle\n"
                "    needs: [authorize, package]\n    if: always()",
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


if __name__ == "__main__":
    unittest.main(verbosity=2)
