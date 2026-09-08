# Remote tools and WSL

## Remote tools

Run remote-access clients in the real shell. System OpenSSH and other
command-line tools keep their own parsing, credentials, configuration,
authentication, and network behavior. Automexia supplies terminal input/output,
resize, tabs, panes, search, selection, exit reporting, and cleanup.

## WSL

A WSL distribution is an ordinary shell environment. Use its native package
manager, shell configuration, OpenSSH client, and developer tools. Automexia
does not silently rewrite distribution profiles or share terminal state across
sessions.

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
