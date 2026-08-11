# ADR 0012 — Generated Rust is normalized before release verification

Status: accepted in v0.3.13

## Context

Automexia applies semantic source transforms to a pinned Rio-derived checkout. A semantic transform can be correct while producing whitespace/import ordering that differs from the exact `rustfmt` selected by `rust-toolchain.toml`. A real Windows v0.3.12 release run demonstrated this: all semantic/source gates passed, then `cargo fmt --check` stopped the build on formatting-only diffs.

## Decision

Formatting is part of **source generation**, not a release-build mutation. `BOOTSTRAP-WINDOWS.ps1` and direct `APPLY-AUTOMEXIA.ps1` invoke `FORMAT-WINDOWS.ps1` when Cargo is available. That script runs the checkout-pinned `cargo fmt --all` and immediately verifies the result.

`BUILD-WINDOWS.ps1`, `CHECK-WINDOWS.ps1`, and `DEV-WINDOWS.ps1` call the same formatter with `-CheckOnly`. They never rewrite source. This keeps release verification deterministic while making patch-generated source conform to the same formatting contract as hand-written Rust.

## Consequences

- a normal bootstrap on a Rust-equipped development host yields rustfmt-clean source before Cargo checks;
- a `-SkipBuild` bootstrap without Cargo can still prepare source, but prints an explicit warning and must be normalized after Rust is installed;
- release/check/dev scripts fail if source was generated or edited without normalization;
- formatting behavior follows the pinned toolchain rather than a hand-maintained Python approximation of rustfmt.
