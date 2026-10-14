# Remote tools and WSL

## Remote tools

Run remote clients in your shell. They manage command parsing, credentials,
configuration, authentication, and networking. Automexia manages the local
terminal, navigation, session exit, and cleanup.

## WSL

Windows Subsystem for Linux (WSL) runs Linux environments on Windows.

Use each WSL distribution's package manager, shell configuration, OpenSSH, and
development tools. Automexia does not silently rewrite profiles or share
terminal state between sessions.

Session-local integration may provide bounded prompt metadata. If it is missing
or disabled, the WSL shell continues normally.

Use the Windows Automexia application to open `wsl.exe`. The distro chooses its
Linux shell and tools. A distro's WSL version (1 or 2) is separate from the
installed WSL application's version. Check each distro with `wsl --list --verbose`.

| Feature | Requirements |
| --- | --- |
| Terminal input, output, tabs and splits | A working WSL distro; no systemd requirement |
| OS, user, directory and shell information | Automexia integration for Bash, Zsh or Fish in that distro |
| Git branch | A Git repository and integrated shell in that distro |
| Other tool information | Published shell selections or supported configuration readers; see the limitations below |
| Clone the current WSL shell | Complete shell metadata; recent `wsl.exe` supporting `--distribution`, `--user`, `--cd` and `--exec` |
| Linux GUI applications | Separate WSLg support on WSL 2; shell tests do not verify this |

If you installed a distro after Automexia, run
`automexia shell-integration install` from Windows, then open a new WSL shell.
This explicitly installs the prompt hooks. Shells without an Automexia adapter,
such as Ash, still work as terminals but do not provide the enhanced prompt data.

Cloning checks that the distro is available without blocking the window. Wait
for that check before requesting another clone. Changing panes, shells or
directories during the check cancels it; retry from the intended session.

Compatibility depends on the WSL release, distro and shell together. Tests for
one combination do not prove every distro or older WSL release. See the
[explicit WSL test matrix](../TESTING.md#wsl-version-distro-and-shell-matrix).

Kubernetes prompt badges read the WSL home configuration or shell-specific
KUBECONFIG through a bounded background helper. They show a larger Kubernetes
icon and a clean namespace label, with full context in the accessible description,
and do not use the Windows host's cluster as a fallback. See
[prompt context behavior and limitations](../PROMPT-CONTEXT-ASSURANCE.md).

Git branches work in Linux-native folders and linked worktrees through the same
bounded helper. Inaccessible files, symbolic links and timed-out reads omit the
badge. Docker and cloud selections can come from shell metadata; their unexported
Linux configuration defaults are not yet read by the Windows app.

## Safety and troubleshooting

Use authorized destinations and synthetic documentation examples. Paste never
adds Enter. Keep credentials and real host details out of screenshots and logs.

When a problem occurs, compare the same command in another terminal, identify
the exact Windows/WSL/shell versions, preserve the tool's diagnostic, and verify
that closing the pane removes the owned local process.

See [WSL development](../WSL-DEVELOPMENT.md) and
[Remote connections](../guide/remote-connections.md).
