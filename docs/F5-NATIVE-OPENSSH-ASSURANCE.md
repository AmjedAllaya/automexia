# Native system-OpenSSH assurance

This page covers manual system-OpenSSH interoperability as a public free
terminal behavior. It does not authorize or describe a managed connection
product.

## Test fixture

Use an authorized disposable loopback OpenSSH server and fictional values.
Record exact Automexia, operating-system, OpenSSH, shell, and package versions.
Never use production hosts or publish credentials, configuration, or remote
output.

## Scenarios

- start `ssh` with normal shell syntax and an exact loopback destination;
- review host-key and authentication prompts owned by OpenSSH;
- type, paste without Enter, interrupt, search, select, copy, and resize;
- use tabs and panes while keeping sessions isolated;
- exit normally, cancel, close the pane, and close the application;
- exercise connection failure and changed fixture identity; and
- inspect owned local processes, PTYs, handles, and workers after cleanup.

## Expected result

Automexia behaves as an ordinary terminal. It does not take credential,
host-key, configuration, authentication, or network ownership. Input reaches
only the focused session, resize remains coherent, diagnostics remain visible,
and all owned local session resources are cleaned up.

Native results apply only to the exact platform and environment that ran.
