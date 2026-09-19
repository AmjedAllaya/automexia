# ADR 0065: Explicit directory-only desktop handoff

Status: accepted for `amx open [directory]`.

## Placement and reuse

The application CLI owns the explicit request; `directory_open` selects the
directory policy, the shared `desktop_path` owner resolves and validates the
destination, and the existing `desktop_open` adapter hands it to the desktop.
No renderer, PTY, startup hook, extension or background service acquires new
authority. There is no new crate or dependency. A separate file-manager extension
or a custom file browser would duplicate the installed desktop and add lifecycle
cost without improving this one-shot contract. Generic Quick Action execution
remains disabled; directory opening does not authorize project scripts.

## Contract and boundaries

The default is the current directory. Existing directories only are accepted;
regular files, missing paths, invalid text and unsafe Windows representations
fail with content-free errors. Inputs and resolved filesystem paths are bounded
to 4096 UTF-8 bytes. Controls and bidi-formatting characters are rejected.
Native canonicalization preserves symlinks and directory identity. Windows
verbatim drive/UNC prefixes are translated for the desktop only after rejecting
components that would alias reserved devices or change under normalization.

For Windows-backed WSL, one fixed Python probe resolves POSIX symlinks and parent
segments inside the guest. It runs as an isolated child of the existing leased
supervisor, not on its cancellation loop. The probe accepts one path argument,
not source code. Its JSON result must identify an absolute canonical directory
and pass host-representation checks before conversion to a WSL UNC destination.
The existing process/output limits, 12-second guest deadline and 15-second host
deadline apply. Python 3.10 or newer is required inside this distribution.
Native Linux/macOS do not use this helper.

`--preview` performs resolution but never calls a desktop handler. It emits JSON
on stdout, including the actual destination: users must review it before sharing.
There is no Automexia history, cache or configuration write. Windows uses the
directory-specific `explore` verb with COM lifetime management; Linux uses
`xdg-open` and macOS uses `/usr/bin/open -R` to reveal the directory in Finder.
The reveal-only macOS behavior avoids launching directory-shaped application
bundles through their default action; a filename-extension denylist would miss
custom package types. These remain exact-argument desktop adapters.
The desktop owns the resulting application lifetime. A successful handoff does
not prove a window became visible; Unix spawn success also does not prove the
association helper's later success.

Filesystem resolution is not a sandbox. Explicit network-mounted paths may
contact their filesystem, native filesystem calls can stall in the OS, and an
external application can observe a path changed after validation. No project
code is intentionally executed, but installed handlers are outside Automexia's
trust boundary. The command does not alter associations or install anything.

## Assurance and rollback

Unit tests cover preview isolation, redacted failures/debug, directory-only
resolution, byte boundaries, reserved device names and UNC/device confusion.
Native CLI previews assert exact destinations and an unchanged storage tree.
The real WSL suite verifies guest symlinks, parent resolution, host conversion
and hostile Python imports while retaining process-lease tests. Its Windows
executable path is not native Linux Rust evidence. The checked mapping benchmark
measures pure conversion, not filesystem, process, file-manager or GUI latency.
Actual desktop association behavior on each OS remains a manual native gate.

Remove the route and probe to revert; no persisted data or migration is involved.
`AUTOMEXIA_AMX=0` disables the session helper without changing global commands.

## Primary references

- [xdg-open manual](https://portland.freedesktop.org/doc/xdg-open.html)
- [ShellExecuteW](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shellexecutew)
- [Windows path and device names](https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file)
- [WSL filesystem interoperability](https://learn.microsoft.com/en-us/windows/wsl/filesystems)
- [Python realpath](https://docs.python.org/3/library/os.path.html#os.path.realpath)
- [Apple bundle and package semantics](https://developer.apple.com/library/archive/documentation/CoreFoundation/Conceptual/CFBundles/AboutBundles/AboutBundles.html)
- [Existing local-tool process ownership](0064-bounded-explicit-local-tools.md)
