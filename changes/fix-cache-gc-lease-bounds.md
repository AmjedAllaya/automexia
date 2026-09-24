# Fix cache cleanup leases and traversal limits

- Keep cache cleanup admission and acquired native leases held through deletion so concurrent cache users cannot enter after eligibility checks.
- Preserve active cache users, reject concurrent collectors, and release locks on errors; dry runs remain read-only.
- Enforce file, discovered-directory, lease-entry, and retained-handle limits while scanning, before materializing directory contents.
- Add isolated temporary-fixture and native-process regressions for cleanup races, bounded traversal, and failure recovery.
