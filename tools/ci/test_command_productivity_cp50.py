#!/usr/bin/env python3
"""Mutation and boundary tests for the CP5.0 autocomplete research contract."""

from copy import deepcopy
import importlib.util
import json
from pathlib import Path
import sys
import unittest


ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "check_command_productivity_cp50",
    ROOT / "tools/ci/check_command_productivity_cp50.py",
)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load CP5.0 checker")
POLICY = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = POLICY
SPEC.loader.exec_module(POLICY)
CONTRACT = json.loads(
    (
        ROOT
        / "tests/fixtures/command-productivity/cp50-research-contract-v1.json"
    ).read_text(encoding="utf-8")
)


class Cp50PolicyTests(unittest.TestCase):
    def test_repository_contract_validates(self) -> None:
        counts = POLICY.validate_repository(ROOT)
        self.assertEqual(counts["shells"], 7)
        self.assertEqual(counts["corpus_sizes"], 3)
        self.assertEqual(counts["dependency_decisions"], 4)

    def test_duplicate_contract_key_is_rejected(self) -> None:
        with self.assertRaisesRegex(POLICY.Cp50Error, "duplicate key"):
            POLICY.parse_contract('{"schema":1,"schema":1}')



    def test_runtime_activation_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["capabilities"]["runtime_activation"] = True
        with self.assertRaisesRegex(POLICY.Cp50Error, "capability boundary"):
            POLICY.validate_contract(changed)

    def test_profile_or_keybinding_mutation_is_rejected(self) -> None:
        for capability in ("profile_mutation", "keybinding_mutation"):
            with self.subTest(capability=capability):
                changed = deepcopy(CONTRACT)
                changed["capabilities"][capability] = True
                with self.assertRaisesRegex(POLICY.Cp50Error, "capability boundary"):
                    POLICY.validate_contract(changed)

    def test_missing_shell_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["shell_matrix"] = changed["shell_matrix"][:-1]
        with self.assertRaisesRegex(POLICY.Cp50Error, "shell matrix"):
            POLICY.validate_contract(changed)

    def test_unreviewed_p2_approval_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["decision"]["p2"] = "approved"
        with self.assertRaisesRegex(POLICY.Cp50Error, "decision"):
            POLICY.validate_contract(changed)

    def test_runtime_matcher_dependency_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["decision"]["runtime_dependency"] = "nucleo-matcher"
        with self.assertRaisesRegex(POLICY.Cp50Error, "decision"):
            POLICY.validate_contract(changed)

    def test_stale_candidate_batch_is_rejected(self) -> None:
        snapshot = POLICY.EditorSnapshot(
            buffer="kubectl get dep",
            cursor_byte=15,
            replacement_start_byte=12,
            replacement_end_byte=15,
            generation=8,
        )
        batch = POLICY.CandidateBatch(
            generation=7,
            candidates=("deployments",),
        )
        with self.assertRaisesRegex(POLICY.Cp50Error, "stale generation"):
            POLICY.build_insertion_request(snapshot, batch, 0)

    def test_non_utf8_boundary_span_is_rejected(self) -> None:
        # Adding one byte to the encoded "f" position lands inside the two-byte
        # final character, reproducing a span that is numeric but not editable.
        snapshot = POLICY.EditorSnapshot(
            buffer="echo café",
            cursor_byte=len("echo café".encode()),
            replacement_start_byte=len("echo caf".encode()) + 1,
            replacement_end_byte=len("echo café".encode()),
            generation=4,
        )
        batch = POLICY.CandidateBatch(generation=4, candidates=("café",))
        with self.assertRaisesRegex(POLICY.Cp50Error, "UTF-8 boundary"):
            POLICY.build_insertion_request(snapshot, batch, 0)

    def test_control_character_candidate_is_rejected(self) -> None:
        snapshot = POLICY.EditorSnapshot(
            buffer="git sta",
            cursor_byte=7,
            replacement_start_byte=4,
            replacement_end_byte=7,
            generation=1,
        )
        batch = POLICY.CandidateBatch(
            generation=1,
            candidates=("status\r\nshutdown",),
        )
        with self.assertRaisesRegex(POLICY.Cp50Error, "control character"):
            POLICY.build_insertion_request(snapshot, batch, 0)

    def test_insertion_request_never_executes(self) -> None:
        snapshot = POLICY.EditorSnapshot(
            buffer="kubectl get dep",
            cursor_byte=15,
            replacement_start_byte=12,
            replacement_end_byte=15,
            generation=9,
        )
        batch = POLICY.CandidateBatch(
            generation=9,
            candidates=("deployments", "daemonsets"),
        )
        request = POLICY.build_insertion_request(snapshot, batch, 0)
        # Insertion may replace editor bytes, but it must never manufacture the
        # Enter/execute authority owned by the terminal input path.
        self.assertEqual(request.replacement, "deployments")
        self.assertEqual((request.start_byte, request.end_byte), (12, 15))
        self.assertFalse(request.execute)

    def test_unselected_unsafe_candidate_is_rejected(self) -> None:
        snapshot = POLICY.EditorSnapshot(
            buffer="git sta",
            cursor_byte=7,
            replacement_start_byte=4,
            replacement_end_byte=7,
            generation=2,
        )
        batch = POLICY.CandidateBatch(
            generation=2,
            candidates=("status", "reset\x1b[31m"),
        )
        # Validate the complete untrusted batch before indexing it. A hidden
        # hostile candidate must not survive merely because index zero is safe.
        with self.assertRaisesRegex(POLICY.Cp50Error, "character"):
            POLICY.build_insertion_request(snapshot, batch, 0)

    def test_bidirectional_override_candidate_is_rejected(self) -> None:
        snapshot = POLICY.EditorSnapshot(
            buffer="ssh prod",
            cursor_byte=8,
            replacement_start_byte=4,
            replacement_end_byte=8,
            generation=3,
        )
        batch = POLICY.CandidateBatch(
            generation=3,
            candidates=("prod\u202etxt",),
        )
        with self.assertRaisesRegex(POLICY.Cp50Error, "unsafe character"):
            POLICY.build_insertion_request(snapshot, batch, 0)

if __name__ == "__main__":
    unittest.main()
