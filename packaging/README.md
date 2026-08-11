# Packaging

`cargo-packager` owns MSI and macOS app/DMG generation plus Debian bundle
metadata. nFPM provides consistent DEB/RPM output. Repository automation owns
portable archives, SHA-256 checksums, SBOM generation, signing orchestration,
attestations, and validation.

`cargo xtask package --check` validates metadata without mutating the tree.
`cargo xtask package --target <triple>` builds the requested release binary.
Nightly workflows may use the explicit placeholder mark; `cargo xtask release`
blocks stable publication until the final asset manifest and signing prerequisites
are satisfied.
