# Contributing to Automexia Terminal

Thank you for contributing. By submitting a contribution you agree to the
Developer Certificate of Origin 1.1. Add a sign-off to every commit with
`git commit -s`; a pull request cannot merge if any commit lacks it.

## Start here

1. Install the pinned Rust toolchain and platform dependencies.
2. Run `cargo dev` once to validate, build, smoke, and launch the project.
3. Create a focused branch from `main`.
4. Add or update a fragment under `changes/`.
5. Make the smallest coherent change and its tests.
6. Run `cargo ready` before pushing.
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
- Follow `docs/BRANDING.md` for logo changes; never overwrite the canonical
  source or approve redistribution rights without reviewable evidence.
- Engine changes require a focused regression test even when inherited engine
  files are excluded from the untouched changed-line threshold.
- Do not introduce network access, arbitrary process execution, or new extension
  capabilities without a security review and explicit least-privilege manifest.

Documentation-only, tests-only, or internal-maintenance PRs may omit a changelog
fragment only when the corresponding repository label is applied.

## One-command workflows

```text
cargo dev       # complete local gate, then launch Automexia
cargo automexia # fast incremental build and launch
cargo ready     # complete local gate without launching
cargo storage   # report target size, free space, and largest target children
cargo purge     # remove Cargo artifacts after closing Automexia windows
```

`cargo ready` is the required contributor command. It includes tool and
structured-file validation, all Automexia verification scopes, package metadata,
rustfmt, locked workspace checks, warning-denied Clippy, workspace tests,
dependency policy, a debug build, and executable identity smoke.
Compilation-heavy checks run with incremental compilation disabled inside an
isolated target that is deleted on both success and ordinary failure. This
keeps a complete contributor gate from permanently multiplying workspace
artifacts. The final application build remains incremental for fast daily use.
The gate requires 12 GiB free on the selected target filesystem; the app-only
workflow requires 4 GiB.

Set `CARGO_TARGET_DIR` to place both persistent and isolated artifacts on a
different filesystem. Diagnostic reproductions may set
`AUTOMEXIA_KEEP_VERIFY_TARGET=1` to retain the isolated target deliberately;
remove it afterward with `cargo purge`. Threshold overrides
`AUTOMEXIA_VERIFY_MIN_FREE_GIB`, `AUTOMEXIA_BUILD_MIN_FREE_GIB`, and
`AUTOMEXIA_TARGET_WARN_GIB` accept integer GiB values, but lowering the safety
minimums is not recommended.

Individual `cargo xtask` commands remain available for focused diagnosis, but
contributors do not need to assemble the normal gate manually. Platform-specific
changes must also run on their native OS. See `docs/TESTING.md` for X11/Wayland,
MSVC/ARM64, macOS universal, coverage, sanitizer, fuzz, benchmark, and package
matrices.

## Reviews and merging

`main` requires passing checks, resolved conversations, one approval, and the
applicable CODEOWNERS approval. Protected release, signing, security,
capability, provenance, policy, and audited-engine-base paths require two
approvals. Stale approvals are dismissed. Maintainers squash-merge and delete
the source branch; force pushes to `main` are prohibited.

Security reports must follow `SECURITY.md`, not public issues.
