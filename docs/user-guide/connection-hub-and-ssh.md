# Connection Hub and SSH

> **Connection Hub is not available as a user-facing feature yet.**
>
> In the current documented release there is **no Connection Hub window, menu item, shortcut, CLI command, or configuration screen** to open. Automexia contains internal inventory/review/planning models for a future managed SSH experience, but process/network authority is intentionally disabled. To connect to remote machines today, use the system OpenSSH client from the Automexia shell.

## What to use today

The supported current workflow is:

```text
Automexia terminal
    ↓
your real shell
    ↓
system OpenSSH (`ssh`, `scp`, `sftp`)
    ↓
your existing SSH config / agent / keys / known_hosts
```

Automexia hosts the terminal session. OpenSSH remains responsible for SSH configuration, authentication, host-key verification, jump hosts, forwarding, certificates, agents, and the network connection.

## Where SSH configuration lives

Use the normal OpenSSH configuration file for your operating system.

| Platform | Typical user configuration |
|---|---|
| Linux | `~/.ssh/config` |
| macOS | `~/.ssh/config` |
| Windows OpenSSH | `%USERPROFILE%\.ssh\config` |

Automexia does not currently provide a separate Connection Hub configuration file that users should edit.

## Configure a simple saved host

Add a host block to your OpenSSH config:

```sshconfig
Host production
    HostName 203.0.113.20
    User deploy
    Port 22
    IdentityFile ~/.ssh/id_ed25519
```

Then, inside Automexia, connect with:

```text
ssh production
```

This is the current equivalent of a "saved connection". The name `production` is an OpenSSH alias, not an Automexia Connection Hub record.

## Configure several environments

```sshconfig
Host dev
    HostName dev.example.com
    User alex
    IdentityFile ~/.ssh/id_ed25519

Host staging
    HostName staging.example.com
    User deploy
    IdentityFile ~/.ssh/id_ed25519

Host production
    HostName prod.example.com
    User deploy
    IdentityFile ~/.ssh/id_ed25519_prod
```

Use them directly:

```text
ssh dev
ssh staging
ssh production
```

This approach is recommended for repeatable hosts because OpenSSH already owns the complete precedence and authentication model.

## Connect through a bastion / jump host

Configure the route in OpenSSH:

```sshconfig
Host bastion
    HostName bastion.example.com
    User ops
    IdentityFile ~/.ssh/id_ed25519

Host private-app
    HostName 10.20.0.15
    User deploy
    ProxyJump bastion
    IdentityFile ~/.ssh/id_ed25519
```

Then connect normally:

```text
ssh private-app
```

You can also use an explicit one-off jump without saving it:

```text
ssh -J bastion private-app
```

## Use SSH keys and an agent

OpenSSH should continue to own private-key handling. A typical workflow is:

```text
ssh-add ~/.ssh/id_ed25519
ssh production
```

On platforms with an OS keychain, SSH agent, hardware token, certificate workflow, or organization-specific credential tooling, keep using that established mechanism. Do not put private keys, passwords, or tokens into Automexia Quick Actions, aliases, logs, or documentation examples.

## Copy files

Use the same OpenSSH alias with `scp`:

```text
scp build.tar.gz production:/tmp/
scp production:/var/log/app.log .
```

Or use `sftp`:

```text
sftp production
```

There is no current Connection Hub remote-file browser.

## Port forwarding

Local forwarding remains an explicit OpenSSH operation:

```text
ssh -L 8080:localhost:80 production
```

A saved form can live in OpenSSH config:

```sshconfig
Host production-web
    HostName prod.example.com
    User deploy
    LocalForward 8080 localhost:80
```

Then:

```text
ssh production-web
```

Automexia does not silently create, remember, or replay tunnels in the current release.

## Inspect and troubleshoot an SSH host

To see what OpenSSH resolves for an alias, run the system client explicitly:

```text
ssh -G production
```

For connection diagnostics:

```text
ssh -v production
```

For deeper OpenSSH diagnostics, increase verbosity only when needed:

```text
ssh -vv production
```

Be careful when sharing verbose output because it can contain usernames, hostnames, local paths, and environment details.

## A useful Automexia remote-work layout

A practical layout today is:

```text
window-level tab: application
├── pane 1: local source/build shell
└── pane 2: ssh production

window-level tab: operations
├── pane 1: ssh bastion
└── pane 2: kubectl / provider CLI / logs
```

Use a fresh split for a clean shell before connecting, or clone a split when you want the new pane to begin with the same local profile/directory. See [Workspaces, tabs, and panes](workspace.md) for the exact distinction.

## So what is Connection Hub?

Connection Hub is the planned structured UI around remote connections. Its purpose is to make remote work easier to discover and review without replacing OpenSSH.

The intended flow is conceptually:

```text
Discover known connection information
        ↓
Select a host / connection
        ↓
Explain where the public values came from
        ↓
Review executable + exact arguments + route/tunnel intent
        ↓
Approve the required launch capability
        ↓
Launch system OpenSSH
        ↓
Show session-scoped connection/context state
```

The important point is that **the last launch step is not enabled in the current product**. Internal record, inventory, review, and dry-run models should not be interpreted as a hidden feature that you can configure or activate.

## What Connection Hub is expected to add later

When the managed feature is shipped, it is intended to add structure around things that are awkward to represent only as shell commands, such as:

- discovering known local SSH aliases;
- presenting connection records in a searchable UI;
- explaining the source/state of public connection values;
- reviewing the exact executable and arguments before launch;
- representing tunnel/route intent explicitly;
- displaying host/provider/session context without mixing sessions;
- launching only through a reviewed application-owned boundary while system OpenSSH remains the SSH implementation.

These are roadmap goals, not current user instructions.

## What you cannot configure yet

There is currently no supported user workflow for:

- opening a Connection Hub;
- adding a Connection Hub host through the UI;
- importing hosts into a Connection Hub screen;
- assigning a Connection Hub keyboard shortcut;
- launching `automexia connection ...` or similar CLI commands;
- enabling managed SSH in `automexia.toml`;
- asking Automexia to store SSH passwords/private keys;
- using one-click provider authentication from the Hub;
- using an Automexia-managed remote-file browser.

If you find internal `devops-ssh` records or planning models in the repository, treat them as implementation foundations, not as public configuration.

## Current vs future: quick decision table

| You want to… | Use now |
|---|---|
| Save a host | `Host` entry in OpenSSH config |
| Connect | `ssh alias` |
| Use a jump host | `ProxyJump` or `ssh -J` |
| Choose a key | `IdentityFile` / SSH agent |
| Transfer files | `scp` or `sftp` |
| Forward a port | `ssh -L`, `-R`, `-D` or OpenSSH config |
| Diagnose config | `ssh -G alias` |
| Diagnose a connection | `ssh -v alias` |
| Work local + remote side by side | Automexia splits + system `ssh` |
| Browse/review managed connections in a structured Automexia UI | **Not available yet — planned Connection Hub** |
| Let Automexia launch a reviewed managed SSH session | **Not available yet** |

For the higher-level remote workflow, see [Remote sessions and WSL](remote-and-wsl.md). For implementation/security boundaries and roadmap detail, see [Remote connections](../guide/remote-connections.md).
