# Contributing to Automexia Terminal

Thank you for contributing. By submitting a contribution you agree to the
Developer Certificate of Origin 1.1. Add a sign-off to every commit with
`git commit -s`; a pull request cannot merge if any commit lacks it.

## Start here

1. Install the pinned Rust toolchain and platform dependencies.
2. Run `cargo xtask doctor`.
3. Create a focused branch from `main`.
4. Add or update a fragment under `changes/`.
5. Make the smallest coherent change and its tests.
6. Run `cargo xtask ci` and `cargo xtask package --check`.
7. Open a pull request using the repository template.

All PR policy jobs run for every pull request. Path filters may add expensive
domain checks but never remove the base policy suite.

## Change requirements

- Use a conventional PR title, for example `fix(renderer): preserve prompt anchor`.
- Update configuration, CLI, migration, architecture, and support docs when a
  public contract changes.
- Add an ADR for dependency boundaries, persistence, threading,
  security/capabilities, or public-behavior decisions.
- Include screenshots or renderer-neutral goldens for visible UI changes.
- Engine changes require a focused regression test even when inherited engine
  files are excluded from the untouched changed-line threshold.
- Do not introduce network access, arbitrary process execution, or new extension
  capabilities without a security review and explicit least-privilege manifest.

Documentation-only, tests-only, or internal-maintenance PRs may omit a changelog
fragment only when the corresponding repository label is applied.

## Required commands

```text
cargo xtask verify architecture
cargo xtask verify identity
cargo xtask verify provenance
cargo xtask test conformance
cargo xtask ci
cargo xtask package --check
```

Platform-specific changes must also run on their native OS. See
`docs/TESTING.md` for X11/Wayland, MSVC/ARM64, macOS universal, coverage,
sanitizer, fuzz, benchmark, and package matrices.

## Reviews and merging

`main` requires passing checks, resolved conversations, one approval, and the
applicable CODEOWNERS approval. Protected release, signing, security,
capability, provenance, policy, and audited-engine-base paths require two
approvals. Stale approvals are dismissed. Maintainers squash-merge and delete
the source branch; force pushes to `main` are prohibited.

Security reports must follow `SECURITY.md`, not public issues.
