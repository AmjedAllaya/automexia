#!/usr/bin/env python3
"""Mutation tests for the accepted CP5.1-CP5.6 editor-bridge contract."""

from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path
import sys
import unittest


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools" / "ci"))

import check_command_productivity_cp51 as checker  # noqa: E402


class Cp51ContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.document = checker.parse_contract(
            (ROOT / checker.CONTRACT_PATH).read_text(encoding="utf-8")
        )

    def assert_rejected(self, mutate) -> None:  # type: ignore[no-untyped-def]
        # Route every mutation through the production validator while keeping
        # the parsed canonical contract immutable for the next scenario.
        changed = deepcopy(self.document)
        mutate(changed)
        with self.assertRaises(checker.Cp51Error):
            checker.validate_contract(changed)

    def test_canonical_contract_and_documents_validate(self) -> None:
        counts = checker.validate_repository()
        self.assertEqual(counts, {"threats": 6, "sources": 6, "shells": 7, "limits": 15})

    def test_duplicate_json_keys_are_rejected(self) -> None:
        with self.assertRaises(checker.Cp51Error):
            checker.parse_contract('{"schema": 1, "schema": 2}')

    def test_source_authority_is_accepted_but_preview_cannot_activate(self) -> None:
        self.assert_rejected(lambda value: value["authority"].update(accepted=False))
        self.assert_rejected(
            lambda value: value["authority"].update(runtime_activation=True)
        )

    def test_transport_downgrades_are_rejected(self) -> None:
        self.assert_rejected(
            lambda value: value["transport"]["windows"].update(remote_clients="accepted")
        )
        self.assert_rejected(
            lambda value: value["transport"]["unix"].update(socket_mode="0666")
        )
        self.assert_rejected(lambda value: value["transport"].update(forbidden=["osc"]))

    def test_peer_capability_and_replay_downgrades_are_rejected(self) -> None:
        self.assert_rejected(
            lambda value: value["identity_binding"].update(capability_bytes=8)
        )
        self.assert_rejected(
            lambda value: value["identity_binding"].update(replay_window=1)
        )
        self.assert_rejected(
            lambda value: value["identity_binding"]["required"].remove("session")
        )

    def test_framing_allocation_and_schema_downgrades_are_rejected(self) -> None:
        self.assert_rejected(
            lambda value: value["framing"].update(unknown_fields="ignore")
        )
        self.assert_rejected(
            lambda value: value["framing"].update(
                allocation_rule="allocate-before-length-check"
            )
        )

    def test_request_privacy_and_span_fields_are_required(self) -> None:
        self.assert_rejected(
            lambda value: value["request"]["required_fields"].remove("replacement_span")
        )
        self.assert_rejected(
            lambda value: value["request"].update(persistence="history-database")
        )
        self.assert_rejected(
            lambda value: value["request"].update(logging="full-buffer")
        )

    def test_candidate_identity_display_and_no_enter_are_required(self) -> None:
        self.assert_rejected(
            lambda value: value["candidate"]["required_fields"].remove("insertion")
        )
        self.assert_rejected(
            lambda value: value["candidate"].update(acceptance="insert-and-enter")
        )
        self.assert_rejected(
            lambda value: value["candidate"].update(display_policy="trusted-markup")
        )

    def test_each_threat_needs_controls_mutations_owner_and_residual_risk(self) -> None:
        self.assert_rejected(lambda value: value["security_threats"].pop())
        self.assert_rejected(
            lambda value: value["security_threats"][0].update(controls=[])
        )
        self.assert_rejected(
            lambda value: value["security_threats"][1].update(hostile_mutations=[])
        )
        self.assert_rejected(
            lambda value: value["security_threats"][2].update(verification_owner=[])
        )
        self.assert_rejected(
            lambda value: value["security_threats"][3].update(residual_risk="")
        )

    def test_reviewed_control_and_external_gate_text_is_digest_frozen(self) -> None:
        self.assert_rejected(
            lambda value: value["security_threats"][0]["controls"].__setitem__(
                0, "weaker but nonempty control"
            )
        )
        self.assert_rejected(
            lambda value: value["external_gates"].__setitem__(
                0, "review no longer required"
            )
        )

    def test_source_priority_and_io_are_frozen(self) -> None:
        self.assert_rejected(lambda value: value["sources"].reverse())
        self.assert_rejected(
            lambda value: value["sources"][0].update(io="provider-network")
        )
        self.assert_rejected(
            lambda value: value["sources"].append(
                {
                    "id": "ai",
                    "priority": 7,
                    "opt_in": True,
                    "owner": "remote-service",
                    "io": "provider-network",
                }
            )
        )

    def test_ranking_and_stable_ties_are_frozen(self) -> None:
        self.assert_rejected(lambda value: value["ranking"]["order"].reverse())
        self.assert_rejected(
            lambda value: value["ranking"].update(tie_breakers=["random"])
        )

    def test_ui_dismissal_enter_and_shortcut_policy_are_frozen(self) -> None:
        self.assert_rejected(
            lambda value: value["ui"]["dismiss_on"].remove("ime-start")
        )
        self.assert_rejected(
            lambda value: value["ui"].update(enter_behavior="accept-and-submit")
        )
        self.assert_rejected(
            lambda value: value["ui"].update(shortcut_policy="global-ctrl-space")
        )

    def test_fish_collision_and_truthful_fallbacks_are_frozen(self) -> None:
        # Resolve rows by stable shell identity rather than array position; the
        # assertion is about each shell contract, not incidental fixture order.
        fish = next(
            index
            for index, row in enumerate(self.document["shell_matrix"])
            if row["id"] == "fish"
        )
        self.assert_rejected(
            lambda value: value["shell_matrix"][fish].update(shortcut="ctrl-space")
        )
        cmd = next(
            index
            for index, row in enumerate(self.document["shell_matrix"])
            if row["id"] == "cmd"
        )
        self.assert_rejected(
            lambda value: value["shell_matrix"][cmd].update(verdict="supported")
        )

    def test_resource_deadline_and_queue_downgrades_are_rejected(self) -> None:
        self.assert_rejected(lambda value: value["limits"].update(buffer_bytes=65_536))
        self.assert_rejected(
            lambda value: value["limits"].update(per_pane_queued_requests=8)
        )
        self.assert_rejected(
            lambda value: value["lifecycle"].update(queue="unbounded")
        )
        self.assert_rejected(
            lambda value: value["lifecycle"].update(
                kill_switch="restart-and-edit-profile"
            )
        )

    def test_verification_and_external_gates_cannot_be_erased(self) -> None:
        self.assert_rejected(lambda value: value["verification"].pop("privacy"))
        self.assert_rejected(lambda value: value.update(external_gates=[]))

    def test_contract_round_trip_is_stable(self) -> None:
        encoded = json.dumps(self.document, ensure_ascii=False, sort_keys=True)
        self.assertEqual(checker.parse_contract(encoded), self.document)


if __name__ == "__main__":
    unittest.main()
