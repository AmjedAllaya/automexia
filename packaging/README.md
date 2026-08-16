# Packaging

`cargo-packager` owns Windows x86_64 MSI and macOS app/DMG generation plus
Debian bundle metadata. Its WiX 3 backend cannot emit ARM64 MSI databases, so
Windows ARM64 uses the repository-owned `automexia-arm64.wxs` source and the
WiX 5.0.2 .NET tool pinned in `.config/dotnet-tools.json`. nFPM provides
consistent DEB/RPM output. Repository automation owns portable archives,
SHA-256 checksums, SBOM generation, signing orchestration, attestations, and
validation.

The public release directory is governed by
`tests/assurance/release-trust-policy-v1.json` and
`tools/ci/release_trust.py`. It accepts exactly the two-architecture Windows
MSI/ZIP, universal macOS DMG, and two-architecture Linux DEB/RPM/tar matrix,
with bounded sizes and no symlinks or raw executables. Final-package SBOMs,
streamed checksums, provenance/SBOM attestations, manifest, digest benchmark,
and controlled Windows trust evidence are added only after that package
allowlist passes. See `docs/RELEASE-TRUST.md` for signing backends and operator
verification.

`cargo xtask package --check` validates metadata without mutating the tree.
`cargo xtask package --target <triple>` builds the requested release binary.
ARM64 Windows MSI packaging requires the .NET SDK; xtask restores the pinned
local WiX tool before compiling the installer and never depends on a moving
global WiX installation.

Linux packaging requires `nFPM`, `scdoc`, `gzip`, and `tic`. The repository
manifest assigns deterministic 0644/0755 modes instead of inheriting host
checkout permissions, declares the dynamically linked Debian/RPM runtime
libraries, and ships compressed man/changelog files plus Debian copyright and
third-party notices. Release CI validates the desktop and AppStream metadata,
runs Lintian, and performs clean DEB/RPM install and uninstall smoke tests.
Nightly workflows use the supplied Automexia raster mark and its audited
platform derivatives. `cargo xtask release` blocks stable publication until the
complete vector brand kit, redistribution approval, and signing prerequisites
are satisfied.
