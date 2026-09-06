# Automexia Terminal

Automexia is a keyboard-first terminal focused on fast local shell work,
independent sessions, clear command output, and user control.

This repository documents only free baseline terminal behavior.
Unreleased advanced features and all commercial plans are maintained privately.

## Public capabilities

- native local shell sessions on supported platforms;
- windows, tabs, pane-local tabs, splits, focus, and resize;
- scrollback, reflow, scoped search, selection, copy, and paste;
- Unicode text, hyperlinks, mouse reporting, and common terminal protocols;
- themes, fonts, cursor and appearance preferences;
- session-local shell integration with native-shell ownership;
- command-boundary navigation and a keyboard command palette;
- Sixel, Kitty, and iTerm2 images plus bounded local raster preview;
- ordinary system OpenSSH interoperability and explicit local inventory; and
- accessibility, packaging, migration, security, and contributor tooling.

See [Features](docs/FEATURES.md) for the public catalog.

## Install and start

Linux v0.4.0 Early Access is available for x64 and Arm64 as DEB, RPM and portable
archives from the [official release](https://github.com/AmjedAllaya/automexia-releases/releases/tag/v0.4.0).
Verify the signature and package checksum before installation. This is an
official prerelease, not a stable Windows/macOS release.

Follow [Installation](docs/INSTALLATION.md) and
[Getting started](docs/GETTING-STARTED.md).

The application command is:

```text
automexia [OPTIONS] [COMMAND]
```

Example:

```text
automexia --working-dir ./project
```

Use `-e, --command <PROGRAM> [ARGS...]` to launch an explicit program. It is
the final Automexia option because remaining values are exact program arguments.
Use the real shell for pipelines, redirects, aliases, functions, and expansion.

## Work in the shell

PowerShell, CMD, Bash, Zsh, Fish, and other supported shells keep ownership of
command editing, history, quoting, completion, aliases, pipelines, and
execution. Automexia does not reconstruct commands from terminal cells and
never adds Enter to pasted or inserted text.

Run normal tools exactly as in another terminal:

```text
git status
cargo test
ssh alice@example.invalid
```

## Essential shortcuts

Use [Keyboard](docs/KEYBOARD.md) as the exact authority. Common defaults include
new tab, split, pane focus, search, command palette, copy, paste, and font-size
controls. Platform bindings preserve normal shell interrupt and editing
behavior.

## OpenSSH

Run the system `ssh`, `scp`, and `sftp` clients in a pane. OpenSSH and the
operating system retain configuration, credentials, agents, host keys,
authentication, and networking. Automexia owns terminal input/output, resize,
search, selection, tabs, panes, exit reporting, and cleanup.

See [Remote connections](docs/guide/remote-connections.md).

## Configuration

Automexia uses bounded TOML configuration with transactional reload and
last-known-good behavior. Existing files are not overwritten by starter
generation or migration.

See [Configuration](docs/CONFIGURATION.md) and [Migration](docs/MIGRATION.md).

## Build and contribute

Read [Contributing](CONTRIBUTING.md), [Architecture](docs/ARCHITECTURE.md),
[Testing](docs/TESTING.md), and [Contributor commands](docs/CLI-REFERENCE.md).

A typical non-launching contributor gate is:

```text
cargo ready
```

Before publishing documentation, verify links, confidential-data hygiene, and
the [private documentation boundary](docs/PRIVATE-DOCUMENTATION-POLICY.md).

## Project status

Source, tests, cross-compiles, and native release evidence are distinct. Public
availability is stated only in the feature catalog and release documentation.
Internal names, fixtures, or disabled source are not product announcements.

## License and security

See [LICENSE](LICENSE), [third-party notices](THIRD_PARTY_NOTICES.md), and
[Security policy](SECURITY.md).
