Added
- Add an explicitly owner-authorized first-alpha release channel with independent native package and reproducibility gates, source-bound signed checksums, and truthful unsigned Windows/non-notarized macOS disclosures. Stable-release requirements remain unchanged.

- Keep alpha CI read-only and publish through a verified owner command; retain native evidence, signed checksums, source-bound notes, exact draft IDs and immutable release checks. Install required Unicode font fixtures in both release-quality jobs.
- Correct native lipo invocation and re-sign merged Mach-O slices before execution; bind the Windows vendor path through the packager isolation boundary, ship complete ARM64 shell resources in repairable MSI components, and resolve DEB paths before apt installation. Exercise these paths in package regressions.
