#!/usr/bin/env python3
"""Mutation tests for runtime and release trust boundaries."""

from __future__ import annotations

import copy
import importlib.util
from pathlib import Path
import unittest

import yaml


MODULE_PATH = Path(__file__).with_name("check_runtime_trust.py")
SPEC = importlib.util.spec_from_file_location("automexia_runtime_trust", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load tools/ci/check_runtime_trust.py")
TRUST = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(TRUST)
ROOT = MODULE_PATH.parents[2]


class RuntimeTrustTests(unittest.TestCase):
    def read(self, path: str) -> str:
        return (ROOT / path).read_text(encoding="utf-8")

    def test_current_repository_satisfies_the_contract(self) -> None:
        TRUST.validate_repository()

    def test_automatic_shell_installer_is_rejected(self) -> None:
        source = self.read("tools/xtask/src/main.rs")
        altered = source.replace(
            "LaunchPhase::Smoke,",
            "LaunchPhase::Smoke, LaunchPhase::InstallShellIntegration,",
            1,
        )
        with self.assertRaisesRegex(TRUST.RuntimeTrustError, "must not provision"):
            TRUST.validate_launcher(altered)

    def test_maintainer_execution_policy_bypass_is_rejected(self) -> None:
        source = self.read("tools/xtask/src/main.rs")
        altered = source.replace(
            '"-NoProfile",',
            '"-NoProfile", "-ExecutionPolicy", "Bypass",',
            1,
        )
        with self.assertRaisesRegex(TRUST.RuntimeTrustError, "execution policy"):
            TRUST.validate_launcher(altered)

    def test_qa_execution_policy_bypass_is_rejected(self) -> None:
        source = self.read("tools/ci/qa.py").replace(
            '"-NonInteractive",',
            '"-ExecutionPolicy", "Bypass",',
            1,
        )
        with self.assertRaisesRegex(TRUST.RuntimeTrustError, "QA PowerShell"):
            TRUST.validate_qa_runner(source)

    def test_elevated_windows_manifest_is_rejected(self) -> None:
        manifest = self.read("packaging/windows/automexia.manifest").replace(
            'level="asInvoker"', 'level="requireAdministrator"'
        )
        with self.assertRaisesRegex(TRUST.RuntimeTrustError, "asInvoker"):
            TRUST.validate_manifest(
                manifest, self.read("apps/automexia-terminal/build.rs")
            )

    def test_encoded_wsl_shell_transport_is_rejected(self) -> None:
        source = self.read("shell-integration/windows-wsl.ps1").replace(
            "--exec sh -s", '--exec sh -c "base64 -d | sh"'
        )
        with self.assertRaisesRegex(TRUST.RuntimeTrustError, "bounded raw|encoded or nested"):
            TRUST.validate_wsl(source)

    def test_unsigned_script_packaging_is_rejected(self) -> None:
        workflow = yaml.safe_load(self.read(".github/workflows/release.yml"))
        altered = copy.deepcopy(workflow)
        altered["jobs"]["package-windows"]["steps"] = [
            step
            for step in altered["jobs"]["package-windows"]["steps"]
            if step.get("name") != "Sign PowerShell assets with Azure Artifact Signing"
        ]
        with self.assertRaisesRegex(TRUST.RuntimeTrustError, "recursively sign"):
            TRUST.validate_release(altered)

    def test_publication_cannot_bypass_cold_build_comparison(self) -> None:
        workflow = yaml.safe_load(self.read(".github/workflows/release.yml"))
        altered = copy.deepcopy(workflow)
        altered["jobs"]["publish"]["needs"].remove("reproducibility-linux")
        with self.assertRaisesRegex(TRUST.RuntimeTrustError, "cold-build reproducibility"):
            TRUST.validate_release(altered)

    def test_mutable_or_overwriting_publication_is_rejected(self) -> None:
        workflow = yaml.safe_load(self.read(".github/workflows/release.yml"))
        altered = copy.deepcopy(workflow)
        publish = altered["jobs"]["publish"]
        for step in publish["steps"]:
            if step.get("name") == "Require immutable GitHub releases":
                step["run"] = "echo skipped"
            if step.get("name") == "Publish protected-tag assets":
                step["run"] = 'gh release upload "$GITHUB_REF_NAME" --clobber'
        with self.assertRaisesRegex(TRUST.RuntimeTrustError, "immutable releases"):
            TRUST.validate_release(altered)

    def test_package_only_sbom_input_is_rejected(self) -> None:
        workflow = yaml.safe_load(self.read(".github/workflows/release.yml"))
        altered = copy.deepcopy(workflow)
        publish = altered["jobs"]["publish"]
        for step in publish["steps"]:
            if "anchore/sbom-action@" in str(step.get("uses", "")):
                step["with"]["path"] = "packages"
        with self.assertRaisesRegex(TRUST.RuntimeTrustError, "locked Rust dependency|enriched"):
            TRUST.validate_release(altered)

    def test_unbounded_portable_resources_are_rejected(self) -> None:
        xtask = self.read("tools/xtask/src/main.rs").replace(
            "copy_bounded_resource_tree(\n        &root().join",
            "copy_resource_tree(\n        &root().join",
            1,
        )
        with self.assertRaisesRegex(TRUST.RuntimeTrustError, "bounded"):
            TRUST.validate_packaging(
                xtask, self.read("packaging/windows/automexia-arm64.wxs")
            )


if __name__ == "__main__":
    unittest.main()
