# Toolchain policy

Production CI uses Rust **1.98.0** explicitly in workflow jobs.
Fuzzing/Miri/sanitizer commands use the date-pinned `nightly-2026-08-25`
toolchain. External GitHub Actions use immutable full-SHA references; Cargo
helper tools use explicit versions. Workflow linting uses actionlint 1.7.12
with its published Linux-amd64 SHA-256 checksum. Repository policy parsing pins
PyYAML 6.0.3, source scanning pins Semgrep Community Edition 1.175.0, and
workflow auditing installs zizmor 1.21.0 through the pinned install Action.

Do not replace pinned versions with `latest`, moving major tags, `main`, or an unversioned installer in a production workflow.
