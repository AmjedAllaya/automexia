#!/usr/bin/env python3
"""Mutation tests for the public Production Operations planning boundary."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile
import unittest


MODULE_PATH = Path(__file__).with_name("check_production_operations_po0.py")
SPEC = importlib.util.spec_from_file_location(
    "automexia_production_operations_boundary", MODULE_PATH
)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load Production Operations boundary checker")
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)


class ProductionOperationsBoundaryTests(unittest.TestCase):
    def make_repository(self) -> tuple[tempfile.TemporaryDirectory[str], Path]:
        temporary = tempfile.TemporaryDirectory()
        root = Path(temporary.name)
        for relative, markers in CHECKER.REQUIRED_DOCUMENTS.items():
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("\n".join(markers) + "\n", encoding="utf-8")
        for relative in (
            Path("Cargo.toml"),
            Path("docs/CONFIGURATION.md"),
            Path("docs/reference/configuration.md"),
        ):
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("# safe fixture\n", encoding="utf-8")
        source = root / "apps/automexia-terminal/src/lib.rs"
        source.parent.mkdir(parents=True, exist_ok=True)
        source.write_text("pub fn terminal_remains_available() {}\n", encoding="utf-8")
        return temporary, root

    def test_canonical_repository_passes(self) -> None:
        counts = CHECKER.validate_repository()
        self.assertEqual(counts["documents"], 5)
        self.assertEqual(counts["private_artifacts"], 0)
        self.assertGreater(counts["source_files_checked"], 100)

    def test_missing_public_status_fails_closed(self) -> None:
        temporary, root = self.make_repository()
        with temporary:
            relative = Path("docs/SITUATION-AWARE-PRODUCTION-OPERATIONS.md")
            (root / relative).write_text("# incomplete\n", encoding="utf-8")
            with self.assertRaisesRegex(
                CHECKER.ProductionOperationsBoundaryError, "lost public boundary"
            ):
                CHECKER.validate_repository(root)

    def test_private_planning_fixture_is_rejected(self) -> None:
        temporary, root = self.make_repository()
        with temporary:
            fixture = root / CHECKER.PRIVATE_FIXTURE
            fixture.parent.mkdir(parents=True, exist_ok=True)
            fixture.write_text('{"phase":"PO0"}\n', encoding="utf-8")
            with self.assertRaisesRegex(
                CHECKER.ProductionOperationsBoundaryError, "must not be tracked"
            ):
                CHECKER.validate_repository(root)

    def test_exact_schema_and_digest_markers_are_rejected(self) -> None:
        for marker in CHECKER.FORBIDDEN_PUBLIC_MARKERS:
            with self.subTest(marker=marker):
                temporary, root = self.make_repository()
                with temporary:
                    relative = next(iter(CHECKER.REQUIRED_DOCUMENTS))
                    path = root / relative
                    path.write_text(
                        path.read_text(encoding="utf-8") + marker + "\n",
                        encoding="utf-8",
                    )
                    with self.assertRaisesRegex(
                        CHECKER.ProductionOperationsBoundaryError,
                        "local-only planning detail",
                    ):
                        CHECKER.validate_repository(root)

    def test_runtime_path_is_rejected(self) -> None:
        temporary, root = self.make_repository()
        with temporary:
            path = root / CHECKER.FORBIDDEN_RUNTIME_PATHS[0]
            path.mkdir(parents=True)
            with self.assertRaisesRegex(
                CHECKER.ProductionOperationsBoundaryError, "runtime path"
            ):
                CHECKER.validate_repository(root)

    def test_every_activation_marker_is_rejected(self) -> None:
        for marker in CHECKER.FORBIDDEN_SOURCE_MARKERS:
            with self.subTest(marker=marker):
                temporary, root = self.make_repository()
                with temporary:
                    source = root / "apps/automexia-terminal/src/lib.rs"
                    source.write_text(f"// {marker}\n", encoding="utf-8")
                    with self.assertRaisesRegex(
                        CHECKER.ProductionOperationsBoundaryError,
                        "activating marker",
                    ):
                        CHECKER.validate_repository(root)

    def test_configuration_and_workspace_activation_are_rejected(self) -> None:
        temporary, root = self.make_repository()
        with temporary:
            (root / "docs/CONFIGURATION.md").write_text(
                "[production-operations]\n", encoding="utf-8"
            )
            with self.assertRaisesRegex(
                CHECKER.ProductionOperationsBoundaryError, "unimplemented settings"
            ):
                CHECKER.validate_repository(root)

        temporary, root = self.make_repository()
        with temporary:
            (root / "Cargo.toml").write_text(
                'automexia-operations-model = { path = "model" }\n',
                encoding="utf-8",
            )
            with self.assertRaisesRegex(
                CHECKER.ProductionOperationsBoundaryError, "model crate"
            ):
                CHECKER.validate_repository(root)


if __name__ == "__main__":
    unittest.main()
