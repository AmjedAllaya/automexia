# Security boundaries — GitHub Free private edition

* Stable releases are triggered only by a merged same-repository `release/X.Y.Z` pull request into `main`.
* Release PR metadata, source SHA, current `main`, Cargo version, existing tags/releases, and optional approval count are verified before builds proceed.
* Release PRs may not modify `.github/workflows/release.yml` or `.github/scripts/release_*`; CI/security changes must be merged separately.
* Default workflow permissions are read-only. Only the final publication job has `contents: write`; it does not check out repository source.
* External Actions are pinned to immutable commit SHAs and audited by the workflow-security job.
* Paid-only private artifact attestations and dependency-review APIs are intentionally not required. SBOMs, SHA-256 manifests, cargo-audit, cargo-deny, native final-package tests, and release-manifest verification are the compensating controls.
* GitHub Free cannot prevent an administrator or trusted writer from changing `main`/workflow policy. Account permissions therefore remain part of the trusted computing base.
