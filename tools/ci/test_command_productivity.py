#!/usr/bin/env python3
"""Mutation tests for the CP0 command-productivity policy checker."""

from __future__ import annotations

from copy import deepcopy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "check_command_productivity", ROOT / "tools/ci/check_command_productivity.py"
)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load tools/ci/check_command_productivity.py")
POLICY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(POLICY)


def load_fixture(name: str) -> dict:
    path = ROOT / "tests/fixtures/command-productivity" / name
    return json.loads(path.read_text(encoding="utf-8"))


def apply_mutation(document: dict, case: dict) -> None:
    path = case["path"]
    target = document
    for component in path[:-1]:
        target = target[component]
    leaf = path[-1]
    operation = case["operation"]
    if operation == "set":
        target[leaf] = case["value"]
    elif operation == "append":
        target[leaf].append(case["value"])
    elif operation == "delete":
        del target[leaf]
    else:
        raise AssertionError(f"unsupported hostile mutation operation: {operation}")


class CommandProductivityPolicyTests(unittest.TestCase):
    def setUp(self) -> None:
        self.contract = load_fixture("cp0-contract-v1.json")
        self.threats = load_fixture("cp0-threats-v1.json")
        self.hostile = load_fixture("cp0-hostile-mutations-v1.json")

    def test_repository_contract_validates(self) -> None:
        counts = POLICY.validate_repository(ROOT)
        self.assertEqual(counts["shells"], 5)
        self.assertEqual(counts["discoveries"], 5)
        self.assertEqual(counts["providers"], 11)
        self.assertEqual(counts["threats"], 16)
        self.assertGreater(counts["runtime_files"], 100)
        self.assertEqual(counts["cp2_pure_action_files"], 7)
        self.assertEqual(counts["cp4_pure_action_files"], 1)
        self.assertEqual(counts["cp4_provider_action_files"], 6)
        self.assertEqual(counts["cp5_suggestion_source_files"], 10)
        self.assertEqual(counts["cp4_application_files"], 1)
        self.assertEqual(counts["cp2_persistence_files"], 14)

    def test_versioned_hostile_mutation_corpus_is_rejected(self) -> None:
        self.assertEqual(set(self.hostile), {"schema", "phase", "cases"})
        self.assertEqual(self.hostile["schema"], 1)
        self.assertEqual(self.hostile["phase"], "CP0")
        identifiers = [case["id"] for case in self.hostile["cases"]]
        self.assertEqual(len(identifiers), len(set(identifiers)))
        self.assertEqual(len(identifiers), 11)
        for case in self.hostile["cases"]:
            with self.subTest(case=case["id"]):
                document = deepcopy(
                    self.contract if case["document"] == "contract" else self.threats
                )
                apply_mutation(document, case)
                validator = (
                    POLICY.validate_contract
                    if case["document"] == "contract"
                    else POLICY.validate_threats
                )
                with self.assertRaisesRegex(
                    POLICY.CommandProductivityError, case["error"]
                ):
                    validator(document)

    def test_missing_shell_is_rejected(self) -> None:
        mutated = deepcopy(self.contract)
        mutated["shells"] = [
            shell for shell in mutated["shells"] if shell["id"] != "fish"
        ]
        with self.assertRaisesRegex(POLICY.CommandProductivityError, "shell matrix"):
            POLICY.validate_contract(mutated)

    def test_weakened_precedence_is_rejected(self) -> None:
        mutated = deepcopy(self.contract)
        mutated["precedence"][0:2] = ["capsule", "session"]
        with self.assertRaisesRegex(POLICY.CommandProductivityError, "precedence"):
            POLICY.validate_contract(mutated)

    def test_provider_keystroke_execution_is_rejected(self) -> None:
        mutated = deepcopy(self.contract)
        mutated["providers"][0]["keystroke"] = True
        with self.assertRaisesRegex(POLICY.CommandProductivityError, "per keystroke"):
            POLICY.validate_contract(mutated)

    def test_resource_ceiling_increase_is_rejected(self) -> None:
        mutated = deepcopy(self.contract)
        mutated["limits"]["source_file_bytes"] += 1
        with self.assertRaisesRegex(POLICY.CommandProductivityError, "resource ceilings"):
            POLICY.validate_contract(mutated)

    def test_duplicate_threat_is_rejected(self) -> None:
        mutated = deepcopy(self.threats)
        mutated["threats"].append(deepcopy(mutated["threats"][0]))
        with self.assertRaisesRegex(POLICY.CommandProductivityError, "duplicate id"):
            POLICY.validate_threats(mutated)

    def test_missing_threat_is_rejected(self) -> None:
        mutated = deepcopy(self.threats)
        mutated["threats"].pop()
        with self.assertRaisesRegex(POLICY.CommandProductivityError, "threat catalog"):
            POLICY.validate_threats(mutated)

    def test_unaccepted_adr_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative in (
                "docs/adr/0015-shell-native-completion-and-typed-quick-actions.md",
                "docs/COMMAND-PRODUCTIVITY.md",
                "docs/COMMAND-PRODUCTIVITY-COMPATIBILITY.md",
                "docs/COMMAND-PRODUCTIVITY-THREAT-MODEL.md",
                "docs/STABILIZATION-ROADMAP.md",
            ):
                destination = root / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                content = (ROOT / relative).read_text(encoding="utf-8")
                destination.write_text(
                    content.replace("- Status: Accepted for v0.5", "- Status: Proposed for v0.5"),
                    encoding="utf-8",
                )
            with self.assertRaisesRegex(POLICY.CommandProductivityError, "Status"):
                POLICY.validate_documents(root)

    def test_shell_startup_provider_hook_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "shell-integration").mkdir(parents=True)
            (root / "shell-integration/automexia.sh").write_text(
                "docker   \n completion bash\n", encoding="utf-8"
            )
            for relative in (
                "apps/automexia-terminal/src/renderer",
                "apps/automexia-terminal/src/screen",
                "rio-vt/src",
                "teletypewriter/src",
            ):
                path = root / relative
                path.mkdir(parents=True)
                (path / "safe.rs").write_text("fn safe() {}\n", encoding="utf-8")
            test = root / "tools/ci/test_shell_integration.ps1"
            test.parent.mkdir(parents=True)
            test.write_text(
                "alias docker alias kubectl function ax function kgp\n", encoding="utf-8"
            )
            with self.assertRaisesRegex(POLICY.CommandProductivityError, "shell startup"):
                POLICY.validate_pre_activation(root)

    def test_reviewed_cp5_adapter_is_inert_not_startup_activation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            adapter = (
                root
                / "shell-integration/suggestions/fish/automexia-suggestions.fish"
            )
            adapter.parent.mkdir(parents=True)
            adapter.write_text("complete -C\n", encoding="utf-8")
            self.assertEqual(POLICY.validate_shell_pre_activation(root), 1)

            active = root / "shell-integration/automexia.fish"
            active.write_text("complete -c kubectl\n", encoding="utf-8")
            with self.assertRaisesRegex(POLICY.CommandProductivityError, "shell startup"):
                POLICY.validate_shell_pre_activation(root)

    def test_terminal_grid_command_inference_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "shell-integration").mkdir(parents=True)
            (root / "shell-integration/safe.sh").write_text("true\n", encoding="utf-8")
            for relative in (
                "apps/automexia-terminal/src/renderer",
                "apps/automexia-terminal/src/screen",
                "rio-vt/src",
                "teletypewriter/src",
            ):
                path = root / relative
                path.mkdir(parents=True)
                content = (
                    "fn infer() { let _ = QuickAction; let _ = terminal.grid; }\n"
                    if relative.endswith("renderer")
                    else "fn safe() {}\n"
                )
                (path / "source.rs").write_text(content, encoding="utf-8")
            test = root / "tools/ci/test_shell_integration.ps1"
            test.parent.mkdir(parents=True)
            test.write_text(
                "alias docker alias kubectl function ax function kgp\n", encoding="utf-8"
            )
            with self.assertRaisesRegex(POLICY.CommandProductivityError, "grid inference"):
                POLICY.validate_pre_activation(root)

    def test_runtime_productivity_activation_outside_ui_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text(
                '[workspace]\nmembers = ["automexia-devops"]\n',
                encoding="utf-8",
            )
            runtime = root / "automexia-devops/src"
            runtime.mkdir(parents=True)
            (runtime / "lib.rs").write_text(
                "struct QuickActionRuntime;\n", encoding="utf-8"
            )
            (root / "shell-integration").mkdir(parents=True)
            (root / "shell-integration/safe.sh").write_text(
                "true\n", encoding="utf-8"
            )
            for relative in (
                "apps/automexia-terminal/src/renderer",
                "apps/automexia-terminal/src/screen",
                "rio-vt/src",
                "teletypewriter/src",
            ):
                path = root / relative
                path.mkdir(parents=True)
                (path / "safe.rs").write_text("fn safe() {}\n", encoding="utf-8")
            shell_test = root / "tools/ci/test_shell_integration.ps1"
            shell_test.parent.mkdir(parents=True)
            shell_test.write_text(
                "alias docker alias kubectl function ax function kgp\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(
                POLICY.CommandProductivityError,
                "non-runtime CP0",
            ):
                POLICY.validate_pre_activation(root)

    def test_pure_action_model_rejects_capability_bearing_source(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            files = []
            for relative in sorted(POLICY.PURE_ACTION_FILES):
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("struct QuickAction;\n", encoding="utf-8")
                files.append(path)
            for capability in (
                "use std::fs;",
                "use std::process::Command;",
                "use std::net::TcpStream;",
                "tokio::spawn(task);",
                "async_std::task::spawn(task);",
                "ureq::get(url);",
                "hyper::client();",
                "std::env::var(name);",
                "automexia_ui_model::Panel;",
                "rio_vt::Terminal;",
                "teletypewriter::Pty;",
                "unsafe { invoke(); }",
            ):
                with self.subTest(capability=capability):
                    files[0].write_text(
                        f"{capability}\nstruct QuickAction;\n",
                        encoding="utf-8",
                    )
                    with self.assertRaisesRegex(
                        POLICY.CommandProductivityError, "capability-free CP2/CP3/CP4"
                    ):
                        POLICY.validate_pure_action_sources(root, files)

    def test_pure_action_model_rejects_unreviewed_source_expansion(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            files = []
            for relative in sorted(POLICY.PURE_ACTION_FILES):
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("struct QuickAction;\n", encoding="utf-8")
                files.append(path)
            unexpected = root / "automexia-command-productivity/src/actions/runtime.rs"
            unexpected.write_text("struct Runtime;\n", encoding="utf-8")
            files.append(unexpected)
            with self.assertRaisesRegex(
                POLICY.CommandProductivityError, "exact reviewed boundary"
            ):
                POLICY.validate_pure_action_sources(root, files)

    def test_cp4_provider_contributors_are_an_exact_reviewed_set(self) -> None:
        self.assertEqual(
            POLICY.CP4_PROVIDER_ACTION_FILES,
            {
                "extensions/devops-aws/src/lib.rs",
                "extensions/devops-azure/src/lib.rs",
                "extensions/devops-gcp/src/lib.rs",
                "extensions/devops-kubernetes/src/implementation.rs",
                "extensions/devops-openshift/src/implementation.rs",
                "extensions/devops-teleport/src/lib.rs",
            },
        )

    def test_cp4_provider_projection_has_a_separate_pure_source_owner(self) -> None:
        self.assertEqual(
            POLICY.CP4_PURE_ACTION_FILES,
            {"automexia-command-productivity/src/actions/provider.rs"},
        )
        self.assertTrue(
            POLICY.CP2_PURE_ACTION_FILES.isdisjoint(
                POLICY.CP4_PURE_ACTION_FILES
            )
        )

    def test_cp4_application_composition_has_one_reviewed_owner(self) -> None:
        self.assertEqual(
            POLICY.CP4_APPLICATION_FILES,
            {
                "apps/automexia-terminal/src/automexia/quick_actions/providers.rs",
            },
        )
    def _persistence_fixture(self, root: Path) -> list[Path]:
        files = []
        for relative in sorted(POLICY.CP2_PERSISTENCE_FILES):
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("struct PersistenceBoundary;\n", encoding="utf-8")
            files.append(path)
        wiring = root / "apps/automexia-terminal/src/automexia/mod.rs"
        wiring.parent.mkdir(parents=True, exist_ok=True)
        wiring.write_text("pub mod quick_actions;\n", encoding="utf-8")
        return files

    def test_persistence_boundary_rejects_capability_bearing_source(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            files = self._persistence_fixture(root)
            target = next(path for path in files if path.name == "mod.rs")
            for capability in (
                "use std::net::TcpStream;",
                "use std::process::Command;",
                'let url = "https://example.invalid"; std::process::Command::new("x");',
                "std::env::var(name);",
                "clipboard::read();",
                "launch_broker::launch();",
                "shell_integration::install();",
                "rio_vt::Terminal::new();",
                "teletypewriter::Pty::new();",
                "reqwest::get(url);",
                "ureq::get(url);",
                "hyper::client();",
                "tokio::spawn(task);",
                "async_std::task::spawn(task);",
                "terminal.grid.read();",
                "visible_text();",
                "raw_cursor_line_text();",
                "unsafe { invoke(); }",
            ):
                with self.subTest(capability=capability):
                    target.write_text(capability + "\n", encoding="utf-8")
                    with self.assertRaisesRegex(
                        POLICY.CommandProductivityError,
                        "persistence-only capability boundary|unsafe code",
                    ):
                        POLICY.validate_persistence_sources(root, files)

    def test_persistence_boundary_allows_documented_denials(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            files = self._persistence_fixture(root)
            target = next(path for path in files if path.name == "mod.rs")
            target.write_text(
                "//! No network, process, clipboard, or PTY authority.\n"
                "/* shell_integration and launch_broker remain denied. */\n"
                "const DENIED: &str = \"std::process clipboard::read https://example\";\n"
                "struct PersistenceBoundary;\n",
                encoding="utf-8",
            )
            self.assertEqual(
                POLICY.validate_persistence_sources(root, files),
                POLICY.CP2_PERSISTENCE_FILES,
            )

    def test_persistence_boundary_rejects_unreviewed_source_expansion(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            files = self._persistence_fixture(root)
            unexpected = (
                root
                / "apps/automexia-terminal/src/automexia/quick_actions/runtime.rs"
            )
            unexpected.write_text("struct Runtime;\n", encoding="utf-8")
            files.append(unexpected)
            with self.assertRaisesRegex(
                POLICY.CommandProductivityError, "exact reviewed boundary"
            ):
                POLICY.validate_persistence_sources(root, files)
    def test_persistence_boundary_accepts_exact_reviewed_cp31_expansion(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            files = self._persistence_fixture(root)
            for relative in sorted(POLICY.CP31_PUBLICATION_FILES):
                path = root / relative
                path.write_text("struct PublicationBoundary;\n", encoding="utf-8")
                files.append(path)
            self.assertEqual(
                POLICY.validate_persistence_sources(root, files),
                POLICY.CP2_PERSISTENCE_FILES | POLICY.CP31_PUBLICATION_FILES,
            )

    def test_persistence_boundary_accepts_and_confines_cp32_pack_cli(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            files = self._persistence_fixture(root)
            relative = next(iter(POLICY.CP32_PACK_FILES))
            pack_cli = root / relative
            pack_cli.write_text("struct PackCliBoundary;\n", encoding="utf-8")
            files.append(pack_cli)
            self.assertEqual(
                POLICY.validate_persistence_sources(root, files),
                POLICY.CP2_PERSISTENCE_FILES | POLICY.CP32_PACK_FILES,
            )
            pack_cli.write_text("use std::process::Command;\n", encoding="utf-8")
            with self.assertRaisesRegex(
                POLICY.CommandProductivityError, "CP3.2 pack CLI-only"
            ):
                POLICY.validate_persistence_sources(root, files)

    def test_persistence_boundary_accepts_and_confines_cp33_workspace_files(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            files = self._persistence_fixture(root)
            workspace_files = sorted(POLICY.CP33_WORKSPACE_FILES)
            first = root / workspace_files[0]
            first.write_text("struct WorkspaceBoundary;\n", encoding="utf-8")
            files.append(first)
            with self.assertRaisesRegex(
                POLICY.CommandProductivityError, "exact reviewed boundary"
            ):
                POLICY.validate_persistence_sources(root, files)

            second = root / workspace_files[1]
            second.write_text("struct WorkspaceBoundary;\n", encoding="utf-8")
            files.append(second)
            self.assertEqual(
                POLICY.validate_persistence_sources(root, files),
                POLICY.CP2_PERSISTENCE_FILES | POLICY.CP33_WORKSPACE_FILES,
            )
            first.write_text("use std::process::Command;\n", encoding="utf-8")
            with self.assertRaisesRegex(
                POLICY.CommandProductivityError, "CP3.3 import/workspace-only"
            ):
                POLICY.validate_persistence_sources(root, files)

    def test_persistence_boundary_rejects_partial_or_capability_bearing_cp31(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            files = self._persistence_fixture(root)
            publication = sorted(POLICY.CP31_PUBLICATION_FILES)
            first = root / publication[0]
            first.write_text("struct PublicationBoundary;\n", encoding="utf-8")
            files.append(first)
            with self.assertRaisesRegex(
                POLICY.CommandProductivityError, "exact reviewed boundary"
            ):
                POLICY.validate_persistence_sources(root, files)

            second = root / publication[1]
            second.write_text("use std::net::TcpStream;\n", encoding="utf-8")
            files.append(second)
            with self.assertRaisesRegex(
                POLICY.CommandProductivityError, "CP3.1 publication-only"
            ):
                POLICY.validate_persistence_sources(root, files)

    def test_scanned_source_size_ceiling_is_enforced_before_read(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "oversized.rs"
            with path.open("wb") as destination:
                destination.seek(POLICY.SCANNED_SOURCE_MAX_BYTES)
                destination.write(b"x")
            with self.assertRaisesRegex(
                POLICY.CommandProductivityError,
                "exceeds",
            ):
                POLICY.read_lower(path)

    def test_policy_reader_rejects_symbolic_links_before_read(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "fixture.json"
            path.write_text("{}\n", encoding="utf-8")
            with patch.object(Path, "is_symlink", return_value=True):
                with self.assertRaisesRegex(
                    POLICY.CommandProductivityError,
                    "must not be a symbolic link",
                ):
                    POLICY.bounded_read_text(
                        path,
                        POLICY.POLICY_DOCUMENT_MAX_BYTES,
                        "test policy",
                    )

    def test_source_file_count_ceiling_is_enforced(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            sources = root / "shell-integration"
            sources.mkdir()
            (sources / "one.sh").write_text("true\n", encoding="utf-8")
            (sources / "two.sh").write_text("true\n", encoding="utf-8")
            with patch.object(POLICY, "SCANNED_SOURCE_MAX_FILES", 1):
                with self.assertRaisesRegex(
                    POLICY.CommandProductivityError,
                    "file scan limit",
                ):
                    POLICY.source_files(root, "shell-integration")

    def test_workspace_member_cannot_escape_repository_boundary(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text(
                '[workspace]\nmembers = ["../outside"]\n',
                encoding="utf-8",
            )
            with self.assertRaisesRegex(
                POLICY.CommandProductivityError,
                "escapes the repository boundary",
            ):
                POLICY.workspace_runtime_files(root)

    def test_missing_ci_wiring_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            files = {
                "tools/ci/validate_repository.py": (
                    'validate_command_productivity()\n'
                    '"command productivity CP0"\n'
                ),
                "tools/xtask/src/main.rs": (
                    'run_python("tools/ci/check_command_productivity.py")?;\n'
                ),
                ".github/workflows/ci.yml": "name: policy\n",
            }
            for relative, content in files.items():
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content, encoding="utf-8")
            with self.assertRaisesRegex(
                POLICY.CommandProductivityError,
                "test_command_productivity.py",
            ):
                POLICY.validate_wiring(root)


if __name__ == "__main__":
    unittest.main()
