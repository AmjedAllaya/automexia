# Changelog
## CP3.2 reviewed DevOps Quick Action packs

- Added 11 immutable Git, Docker/Compose, Kubernetes, OpenShift, Helm,
  Terraform, OpenTofu, AWS, Azure, Google Cloud, and OpenSSH manifests with 33
  typed, disabled-by-default actions and no default aliases.
- Added pure provider absence/version/completion health, effect/risk floors,
  manifest-aware alias denial, overlay-safe updates, stale-digest/version-
  regression rejection, and explicit deprecation replacements.
- Added dry-run-first `automexia packs list/show/doctor/enable`; preview exposes
  exact argv, effect, risk, documentation, alias eligibility, registry digest,
  and revision. Applied enablement rejects an already-stale revision before
  store creation, rechecks compare-and-swap, refuses overwrite, and leaves
  aliases disabled.
- Froze the complete reviewed registry payload with an initialization-time
  digest assertion, tightened HTTPS host and completion-policy validation, and
  fixed version-only upgrades to remain unchanged while functional metadata
  changes are updated.
- Added 14 pack unit/integration cases, five CLI parser/rendering/preflight
  cases, Criterion targets, nightly fuzzing, an exact-payload machine contract,
  eight mutation tests, CI/xtask wiring, and synchronized architecture, roadmap,
  testing, security, CLI, and feature documentation.

All notable changes to Automexia Terminal are documented here. Pull requests add
fragments under `changes/`; the release task assembles reviewed fragments for a
tagged release.

## [Unreleased]

## [0.3.13] - 2026-08-11

- Audited downstream baseline applied to Rio commit
  `7d595af583f6ef1ea6036a66b367ba1e5a84d4a2`.
