Install the existing pinned Python YAML dependency in every workflow job that
runs workspace tests, coverage or full QA, including release preflight. Each
job now installs it after selecting Python and before running policy checks.
Add a workflow regression that rejects missing, late or conditional installs
instead of relying on packages preinstalled on a runner or in a different job.
