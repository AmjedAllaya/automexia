---
category: Fixed
summary: Hardened Ghostty nightly-fuzz assurance against workflow-format drift.
---

The Ghostty assurance checker now parses the nightly fuzz matrix instead of
matching one serialization of it. It verifies all three relevant fuzz targets
and reports their truthful count, while mutation coverage rejects removal of
any target.
