#!/usr/bin/env python3
"""Mutation tests for the planned CP2/CP3 alias specification contract."""

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
    "check_devops_alias_spec",
    ROOT / "tools/ci/check_devops_alias_spec.py",
)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load CP2/CP3 alias specification checker")
POLICY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(POLICY)
CONTRACT = json.loads(
    (ROOT / "tests/fixtures/command-productivity/cp2-cp3-alias-spec-v1.json")
    .read_text(encoding="utf-8")
)


class AliasSpecificationTests(unittest.TestCase):
    def test_repository_contract_validates(self) -> None:
        counts = POLICY.validate_repository(ROOT)
        self.assertEqual(
            counts,
            {
                "shells": 5,
                "providers": 11,
                "scopes": 6,
                "verification_domains": 10,
                "ux_invariants": 8,
                "model_files": 7,
                "persistence_files": 13,
                "hostile_cases": 11,
                "wiring": 10,
            },
        )

    def test_runtime_activation_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["status"] = "active"
        with self.assertRaisesRegex(POLICY.AliasSpecError, "remain planned"):
            POLICY.validate_contract(changed)

    def test_cp33_stage_cannot_regress_or_overclaim_later_phases(self) -> None:
        for stage in (
            "CP2.2-action-search-review-insert-copy",
            "CP3.0-pure-projection-compiler-activation-disabled",
            "CP3.1-persistent-explicit-opt-in-aliases",
            "CP3.3-context-aware-pack-automation",
        ):
            with self.subTest(stage=stage):
                changed = deepcopy(CONTRACT)
                changed["implemented_stage"] = stage
                with self.assertRaisesRegex(POLICY.AliasSpecError, "through reviewed CP3.3"):
                    POLICY.validate_contract(changed)

    def test_pure_model_source_boundary_cannot_expand(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["model_files"].append("automexia-devops/src/actions/runtime.rs")
        with self.assertRaisesRegex(POLICY.AliasSpecError, "pure model source"):
            POLICY.validate_contract(changed)

    def test_hostile_fixture_authority_cannot_move(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["hostile_fixture"] = "tests/fixtures/unreviewed.json"
        with self.assertRaisesRegex(POLICY.AliasSpecError, "hostile fixture authority"):
            POLICY.validate_contract(changed)

    def test_cp33_application_source_boundary_cannot_expand(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["activation_files"].append("shell-integration/aliases.sh")
        with self.assertRaisesRegex(POLICY.AliasSpecError, "application source"):
            POLICY.validate_contract(changed)

    def test_missing_shell_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        del changed["shells"]["fish"]
        with self.assertRaisesRegex(POLICY.AliasSpecError, "shell/platform"):
            POLICY.validate_contract(changed)

    def test_missing_provider_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["providers"].remove("openshift")
        with self.assertRaisesRegex(POLICY.AliasSpecError, "provider catalog"):
            POLICY.validate_contract(changed)

    def test_precedence_change_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["scopes"][0:2] = ["capsule", "session"]
        with self.assertRaisesRegex(POLICY.AliasSpecError, "scope precedence"):
            POLICY.validate_contract(changed)

    def test_resource_ceiling_increase_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["limits"]["enabled_aliases"] += 1
        with self.assertRaisesRegex(POLICY.AliasSpecError, "resource ceilings"):
            POLICY.validate_contract(changed)

    def test_default_alias_activation_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["defaults"]["builtin_aliases_enabled"] = True
        with self.assertRaisesRegex(POLICY.AliasSpecError, "safe defaults"):
            POLICY.validate_contract(changed)

    def test_destructive_alias_eligibility_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["risk_alias_eligibility"]["Destructive"] = "eligible"
        with self.assertRaisesRegex(POLICY.AliasSpecError, "risk eligibility"):
            POLICY.validate_contract(changed)

    def test_capability_escalation_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["capabilities"]["secret_read"] = True
        with self.assertRaisesRegex(POLICY.AliasSpecError, "capability boundary"):
            POLICY.validate_contract(changed)

    def test_unknown_contract_field_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["unreviewed"] = True
        with self.assertRaisesRegex(POLICY.AliasSpecError, "contract keys"):
            POLICY.validate_contract(changed)

    def test_typed_action_field_removal_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["required_action_fields"].remove("risk")
        with self.assertRaisesRegex(POLICY.AliasSpecError, "typed action fields"):
            POLICY.validate_contract(changed)

    def test_typed_document_field_removal_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["required_document_fields"].remove("revision")
        with self.assertRaisesRegex(POLICY.AliasSpecError, "typed document fields"):
            POLICY.validate_contract(changed)

    def test_non_atomic_persistence_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["persistence"]["publication"] = "best-effort"
        with self.assertRaisesRegex(POLICY.AliasSpecError, "persistence contract"):
            POLICY.validate_contract(changed)

    def test_missing_assurance_domain_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["verification_domains"].remove("ui-and-accessibility")
        with self.assertRaisesRegex(POLICY.AliasSpecError, "verification matrix"):
            POLICY.validate_contract(changed)

    def test_shell_evaluation_escalation_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["capabilities"]["shell_evaluation"] = True
        with self.assertRaisesRegex(POLICY.AliasSpecError, "capability boundary"):
            POLICY.validate_contract(changed)

    def test_unreviewed_model_enum_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["model_enums"]["templates"].append("ShellText")
        with self.assertRaisesRegex(POLICY.AliasSpecError, "typed model enums"):
            POLICY.validate_contract(changed)

    def test_weakened_alias_grammar_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["alias_policy"]["portable_pattern"] = ".*"
        with self.assertRaisesRegex(POLICY.AliasSpecError, "portable alias policy"):
            POLICY.validate_contract(changed)

    def test_missing_shell_projection_mode_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        del changed["projection_modes"]["cmd"]
        with self.assertRaisesRegex(POLICY.AliasSpecError, "shell projection modes"):
            POLICY.validate_contract(changed)

    def test_missing_health_state_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["health_states"].remove("Tampered artifact")
        with self.assertRaisesRegex(POLICY.AliasSpecError, "health-state model"):
            POLICY.validate_contract(changed)

    def test_performance_regression_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["performance_targets"]["warm_load_p95_ms"] += 1
        with self.assertRaisesRegex(POLICY.AliasSpecError, "performance ratchets"):
            POLICY.validate_contract(changed)

    def test_missing_accessibility_invariant_is_rejected(self) -> None:
        changed = deepcopy(CONTRACT)
        changed["ux_invariants"].remove("responsive-at-400-percent")
        with self.assertRaisesRegex(POLICY.AliasSpecError, "UX/accessibility"):
            POLICY.validate_contract(changed)

    def test_duplicate_json_key_is_rejected(self) -> None:
        with self.assertRaisesRegex(POLICY.AliasSpecError, "duplicate JSON key"):
            POLICY.load_contract('{"schema": 1, "schema": 2}')

    def test_nonstandard_json_constant_is_rejected(self) -> None:
        with self.assertRaisesRegex(POLICY.AliasSpecError, "non-standard JSON"):
            POLICY.load_contract('{"schema": NaN}')

    def test_document_performance_drift_is_rejected(self) -> None:
        text = (ROOT / "docs/DEVOPS-ALIASES.md").read_text(encoding="utf-8")
        changed = text.replace(
            "<= 16 ms p95; deterministic and allocation-bounded",
            "<= 32 ms p95; deterministic and allocation-bounded",
        )
        with self.assertRaisesRegex(POLICY.AliasSpecError, "controls missing"):
            POLICY.validate_spec_text(changed)

    def test_missing_security_section_is_rejected(self) -> None:
        text = (ROOT / "docs/DEVOPS-ALIASES.md").read_text(encoding="utf-8")
        changed = text.replace("## Security and privacy", "## Safety")
        with self.assertRaisesRegex(POLICY.AliasSpecError, "headings missing"):
            POLICY.validate_spec_text(changed)

    def test_false_shipped_claim_is_rejected(self) -> None:
        text = (ROOT / "docs/DEVOPS-ALIASES.md").read_text(encoding="utf-8")
        changed = text.replace(
            "Status: CP2.0-CP3.3",
            "Status: shipped CP2.0-CP3.3",
            1,
        )
        with self.assertRaisesRegex(POLICY.AliasSpecError, "must not overclaim"):
            POLICY.validate_spec_text(changed)

    def test_missing_cross_link_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative in POLICY.WIRING:
                destination = root / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                content = (ROOT / relative).read_text(encoding="utf-8")
                if relative == "docs/ROADMAP.md":
                    content = content.replace("DEVOPS-ALIASES.md", "missing.md")
                destination.write_text(content, encoding="utf-8")
            with self.assertRaisesRegex(POLICY.AliasSpecError, "ROADMAP.md"):
                POLICY.validate_wiring(root)

    def test_missing_ci_owner_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for relative in POLICY.WIRING:
                destination = root / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                content = (ROOT / relative).read_text(encoding="utf-8")
                if relative == ".github/workflows/ci.yml":
                    content = content.replace(
                        "test_devops_alias_spec.py",
                        "missing_alias_spec_test.py",
                    )
                destination.write_text(content, encoding="utf-8")
            with self.assertRaisesRegex(POLICY.AliasSpecError, "ci.yml"):
                POLICY.validate_wiring(root)

    def test_policy_reader_rejects_symbolic_links(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "contract.json"
            path.write_text("{}\n", encoding="utf-8")
            with patch.object(Path, "is_symlink", return_value=True):
                with self.assertRaisesRegex(POLICY.AliasSpecError, "symbolic link"):
                    POLICY.bounded_text(path, 64, "test contract")

    def test_policy_reader_rejects_oversized_files(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "contract.json"
            path.write_bytes(b"x" * 65)
            with self.assertRaisesRegex(POLICY.AliasSpecError, "exceeds"):
                POLICY.bounded_text(path, 64, "test contract")


if __name__ == "__main__":
    unittest.main()
