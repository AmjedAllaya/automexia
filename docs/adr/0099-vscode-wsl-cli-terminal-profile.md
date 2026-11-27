# ADR 0099: Automexia CLI profile in VS Code

Status: Accepted for implementation.

## Context

Users want to select Automexia from VS Code's terminal dropdown and run its CLI
commands in the same panel with their default shell. VS Code supports
contributed profiles and owns the panel renderer and PTY. Automexia's desktop
executable uses a native window when started without a CLI command.
[VS Code terminal profiles](https://code.visualstudio.com/docs/terminal/profiles)

## Decision and ownership

The optional VS Code workspace extension contributes the **Automexia** profile
in trusted local Windows, macOS, and Linux workspaces and trusted WSL-connected
Linux workspaces. The provider leaves the shell executable and arguments for
VS Code to resolve from its default terminal profile. VS Code manages the
working directory, process, terminal I/O, rendering, shell integration, and
disposal. The extension and profile IDs stay stable so prior VSIX installs can
upgrade in place.
[VS Code terminal contribution API](https://code.visualstudio.com/api/references/contribution-points#contributes.terminal)

The user's Automexia installation or source build owns the `amx` and `automexia`
executables, which use `.exe` names on Windows. The VSIX contains the provider
and can be installed from any location VS Code can read. With no custom binary
directory, the selected shell uses its normal command lookup. A custom
directory is prepended using the host's `PATH` separator, with case-insensitive
environment lookup on Windows. VS Code can supply a later `PATH` override from
the default terminal profile, and shell startup can set `PATH` again. The
provider performs no checkout search or terminal input and output work.

The binary directory setting has machine scope so the user of the local host
or WSL distribution controls the executables added to the profile. The provider
validates the configured path as one `PATH` entry and resolves its target on
the extension host to check the executable pair. A valid link path can serve
as the initial `PATH` entry. An invalid explicit directory prompts the user to
correct the setting before retrying. WSL users keep their explicit settings
and can choose the shell through VS Code's default profile selection.
Uninstalling the VSIX removes the contributed profile. The configuration
setting can then be removed from User or Remote Settings.

## Verification

Node tests and the benchmark exercise provider behavior, including platform
path handling and executable checks. Verify the installed VSIX, dropdown,
shell and CLI behavior, and process cleanup through the editor's real terminal
in disposable Windows, macOS, Linux, and WSL 2 environments.
