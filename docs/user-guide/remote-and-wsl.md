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

Kubernetes prompt badges read the WSL home configuration or shell-specific
KUBECONFIG through a bounded background helper. They show a larger Kubernetes
icon and a clean namespace label, with full context in the accessible description,
and do not use the Windows host's cluster as a fallback. See
[prompt context behavior and limitations](../PROMPT-CONTEXT-ASSURANCE.md).

## Safety and troubleshooting

Use authorized destinations and synthetic documentation examples. Paste never
adds Enter. Keep credentials and real host details out of screenshots and logs.

When a problem occurs, compare the same command in another terminal, identify
the exact Windows/WSL/shell versions, preserve the tool's diagnostic, and verify
that closing the pane removes the owned local process.

See [WSL development](../WSL-DEVELOPMENT.md) and
[Remote connections](../guide/remote-connections.md).
