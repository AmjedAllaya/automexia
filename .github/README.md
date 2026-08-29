# Automexia GitHub automation — Free/private production edition

This directory is a complete replacement for the repository `.github/` tree. It is intentionally designed for a **private repository on GitHub Free**.

* Ordinary PR CI uses Linux only to conserve the free Actions allowance.
* A stable release starts only after an **internal `release/X.Y.Z` pull request is merged into `main`**.
* Ordinary non-release PR closures may instantiate a lightweight release-gate run because GitHub cannot head-branch-filter the `closed` event; every release job is then skipped cleanly and nothing is published.
* The release workflow reruns mandatory quality/security gates after merge, then uses native Windows/Linux/macOS runners for production artifacts.
* No private CodeQL upload, dependency-review action, protected environment reviewer, private artifact attestation, or private ruleset is required.
* There is intentionally **no `codeql.yml`** and no Release Drafter workflow in this edition.

Read `FREE-PRIVATE-PRODUCTION-SETUP.md` before the first release. If jobs fail before checkout/build begins, see `ACTIONS-STARTUP-TROUBLESHOOTING.md`.
