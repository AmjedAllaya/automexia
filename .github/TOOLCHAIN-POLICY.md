# Toolchain policy

Production CI uses Rust **1.96.1**, matching `rust-toolchain.toml`, through
explicit `RUSTUP_TOOLCHAIN` selection. Jobs verify actual rustc and Cargo
versions and commit identities before use. The workspace minimum Rust version
is a separate verified, locked all-target/all-feature build contract.
Compiler jobs explicitly bootstrap Python 3.12 with the reviewed immutable
setup action; Ubuntu 22.04 system Python is not a supported parser bootstrap.
Fuzzing/Miri/sanitizer commands use the date-pinned `nightly-2026-08-25`
toolchain. External GitHub Actions use immutable full-SHA references; Cargo
helper tools use explicit versions. Workflow linting uses actionlint 1.7.12 and
ShellCheck 0.11.0 with their published platform archive SHA-256 checksums; the
ShellCheck executable is passed explicitly so local and hosted linting cannot
silently diverge when a host tool is absent. Repository policy parsing pins
PyYAML 6.0.3, source scanning pins Semgrep Community Edition 1.175.0, and
workflow auditing installs zizmor 1.21.0 through the pinned install Action.

Do not replace pinned versions with `latest`, moving major tags, `main`, or an unversioned installer in a production workflow.
