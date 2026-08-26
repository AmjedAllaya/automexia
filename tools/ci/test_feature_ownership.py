#!/usr/bin/env python3
"""Mutation tests for core/domain/extension feature ownership."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "check_feature_ownership", ROOT / "tools/ci/check_feature_ownership.py"
)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load feature ownership checker")
policy = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(policy)


class FeatureOwnershipTests(unittest.TestCase):
    def write(self, root: Path, relative: str, contents: str) -> None:
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(contents, encoding="utf-8")

    def fixture(self, root: Path) -> None:
        self.write(
            root,
            "automexia-command-productivity/Cargo.toml",
            'name = "automexia-command-productivity"\n'
            "automexia-connectivity = {}\n"
            "automexia-extension-api = {}\n",
        )
        self.write(
            root,
            "automexia-connectivity/Cargo.toml",
            'name = "automexia-connectivity"\n',
        )
        self.write(
            root,
            "automexia-devops/Cargo.toml",
            'name = "automexia-devops"\n',
        )
        required_dependencies = (
            "automexia-command-productivity = {}\n"
            "automexia-connectivity = {}\n"
        )
        self.write(root, "automexia-ui-model/Cargo.toml", required_dependencies)
        self.write(
            root,
            "apps/automexia-terminal/Cargo.toml",
            required_dependencies,
        )
        self.write(
            root,
            "apps/automexia-terminal/src/renderer/command_results.rs",
            "pub struct CommandResults;\n",
        )
        self.write(
            root,
            "apps/automexia-terminal/src/renderer/devops_status.rs",
            "pub struct DevOpsStatus;\n",
        )
        self.write(
            root,
            "apps/automexia-terminal/src/renderer/mod.rs",
            """
pub struct Renderer {
    command_results: command_results::CommandResults,
    command_result_states: FxHashMap<usize, command_results::CommandResults>,
}
fn render() {
        if self.devops_enabled {
            self.devops_status.render();
        }
        if self.command_result_route != Some(active_route) {
            self.command_results.clear();
        }
        self.command_results.render_command_results();
}
    pub fn sync_extension_state(&mut self) -> bool {
        self.devops_status.clear();
    }
    pub(crate) fn native_test_pane_context() {}
""",
        )
        provider_manifest = (
            "automexia-command-productivity = {}\n"
            "automexia-connectivity = {}\n"
            "automexia-extension-api = {}\n"
        )
        for extension in (
            "devops-aws",
            "devops-azure",
            "devops-gcp",
            "devops-kubernetes",
            "devops-openshift",
            "devops-teleport",
        ):
            self.write(root, f"extensions/{extension}/Cargo.toml", provider_manifest)
        for engine in ("rio-vt", "teletypewriter", "sugarloaf", "rio-window"):
            self.write(root, f"{engine}/Cargo.toml", "[package]\n")

    def validate(self, root: Path) -> None:
        with mock.patch.object(policy, "ROOT", root):
            self.assertEqual(policy.main(), 0)

    def test_canonical_repository_passes(self) -> None:
        self.assertEqual(policy.main(), 0)

    def test_minimal_correct_ownership_fixture_passes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root)
            self.validate(root)

    def test_devops_gated_core_result_paint_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root)
            renderer = root / "apps/automexia-terminal/src/renderer/mod.rs"
            contents = renderer.read_text(encoding="utf-8")
            contents = contents.replace(
                "            self.devops_status.render();",
                "            self.devops_status.render();\n"
                "            self.command_results.render_command_results();",
            )
            renderer.write_text(contents, encoding="utf-8")
            with mock.patch.object(policy, "ROOT", root):
                with self.assertRaises(AssertionError):
                    policy.main()

    def test_mixed_devops_source_and_provider_dependency_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root)
            self.write(root, "automexia-devops/src/actions/mod.rs", "")
            provider = root / "extensions/devops-aws/Cargo.toml"
            provider.write_text(
                provider.read_text(encoding="utf-8")
                + "automexia-devops = {}\n",
                encoding="utf-8",
            )
            with mock.patch.object(policy, "ROOT", root):
                with self.assertRaises(AssertionError):
                    policy.main()

    def test_terminal_engine_domain_dependency_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.fixture(root)
            self.write(
                root,
                "rio-vt/Cargo.toml",
                "[dependencies]\nautomexia-connectivity = {}\n",
            )
            with mock.patch.object(policy, "ROOT", root):
                with self.assertRaises(AssertionError):
                    policy.main()


if __name__ == "__main__":
    unittest.main()
