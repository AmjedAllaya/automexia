#!/usr/bin/env python3
"""Mutation tests for the feature test reinforcement contract."""

from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


MODULE_PATH = Path(__file__).with_name("check_feature_test_reinforcement.py")
SPEC = importlib.util.spec_from_file_location("automexia_feature_test_reinforcement", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load feature test reinforcement checker")
REINFORCEMENT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(REINFORCEMENT)


class FeatureTestReinforcementTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.document = json.loads(REINFORCEMENT.DEFAULT_CONTRACT.read_text(encoding="utf-8"))

    def validate(self, document: dict) -> dict[str, int]:
        return REINFORCEMENT.validate_document(document)

    def test_canonical_contract_covers_every_feature_with_detailed_reinforcement(self) -> None:
        counts = self.validate(copy.deepcopy(self.document))
        self.assertEqual(counts["features"], 40)
        self.assertGreaterEqual(counts["needed_tests"], counts["features"] * 3)
        self.assertGreaterEqual(counts["evidence_owners"], counts["features"])
        self.assertEqual(counts["plan_anchors"], counts["features"])

    def test_missing_reordered_or_extra_feature_is_rejected(self) -> None:
        missing = copy.deepcopy(self.document)
        missing["features"].pop()
        with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "order/coverage"):
            self.validate(missing)

        reordered = copy.deepcopy(self.document)
        reordered["features"][0], reordered["features"][1] = reordered["features"][1], reordered["features"][0]
        with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "order/coverage"):
            self.validate(reordered)

    def test_visual_feature_cannot_lose_pixel_or_accessibility_proof(self) -> None:
        for field, value, message in (
            ("test_layers", "visual-pixel", "visual and accessibility layers"),
            ("oracles", "pixel-exact", "exact pixel/accessibility oracles"),
            ("oracles", "accessibility-tree-event", "exact pixel/accessibility oracles"),
        ):
            with self.subTest(field=field, value=value):
                document = copy.deepcopy(self.document)
                feature = next(item for item in document["features"] if item["flags"]["visual"])
                feature[field].remove(value)
                with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, message):
                    self.validate(document)

    def test_native_security_persistence_and_performance_layers_are_mandatory(self) -> None:
        mutations = (
            ("native", "test_layers", "native-e2e", "native end-to-end"),
            ("security", "oracles", "side-effect-absence", "absence proof"),
            ("persistence", "test_layers", "recovery-migration", "recovery/round-trip"),
            ("performance", "oracles", "latency-ratchet", "resource and latency"),
        )
        for flag, field, value, message in mutations:
            with self.subTest(flag=flag, value=value):
                document = copy.deepcopy(self.document)
                feature = next(item for item in document["features"] if item["flags"][flag])
                feature[field].remove(value)
                with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, message):
                    self.validate(document)

    def test_feature_requires_negative_tests_exit_scope_and_real_evidence_owner(self) -> None:
        document = copy.deepcopy(self.document)
        feature = document["features"][0]
        feature["needed_tests"] = ["Valid scenario A", "Valid scenario B", "Valid scenario C"]
        with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "negative or boundary"):
            self.validate(document)

        document = copy.deepcopy(self.document)
        document["features"][0]["exit_criteria"] = ["The model passes exactly", "The checker mutations fail"]
        with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "scope native/external"):
            self.validate(document)

        document = copy.deepcopy(self.document)
        document["features"][0]["evidence_owners"] = ["tests/missing-owner.rs"]
        with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "missing path"):
            self.validate(document)

    def test_plan_anchor_and_interaction_partner_cannot_drift(self) -> None:
        document = copy.deepcopy(self.document)
        document["features"][0]["plan_anchor"] = "missing-plan-section"
        with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "exact plan section"):
            self.validate(document)

        document = copy.deepcopy(self.document)
        document["features"][0]["interaction_partners"] = ["unknown-feature"]
        with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "unknown interaction"):
            self.validate(document)

    def test_duplicate_json_keys_and_absolute_claims_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "duplicate.json"
            path.write_text('{"schema":1,"schema":1}', encoding="utf-8")
            with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "duplicate key"):
                REINFORCEMENT._load_json(path, "fixture")

        document = copy.deepcopy(self.document)
        document["features"][0]["needed_tests"][0] = "Prove that no issues can escape this feature"
        with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "absolute claim"):
            self.validate(document)

    def test_default_visual_policy_cannot_tolerate_one_pixel_or_mask_it(self) -> None:
        original = REINFORCEMENT.DEFAULT_CONTRACT
        with tempfile.TemporaryDirectory(dir=REINFORCEMENT.ROOT) as directory:
            root = Path(directory)
            policy = root / "visual.json"
            policy.write_text(
                json.dumps(
                    {
                        "schema": 1,
                        "max_channel_delta": 1,
                        "max_changed_pixel_ratio": 0.0,
                        "masks": [],
                    }
                ),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "one changed pixel"):
                REINFORCEMENT._validate_exact_visual_policy(policy)

            policy.write_text(
                json.dumps(
                    {
                        "schema": 1,
                        "max_channel_delta": 0,
                        "max_changed_pixel_ratio": 0.0,
                        "masks": [{"x": 0, "y": 0, "width": 1, "height": 1}],
                    }
                ),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(REINFORCEMENT.ReinforcementError, "cannot mask pixels"):
                REINFORCEMENT._validate_exact_visual_policy(policy)
        self.assertEqual(REINFORCEMENT.DEFAULT_CONTRACT, original)

    def test_renderer_chrome_specific_scenario_details_cannot_be_weakened(self) -> None:
        for field, detail in (
            ("needed_tests", "threshold-minus-one"),
            ("needed_tests", "40 logical-pixel interaction targets"),
            ("needed_tests", "zero PTY input"),
            ("needed_tests", "WGPU and CPU"),
            ("verification_reinforcements", "42-pixel header"),
            ("verification_reinforcements", "full-shelf RGBA"),
            ("checker_reinforcements", "184-pixel default tab cap"),
        ):
            with self.subTest(field=field, detail=detail):
                document = copy.deepcopy(self.document)
                feature = next(
                    item
                    for item in document["features"]
                    if item["id"] == "renderer-fonts-responsive-ui"
                )
                feature[field] = [
                    item.replace(detail, "generic visual coverage")
                    for item in feature[field]
                ]
                with self.assertRaisesRegex(
                    REINFORCEMENT.ReinforcementError, "required scenario detail"
                ):
                    self.validate(document)


if __name__ == "__main__":
    unittest.main(verbosity=2)
