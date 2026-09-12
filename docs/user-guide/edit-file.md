# Open a file in your editor

In an integrated shell:

```sh
amx edit src/main.rs --line 42
amx edit src/main.rs --line 42 --column 7
amx edit --preview "src/example file.rs"
amx edit --editor vscode-insiders src/main.rs
amx edit -- --leading-minus.rs
```

The standalone executable uses `automexia edit` with the same arguments.
Visual Studio Code is the default. It must already be installed and registered
as the desktop handler for `vscode:` URLs. Insiders uses `vscode-insiders:`.
Automexia never installs an editor, extension or remote server for this command.
The editor used to open Automexia's own settings in a terminal is unchanged.

Only existing regular files are accepted. Relative paths use your current
directory; symlinks resolve on the filesystem that owns the session. Line and
column are one-based, default to 1, and must be at most 2147483647. The editor
decides how to clamp a position past the end of a file; a column is an editor
position, not a terminal cell count.

## Choose or disable the editor

Create `amx.toml` in your **user Automexia configuration directory**, alongside
`config.toml`, if you want to change the default:

```toml
version = 1
editor = "vscode-insiders"
```

Supported values are `vscode`, `vscode-insiders`, and `disabled`. `--editor`
overrides the preference for one invocation, but cannot bypass `disabled`.
To undo a customization, edit or remove this file yourself. Automexia reads it
only when explicitly asked to edit a file, and never creates or updates it.
It must be a regular unlinked UTF-8 file no larger than 16 KiB. Unknown fields,
unknown versions and invalid editor values fail with an explanation, not a
silent fallback. Project-local settings and `EDITOR`/`VISUAL` shell strings do
not choose an executable. Arbitrary editor command templates are not supported.

## Preview and platform behavior

`--preview` resolves the path and prints one JSON destination without opening
the editor or writing configuration. The destination includes your real path;
review it before sharing or saving it. The command itself keeps no history.
Your shell and editor may have their own history or recent-file lists.

Windows-backed WSL resolves the file in the guest using installed Python 3.10+
and opens its Windows-accessible UNC path in the host editor. It reads the host
Automexia preferences; it does not request Remote-WSL installation. Native
Linux/macOS builds use their own filesystem and registered desktop handler.
Allow normal editor trust or UNC prompts only after reviewing them; Automexia
does not approve them automatically. Editor plugins and network activity follow
the editor's settings, not Automexia's policy.

Controls, bidi-formatting characters, unsupported Windows path components,
ambiguous Unix colon/backslash filenames and `.code-workspace` manifests are
rejected to avoid opening a different file or a workspace. Paths are limited to
4096 UTF-8 bytes. Use the editor's own Open File dialog for unsupported names.
No file content is read by Automexia, but metadata lookup on a mounted filesystem
can contact that filesystem or wait on the OS. A file can change after checking;
this desktop handoff is not a filesystem sandbox or an atomic handle transfer.

A successful response means **handoff requested**, not that a window was proven
visible. If no editor opens, check the installed edition, its URL registration,
desktop session and permission prompts. Unix desktop helpers may report errors
after handoff. No software is installed as a fallback.
