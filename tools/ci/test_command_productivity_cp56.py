#!/usr/bin/env python3
"""Mutation tests for the disabled CP5 source implementation boundary."""

from __future__ import annotations

from copy import deepcopy
from pathlib import Path
import sys
import unittest


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools" / "ci"))

import check_command_productivity_cp56 as checker  # noqa: E402


class Cp56ImplementationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.texts = {
            relative: (ROOT / relative).read_text(encoding="utf-8")
            for relative in checker.REQUIRED
        }

    def test_repository_source_boundary_validates(self) -> None:
        counts = checker.validate_repository()
        self.assertEqual(counts["shell_adapters"], 4)
        self.assertEqual(counts["external_shell_gates"], 3)

    def test_missing_control_is_rejected(self) -> None:
        changed = deepcopy(self.texts)
        path = "apps/automexia-terminal/src/automexia/suggestions/platform/windows.rs"
        changed[path] = changed[path].replace("PIPE_REJECT_REMOTE_CLIENTS", "REMOVED")
        with self.assertRaises(checker.Cp56Error):
            checker.validate_texts(changed)

    def test_missing_lifecycle_transaction_is_rejected(self) -> None:
        changed = deepcopy(self.texts)
        path = "apps/automexia-terminal/src/automexia/suggestions/service.rs"
        changed[path] = changed[path].replace("lifecycle: Mutex<()>", "removed")
        with self.assertRaises(checker.Cp56Error):
            checker.validate_texts(changed)

    def test_network_transport_is_rejected(self) -> None:
        changed = deepcopy(self.texts)
        path = "apps/automexia-terminal/src/automexia/suggestions/service.rs"
        changed[path] += "\nTcpListener\n"
        with self.assertRaises(checker.Cp56Error):
            checker.validate_texts(changed)

    def test_shell_process_and_enter_authority_are_rejected(self) -> None:
        for mutation in ("Start-Process", "Invoke-WebRequest", "Enter"):
            changed = deepcopy(self.texts)
            path = "shell-integration/suggestions/powershell/automexia-suggestions.ps1"
            changed[path] += f"\n{mutation}\n"
            with self.assertRaises(checker.Cp56Error):
                checker.validate_texts(changed)

    def test_missing_preview_guard_is_rejected(self) -> None:
        changed = deepcopy(self.texts)
        path = "shell-integration/suggestions/fish/automexia-suggestions.fish"
        changed[path] = changed[path].replace("preview-disabled", "removed")
        with self.assertRaises(checker.Cp56Error):
            checker.validate_texts(changed)

    def test_helper_transport_and_session_translation_cannot_be_removed(self) -> None:
        for path, marker in (
            (
                "apps/automexia-terminal/src/automexia/suggestions/helper_transport.rs",
                "declared > SuggestionLimits::BATCH_BYTES",
            ),
            (
                "apps/automexia-terminal/src/automexia/suggestions/helper.rs",
                "AcceptanceContext::from_request",
            ),
            (
                "tools/ci/test_cp5_native_shell_bridge.py",
                "pass_fds",
            ),
        ):
            changed = deepcopy(self.texts)
            changed[path] = changed[path].replace(marker, "removed")
            with self.assertRaises(checker.Cp56Error):
                checker.validate_texts(changed)

    def test_every_shell_requires_response_and_stale_state_guards(self) -> None:
        for path in (
            "shell-integration/suggestions/powershell/automexia-suggestions.ps1",
            "shell-integration/suggestions/bash/automexia-suggestions.bash",
            "shell-integration/suggestions/zsh/automexia-suggestions.zsh",
            "shell-integration/suggestions/fish/automexia-suggestions.fish",
        ):
            changed = deepcopy(self.texts)
            changed[path] = changed[path].replace(
                "stale-editor-state" if "powershell" not in path else "GetBufferState",
                "removed",
            )
            with self.assertRaises(checker.Cp56Error):
                checker.validate_texts(changed)

    def test_authenticated_reply_and_route_exchange_cannot_be_removed(self) -> None:
        for path, marker in (
            (
                "automexia-devops/src/suggestions/reply.rs",
                "constant_time_eq",
            ),
            (
                "apps/automexia-terminal/src/automexia/suggestions/endpoint_service.rs",
                "submit_and_wait_for_ui",
            ),
            (
                "apps/automexia-terminal/tests/suggestion_publication.rs",
                "app_route_exchange_reads_submission_and_writes_exact_authenticated_reply",
            ),
            (
                "tools/ci/test_cp5_native_shell_adapters.py",
                "Fish response descriptor mismatch",
            ),
            (
                "shell-integration/suggestions/powershell/automexia-suggestions.ps1",
                "Read-AutomexiaBoundedResponseLine",
            ),
            (
                "shell-integration/suggestions/bash/automexia-suggestions.bash",
                "read -r -t 30 -n 2176",
            ),
            (
                "shell-integration/suggestions/zsh/automexia-suggestions.zsh",
                "sysread -i $fd -s 2176 -t 30",
            ),
        ):
            changed = deepcopy(self.texts)
            changed[path] = changed[path].replace(marker, "removed")
            with self.assertRaises(checker.Cp56Error):
                checker.validate_texts(changed)

    def test_all_shells_keep_strict_utf8_control_and_bidi_guards(self) -> None:
        for path, marker in (
            (
                "shell-integration/suggestions/powershell/automexia-suggestions.ps1",
                r"\p{Cc}",
            ),
            (
                "shell-integration/suggestions/bash/automexia-suggestions.bash",
                "b0 == 216 && b1 == 156",
            ),
            (
                "shell-integration/suggestions/zsh/automexia-suggestions.zsh",
                "b0 == 216 && b1 == 156",
            ),
            (
                "shell-integration/suggestions/fish/automexia-suggestions.fish",
                "$b0 -ne 216 -o $b1 -ne 156",
            ),
            ("tools/ci/test_cp5_native_shell_bridge.py", "E280AE"),
            ("tools/ci/test_cp5_native_shell_adapters.py", "E280AE"),
            ("tools/ci/test_cp5_native_powershell_bridge.ps1", "E280AE"),
            ("tools/ci/test_cp5_native_shell_bridge.py", "F4908080"),
            ("tools/ci/test_cp5_native_shell_adapters.py", "F4908080"),
            ("tools/ci/test_cp5_native_powershell_bridge.ps1", "F4908080"),
        ):
            changed = deepcopy(self.texts)
            changed[path] = changed[path].replace(marker, "removed")
            with self.assertRaises(checker.Cp56Error):
                checker.validate_texts(changed)

    def test_fish_fixed_handles_and_noninteractive_reply_path_are_enforced(self) -> None:
        path = "shell-integration/suggestions/fish/automexia-suggestions.fish"
        for old, new in (
            ("</dev/fd/4", "<&4"),
            ("--nchars 2176", "--nchars 2200"),
            ("set -l fish_read_limit 2176", "set -l fish_read_limit 104857600"),
            ("set -l fish_read_limit 4096", "set -l fish_read_limit 104857600"),
            (
                'complete -C "$line" | while read --local --line completion',
                'for completion in (complete -C "$line")',
            ),
            ('AUTOMEXIA_SUGGESTION_RESPONSE_FD" != 4', 'AUTOMEXIA_SUGGESTION_RESPONSE_FD" != 9'),
            ("set -g __automexia_suggestion_reason accepted", "set -g __automexia_suggestion_reason accepted\n    commandline --function repaint"),
        ):
            changed = deepcopy(self.texts)
            changed[path] = changed[path].replace(old, new)
            with self.assertRaises(checker.Cp56Error):
                checker.validate_texts(changed)
    def test_shell_scaffolding_cannot_be_mislabeled_complete(self) -> None:
        changed = deepcopy(self.texts)
        path = "shell-integration/suggestions/README.md"
        changed[path] = changed[path].replace(
            "not an activated product bridge", "activated product bridge"
        )
        with self.assertRaises(checker.Cp56Error):
            checker.validate_texts(changed)


if __name__ == "__main__":
    unittest.main()
