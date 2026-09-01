# Contributing to Automexia Terminal

Thank you for contributing. By submitting a contribution you agree to the
Developer Certificate of Origin 1.1. Add a sign-off to every commit with
`git commit -s`; a pull request cannot merge if any commit lacks it.

AI coding agents must also follow the repository-level
[AI contributor workflow](AGENTS.md). It defines the required audit, research,
planning, test-driven implementation, evidence, documentation, and safe
commit/push sequence without replacing this human contributor policy.

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
- Follow the [documentation contribution guide](docs/DOCUMENTATION.md). Every
  new or materially changed feature must retain guide, reference, and
  explanation/ADR ownership in the machine-checked assurance ledger.
- Keep public purpose, audience, value, and messaging aligned with the
  [product vision](docs/PRODUCT-VISION.md) and [brand guide](docs/BRANDING.md).
  Explain value before mechanism, and never present research as shipped work.
- Add an ADR for dependency boundaries, persistence, threading,
  security/capabilities, or public-behavior decisions.
- Include screenshots or renderer-neutral goldens for visible UI changes.
- For brand assets, never overwrite the canonical source or approve
  redistribution rights without reviewable evidence.
- Engine changes require a focused regression test even when inherited engine
  files are excluded from the untouched changed-line threshold.
- Do not introduce network access, arbitrary process execution, or new extension
  capabilities without a security review and explicit least-privilege manifest.
- For keyboard changes, update the shipped
  [compatibility matrix](docs/GHOSTTY-KEYBOARD-COMPATIBILITY.md), collision tests,
  action dispatch, palette discovery, and the
  [full compatibility roadmap](docs/GHOSTTY-COMPATIBILITY-ROADMAP.md) in the
  same pull request. Do not claim exact profile parity from a hand-maintained
  default table.

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

The two launching commands automatically provision the repository-owned shell
support before spawning Automexia. This includes PowerShell, CMD, and WSL on
Windows, or Bash, Zsh, and user-local terminfo on Unix. The operation is
source-aware and idempotent, so contributors never need to run an integration
installer or restart a just-launched window. A provisioning error fails the
launch instead of silently dropping prompt, context, or listing features.
Non-launching `cargo ready`, `cargo check`, and CI intentionally do not change
user profiles.

`cargo ready` is the required contributor command. It includes tool and
structured-file validation, all Automexia verification scopes, package metadata,
rustfmt, locked all-feature workspace checks, warning-denied all-feature Clippy,
all-feature workspace tests, dependency policy, a debug build, and executable
identity smoke.
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

Windows and WSL builds use separate native checkouts. Windows Cargo/MSVC and
native ConPTY/GPU/package work stays on NTFS; Linux Cargo and Unix PTY work runs
from a WSL path such as `~/src/automexia-terminal`. The project commands
reject source or target storage under `/mnt/<drive>` before expensive work.
Run `cargo xtask doctor` to verify the mode and follow
[Windows and WSL development](docs/WSL-DEVELOPMENT.md) to synchronize the two
checkouts through Git without sharing build artifacts.

Individual `cargo xtask` commands remain available for focused diagnosis, but
contributors do not need to assemble the normal gate manually. Platform-specific
changes must also run on their native OS. See `docs/TESTING.md` for X11/Wayland,
MSVC/ARM64, macOS universal, coverage, sanitizer, fuzz, benchmark, and package
matrices.

New or materially changed product features must update
`tests/assurance/feature-matrix.json` with correctness, security, performance,
resource-lifetime, storage, resilience, accessibility, visual, and native-host
evidence plus canonical guide, reference, and explanation links. Repository
validation rejects unowned workspace members, benchmark targets, fuzz targets,
workflow jobs, documentation categories, and evidence paths. Image decoder,
preview, renderer, or graphics-protocol changes must additionally
run `cargo xtask test image-rendering`; Windows rendering/lifecycle changes run
`cargo xtask test image-rendering --native-gui` before review.
Every changed feature must also update its entry in
`tests/assurance/feature-test-reinforcement-v1.json` and the matching section
in `docs/FEATURE-TEST-REINFORCEMENT.md`. The entry owns scenario boundaries,
real-path tests, independent oracles, cross-feature interactions, checker
mutations, native/platform scope, and exit criteria. Run:

```text
python tools/ci/check_feature_test_reinforcement.py
python tools/ci/test_feature_test_reinforcement.py
```


## Reviews and merging

The versioned project policy requires passing checks, resolved conversations,
one approval, and the applicable CODEOWNERS approval. Protected release,
signing, security, capability, provenance, policy, and audited-engine-base paths
require two approvals. On a private repository using GitHub Free these review
and ruleset requirements are a maintainer convention because GitHub does not
server-enforce them; the authenticated release audit therefore fails closed
until the required external governance is available. Maintainers squash-merge and delete
the source branch; force pushes to `main` are prohibited.

Security reports must follow `SECURITY.md`, not public issues.
