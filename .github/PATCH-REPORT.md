# GitHub Free private hardening patch

This edition removes dependencies on private GitHub Enterprise/Code Security features and changes stable release authorization from a manually pushed `v*` tag to a successfully merged internal `release/X.Y.Z` pull request.

Key controls: exact merged source validation; current-main validation; stable SemVer/Cargo version match; optional distinct approval threshold; immutable external action pins; read-only default permissions; native Windows/Linux/macOS architecture builds; final package verification; SBOMs and SHA-256 manifest; no private artifact attestations; no paid dependency-review action; a single minimal `contents: write` publication job; tag created only after all release gates pass.
