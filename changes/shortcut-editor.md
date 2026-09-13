### Added

- Double-click command-palette shortcut badges or press F2 on a selected command
  to record a new binding, review conflicts, save, cancel or reset to config.
- Persist single-chord UI overrides with existing private settings, live binding
  updates, write acknowledgements and explicit session-only error feedback.

### Fixed

- Keep palette IME, dropped files and paste out of terminal input; preserve
  query/selection and cancel stale queued edits after binding changes.
- Retain failed settings-write state after reporting the error, preventing flush
  from treating an acknowledged failure as a successful save.

### Validation

- Added platform-table, profile/collision, gesture, repeat, reset/retry, layout,
  persistence, separate-process restart and correctness-checked benchmark coverage.
- Native desktop frames and screen-reader delivery remain separate external gates.
