# ADR 0066: Explicit editor file handoff

Status: accepted for the implemented `amx edit` command.

## Placement and alternatives

The existing application CLI/broker owns this explicit desktop action, beside
browser and directory handoffs. It belongs neither to VT/render/input core nor
to a new extension: it has the same narrow lifecycle and installed-handler
boundary as existing actions. `desktop_path` now owns the common path mechanism
for two real consumers; callers select directory versus regular file. Existing
path regressions remain, with separate real-file tests. No new dependency,
service, cache, watcher or startup I/O is introduced.

VS Code's registered file protocol avoids shell evaluation and Windows batch
launchers. Supervised local-tool jobs are not used to own an editor's lifetime:
the desktop owns that application. Existing terminal-settings editor config is
intentionally separate because it launches a PTY-hosted editor, not this desktop
handoff. Arbitrary command templates and Remote-WSL activation were rejected to
avoid new execution authority and automatic remote bootstrap downloads.

## Policy and resource boundaries

The default is VS Code. A strict user-root `amx.toml`, bounded to 16 KiB and
read through the existing identity-checked regular-file reader, selects VS Code,
Insiders or disabled. Unknown versions/fields and unreadable, linked, invalid or
oversized files fail closed. A command-line choice cannot bypass disabled.
There are no writes, migrations or generated settings; project config is not
discovered. Removing/changing this user file rolls back the preference.

Paths resolve on their owning OS, must name existing regular files and retain
the shared 4096-byte and hostile-text policy. The existing isolated, leased WSL
probe accepts a second fixed file-kind route; it cannot evaluate supplied source.
Host UNC file opening does not request Remote-WSL. The protocol authority is
always `file`, the scheme is an enum, and components are encoded using the
existing URL crate. Line/column are positive signed-32-bit-compatible values.
Source inspection shows VS Code decodes before parsing colon positions; reject
ambiguous paths. Workspace manifests are rejected rather than promoted into a
workspace-open action. Generic Quick Action execution remains blocked.

Preview performs metadata resolution and one bounded preferences read, but never
calls the desktop adapter. Actual invocation reports only a requested handoff.
Native path metadata can stall, and Unix helper spawn success does not certify
association success. A path can change between validation and editor access.
The editor's extensions, trust prompts, network behavior and recent files are
external policy. No such work runs on terminal input/render/startup paths.

## Verification and limitations

Tests cover exact URIs, Unicode/symbols, percent-encoding, UNC/drive paths,
position and config boundaries, disabled/invalid preferences, no-launch previews,
no writes, regular-file versus directory authority and content-free errors.
Real executable tests source shipped shell adapters and check independent URI
oracles. Guest tests resolve file symlinks and reject project Python imports.
The reinforcement checker mutation-tests the required owner contracts.
The correctness-checked URI benchmark excludes filesystem/process/desktop latency.

Native desktop activation, window visibility, editor trust/UNC prompts and actual
cursor placement require an installed editor on each claimed platform. Preview
coverage and a Windows executable in WSL are not native Linux/macOS GUI proof.
See [user behavior](../user-guide/edit-file.md) and [test instructions](../TESTING.md).

## Primary sources

- [VS Code CLI and file URL contract](https://code.visualstudio.com/docs/configure/command-line)
- [VS Code URI filesystem conversion](https://github.com/microsoft/vscode/blob/main/src/vs/base/common/uri.ts)
- [VS Code line/column parsing](https://github.com/microsoft/vscode/blob/main/src/vs/base/common/extpath.ts)
- [VS Code protocol activation](https://github.com/microsoft/vscode/blob/main/src/vs/code/electron-main/app.ts)
- [WSL filesystem and environment interoperability](https://learn.microsoft.com/en-us/windows/wsl/filesystems)
- [Directory desktop boundary](0065-explicit-directory-handoff.md)
