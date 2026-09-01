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
    @staticmethod
    def detailed_legacy_fixture() -> tuple[str, str, set[str]]:
        lines = [
            "# Phase implementation audit",
            "",
            "Audited source baseline: " + "a" * 40,
            "",
            " ".join(AUDIT.REQUIRED_EVIDENCE_TERMS),
            "",
        ]
        for source in AUDIT.CANONICAL_PHASE_SOURCES:
            lines.append(f"[{Path(source).name}]({Path(source).name})")
        lines.extend(
            [
                "",
                "## Status rules",
                "",
                "## Audit method and current evidence",
                "",
                "## Executive phase matrix",
                "",
                "| Track | Phase | Implementation | Evidence | Remaining |",
                "|---|---|---|---|---|",
                "| Command | CP1 | **Fully implemented at source boundary.** | fixture | fixture |",
                "",
            ]
        )
        emitted = {
            "# Phase implementation audit",
            "## Status rules",
            "## Audit method and current evidence",
            "## Executive phase matrix",
        }
        for heading in AUDIT.REQUIRED_GLOBAL_HEADINGS:
            if heading in emitted:
                continue
            if heading == "### v0.5 assurance maturation":
                lines.extend([heading, "", "**Partially implemented.**", ""])
            else:
                lines.extend([heading, ""])
            emitted.add(heading)
        for prefix in AUDIT.REQUIRED_PHASE_PREFIXES:
            if prefix == "### v0.5 assurance maturation":
                heading = prefix
                status = "**Partially implemented.**"
            elif prefix == "### D1 ":
                heading = "### D1 — private contracts and bounded stable types"
                status = "**Fully implemented at source boundary.**"
            else:
                heading = prefix + "fixture"
                status = "**Partially implemented.**"
            lines.extend([heading, "", status, ""])
        text = "\n".join(lines) + "\n"
        phases = AUDIT.audit_heading_phase_ids(text.splitlines())
        roadmap = (
            "# Roadmap\n\n"
            f"{AUDIT.ROADMAP_STATUS_START}\n"
            "| Status | Phase | Note |\n"
            "|---|---|---|\n"
            "| **Fully done** | CP1 | fixture |\n"
            f"{AUDIT.ROADMAP_STATUS_END}\n"
        )
        return text, roadmap, phases

    @classmethod
    def setUpClass(cls) -> None:
        cls.text, cls.roadmap, cls.phases = cls.detailed_legacy_fixture()

    def test_repository_audit_passes(self) -> None:
        counts = AUDIT.validate()
        self.assertEqual(counts["phase_sections"], 0)
        self.assertEqual(counts["canonical_phases"], 0)
        self.assertEqual(counts["evidence_dimensions"], 0)
        self.assertEqual(counts["roadmap_statuses"], len(AUDIT.PUBLIC_AUDIT_AREAS))

    def test_public_summary_missing_area_is_rejected(self) -> None:
        audit = (AUDIT.ROOT / AUDIT.AUDIT_PATH).read_text(encoding="utf-8")
        roadmap = (AUDIT.ROOT / AUDIT.ROADMAP_PATH).read_text(encoding="utf-8")
        mutated = audit.replace("Automation Studio", "Removed editor", 1)
        with self.assertRaisesRegex(AUDIT.PhaseAuditError, "missing areas"):
            AUDIT.validate_public_summary(mutated, roadmap)

    def test_public_roadmap_private_phase_code_is_rejected(self) -> None:
        audit = (AUDIT.ROOT / AUDIT.AUDIT_PATH).read_text(encoding="utf-8")
        roadmap = (AUDIT.ROOT / AUDIT.ROADMAP_PATH).read_text(encoding="utf-8")
        with self.assertRaisesRegex(AUDIT.PhaseAuditError, "phase codes"):
            AUDIT.validate_public_summary(audit, roadmap + "\nPO7\n")

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

    def test_roadmap_status_mismatch_is_rejected(self) -> None:
        mutated = self.roadmap.replace(
            "| **Fully done** | CP1 |",
            "| **Partially done** | CP1 |",
            1,
        )
        with self.assertRaisesRegex(AUDIT.PhaseAuditError, "CP1"):
            AUDIT.validate_roadmap_status_register(self.text, mutated)

    def test_nonstandard_roadmap_status_is_rejected(self) -> None:
        mutated = self.roadmap.replace(
            "| **Fully done** | CP1 |",
            "| **Complete** | CP1 |",
            1,
        )
        with self.assertRaisesRegex(AUDIT.PhaseAuditError, "invalid status label"):
            AUDIT.validate_roadmap_status_register(self.text, mutated)

    def test_missing_roadmap_feature_status_is_rejected(self) -> None:
        mutated = re.sub(
            r"^\| \*\*Fully done\*\* \| CP1 \|.*\n",
            "",
            self.roadmap,
            count=1,
            flags=re.MULTILINE,
        )
        with self.assertRaisesRegex(
            AUDIT.PhaseAuditError, "do not match|contains no feature rows"
        ):
            AUDIT.validate_roadmap_status_register(self.text, mutated)

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
