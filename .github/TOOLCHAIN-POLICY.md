# Toolchain policy

Release workflows pin Rust stable to 1.98.0 and exploratory nightly
checks to nightly-2026-08-25. Updates require an ordinary reviewed pull
request, successful CI/security scans, and release-candidate validation.
External GitHub Actions use immutable commit SHAs; human-readable
version comments should be updated with each pin.
