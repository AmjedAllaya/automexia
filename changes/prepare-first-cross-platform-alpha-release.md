Changed

- Prepare `0.0.1_alpha` for Automexia's own GitHub Releases, with independent
Windows, Linux and macOS verification. Disable the former separate Linux
publisher and preserve signing, source review and immutable publication gates.

- Fix alpha/native version mappings, desktop packager arguments, final artifact
selection, local tag verification, deterministic portable archives, and public
SBOM validation. Compare all four unsigned runtimes on all six native targets
against both independent builds and the inputs selected for packaging.

- This source change is release preparation, not proof of publication. Windows
Authenticode, Apple Developer ID/notarization, hosted protection, controlled
hardware evidence and final approval still require their own passing evidence.
