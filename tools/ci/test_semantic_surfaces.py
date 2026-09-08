"""Mutation tests for the source guard; Rust tests prove actual behavior."""
from pathlib import Path
import re
import unittest
import check_semantic_surfaces as checker


class SemanticSurfaceGuards(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        root = Path(__file__).resolve().parents[2]
        cls.sources = {owner: (root / path).read_text(encoding="utf-8") for owner, path in checker.SOURCES.items()}

    def test_current_sources(self) -> None:
        checker.validate_sources(self.sources)

    def test_boundary_mutations_are_rejected(self) -> None:
        mutations = [
            ("contract", "20_000", "20_001"),
            ("contract", "320_000", "640_000"),
            ("contract", "4 * 1024 * 1024", "8 * 1024 * 1024"),
            ("contract", "duplicate row identifier", "removed identity check"),
            ("contract", "(<identifier>)", "unredacted"),
            ("contract", '.field("rows", &self.rows().len())', '.field("rows", &self.rows())'),
            ("bounds", "items.len() < N", "items.len() <= N"),
            ("bounds", "C.saturating_sub(total.cells)", "usize::MAX"),
            ("bounds", "B.saturating_sub(total.text)", "usize::MAX"),
            ("bounds", "next_element_seed(RejectElement)", "next_element::<T>()"),
            ("host", "frame.len() > MAX_SURFACE_FRAME_BYTES", "false"),
            ("host", "grant.capability != Capability::UiOverlay", "false"),
            ("host", "grant.session_id != binding.session", "false"),
            ("host", "grant.operation_id != binding.operation", "false"),
            ("host", "grant.capsule_revision != binding.capsule_revision", "false"),
            ("host", "now_ms >= grant.expires_at_ms", "false"),
            ("host", "now_ms >= self.expires_at_ms", "false"),
            ("host", "self.once && self.consumed", "false"),
            ("host", "update.binding() != &self.binding", "false"),
            ("host", "update.generation() != self.generation", "false"),
            ("host", "update.revision() <= self.revision", "false"),
            ("host", "self.snapshot = None", "self.snapshot.take()"),
            ("benchmark", "construct_validate_drop", "removed_constructor_case"),
            ("benchmark", "BatchSize::PerIteration", "BatchSize::SmallInput"),
            ("benchmark", "SamplingMode::Flat", "SamplingMode::Auto"),
            ("benchmark", "bounded_summary", "removed_diagnostic_case"),
        ]
        for owner, old, new in mutations:
            with self.subTest(owner=owner, boundary=old):
                self.assertIn(old, self.sources[owner])
                sources = dict(self.sources)
                sources[owner] = sources[owner].replace(old, new, 1)
                with self.assertRaises(ValueError):
                    checker.validate_sources(sources)

    def test_each_benchmark_group_keeps_flat_sampling(self) -> None:
        # Mutating only the first occurrence missed a different group's setting.
        benchmark = self.sources["benchmark"]
        occurrences = list(re.finditer(r"SamplingMode::Flat", benchmark))
        self.assertEqual(len(occurrences), 3)
        for occurrence in occurrences:
            with self.subTest(group_offset=occurrence.start()):
                sources = dict(self.sources)
                sources["benchmark"] = (
                    benchmark[:occurrence.start()] + "SamplingMode::Auto"
                    + benchmark[occurrence.end():]
                )
                with self.assertRaises(ValueError):
                    checker.validate_sources(sources)

    def test_unrelated_authority_is_rejected(self) -> None:
        for token in ("std::fs", "std::net", "std::process", "automexia_devops", "rio_vt"):
            sources = dict(self.sources)
            sources["contract"] += f"\nuse {token};\n"
            with self.assertRaises(ValueError):
                checker.validate_sources(sources)


if __name__ == "__main__":
    unittest.main()
