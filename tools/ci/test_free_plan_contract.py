#!/usr/bin/env python3
"""Mutation coverage for the GitHub-Free/private workflow contract."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
import tomllib
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
CHECKER = ROOT / ".github" / "scripts" / "check_free_plan_contract.py"
TEST_TEMP_PARENT = Path(ROOT.anchor) if os.name == "nt" else None
CACHE_INITIALIZER = r'''printf 'SCCACHE_GHA_VERSION=automexia-rust-%s-v1\n' "$RUSTUP_TOOLCHAIN" >> "$GITHUB_ENV"'''


class FreePlanContractTests(unittest.TestCase):
    @staticmethod
    def populate_contract_root(root: Path) -> None:
        shutil.copytree(ROOT / ".github", root / ".github")
        shutil.copy2(ROOT / "rust-toolchain.toml", root / "rust-toolchain.toml")
        for package in ("sugarloaf", "rio-backend"):
            (root / package).mkdir()
            shutil.copy2(
                ROOT / package / "Cargo.toml",
                root / package / "Cargo.toml",
            )

    def run_checker_with_replacement(
        self, old: str, new: str, workflow_name: str = "ci.yml"
    ) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            root = Path(temporary)
            self.populate_contract_root(root)
            workflow = root / ".github" / "workflows" / workflow_name
            source = workflow.read_text(encoding="utf-8")
            self.assertIn(old, source)
            workflow.write_text(source.replace(old, new, 1), encoding="utf-8")
            return subprocess.run(
                [sys.executable, str(CHECKER)],
                cwd=root,
                capture_output=True,
                text=True,
                timeout=30,
                check=False,
            )

    def run_checker_with_manifest_replacement(
        self,
        old: str,
        new: str,
        manifest_path: str = "sugarloaf/Cargo.toml",
    ) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            root = Path(temporary)
            self.populate_contract_root(root)
            manifest = root / manifest_path
            source = manifest.read_text(encoding="utf-8")
            self.assertIn(old, source)
            manifest.write_text(source.replace(old, new, 1), encoding="utf-8")
            return subprocess.run(
                [sys.executable, str(CHECKER)],
                cwd=root,
                capture_output=True,
                text=True,
                timeout=30,
                check=False,
            )

    def run_checker_without_manifest(
        self, manifest_path: str = "sugarloaf/Cargo.toml"
    ) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            root = Path(temporary)
            self.populate_contract_root(root)
            (root / manifest_path).unlink()
            return subprocess.run(
                [sys.executable, str(CHECKER)],
                cwd=root,
                capture_output=True,
                text=True,
                timeout=30,
                check=False,
            )

    def run_checker(self, mutation: tuple[str, str] | None = None) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            root = Path(temporary)
            self.populate_contract_root(root)
            if mutation is not None:
                relative, addition = mutation
                path = root / ".github" / relative
                path.write_text(
                    path.read_text(encoding="utf-8") + addition,
                    encoding="utf-8",
                )
            return subprocess.run(
                [sys.executable, str(CHECKER)],
                cwd=root,
                capture_output=True,
                text=True,
                timeout=30,
                check=False,
            )

    def test_current_contract_passes(self) -> None:
        completed = self.run_checker()
        self.assertEqual(completed.returncode, 0, completed.stderr)

    def test_floating_workflow_compiler_is_rejected(self) -> None:
        # Installing one compiler must not leave the checkout selecting another.
        completed = self.run_checker_with_replacement(
            "RUSTUP_TOOLCHAIN: '1.96.1'",
            "RUSTUP_TOOLCHAIN: 'stable'",
        )
        self.assertNotEqual(completed.returncode, 0, completed.stderr)
        self.assertIn("compiler", completed.stderr)

    def test_every_stable_workflow_selects_the_canonical_compiler(self) -> None:
        pin = tomllib.loads(
            (ROOT / "rust-toolchain.toml").read_text(encoding="utf-8")
        )["toolchain"]["channel"]
        assignment = f"  RUSTUP_TOOLCHAIN: '{pin}'"
        # These independent mutations cover selection, scope and ambiguity; a
        # matching install command alone did not enforce the effective compiler.
        for workflow_name in (
            "ci.yml", "linux-early-access.yml", "release.yml", "nightly.yml"
        ):
            for replacement in (
                "  RUSTUP_TOOLCHAIN: '0.0.0'",
                "  RUSTUP_TOOLCHAIN: 'stable'",
                "",
                "    " + assignment,
                assignment + "\n" + assignment,
            ):
                with self.subTest(workflow=workflow_name, replacement=replacement):
                    completed = self.run_checker_with_replacement(
                        assignment, replacement, workflow_name
                    )
                    self.assertNotEqual(completed.returncode, 0)
                    self.assertIn(
                        f"{workflow_name} compiler selection", completed.stderr
                    )

    def test_compiler_authority_is_bounded_exact_and_fail_closed(self) -> None:
        source = (ROOT / "rust-toolchain.toml").read_bytes()
        limit = 16 * 1024
        at_limit = source + b"\n#" + b"p" * (limit - len(source) - 2)
        cases = (
            (None, False),
            (b"[toolchain", False),
            (b"toolchain = []", False),
            (b"[toolchain]\nchannel = 123", False),
            (b'[toolchain]\nchannel = "stable"', False),
            (b"\xff", False),
            (at_limit, True),
            (at_limit + b"p", False),
        )
        for index, (content, valid) in enumerate(cases):
            with self.subTest(case=index), tempfile.TemporaryDirectory(
                dir=TEST_TEMP_PARENT
            ) as temporary:
                root = Path(temporary)
                self.populate_contract_root(root)
                command = [sys.executable, str(CHECKER)]
                baseline = subprocess.run(
                    command, cwd=root, capture_output=True, text=True,
                    timeout=30, check=False,
                )
                self.assertEqual(baseline.returncode, 0, baseline.stderr)
                path = root / "rust-toolchain.toml"
                if content is None:
                    path.unlink()
                else:
                    path.write_bytes(content)
                completed = subprocess.run(
                    command, cwd=root, capture_output=True, text=True,
                    timeout=30, check=False,
                )
                self.assertEqual(completed.returncode == 0, valid, completed.stderr)
                if not valid:
                    self.assertIn("compiler authority", completed.stderr)
                    self.assertNotIn(str(root), completed.stderr)
                    self.assertNotIn("Traceback", completed.stderr)

    def test_legacy_override_and_global_default_mutation_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            root = Path(temporary)
            self.populate_contract_root(root)
            # The legacy file wins over rust-toolchain.toml, even with both
            # present; a stale contributor artifact must not silently win.
            (root / "rust-toolchain").write_text("stable\n", encoding="utf-8")
            completed = subprocess.run(
                [sys.executable, str(CHECKER)], cwd=root, capture_output=True,
                text=True, timeout=30, check=False,
            )
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("shadowed by legacy rust-toolchain", completed.stderr)
        for workflow_name in ("ci.yml", "linux-early-access.yml", "release.yml"):
            completed = self.run_checker_with_replacement(
                "rustc --version", 'rustup default "$RUSTUP_TOOLCHAIN"', workflow_name
            )
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("without changing runner defaults", completed.stderr)

    def test_linux_release_cache_cannot_ignore_the_selected_compiler(self) -> None:
        completed = self.run_checker_with_replacement(
            "SCCACHE_GHA_VERSION=automexia-rust-%s-v1",
            "SCCACHE_GHA_VERSION: automexia-rust-stale-v1",
            "linux-early-access.yml",
        )
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("compiler cache", completed.stderr)

    def test_quality_cache_is_initialized_once_before_startup(self) -> None:
        for workflow_name in ("ci.yml", "linux-early-access.yml"):
            source = (ROOT / ".github/workflows" / workflow_name).read_text(encoding="utf-8")
            self.assertEqual(source.count(CACHE_INITIALIZER), 1)
            self.assertLess(source.index(CACHE_INITIALIZER), source.index("id: sccache"))
            self.assertNotIn("SCCACHE_GHA_VERSION:", source)
            # GitHub rejects env-context references in job-level env. Keep the
            # runner-file handoff before sccache starts, and preserve other values.
            for replacement in (
                "",
                CACHE_INITIALIZER + "\n          " + CACHE_INITIALIZER,
                "# " + CACHE_INITIALIZER,
                CACHE_INITIALIZER.replace('"$RUSTUP_TOOLCHAIN"', '"stable"'),
                CACHE_INITIALIZER.replace(">>", ">"),
                'SCCACHE_GHA_VERSION: automexia-rust-${{ env.RUSTUP_TOOLCHAIN }}-v1',
            ):
                with self.subTest(workflow=workflow_name, replacement=replacement):
                    completed = self.run_checker_with_replacement(
                        CACHE_INITIALIZER, replacement, workflow_name
                    )
                    self.assertNotEqual(completed.returncode, 0)
                    self.assertIn("compiler cache", completed.stderr)
            moved = source.replace("          " + CACHE_INITIALIZER + "\n", "")
            moved = moved.replace("        id: sccache", "          " + CACHE_INITIALIZER + "\n        id: sccache")
            completed = self.run_checker_with_replacement(source, moved, workflow_name)
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("compiler cache", completed.stderr)

    @unittest.skipUnless(sys.platform == "linux", "native Linux Bash runner contract")
    def test_linux_runner_cache_handoff_preserves_environment_file(self) -> None:
        bash = shutil.which("bash")
        self.assertIsNotNone(bash, "native Linux workflow tests require Bash")
        for workflow_name in ("ci.yml", "linux-early-access.yml"):
            source = (ROOT / ".github/workflows" / workflow_name).read_text(encoding="utf-8")
            lines = [line.strip() for line in source.splitlines() if "SCCACHE_GHA_VERSION" in line]
            self.assertEqual(len(lines), 1)
            for pin, expected in (
                ("1.96.1", "OTHER=kept\nSCCACHE_GHA_VERSION=automexia-rust-1.96.1-v1\n"),
                ("2.0.0", "OTHER=kept\nSCCACHE_GHA_VERSION=automexia-rust-2.0.0-v1\n"),
            ):
                with self.subTest(workflow=workflow_name, pin=pin), tempfile.TemporaryDirectory() as temporary:
                    root = Path(temporary)
                    script = root / "setup.sh"
                    script.write_text("set -euo pipefail\n" + lines[0] + "\n", encoding="utf-8")
                    handoff = root / "environment"
                    handoff.write_text("OTHER=kept\n", encoding="utf-8")
                    completed = subprocess.run(
                        [bash, "--noprofile", "--norc", str(script)],
                        env={**os.environ, "RUSTUP_TOOLCHAIN": pin, "GITHUB_ENV": str(handoff)},
                        capture_output=True, timeout=15, check=False,
                    )
                    self.assertEqual(completed.returncode, 0)
                    self.assertEqual(completed.stdout, b"")
                    self.assertEqual(completed.stderr, b"")
                    self.assertEqual(handoff.read_text(encoding="utf-8"), expected)

    def test_stable_release_does_not_start_for_linux_early_access(self) -> None:
        exclusion = "!startsWith(github.event.pull_request.head.ref, 'release/linux/') &&"
        source = (ROOT / ".github/workflows/release.yml").read_text(encoding="utf-8")
        # A Linux merge used to start stable authorization and fail its stricter
        # branch grammar. Skip that lane before any stable signing job can run.
        self.assertIn(exclusion, source)
        for replacement in ("", exclusion[1:], "true &&"):
            with self.subTest(replacement=replacement):
                with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
                    root = Path(temporary)
                    self.populate_contract_root(root)
                    candidate = root / ".github/workflows/release.yml"
                    candidate.write_text(source.replace(exclusion, replacement, 1), encoding="utf-8")
                    completed = subprocess.run(
                        [sys.executable, str(CHECKER)], cwd=root,
                        capture_output=True, text=True, timeout=30, check=False,
                    )
                    self.assertNotEqual(completed.returncode, 0)
                    self.assertIn("stable release must exclude Linux", completed.stderr)

    def test_ci_remains_the_automatic_free_hosted_push_and_pr_pipeline(self) -> None:
        for old, new in (
            ("  push:\n    branches: [main]\n", ""),
            (
                "  workflow_dispatch:\n",
                "  workflow_dispatch:\n  schedule:\n    - cron: '0 0 * * *'\n",
            ),
        ):
            with self.subTest(mutation=old):
                completed = self.run_checker_with_replacement(old, new)
                self.assertNotEqual(completed.returncode, 0)
                self.assertRegex(completed.stderr, r"automatic free hosted|scheduled")

    def test_private_environment_is_rejected(self) -> None:
        completed = self.run_checker(
            ("workflows/s2-assurance.yml", "\n  environment: stable-release\n")
        )
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("private GitHub environments", completed.stderr)

    def test_paid_or_unbounded_workflows_are_rejected(self) -> None:
        cases = (
            (
                "workflows/ci.yml",
                "\n      - uses: actions/attest@0123456789012345678901234567890123456789\n",
                "private GitHub artifact attestations",
            ),
            (
                "workflows/nightly.yml",
                "\nschedule:\n  - cron: '0 0 * * *'\n",
                "manual-only",
            ),
        )
        for relative, addition, expected in cases:
            with self.subTest(expected=expected):
                completed = self.run_checker((relative, addition))
                self.assertNotEqual(completed.returncode, 0)
                self.assertIn(expected, completed.stderr)

    def test_only_the_release_coverage_job_may_use_a_hosted_windows_runner(self) -> None:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            root = Path(temporary)
            shutil.copytree(ROOT / ".github", root / ".github")
            workflow = root / ".github" / "workflows" / "ci.yml"
            source = workflow.read_text(encoding="utf-8")
            source = source.replace(
                "  quality:\n", "  unexpected-windows:\n    runs-on: windows-2025\n  quality:\n", 1
            )
            workflow.write_text(source, encoding="utf-8")
            completed = subprocess.run(
                [sys.executable, str(CHECKER)],
                cwd=root,
                capture_output=True,
                text=True,
                timeout=30,
                check=False,
            )
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("only release-candidate-coverage", completed.stderr)

    def test_required_hosted_assurance_scanners_cannot_be_removed(self) -> None:
        completed = self.run_checker(
            (
                "workflows/ci.yml",
                "\n# semgrep==1.175.0\n",
            )
        )
        # Appending a fragment must not make a complete workflow less strict.
        self.assertEqual(completed.returncode, 0, completed.stderr)
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            root = Path(temporary)
            shutil.copytree(ROOT / ".github", root / ".github")
            workflow = root / ".github" / "workflows" / "ci.yml"
            workflow.write_text(
                workflow.read_text(encoding="utf-8").replace(
                    "GITLEAKS_VERSION: '8.30.1'", "GITLEAKS_VERSION: '0.0.0'", 1
                ),
                encoding="utf-8",
            )
            completed = subprocess.run(
                [sys.executable, str(CHECKER)],
                cwd=root,
                capture_output=True,
                text=True,
                timeout=30,
                check=False,
            )
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("GitHub-Free hosted assurance", completed.stderr)
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            root = Path(temporary)
            shutil.copytree(ROOT / ".github", root / ".github")
            workflow = root / ".github" / "workflows" / "ci.yml"
            workflow.write_text(
                workflow.read_text(encoding="utf-8").replace(
                    "SHELLCHECK_VERSION: '0.11.0'", "SHELLCHECK_VERSION: '0.0.0'", 1
                ),
                encoding="utf-8",
            )
            completed = subprocess.run(
                [sys.executable, str(CHECKER)],
                cwd=root,
                capture_output=True,
                text=True,
                timeout=30,
                check=False,
            )
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("GitHub-Free hosted assurance", completed.stderr)

    def test_ordinary_ci_cannot_be_changed_back_to_all_history_scanning(self) -> None:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            root = Path(temporary)
            shutil.copytree(ROOT / ".github", root / ".github")
            workflow = root / ".github" / "workflows" / "ci.yml"
            workflow.write_text(
                workflow.read_text(encoding="utf-8").replace(
                    '--log-opts="$log_opts"', "--log-opts=--all", 1
                ),
                encoding="utf-8",
            )
            completed = subprocess.run(
                [sys.executable, str(CHECKER)],
                cwd=root,
                capture_output=True,
                text=True,
                timeout=30,
                check=False,
            )
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("legacy history", completed.stderr)

    def test_secret_scan_requires_full_history_but_quality_stays_shallow(self) -> None:
        with tempfile.TemporaryDirectory(dir=TEST_TEMP_PARENT) as temporary:
            root = Path(temporary)
            shutil.copytree(ROOT / ".github", root / ".github")
            workflow = root / ".github" / "workflows" / "ci.yml"
            source = workflow.read_text(encoding="utf-8")
            dependency = source.index("  dependency-security:\n")
            fetch = source.index("          fetch-depth: 0\n", dependency)
            workflow.write_text(
                source[:fetch] + source[fetch + len("          fetch-depth: 0\n") :],
                encoding="utf-8",
            )
            completed = subprocess.run(
                [sys.executable, str(CHECKER)],
                cwd=root,
                capture_output=True,
                text=True,
                timeout=30,
                check=False,
            )
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("fetch complete history", completed.stderr)

    def test_quality_job_resource_envelope_cannot_be_weakened(self) -> None:
        # A real hosted run exhausted its free private-runner envelope while
        # linking the all-feature test graph. Each mutation must fail closed.
        cases = (
            ("CARGO_BUILD_JOBS: '1'", "CARGO_BUILD_JOBS: '2'"),
            ("CARGO_PROFILE_DEV_DEBUG: '0'", "CARGO_PROFILE_DEV_DEBUG: '1'"),
            ("CARGO_PROFILE_TEST_DEBUG: '0'", "CARGO_PROFILE_TEST_DEBUG: '1'"),
            ("NEXTEST_TEST_THREADS: '1'", "NEXTEST_TEST_THREADS: '2'"),
            ("run: cargo clean", "run: echo skip-clean"),
            (
                "      CARGO_BUILD_JOBS: '1'\n",
                "      CARGO_BUILD_JOBS: '1'\n      CARGO_BUILD_JOBS: '1'\n",
            ),
        )
        for old, new in cases:
            with self.subTest(setting=old):
                completed = self.run_checker_with_replacement(old, new)
                self.assertNotEqual(completed.returncode, 0)
                self.assertIn("resource envelope", completed.stderr)

        ordered_steps = (
            "      - name: Reclaim lint artifacts before the all-feature test build\n"
            "        run: cargo clean\n\n"
            "      - name: Workspace unit and integration tests\n"
            "        run: cargo nextest run --workspace --all-features --locked --profile ci\n"
        )
        reordered_steps = (
            "      - name: Workspace unit and integration tests\n"
            "        run: cargo nextest run --workspace --all-features --locked --profile ci\n\n"
            "      - name: Reclaim lint artifacts before the all-feature test build\n"
            "        run: cargo clean\n"
        )
        completed = self.run_checker_with_replacement(ordered_steps, reordered_steps)
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("resource envelope", completed.stderr)

    def test_quality_compiler_cache_is_versioned_and_non_shipping(self) -> None:
        workflow = (ROOT / ".github" / "workflows" / "ci.yml").read_text(
            encoding="utf-8"
        )
        quality = workflow.split("  quality:\n", 1)[1].split(
            "  dependency-security:\n", 1
        )[0]
        required = (
            "mozilla-actions/sccache-action@fc920bf0ec8de6ee65d409111f7ec508035751ba",
            "version: v0.16.0",
            "SCCACHE_GHA_ENABLED: 'true'",
            "SCCACHE_GHA_VERSION=automexia-rust-%s-v1",
            "RUSTC_WRAPPER: sccache",
            "sccache --show-stats",
            "cargo-sources-v1-${{ runner.os }}-${{ hashFiles('Cargo.lock') }}",
        )
        for token in required:
            with self.subTest(token=token):
                self.assertEqual(quality.count(token), 1)
        self.assertNotRegex(quality, r"(?m)^\s+target(?:/.*)?\s*$")

        for old, new in (
            (
                "fc920bf0ec8de6ee65d409111f7ec508035751ba",
                "1111111111111111111111111111111111111111",
            ),
            ("version: v0.16.0", "version: v0.15.0"),
            ("SCCACHE_GHA_ENABLED: 'true'", "SCCACHE_GHA_ENABLED: 'false'"),
            (
                "SCCACHE_GHA_VERSION=automexia-rust-%s-v1",
                "SCCACHE_GHA_VERSION: unversioned",
            ),
            ("RUSTC_WRAPPER: sccache", "RUSTC_WRAPPER: rustc"),
            ("run: sccache --show-stats", "run: echo stats-skipped"),
            (
                "cargo-sources-v1-${{ runner.os }}-${{ hashFiles('Cargo.lock') }}",
                "cargo-sources-v1-static",
            ),
        ):
            with self.subTest(mutation=old):
                completed = self.run_checker_with_replacement(old, new)
                self.assertNotEqual(completed.returncode, 0)
                self.assertRegex(
                    completed.stderr.casefold(), r"cache|resource envelope"
                )

    def test_image_rendering_linux_backends_cannot_be_removed(self) -> None:
        # The hosted regression occurred only in the stand-alone Sugarloaf
        # command because the preceding workspace build had hidden this edge.
        complete = 'rio-window = { workspace = true, features = ["x11", "wayland"] }'
        for weakened in (
            'rio-window = { workspace = true, features = ["x11"] }',
            'rio-window = { workspace = true, features = ["wayland"] }',
            "",
            f"{complete}\n{complete}",
        ):
            with self.subTest(weakened=weakened):
                completed = self.run_checker_with_manifest_replacement(
                    complete, weakened
                )
                self.assertNotEqual(completed.returncode, 0)
                self.assertIn(
                    "image-rendering Linux platform contract", completed.stderr
                )

        missing = self.run_checker_without_manifest()
        self.assertNotEqual(missing.returncode, 0)
        self.assertIn("image-rendering Linux platform contract", missing.stderr)

    def test_image_rendering_backend_order_is_not_a_textual_contract(self) -> None:
        completed = self.run_checker_with_manifest_replacement(
            'features = ["x11", "wayland"]',
            'features = ["wayland", "x11"]',
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)

    def test_rio_backend_linux_features_must_reach_optional_window(self) -> None:
        cases = (
            (
                'x11 = ["rio-vt/x11", "rio-window?/x11"]',
                'x11 = ["rio-vt/x11"]',
            ),
            (
                'wayland = ["rio-vt/wayland", "rio-window?/wayland"]',
                'wayland = ["rio-vt/wayland"]',
            ),
            (
                'default = ["renderer", "wayland", "x11", "rio-window", "graphics"]',
                'default = ["renderer", "wayland", "x11", "graphics"]',
            ),
        )
        for complete, weakened in cases:
            with self.subTest(weakened=weakened):
                completed = self.run_checker_with_manifest_replacement(
                    complete,
                    weakened,
                    "rio-backend/Cargo.toml",
                )
                self.assertNotEqual(completed.returncode, 0)
                self.assertIn(
                    "image-rendering rio-backend contract", completed.stderr
                )

        missing = self.run_checker_without_manifest("rio-backend/Cargo.toml")
        self.assertNotEqual(missing.returncode, 0)
        self.assertIn("image-rendering rio-backend contract", missing.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
