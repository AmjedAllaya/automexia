# Changelog
## CP3.2 reviewed DevOps Quick Action packs

- Added 11 immutable Git, Docker/Compose, Kubernetes, OpenShift, Helm,
  Terraform, OpenTofu, AWS, Azure, Google Cloud, and OpenSSH manifests with 33
  typed, disabled-by-default actions and no default aliases.
- Added pure provider absence/version/completion health, effect/risk floors,
  manifest-aware alias denial, overlay-safe updates, stale-digest/version-
  regression rejection, and explicit deprecation replacements.
- Added dry-run-first `automexia packs list/show/doctor/enable`; applied
  enablement requires revision compare-and-swap, refuses overwrite, and leaves
  aliases disabled.
- Added focused tests, CLI parser coverage, Criterion targets, nightly fuzzing,
  a machine contract with mutation tests, CI/xtask wiring, and synchronized
  architecture, roadmap, testing, security, CLI, and feature documentation.

All notable changes to Automexia Terminal are documented here. Pull requests add
fragments under `changes/`; the release task assembles reviewed fragments for a
tagged release.

## [Unreleased]

## [0.3.13] - 2026-08-11

- Audited downstream baseline applied to Rio commit
  `7d595af583f6ef1ea6036a66b367ba1e5a84d4a2`.
