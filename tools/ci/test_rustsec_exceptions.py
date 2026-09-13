#!/usr/bin/env python3
"""Keep Cargo Audit warning exceptions aligned with the reviewed Deny policy."""

from __future__ import annotations

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
EXCEPTIONS = {
    "RUSTSEC-2024-0436",
    "RUSTSEC-2025-0141",
    "RUSTSEC-2026-0192",
    "RUSTSEC-2026-0249",
}


def advisory_ids(path: Path) -> set[str]:
    return set(re.findall(r"RUSTSEC-[0-9]{4}-[0-9]{4}", path.read_text(encoding="utf-8")))


class RustsecExceptionTests(unittest.TestCase):
    def test_audit_and_deny_exceptions_are_exactly_the_reviewed_set(self) -> None:
        self.assertEqual(advisory_ids(ROOT / ".cargo/audit.toml"), EXCEPTIONS)
        self.assertEqual(advisory_ids(ROOT / "deny.toml"), EXCEPTIONS)

    def test_every_exception_has_a_documented_upstream_remediation_path(self) -> None:
        source = (ROOT / ".cargo/audit.toml").read_text(encoding="utf-8")
        for advisory in EXCEPTIONS:
            with self.subTest(advisory=advisory):
                position = source.index(advisory)
                preceding = source[max(0, position - 260):position]
                self.assertIn("replacement", preceding)

    def test_advisory_database_uses_the_runner_owned_path_neutral_cargo_home(self) -> None:
        source = (ROOT / ".cargo/audit.toml").read_text(encoding="utf-8")
        self.assertNotIn("path =", source)
        runner = (ROOT / "tools/ci/github_free_assurance.py").read_text(
            encoding="utf-8"
        )
        self.assertIn('environment["CARGO_HOME"] = str(runtime / "cargo-home")', runner)
        self.assertIn("dev_cache.runtime_root(root=ROOT)", runner)
        self.assertIn('url = "https://github.com/RustSec/advisory-db.git"', source)
        self.assertIn("fetch = true", source)
        self.assertIn("stale = false", source)

    def test_cargo_vet_baseline_is_source_controlled_and_path_free(self) -> None:
        config = (ROOT / "supply-chain/config.toml").read_text(encoding="utf-8")
        guide = (ROOT / "supply-chain/README.md").read_text(encoding="utf-8")
        self.assertIn('[cargo-vet]\nversion = "0.10"', config)
        self.assertIn("[policy.sugarloaf]", config)
        self.assertIn("[[exemptions.", config)
        self.assertNotRegex(config, r"(?i)([a-z]:\\|/home/|/users/)")
        self.assertIn("ratchet", guide)
        self.assertRegex(guide, r"does\s+not\s+certify")


if __name__ == "__main__":
    unittest.main(verbosity=2)
