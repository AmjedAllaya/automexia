# Fix live inline-table discovery

- Prioritize recent table blocks so older output cannot exhaust sparse-row
  admission before newly printed tables are examined.
- Recover verified, soft-wrapped headers and frame rules for long live output
  within the existing history, copy and model budgets.
- Add incremental, narrow-pane, history-pressure and interactive WSL prompt
  regression coverage without depending on command names or window resizing.
- Recognize row-labelled numeric tables with a blank first header cell, including
  Linux memory summaries and device inventories, without dropping their headers
  or sparse continuation rows. The shared detector has no distribution gate.
- Exercise live table toggles against existing output in native Bash, Zsh and
  Fish sessions, retaining source text and command identity.
- Keep lightweight output customizations in the resolved window base so switching
  panes or tabs cannot silently revert inline tables and related settings.
