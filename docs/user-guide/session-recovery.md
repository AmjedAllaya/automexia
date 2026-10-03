# Restore a workspace

Automexia periodically saves the layout of its active workspace. After an app or
machine crash, or after closing the last window, the next launch offers:

- **Restore** (`R`): reopen the saved tabs, splits and pane-local tabs in fresh terminals.
- **Start clean** (`Esc` or `S`): replace the previous workspace with a new terminal.

Tab and the arrow keys select a button; Enter activates it. Start clean is the
initial selection. Closing this choice leaves the saved workspace available for
the next launch. No restored terminal starts before your choice.

Recovery restores layout and selection, window size/position, shell identity,
working folders and custom tab titles/colors. It does **not** resume commands,
running jobs, shell variables, terminal output or scrollback. SSH connections
reopen as local shells: reconnect explicitly, using your normal SSH command.
Authentication, host keys and optional helper upload still follow the ordinary
SSH workflow. No credentials or remote destination is saved by recovery.

Custom titles longer than 128 bytes or containing control characters, invalid
colors, and unusable window positions are omitted without dropping their terminals.

Missing local folders fall back to the profile's starting folder. Unsupported or
failed profiles do not block other terminals; close any blank failed entries and
open the intended shell. WSL distribution names and guest paths are passed as
literal arguments without probing the guest; an unavailable distribution or
guest directory can fail to open. macOS native tab groups reopen as separate
windows. Window placement adjusts to the current display.

## Control what is saved

In `config.toml`:

```toml
[session-recovery]
enabled = true
excluded-profiles = ["cmd", "wsl:Example"]
```

Supported exclusion keys are `configured`, `powershell`, `pwsh`, `cmd`, `bash`,
`zsh`, `fish`, `sh`, `nu`, `wsl`, and `wsl:<distribution>`. Matching ignores case;
`wsl` excludes every distribution. The configured startup shell uses `configured`;
other recognized shells use their shell key. Set `enabled = false` to stop saving and offering recovery.
The exclusion list accepts up to 128 names of at most 132 bytes each.
Existing checkpoints remain on disk, but exclusions also apply when restoring
older checkpoints. Arbitrary executables are excluded; commands and scripts are never replayed.

Private snapshots live under the configuration directory in `state/session-v1`,
separately from preferences. They contain local paths and custom labels, so treat
them as private files. Only one Automexia process owns recovery at a time; another
instance opens normally without replacing its checkpoint. A previous valid copy
can recover an interrupted write. Newer unsupported formats remain untouched.

Recovery is bounded to eight windows and 64 terminals, with up to 28 top-level
tabs per window. Explicitly closed tabs and the Undo Close history are excluded.
The last periodic checkpoint may precede a sudden crash by a few seconds; app
close requests also submit a final checkpoint. Disk failures cannot guarantee a
new checkpoint, and leave the existing valid copy available.
