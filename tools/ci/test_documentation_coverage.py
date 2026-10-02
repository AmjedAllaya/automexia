#!/usr/bin/env python3
"""Regression tests for source-to-documentation coverage validation."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import tempfile
import unittest


MODULE_PATH = Path(__file__).with_name("check_documentation_coverage.py")
SPEC = importlib.util.spec_from_file_location(
    "automexia_documentation_coverage", MODULE_PATH
)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load tools/ci/check_documentation_coverage.py")
COVERAGE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(COVERAGE)


class DocumentationCoverageTests(unittest.TestCase):
    def test_skipped_runtime_fields_are_not_advertised_as_input_settings(self) -> None:
        source = '''
#[serde(skip)]
pub ui_shortcuts: Vec<String>,
#[serde(default, skip_deserializing)]
pub cache: String,
#[serde(skip_serializing)]
pub writable: String,
#[serde(rename = "skip")]
pub renamed: String,
#[serde(skip_serializing_if = "Vec::is_empty")]
pub entries: Vec<String>,
'''
        self.assertEqual(COVERAGE.serde_keys(source), {"writable", "skip", "entries"})
        # Removing the attribute must restore documentation responsibility.
        self.assertIn("ui_shortcuts", COVERAGE.serde_keys(source.replace("#[serde(skip)]", "")))

    def test_canonical_documentation_covers_source_registries(self) -> None:
        counts = COVERAGE.validate()
        self.assertGreaterEqual(counts["pages"], 13)
        self.assertGreaterEqual(counts["config_keys"], 100)
        self.assertGreaterEqual(counts["binding_actions"], 60)
        self.assertEqual(counts["cli_flags"], 17)
        # Keep this reviewed inventory explicit: the coverage checker proves that
        # every registered command is documented, while this assertion catches a
        # parser regression that could otherwise make both sides silently shrink.
        self.assertEqual(
            COVERAGE.application_cli_commands(),
            {
                "actions",
                "aliases",
                "docs",
                "edit",
                "explain",
                "find",
                "google",
                "migrate",
                "open",
                "packs",
                "repo",
                "search",
                "shell-integration",
                "ssh-integration",
                "workspaces",
            },
        )
        self.assertEqual(counts["cli_commands"], 15)
        self.assertGreaterEqual(counts["xtask_commands"], 20)

    def test_missing_config_key_is_rejected(self) -> None:
        with self.assertRaisesRegex(
            COVERAGE.DocumentationCoverageError, "scrollback-history-limit"
        ):
            COVERAGE.require_tokens(
                "configuration reference", {"scrollback-history-limit"}, "other"
            )

    def test_grouped_assurance_usage_expands_to_documented_subcommands(self) -> None:
        commands = COVERAGE.xtask_commands()
        self.assertEqual(
            {command for command in commands if command.startswith("assurance ")},
            {
                "assurance check-policy",
                "assurance install-tools",
                "assurance initialize-vet",
                "assurance install-hook",
                "assurance audit-history-secrets",
                "assurance pre-push",
                "assurance release-local",
                "assurance deep-source",
            },
        )
        self.assertNotIn("install-tools", commands)

    def test_grouped_cache_usage_expands_to_documented_subcommands(self) -> None:
        commands = COVERAGE.xtask_commands()
        self.assertEqual(
            {command for command in commands if command.startswith("cache ")},
            {
                "cache status [--warn-gib N]",
                "cache gc [--scope automatic|tools|worktrees|all] [--grace-hours N] [--apply]",
            },
        )
        self.assertNotIn("cache", commands)

    def test_usage_alternatives_do_not_split_option_value_registries(self) -> None:
        self.assertEqual(
            COVERAGE.split_usage_alternatives(
                "status [--warn N]|generate <--one|--two>|gc [--scope automatic|tools|all] [--apply]"
            ),
            [
                "status [--warn N]",
                "generate <--one|--two>",
                "gc [--scope automatic|tools|all] [--apply]",
            ],
        )
        with self.assertRaisesRegex(
            COVERAGE.DocumentationCoverageError, "unmatched"
        ):
            COVERAGE.split_usage_alternatives("gc [--scope all")

    def test_missing_binding_action_is_rejected(self) -> None:
        with self.assertRaisesRegex(
            COVERAGE.DocumentationCoverageError, "previewselectedimage"
        ):
            COVERAGE.require_tokens(
                "keyboard reference", {"previewselectedimage"}, "copy paste"
            )

    def test_matching_is_case_insensitive(self) -> None:
        self.assertEqual(
            COVERAGE.require_tokens(
                "keyboard reference", {"reloadconfig"}, "ReloadConfig"
            ),
            1,
        )

    def test_top_level_clap_flags_are_included(self) -> None:
        body = """
        #[arg(long)]
        pub list_actions: bool,
        #[arg(long = "working-dir")]
        pub working_directory: Option<PathBuf>,
        #[command(subcommand)]
        pub command: Option<CliCommand>,
        #[command(flatten)]
        pub terminal_options: TerminalOptions,
        """
        self.assertEqual(
            COVERAGE.clap_long_flags(body),
            {"--list-actions", "--working-dir"},
        )

    def test_application_subcommands_are_kebab_case(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "apps" / "automexia-terminal" / "src" / "cli.rs"
            source.parent.mkdir(parents=True)
            source.write_text(
                "pub enum CliCommand {\n"
                "    ShellIntegration(ShellIntegrationCommand),\n"
                "    QuickActions(QuickActionsCommand),\n"
                "}\n",
                encoding="utf-8",
            )
            self.assertEqual(
                COVERAGE.application_cli_commands(root),
                {"shell-integration", "quick-actions"},
            )

    def test_canonical_page_requires_one_level_one_heading(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            page = root / "docs" / "index.md"
            page.parent.mkdir(parents=True)
            page.write_text("# First\n\n# Second\n", encoding="utf-8")
            with self.assertRaisesRegex(
                COVERAGE.DocumentationCoverageError,
                "exactly one level-one heading",
            ):
                COVERAGE.validate_canonical_pages(root, {"docs/index.md"})

if __name__ == "__main__":
    unittest.main(verbosity=2)
