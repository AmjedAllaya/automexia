# Integrate dependency updates without breaking native contracts

- Update compatible production and fuzz dependencies with locked workspace
  verification; keep the toolchain, capability and publication boundaries.
- Retain PHF 0.13.1 for the generated Unicode table. The proposed 0.14 runtime
  compiled but failed the actual variation-lookup regression. An exact pin and
  all 708 entry checks, with invalid-key cases, prevent that silent mismatch.
- Preserve calloop 0.13 and sctk-adwaita 0.10.1 with the existing Smithay 0.19
  adapter. Adwaita 0.12 consumes Smithay 0.21 types and is not an independent
  version-only upgrade. No unsupported native windowing migration is implied.
- A passing host check is not native Linux/macOS, desktop, or release evidence.
- Preserve the reviewed WIT/WAT parser generation with Wasmtime. The existing
  ecosystem gate rejected the grouped parser jump; it remains enforced rather
  than relaxing an accepted sandbox/tooling contract during branch cleanup.
- Keep one reviewed compression and font-parser generation: Flate2 1.1.9 shares
  miniz_oxide 0.8 with PNG, and Skrifa 0.44 shares its parser with Swash. Full QA
  rejected the duplicate generations introduced by the proposed upgrades; the
  dependency policy remains unchanged and both lockfiles retain one owner.
