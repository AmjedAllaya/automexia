Added
- Add an explicitly owner-authorized first-alpha release channel with independent native package and reproducibility gates, source-bound signed checksums, and truthful unsigned Windows/non-notarized macOS disclosures. Stable-release requirements remain unchanged.

- Keep alpha CI read-only and publish through a verified owner command; retain native evidence, signed checksums, source-bound notes, exact draft IDs and immutable release checks. Install required Unicode font fixtures in both release-quality jobs.
- Correct native lipo invocation and re-sign merged Mach-O slices before execution; bind the Windows vendor path through the packager isolation boundary, ship complete ARM64 shell resources in repairable MSI components, and resolve DEB paths before apt installation. Exercise these paths in package regressions.
- Run the shared Linux reproducibility wrapper through Bash, matching its committed non-executable mode; exercise both architecture arguments and both release workflows with real POSIX permissions.
- Canonicalize generated Windows version-resource records before compilation so independent native builds produce identical embedded branding and version data; exercise the real build script in native release regressions.
- Normalize both logical and canonical source paths in release builds, including Unix symlinks and Windows short paths, with native Unix compiler fixtures to reject path-dependent binaries.
- Relativize Apple linker debug-map paths before UUID generation; preserve crash-symbol identifiers and strict byte comparison while testing direct, symlinked, differently sized and Unicode source paths on native macOS.
- Require the exact focused window in native macOS probes, encode standard function keys explicitly, and retain content-free focus diagnostics when a scenario fails. Preserve production blur cancellation and every repeated native scenario.
