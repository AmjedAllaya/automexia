# Verified Rust compiler selection

CI and release builds explicitly select the repository-pinned compiler and
verify both rustc and Cargo before use. Compiler cache labels match that pin,
and the minimum supported Rust version has an explicit locked workspace check.
Workflow mutation and bounded process-probe tests guard drift and ignored
failures. This changes contributor assurance, not terminal runtime behavior,
dependencies, published package versions or release activation.
Native compiler jobs explicitly bootstrap compatible Python. Full-history
secret scanning retains only reviewed exact historical false positives, with
real scanner regressions against new secrets in those same paths.
