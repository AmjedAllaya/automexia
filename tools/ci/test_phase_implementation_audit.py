#!/usr/bin/env python3
"""Mutation tests for the cross-roadmap phase implementation audit."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import re
import unittest


MODULE_PATH = Path(__file__).with_name("check_phase_implementation_audit.py")
SPEC = importlib.util.spec_from_file_location("automexia_phase_audit", MODULE_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError("could not load phase implementation audit checker")
AUDIT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AUDIT)


class PhaseImplementationAuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.text = (AUDIT.ROOT / AUDIT.AUDIT_PATH).read_text(encoding="utf-8")
        cls.phases = AUDIT.canonical_phase_ids()

    def test_repository_audit_passes(self) -> None:
        counts = AUDIT.validate()
        self.assertGreaterEqual(counts["phase_sections"], 40)
        self.assertGreaterEqual(counts["canonical_phases"], 20)
        self.assertEqual(
            counts["evidence_dimensions"], len(AUDIT.REQUIRED_EVIDENCE_TERMS)
        )

    def test_missing_phase_heading_is_rejected(self) -> None:
        mutated = self.text.replace("### D4 ", "### removed-D4 ", 1)
        with self.assertRaisesRegex(AUDIT.PhaseAuditError, "D4"):
            AUDIT.validate_text(mutated, self.phases)

    def test_phase_without_explicit_status_is_rejected(self) -> None:
        heading = "### D1 — private contracts and bounded stable types"
        mutated = self.text.replace(
            heading + "\n\n**Fully implemented at source boundary.**",
            heading + "\n\nImplementation evidence follows.",
            1,
        )
        with self.assertRaisesRegex(AUDIT.PhaseAuditError, "D1"):
            AUDIT.validate_text(mutated, self.phases)

    def test_new_canonical_phase_is_rejected_until_audited(self) -> None:
        mutated = self.text + "\nD8 is mentioned here but has no assessment section.\n"
        with self.assertRaisesRegex(AUDIT.PhaseAuditError, "D8"):
            AUDIT.validate_text(mutated, self.phases | {"D8"})

    def test_declared_phase_ranges_cover_each_member(self) -> None:
        ids = AUDIT.audit_heading_phase_ids(self.text.splitlines())
        self.assertTrue({"D6.0", "D6.1", "D6.5"}.issubset(ids))
        self.assertTrue({"CP5.0", "CP5.3", "CP5.6"}.issubset(ids))
        self.assertTrue({"Phase 0", "Phase 3", "Phase 5"}.issubset(ids))

    def test_missing_source_link_is_rejected(self) -> None:
        mutated = self.text.replace("(CONNECTION-HUB.md)", "(missing-hub.md)", 1)
        with self.assertRaisesRegex(AUDIT.PhaseAuditError, "CONNECTION-HUB"):
            AUDIT.validate_text(mutated, self.phases)

    def test_missing_evidence_dimension_is_rejected(self) -> None:
        mutated = re.sub("code quality", "maintainability", self.text, flags=re.I)
        with self.assertRaisesRegex(AUDIT.PhaseAuditError, "code quality"):
            AUDIT.validate_text(mutated, self.phases)

    def test_bad_source_baseline_is_rejected(self) -> None:
        mutated = re.sub(
            r"Audited source baseline: [0-9a-f]{40}",
            "Audited source baseline: unknown",
            self.text,
            count=1,
        )
        with self.assertRaisesRegex(AUDIT.PhaseAuditError, "source baseline"):
            AUDIT.validate_text(mutated, self.phases)


if __name__ == "__main__":
    unittest.main(verbosity=2)
