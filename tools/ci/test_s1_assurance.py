#!/usr/bin/env python3
"""Mutation tests for the S1 native/visual/resource/accessibility gate."""

from __future__ import annotations

import copy
import datetime as dt
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock


MODULE_PATH = Path(__file__).with_name("s1_assurance.py")
SPEC = importlib.util.spec_from_file_location("automexia_s1_assurance", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load tools/ci/s1_assurance.py")
S1 = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(S1)


def digest(value: bytes = b"fixture") -> str:
    return hashlib.sha256(value).hexdigest()


def environment(environment_id: str, **overrides: object) -> dict[str, object]:
    value: dict[str, object] = {
        "id": environment_id,
        "platform": "windows",
        "architecture": "x86_64",
        "display_server": "win32",
        "renderer_backend": "wgpu",
        "gpu_vendor": "intel",
        "adapter_sha256": digest(environment_id.encode()),
        "driver_version": "32.0.101.6737",
        "os_build": "Windows-11-24H2",
        "display": "2560x1440",
        "power_mode": "balanced",
        "shell_versions": ["PowerShell-7.5.2"],
    }
    value.update(overrides)
    return value


def valid_manifest() -> dict[str, object]:
    policy = S1.load_policy()
    now = dt.datetime(2026, 8, 24, 12, 0, tzinfo=dt.timezone.utc)
    environments = {
        "windows-intel-wgpu": environment("windows-intel-wgpu"),
        "windows-amd-wgpu": environment(
            "windows-amd-wgpu", gpu_vendor="amd"
        ),
        "windows-nvidia-wgpu": environment(
            "windows-nvidia-wgpu", gpu_vendor="nvidia"
        ),
        "windows-rdp-cpu": environment(
            "windows-rdp-cpu",
            renderer_backend="cpu",
            gpu_vendor="software",
            display_server="rdp",
        ),
        "linux-x11": environment(
            "linux-x11",
            platform="linux",
            display_server="x11",
            renderer_backend="wgpu",
            shell_versions=["bash-5.2", "zsh-5.9"],
            os_build="Ubuntu-24.04",
        ),
        "linux-wayland": environment(
            "linux-wayland",
            platform="linux",
            display_server="wayland",
            renderer_backend="wgpu",
            gpu_vendor="amd",
            shell_versions=["bash-5.2", "zsh-5.9"],
            os_build="Fedora-42",
        ),
        "linux-nvidia-wgpu": environment(
            "linux-nvidia-wgpu",
            platform="linux",
            display_server="x11",
            renderer_backend="wgpu",
            gpu_vendor="nvidia",
            shell_versions=["bash-5.2", "zsh-5.9"],
            os_build="Ubuntu-24.04",
        ),
        "macos-intel": environment(
            "macos-intel",
            platform="macos",
            architecture="x86_64",
            display_server="quartz",
            renderer_backend="metal",
            gpu_vendor="intel",
            shell_versions=["zsh-5.9"],
            os_build="macOS-15.6",
        ),
        "macos-apple-silicon": environment(
            "macos-apple-silicon",
            platform="macos",
            architecture="aarch64",
            display_server="quartz",
            renderer_backend="metal",
            gpu_vendor="apple",
            shell_versions=["zsh-5.9"],
            os_build="macOS-15.6",
        ),
    }

    # Build the complete synthetic matrix from policy rather than duplicating its
    # suite list; mutations therefore detect policy/evidence drift in either owner.
    suites = []
    for suite in policy["required_suites"]:
        coverage = suite["coverage"]
        suites.append(
            {
                "id": suite["id"],
                "domain": suite["domain"],
                "environment_id": suite["environment_id"],
                "tool": suite["tool"],
                "tool_version": "1.0.0",
                "started_at_utc": now.isoformat().replace("+00:00", "Z"),
                "duration_ms": 1_000,
                "result": "pass",
                "coverage": copy.deepcopy(coverage),
                "artifacts": [
                    {
                        "kind": suite["artifact_kind"],
                        "sha256": digest(suite["id"].encode()),
                        "bytes": 1024,
                        "privacy": "public-redacted",
                    }
                ],
                "operator": f"operator-{suite['id'][:24]}",
            }
        )

    # Human review applies only to visible and assistive-technology evidence;
    # native resource and resilience suites remain machine-verifiable.
    manual_suite_ids = [
        suite["id"]
        for suite in policy["required_suites"]
        if suite["domain"] in {"visual", "accessibility"}
    ]
    return {
        "schema": 1,
        "evidence_kind": "s1-release-assurance",
        "synthetic": True,
        "policy_sha256": S1.policy_sha256(policy),
        "source_commit": "1" * 40,
        "application": {
            "version": "0.5.0",
            "binary_sha256": digest(b"binary"),
            "package_sha256": digest(b"package"),
            "visual_fixture": "s1-standard-v1",
        },
        "environments": list(environments.values()),
        "suites": suites,
        "reviews": [
            {
                "scope": "visual-and-accessibility",
                "suite_ids": manual_suite_ids,
                "reviewer": "reviewer-release-a11y",
                "reviewed_at_utc": now.isoformat().replace("+00:00", "Z"),
                "review_url": "https://github.com/example/review/1",
                "result": "approved",
            }
        ],
        "redaction": {
            "canaries_checked": 64,
            "canary_leaks": 0,
            "forbidden_fields_absent": True,
        },
    }


class S1AssuranceTests(unittest.TestCase):
    def validate(self, document: dict[str, object], **kwargs: object):
        # Re-enter the bounded evidence-file parser for every mutation so schema,
        # duplicate-key, file identity, and semantic checks stay in the test path.
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "evidence.json"
            path.write_text(json.dumps(document), encoding="utf-8")
            kwargs.setdefault("allow_synthetic", True)
            return S1.validate_manifest(path, **kwargs)

    def reject(self, mutate) -> None:
        document = valid_manifest()
        mutate(document)
        with self.assertRaises(S1.S1AssuranceError):
            self.validate(document)

    def reject_policy(self, mutate) -> None:
        policy = copy.deepcopy(S1.load_policy())
        mutate(policy)
        with self.assertRaises(S1.S1AssuranceError):
            S1.validate_policy(policy)

    def test_complete_matrix_is_bound_to_policy_commit_and_review(self) -> None:
        document = valid_manifest()
        result = self.validate(
            document,
            expected_commit="1" * 40,
            now=dt.datetime(2026, 8, 25, tzinfo=dt.timezone.utc),
        )
        self.assertEqual(result["status"], "pass")
        self.assertEqual(result["suite_count"], len(S1.load_policy()["required_suites"]))
        self.assertEqual(result["missing_suites"], [])

    def test_policy_freezes_required_scenarios_surfaces_and_tasks(self) -> None:
        def remove_native(policy) -> None:
            suite = next(item for item in policy["required_suites"] if item["domain"] == "native")
            suite["coverage"]["scenarios"].remove("command-result-viewport-overflow")

        def remove_resource(policy) -> None:
            suite = next(
                item
                for item in policy["required_suites"]
                if item["tool"] == "native-resource"
            )
            suite["coverage"]["scenarios"].remove("long-output-storm")

        def remove_visual(policy) -> None:
            suite = next(item for item in policy["required_suites"] if item["domain"] == "visual")
            suite["coverage"]["surfaces"].remove("command-result-viewport-overflow")
            suite["coverage"]["capture_count"] = (
                len(suite["coverage"]["themes"])
                * len(suite["coverage"]["scales"])
                * len(suite["coverage"]["viewports"])
                * len(suite["coverage"]["surfaces"])
            )

        def remove_accessibility(policy) -> None:
            suite = next(
                item
                for item in policy["required_suites"]
                if item["domain"] == "accessibility"
            )
            suite["coverage"]["tasks"].remove("command-result-announcement")

        for mutation in (
            remove_native,
            remove_resource,
            remove_visual,
            remove_accessibility,
        ):
            with self.subTest(mutation=mutation.__name__):
                self.reject_policy(mutation)

    def test_missing_suite_is_external_until_release_requires_complete(self) -> None:
        document = valid_manifest()
        removed = document["suites"].pop()
        for review in document["reviews"]:
            review["suite_ids"] = [
                suite_id
                for suite_id in review["suite_ids"]
                if suite_id != removed["id"]
            ]
        result = self.validate(document)
        self.assertEqual(result["status"], "external")
        self.assertEqual(len(result["missing_suites"]), 1)
        with self.assertRaisesRegex(S1.S1AssuranceError, "incomplete"):
            self.validate(document, require_complete=True)

    def test_failures_duplicate_ids_and_unexpected_suites_fail_closed(self) -> None:
        self.reject(lambda value: value["suites"][0].__setitem__("result", "fail"))
        self.reject(lambda value: value["suites"].append(copy.deepcopy(value["suites"][0])))
        self.reject(
            lambda value: value["suites"].append(
                {**copy.deepcopy(value["suites"][0]), "id": "unknown-suite"}
            )
        )

    def test_environment_capabilities_and_exact_coverage_cannot_drift(self) -> None:
        self.reject(
            lambda value: value["environments"][0].__setitem__("gpu_vendor", "unknown")
        )
        self.reject(
            lambda value: value["suites"][0]["coverage"]["scenarios"].pop()
        )
        self.reject(
            lambda value: value["suites"][0].__setitem__(
                "environment_id", "missing-environment"
            )
        )

    def test_visual_matrix_requires_complete_cross_product_and_human_review(self) -> None:
        document = valid_manifest()
        visual = next(item for item in document["suites"] if item["domain"] == "visual")
        visual["coverage"]["cross_product_complete"] = False
        with self.assertRaises(S1.S1AssuranceError):
            self.validate(document)

        self.reject(lambda value: value.__setitem__("reviews", []))
        self.reject(
            lambda value: value["reviews"][0].__setitem__(
                "review_url", "file:///private/review"
            )
        )
        self.reject(
            lambda value: value["reviews"][0].__setitem__(
                "review_url", "https://"
            )
        )
        future_review = valid_manifest()
        future_review["reviews"][0]["reviewed_at_utc"] = "2027-01-01T00:00:00Z"
        with self.assertRaisesRegex(S1.S1AssuranceError, "stale or future"):
            self.validate(
                future_review,
                now=dt.datetime(2026, 8, 25, tzinfo=dt.timezone.utc),
            )

        self.reject(
            lambda value: value["reviews"][0].__setitem__(
                "reviewer", value["suites"][0]["operator"]
            )
        )

    def test_accessibility_matrix_requires_each_controlled_assistive_technology(self) -> None:
        policy = S1.load_policy()
        expected = {"narrator", "nvda", "voiceover", "orca-x11", "orca-wayland"}
        actual = {
            suite["tool"]
            for suite in policy["required_suites"]
            if suite["domain"] == "accessibility"
        }
        self.assertEqual(actual, expected)
        self.reject(
            lambda value: next(
                item for item in value["suites"] if item["tool"] == "nvda"
            ).__setitem__("tool_version", "")
        )

    def test_freshness_artifact_privacy_redaction_and_synthetic_release_rules(self) -> None:
        stale = valid_manifest()
        old = "2026-01-01T00:00:00Z"
        for suite in stale["suites"]:
            suite["started_at_utc"] = old
        with self.assertRaisesRegex(S1.S1AssuranceError, "stale"):
            self.validate(
                stale, now=dt.datetime(2026, 8, 25, tzinfo=dt.timezone.utc)
            )

        self.reject(
            lambda value: value["suites"][0]["artifacts"][0].__setitem__(
                "sha256", "not-a-hash"
            )
        )
        self.reject(
            lambda value: value["redaction"].__setitem__("canary_leaks", 1)
        )
        with self.assertRaisesRegex(S1.S1AssuranceError, "synthetic"):
            self.validate(valid_manifest(), allow_synthetic=False)

    def test_duplicate_keys_symlinks_and_oversized_manifests_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "evidence.json"
            path.write_text('{"schema":1,"schema":1}', encoding="utf-8")
            with self.assertRaisesRegex(S1.S1AssuranceError, "duplicate"):
                S1.load_manifest(path)

            path.write_bytes(b"x" * (S1.MAX_MANIFEST_BYTES + 1))
            with self.assertRaisesRegex(S1.S1AssuranceError, "size"):
                S1.load_manifest(path)

            target = root / "target.json"
            target.write_text(json.dumps(valid_manifest()), encoding="utf-8")
            link = root / "link.json"
            try:
                link.symlink_to(target)
            except OSError:
                return
            with self.assertRaises(S1.S1AssuranceError):
                S1.load_manifest(link)

    def test_release_evidence_uses_current_clean_commit(self) -> None:
        document = valid_manifest()
        document["synthetic"] = False
        with mock.patch.object(S1, "current_source_commit", return_value="1" * 40):
            result = self.validate(
                document,
                allow_synthetic=False,
                expected_commit="1" * 40,
            )
        self.assertEqual(result["status"], "pass")

    def test_source_contract_has_visual_fixture_and_separate_low_resource_phase(self) -> None:
        root = MODULE_PATH.parents[2]
        cargo = (root / "apps/automexia-terminal/Cargo.toml").read_text(encoding="utf-8")
        runtime = (root / "apps/automexia-terminal/src/automexia/runtime.rs").read_text(encoding="utf-8")
        footer = (root / "apps/automexia-terminal/src/renderer/session_footer.rs").read_text(encoding="utf-8")
        verifier = (root / "tests/integration/appverifier-windows.ps1").read_text(encoding="utf-8")
        self.assertIn('visual-test-hooks = ["native-gui-test-hooks"]', cargo)
        self.assertIn("visual_test_snapshot", runtime)
        self.assertIn("frozen_clock_label", footer)
        self.assertIn("/faults", verifier)
        self.assertIn("AUTOMEXIA_S1_LOW_RESOURCE_START", verifier)
        self.assertIn("AUTOMEXIA_S1_LOW_RESOURCE_END", verifier)
        self.assertIn("MaximumVerifierLogBytes", verifier)
        self.assertIn("Severity", verifier)
        self.assertIn("StopCode", verifier)

    def test_recent_u10_surfaces_and_interactions_are_release_evidence(self) -> None:
        policy = S1.load_policy()
        expected_surfaces = {
            "compact-top-shelf",
            "diagnostic-assistant",
            "tab-appearance-picker",
            "command-palette-overflow",
            "connection-hub-setup",
            "connection-hub-direct-entry",
            "command-result-datetime",
            "scrollbar",
            "saved-preferences-restart",
        }
        expected_tasks = {
            "compact-window-chrome",
            "diagnostic-assistant-actions",
            "compatibility-inspector-redaction",
            "tab-appearance-picker",
            "quit-confirmation",
            "command-palette-scroll-position",
            "image-preview-open-close",
            "connection-hub-direct-entry",
            "command-result-datetime",
            "saved-preferences-restart",
            "command-boundary-navigation",
            "scrollbar-position",
        }
        for suite in policy["required_suites"]:
            if suite["domain"] == "visual":
                self.assertTrue(
                    expected_surfaces.issubset(suite["coverage"]["surfaces"]),
                    suite["id"],
                )
            if suite["domain"] == "accessibility":
                self.assertTrue(
                    expected_tasks.issubset(suite["coverage"]["tasks"]),
                    suite["id"],
                )

    def test_visual_matrix_includes_high_contrast_400_percent_and_reduced_motion(self) -> None:
        policy = S1.load_policy()
        for suite in policy["required_suites"]:
            if suite["domain"] != "visual":
                continue
            coverage = suite["coverage"]
            self.assertIn("high-contrast", coverage["themes"], suite["id"])
            self.assertIn("4.0", coverage["scales"], suite["id"])
            self.assertEqual(coverage["motion_profiles"], ["enabled", "reduced"])
            expected = (
                len(coverage["themes"])
                * len(coverage["scales"])
                * len(coverage["viewports"])
                * len(coverage["surfaces"])
                * len(coverage["motion_profiles"])
            )
            self.assertEqual(coverage["capture_count"], expected, suite["id"])

    def test_m8_m12_providers_have_native_resource_visual_and_accessibility_evidence(self) -> None:
        policy = S1.load_policy()
        for suite in policy["required_suites"]:
            domain = suite["domain"]
            coverage = suite["coverage"]
            if domain == "native":
                self.assertIn(
                    "connection-hub-providers-review",
                    coverage["scenarios"],
                    suite["id"],
                )
            elif suite["tool"] == "native-resource":
                self.assertIn(
                    "connection-hub-providers-replacement",
                    coverage["scenarios"],
                    suite["id"],
                )
            elif domain == "visual":
                self.assertIn(
                    "connection-hub-providers-review",
                    coverage["surfaces"],
                    suite["id"],
                )
            elif domain == "accessibility":
                self.assertIn(
                    "connection-hub-providers-review",
                    coverage["tasks"],
                    suite["id"],
                )

    def test_resource_and_visual_matrix_covers_all_claimed_macos_and_linux_gpu_variants(self) -> None:
        policy = S1.load_policy()
        suites = {
            (suite["domain"], suite["environment_id"])
            for suite in policy["required_suites"]
        }
        for required in {
            ("resource", "linux-nvidia-wgpu"),
            ("resource", "macos-intel"),
            ("resource", "macos-apple-silicon"),
            ("visual", "macos-intel"),
            ("visual", "macos-apple-silicon"),
        }:
            self.assertIn(required, suites)


if __name__ == "__main__":
    unittest.main(verbosity=2)
