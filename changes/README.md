# Changelog fragments

Add one Markdown file per user-relevant pull request. Name it
`<issue-or-pr>-<short-description>.md` and begin with exactly one category:
`Added`, `Changed`, `Fixed`, `Security`, `Deprecated`, or `Removed`.

Documentation-only, tests-only, and internal-maintenance PRs may omit a fragment
only when explicitly labeled. Release automation assembles fragments into the
root changelog and removes the consumed files in the release commit.
