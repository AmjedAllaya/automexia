# Connection Hub and remote shells

Automexia is a terminal host for ordinary system remote-access tools.

1. Open a pane with your preferred shell.
2. Run `ssh`, `scp`, or `sftp` using normal shell syntax.
3. Review OpenSSH's host-key and authentication prompts.
4. Use terminal tabs, panes, search, selection, and paste as needed.
5. Exit the remote shell or close the pane to release the owned local session.

OpenSSH and the operating system retain credentials, agents, configuration,
host-key policy, authentication, and networking. Automexia never adds Enter to
pasted text.

## Browse connections with the keyboard

Open **Connection Hub** with `Ctrl+Shift+H` on Windows/Linux or `Cmd+Shift+H`
on macOS. Choose SSH configuration files, review the exact file list, then
confirm the local scan. The Hub does not scan your home folder automatically.
Choose only files you are authorized to inspect; do not select private keys or
vault exports. Included files remain subject to the explicitly granted scope.

Use the arrows to select a connection and Enter to review it. `/` or
`Ctrl/Cmd+F` focuses search. Outside a text field, `G` changes grouping, `V`
filters favorites, `R` filters recent entries, `S` changes the source filter,
and `X` clears filters. Space reviews a favorite change; `T` edits tags.
Confirm the resulting metadata review to save it. Escape cancels the current
edit or review before closing the Hub. An edit stays attached to its original
connection even if the catalog changes; conflicting changes require a new review.

`L` opens a single-host review with separate host, user and port fields. Tab and
Shift+Tab move between fields and actions. Paste works with `Ctrl+V`,
`Ctrl+Shift+V`, `Cmd+V`, or Shift+Insert in text fields. Unsupported control
characters and overlong text are rejected as a whole. Paste never submits a form
or sends terminal input.

From a connection review, `C` copies the prepared SSH command. Return to a
terminal, paste it, review it, and press Enter yourself. A failed clipboard write
is reported as a failure. OpenSSH still owns host-key verification, authentication
and any configured helper commands; reviewing inventory does not approve them.

## What is available

The current Hub supports local inventory, saved SSH connections, external-agent
source settings, search, grouping, reviewed favorites and tags, and connection
preparation. Settings are saved privately. Local file
selection and scanning must be repeated after restarting; a saved favorite alone
does not grant file access or start a connection.

Managed **Connect**, provider login/refresh, and workspace execution remain
disabled pending protected activation and verification. A provider listed in the
Hub is not evidence of an authenticated account. The Hub does not browse vault
records, fetch passwords, or connect
to a cloud account on the user's behalf.

## Save and edit SSH connections

Choose **Saved** or press `N` outside a text field. Press `N` again to add a
connection, or select an existing entry and press Enter to edit it. Enter a name,
host, and optional user and port. Choose the OpenSSH default identity or one of
your configured external-agent sources. Passwords and private keys are never
fields in this form.

Tab and Shift+Tab stay within the form. Left/Right or Enter changes the credential
source. Focus **Save** and press Enter to save; saving never launches a process.
Once a save has been accepted, Escape can leave the editor while the save finishes.
Escape cancels a draft, then returns from the saved list to the Hub. Delete opens
a removal confirmation with Cancel selected. A connection referenced by a
workspace or another connection cannot be removed. Conflicting edits preserve
the draft and require reopening it against the latest saved state.

Saved connections appear immediately in the main catalog and after restart.
The visual editor handles simple explicit SSH hosts. Advanced imported transports,
jumps, tunnels, and recipes retain their existing library definitions and are
not silently simplified by this editor. A source-bound connection cannot fall
back to another identity while managed credential-aware launching is unavailable.

## Configure external-agent sources

Choose **Vaults** or press `K` outside a text field. Press `N` to add a source.
Give it a name and choose System SSH agent, 1Password, Bitwarden, KeePassXC, or
another SSH agent. On Linux/macOS, an optional absolute agent socket path can be
saved; an empty path uses the system agent. Windows uses the native system-agent
endpoint. The provider label describes the intended owner, not a verified login
or a separate running agent.

Tab/Shift+Tab move between fields and actions; Left/Right changes the provider.
Use Save to persist the settings, Escape to cancel, and Delete in the list to
request removal. Sources used by saved connections cannot be removed. Changing a
source invalidates approvals for dependent profiles and workspaces. The vault,
its accounts, keys, and services remain under the vault application's control.

The private connection library uses schema 3. Older schema-1/2 documents load as
migration previews and are upgraded on an explicit successful edit, with the
existing previous-generation recovery retained. Older builds cannot read a
schema-3 library; keep a private copy of the original before upgrading if a
rollback is needed. Later edits also advance the recovery generation. Redacted
library export remains schema 2 and excludes source settings and identity bindings.

## Use an external vault for SSH

Use your vault's supported SSH-agent integration with system OpenSSH. Keep vault
unlocking and key authorization in the vault application; Automexia does not need
your master password, vault export, or private-key contents.

| Credential owner | Setup reference |
| --- | --- |
| 1Password | Enable its [SSH agent](https://www.1password.dev/ssh/agent) and configure your SSH client according to its platform instructions. |
| Bitwarden | Follow the [SSH agent guide](https://bitwarden.com/help/ssh-agent/), including platform-specific agent selection. |
| KeePassXC | Configure [SSH agent integration](https://keepassxc.org/docs/KeePassXC_UserGuide#_ssh_agent_integration) with the external agent. KeePass is a separate application; do not assume identical integration. |

First verify the connection with `ssh` in the intended shell. Windows OpenSSH and
OpenSSH inside WSL run in different environments; configuring a Windows vault
agent does not by itself prove the WSL client can reach it. Automexia does not
change agent services or install a bridge automatically. If the vault is locked,
unlock it externally and retry. Preserve OpenSSH's host-key checks and interactive
password/MFA prompts.

Cloud credentials likewise remain with the provider's own tools and approved
credential store. Use those tools in a terminal; a cached provider label in the
Hub is not a live authentication check.

For troubleshooting, preserve OpenSSH's original diagnostic, confirm the command
works in another ordinary terminal, and inspect
[Remote connections](../guide/remote-connections.md) and
[Troubleshooting](../TROUBLESHOOTING.md).

Unreleased remote products and services are outside public documentation.

## Assurance evidence anchor compatibility

These headings preserve source-owned feature-matrix references after the
public documentation consolidation. They do not expand shipped behavior,
reintroduce private plans, or replace the current status stated above.

### Provider Authentication Framework Current Source Boundary

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Review Only Workspaces And Broadcast

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Review Typed Tunnels After Activation

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Use The Read Only Connection Hub

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### What The Current Managed Ssh Preparation Means

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.
