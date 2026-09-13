# Security boundaries

* Default workflow token permissions are read-only.
* The Stable release workflow contains exactly one `contents: write` grant: the final publication job. That job does not check out repository source or run Cargo/project scripts.
* External Actions must be pinned to exact 40-character commit SHAs.
* Fork-originated `release/*` PRs cannot authorize a release. Release provenance is bound to the merged-event `GITHUB_SHA` and then required to equal the current `main` commit before expensive work begins.
* A release PR may not modify `.github/`, `tools/ci/`, `tools/xtask/`, `packaging/`, or `shell-integration/`; trust/packaging implementation changes must land separately.
* Stable release reruns quality and dependency-security checks after merge; it does not trust that an administrator respected PR checks.
* Linux production binaries are built on Ubuntu 22.04-class native runners and checked against GLIBC 2.35.
* Windows/macOS signing fails closed. Secret-bearing Windows and Apple signing jobs never check out repository source, and they do not execute the signed application while credentials are present. Final signed/notarized distributions are executed later on credential-free native runners.
* No paid-only private GitHub artifact-attestation, Code Security dependency-review, protected-environment-review, private CodeQL-upload, or ruleset feature is required.
* GitHub Free cannot stop a repository administrator from rewriting workflow policy. Treat every write-capable maintainer as part of the trusted computing base.
