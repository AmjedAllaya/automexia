
# Supply-Chain and Dependency Policy Research

Integrate into existing security/release owners.

Recommended:
- committed lockfiles;
- source/artifact checksums;
- review of new dependencies/build scripts;
- SBOM;
- release signing;
- reproducible/offline build path where practical;
- separation of acquisition and network-restricted build for high-assurance releases;
- exact provenance for managed external tools/models.

Dependency updates still require functional/native regression testing.

No proposal dependency gets privileged adoption merely because it is newer.
