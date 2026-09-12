# Open a directory from the terminal

```text
amx open
amx open .
amx open "folder with spaces"
amx open --preview .
amx open -- -leading-hyphen
```

`amx open` opens an existing directory in the desktop file manager. With no
argument it uses the current directory. Outside an integrated Automexia session,
use `automexia open`. Quote spaces and shell symbols normally. This command does
not execute files, create directories, install software or change associations.

`--preview` resolves the path and prints a JSON destination without launching
the file manager. Unlike a search-argument preview, this consults the filesystem
and, in Windows-backed WSL, a supervised guest resolver. The destination can
contain private directory names; review preview output before sharing it.
Automexia does not save it. An explicitly selected network path may contact its
filesystem even during preview.

## Platforms and errors

- Linux/BSD needs a graphical desktop with `xdg-open` and a directory handler.
  A headless session cannot display a local file manager.
- macOS reveals and selects the directory in Finder. Reveal-only behavior avoids
  launching an application bundle, which is also a directory on disk.
- Windows uses the folder-specific Explorer association.
- Windows-backed WSL resolves Linux symlinks inside the pane's distribution,
  then hands its WSL network path to Windows. Install Python 3.10 or newer inside
  that distribution explicitly. Native Linux/macOS do not need this helper.

The command accepts directories only. Missing paths, files, paths over 4096
UTF-8 bytes, controls and bidi-formatting characters are rejected. Linux names
that cannot be represented safely in Windows, including device names and names
ending in a dot or space, require a native Linux file manager instead. A desktop
handoff can succeed before the file manager finishes starting; the CLI does not
certify that its window is visible. Check the installed handler if no window
appears. The desktop and filesystem remain responsible for access permissions.

WSL resolution is cancellable with Ctrl+C and has bounded child cleanup and
deadlines. Native filesystem resolution follows the OS and may wait on a slow
mount. There is no background polling, persistent cache or new daemon.
Set `AUTOMEXIA_AMX=0` before loading shell integration to disable the session
helper. Existing `amx` aliases, functions and executables keep precedence.
