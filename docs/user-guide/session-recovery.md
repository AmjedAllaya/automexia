# Restore a workspace

Automexia saves active tabs, splits, pane-local tabs, selection, window geometry,
working folders and custom tab titles/colors. By default it asks to restore useful
workspaces: multiple terminals, integrated SSH activity, long-running commands or
sustained command activity. An idle window or a few quick commands do not qualify.

Choose **Restore** (`R`) or **Start clean** (`Esc` or `S`). Tab and arrows select a
button; Enter activates it. Closing this choice preserves the saved session.
**Menu > Tabs & Windows > Restore previous session** is also available when startup
opens quietly. Manual restore opens additional windows and keeps current work.
Repeated activation cannot duplicate an in-progress or consumed recovery.

Recovery saves up to the latest **10,000 physical lines per terminal**, including
commands and output, with text styles, Unicode, soft wraps and completed-command
metadata. Recovered history is scrollable above a clearly marked fresh shell.
Saved commands are never rerun. Running jobs, editors, environment variables,
images and active hyperlinks are not resumed. Remote connections reopen locally;
reconnect explicitly using the normal SSH authentication and host-key workflow.
An editor's alternate screen is excluded; its ordinary terminal history remains.

Output can include sensitive information. Checkpoints are encrypted for the local
user: Windows uses DPAPI; Linux/BSD use an unlocked Secret Service and macOS uses
Keychain for an encryption key. Missing protection leaves the last valid checkpoint
untouched and shows an unavailable notice. There is no plaintext fallback. Keep
recovery files private even when encrypted; they are not portable backups.

Linux/BSD credential services may ask for authorization when first creating the
recovery key. Automexia does not request unlocking an existing locked key or
collection. Credential work runs in the background; a refused or unavailable
service cannot cause plaintext history to be saved.

## Control recovery

```toml
[session-recovery]
enabled = true
startup-prompt = "smart" # "always" or "never" also supported
save-history = true
excluded-profiles = ["cmd", "wsl:Example"]
```

`never` skips automatic prompts while preserving manual restore. `save-history =
false` saves only layout; existing saved history remains until its candidate is
consumed or replaced. `enabled = false` stops saving and offering recovery without
deleting existing files. Exclusions apply to capture and restoring older sessions.
Keys are `configured`, `powershell`, `pwsh`, `cmd`, `bash`, `zsh`, `fish`, `sh`, `nu`,
`wsl`, and `wsl:<distribution>`, ignoring case. The configured startup shell uses
`configured`. Up to 128 exclusion names of 132 bytes are allowed.

## Retention and failures

After successful recovery and publication of the new workspace checkpoint,
Automexia removes the consumed manual candidate and obsolete backup. Failed or
cancelled recovery retains retry data. Useful new work, or a session used for
at least 30 minutes and then closed, replaces the older candidate. Short accidental
visits preserve useful previous work. Storage retains at most one current checkpoint,
one write-recovery backup and one manual candidate; abandoned private staging files
are cleaned under the store's exclusive lock.

Snapshots live separately from preferences in `state/session-v1`. Version-1 layouts
migrate to an encrypted version-2 envelope; unsupported newer versions are preserved.
Only one Automexia process owns this store. Saves coalesce and occur at most every
10 seconds, plus a final close checkpoint. A sudden crash may lose the latest interval.
Disk or protection failures preserve prior data rather than promising a new save.

Bounds are eight windows, 64 terminals, 28 top-level tabs per window, 1,024 columns,
two million cells per terminal and four million across the workspace. Wide or very
large workspaces may retain fewer than 10,000 lines per terminal. Serialized and
compressed records are bounded. Explicitly closed tabs and Undo Close history are
excluded. Missing local folders fall back to the current profile's starting folder;
failed profiles do not block the others. Unavailable WSL distributions or guest
folders can fail to open. macOS native tab groups reopen as separate windows.
