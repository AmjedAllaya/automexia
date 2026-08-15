# Packaging

`cargo-packager` owns Windows x86_64 MSI and macOS app/DMG generation plus
Debian bundle metadata. Its WiX 3 backend cannot emit ARM64 MSI databases, so
Windows ARM64 uses the repository-owned `automexia-arm64.wxs` source and the
WiX 5.0.2 .NET tool pinned in `.config/dotnet-tools.json`. nFPM provides
consistent DEB/RPM output. Repository automation owns portable archives,
SHA-256 checksums, SBOM generation, signing orchestration, attestations, and
validation.

`cargo xtask package --check` validates metadata without mutating the tree.
`cargo xtask package --target <triple>` builds the requested release binary.
ARM64 Windows MSI packaging requires the .NET SDK; xtask restores the pinned
local WiX tool before compiling the installer and never depends on a moving
global WiX installation.
Nightly workflows use the supplied Automexia raster mark and its audited
platform derivatives. `cargo xtask release` blocks stable publication until the
complete vector brand kit, redistribution approval, and signing prerequisites
are satisfied.
