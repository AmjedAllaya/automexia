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

With Automexia's Bash or Zsh hooks active, command names, flags, arguments and
shell syntax use distinct theme colors in the prompt and the submitted command
transcript. A pasted block can use these colors on each line of the same prompt.
Program prompts and shell-provided ANSI styles keep their own appearance.

Use the Windows Automexia application to open `wsl.exe`. The distro chooses its
Linux shell and tools. A distro's WSL version (1 or 2) is separate from the
installed WSL application's version. Check each distro with `wsl --list --verbose`.

With one registered distro, enter `wsl` or `wsl.exe` at an integrated Windows
PowerShell or CMD prompt to check its prerequisites on launch. The warning
appears inside Automexia when the shell hook or `eza` is missing. To check a
particular distro, select its WSL profile in **Settings → Profiles** and choose
**Check WSL setup**.

When the Bash hook is missing, choose **Install Bash integration**. This action
is available for the distro's default non-root user when their shell is Bash.
Automexia copies its bundled Bash files into that user's Linux configuration and
adds a managed block to `.bashrc`. When setup confirms the hook, open a new WSL
Bash session that reads `.bashrc` to load the prompt tags and command colors.

Choose **Install eza** separately on supported Ubuntu and Debian setups. It
opens APT in another tab, where APT can request your Linux password and
confirmation. **Cancel** dismisses the setup popup and keeps the WSL session
open. From Windows, `automexia shell-integration doctor --wsl Ubuntu` prints
the prerequisite check. Add `--offer-install` for the command-line `eza` offer.

| Feature | Requirements |
| --- | --- |
| Terminal input, output, tabs and splits | A working WSL distro; no systemd requirement |
| OS, user, directory and shell information | Automexia integration for Bash, Zsh or Fish in that distro |
| Git branch | A Git repository and integrated shell in that distro |
| Icon-aware `ls` and `ll` | Bash or Zsh integration and `eza` inside the distro. `AUTOMEXIA_PLAIN_LS=1` disables enhanced listings. |
| Other tool information | Published shell selections or supported configuration readers; see the limitations below |
| Clone the current WSL shell | Complete shell metadata; recent `wsl.exe` supporting `--distribution`, `--user`, `--cd` and `--exec` |
| Linux GUI applications | Separate WSLg support on WSL 2; shell tests do not verify this |

For other supported shells, use `automexia shell-integration install` from
Windows, then open a new WSL shell. Shells without an Automexia adapter, such
as Ash, still work as terminals with their usual prompt.

When working from a source checkout in WSL, you can install the integration for
that distro by running `sh shell-integration/install-unix.sh` from the repository
root. This updates the managed shell startup blocks in that Linux user account.
Open a new WSL shell to see the prompt tags and command colors. File icons also
need the distro's `eza` package and an interactive listing.

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

In Bash, `set -e` stays active for subsequent commands. An unhandled failure,
such as a missing tool, can end that Bash session. If you launched `wsl` from
PowerShell or Command Prompt, the Windows prompt then resumes in the same
pane. With `errexit` off in your interactive shell, put strict settings inside
a script or a subshell so a failed check returns to the WSL prompt.

For a multiline paste at an empty WSL Bash prompt, choose **Bash batch** in the
paste preview. Review the prepared text, choose **Paste**, then press Enter at
the shell. The prepared batch is one printable line, so Bash line editors
without bracketed paste can also wait for Enter. A child Bash runs the batch
with fail-fast behavior and reports its exit status in the pane. Directory,
shell option, and variable changes stay in the child.
Choose **Paste** directly when you intend changes to persist in the current shell.

```bash
(
  set -euo pipefail
  node --version
  npm --version
)
```

When a problem occurs, compare the same command in another terminal, identify
the exact Windows/WSL/shell versions, preserve the tool's diagnostic, and verify
that closing the pane removes the owned local process.

See [WSL development](../WSL-DEVELOPMENT.md) and
[Remote connections](../guide/remote-connections.md).
