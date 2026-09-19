"""Mutations prevent disconnected guards from posing as decoder assurance."""
from pathlib import Path
import unittest

import check_context_contributions as checker


class ContextAdmissionGuards(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        root = Path(__file__).resolve().parents[2]
        cls.sources = {owner: (root / path).read_text(encoding="utf-8") for owner, path in checker.SOURCES.items()}

    def test_current_sources(self) -> None:
        checker.validate_sources(self.sources)

    def test_removed_or_weakened_admission_and_evidence_are_rejected(self) -> None:
        for owner, old, new in [
            ("contract", '#[serde(deserialize_with = "context_decode::segments")]', ""),
            ("contract", "MAX_STATUS_SEGMENTS: usize = 64", "MAX_STATUS_SEGMENTS: usize = 65"),
            ("decoder", "items.len() < MAX_STATUS_SEGMENTS", "items.len() <= MAX_STATUS_SEGMENTS"),
            ("decoder", "next_element_seed(RejectExcess)", "next_element::<StatusSegment>()"),
            ("tests", "reader.consumed <= prefix.len() + 1", "true"),
            ("benchmark", "assert_eq!(decoded.segments.len(), count)", "assert!(true)"),
            ("manifest", 'name = "context_contributions"', 'name = "missing_context_benchmark"'),
        ]:
            with self.subTest(owner=owner, boundary=old):
                self.assertIn(old, self.sources[owner])
                sources = dict(self.sources)
                sources[owner] = sources[owner].replace(old, new, 1)
                with self.assertRaises(ValueError):
                    checker.validate_sources(sources)

    def test_hostile_hints_and_unrelated_authority_are_rejected(self) -> None:
        for token in ("sequence.size_hint()", "std::fs", "std::net", "std::process"):
            sources = dict(self.sources)
            sources["decoder"] += f"\n{token};\n"
            with self.assertRaises(ValueError):
                checker.validate_sources(sources)


if __name__ == "__main__":
    unittest.main()
