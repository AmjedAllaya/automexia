# Upstream policy

The `rio-upstream` remote tracks https://github.com/raphamorim/rio. Automexia's
audited base is `7d595af583f6ef1ea6036a66b367ba1e5a84d4a2`, tagged locally as
`rio-base-0.5.20-7d595af`.

Upstream updates begin on a short-lived compatibility branch. Maintainers review
the upstream range, dependency/security changes, platform behavior, and conflicts,
then selectively port coherent commits into Automexia with upstream commit IDs in
the commit or PR description. Run the complete conformance and platform matrix
before merging.

Never merge a moving Rio branch directly into stable Automexia. Do not rewrite
upstream authorship, remove inherited headers, or replace the upstream MIT notice.
The legacy bootstrap package remains an external downloadable archive and is not
part of the standalone development workflow.
