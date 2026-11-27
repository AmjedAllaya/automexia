# ADR 0100: Docked Automexia sessions in VS Code

Status: Accepted for preview implementation.

## Context

The VS Code terminal profile from ADR 0099 uses the editor's integrated
terminal. A docked webview gives Automexia a surface for its own interface
inside the editor and runs on the editor side of a VS Code remote connection.
In WSL, its extension host and native shell sessions run in the distribution.
[VS Code view contributions](https://code.visualstudio.com/api/references/contribution-points#contributes.viewsContainers)

## Decision

Contribute a docked **Automexia** panel to the existing VS Code extension.
Use VS Code for panel placement, editor commands, and clipboard access.
The webview owns presentation and sends typed input and resize requests
through the extension host. Retain the CLI terminal profile as another way
to use Automexia commands.

The extension host starts the native `automexia-vscode-host` companion
for a trusted workspace. The companion owns each shell, PTY, terminal
state, and process cleanup. It reuses `librio` and the native PTY
implementation. The extension host selects the shell executable from VS Code
and passes supported explicit arguments. The browser receives render state
for display and uses the editor's equivalents for native window affordances.

The bridge uses typed JSON Lines over child standard input and output.
The webview applies a strict content security policy and the sessions use
local process pipes. Retain at most eight sessions in the panel and request
frames while the panel is visible. Closing the view releases its sessions
and companion processes.

The version 1 bridge advertises optional session context in its ready message.
The companion owns the host, shell, account, and bounded shell-reported
directory facts for each session. It provides the launch directory as a
separately identified fallback and leaves shell startup files as configured.
The extension host may add a branch or abbreviated detached commit from
repositories already known to the VS Code Git extension. A reported path
only participates in matching against that existing catalog. The webview
renders the resulting context for the matching pane. A legacy companion
without the capability continues to provide terminal frames and leaves the
information bar unavailable.

The version 1 ready message can also carry the preset catalog projected from
`automexia-ui-model`. Each entry supplies the preset ID, display label, visual
style, arrangement, spacing, and ordered supported roles. The extension
validates that bounded display data and the webview chooses how to draw it.
Older companions can keep sending frames without the catalog.

The Quick Actions and saved Workspaces browsers use the existing Automexia
CLI catalog owners. An explicit command runs a bounded list request on the
approved extension host. The Quick Pick shows catalog metadata. Desktop
controls retain ownership of action review and execution, connection approval
and launch, and Workspace restore.

The explicit Open Desktop App command starts a separate native Automexia
window on the extension host. The desktop executable retains its ordinary
window and application lifecycle.

An exit response can carry the shell status from the PTY's child notification
and a bounded final frame. A null or omitted status means no shell status was
available. Old companions may omit both fields. A closed companion transport
without an exit response is reported separately from shell termination.

Companion selection remains a host decision. The optional
`automexia.docked.hostExecutable` machine setting selects a file on that
host. In a platform package, the default is the bundled companion for the
extension host's OS and architecture. The source-only development path still
looks in the configured Automexia binary directory and host PATH. Uninstalling
the extension removes the docked view and its extension-owned runtime files.

## Packaging and evidence

Platform VSIXs use VS Code target metadata and the fixed
`runtime/<target>/{companion,desktop}` layout. WSL uses the matching Linux
target. The desktop payload comes from the existing release stage and retains
its runtime layout. The separately staged Windows companion carries the
pinned ConPTY files it needs beside its executable. The macOS app is copied
as a complete bundle. Executable resolution stays within the installed
target directories. The extension can use bundled CLI and desktop binaries
without an installer, download, or workspace-relative lookup.

The repository assembler checks the staged runtime layout and architectures
before producing the VSIX. Package validation has a separate native gate for
installation, executable permissions, dependent libraries, and signature
verification after extraction.

Verify the packaged extension and matching companion in disposable native
environments with no Automexia on PATH and no binary overrides. Check initial
launch, command input, resize, split and tab isolation, PTY exit, process
cleanup, information-bar presets, CLI availability, and desktop commands.
Check explicit binary overrides and source-only installations separately.
The Windows companion probe exercises PowerShell's prompt, input, context,
and exit status. Native evidence for the bundled preview is pending.
