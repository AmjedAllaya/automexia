# Developer testing and release guide

Use [Testing](../TESTING.md) and
[Manual feature testing](../MANUAL-FEATURE-TESTING.md) as the authorities.

## Local sequence

Start with the focused owner, then run formatting, package/workspace tests,
warning-denied lint, repository validation, documentation checks, and
`git diff --check`. Use `cargo ready` for the contributor gate when
applicable.

## Evidence

Add a real-path regression first, then boundary, malformed, stale, cancelled,
concurrent, cleanup, rollback, disable, and recovery cases. Use independent
oracles and assert forbidden side effects.

Visible changes require renderer-neutral state, exact controlled rasters, and
native platform/accessibility evidence. Platform claims name only environments
that actually ran.

## Release

Bind evidence to the exact commit and package. Verify identity, checksums,
licenses, SBOM, provenance, signatures, install, upgrade, rollback, uninstall,
and cleanup. Missing accounts, hardware, native assistive technology, signing,
notarization, or long-duration evidence remains external.

Public release documentation covers only baseline terminal behavior. Unreleased
advanced and commercial release plans remain private.
