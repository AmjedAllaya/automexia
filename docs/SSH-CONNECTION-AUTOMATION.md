# System OpenSSH interoperability

Automexia supports ordinary command-line OpenSSH as a free terminal capability.
This page does not specify a managed connection, provider, automation, or
commercial product.

Implementation contract: `F2/D5.0` capability-free review remains separate
from system OpenSSH execution.

## Use

Run the operating system's `ssh`, `scp`, or `sftp` command in a terminal
pane using normal shell syntax. The shell owns parsing; OpenSSH owns
configuration, credential prompts, agents, host-key verification,
authentication, and network behavior.

Automexia owns terminal input/output, resize, selection, search, tabs, panes,
exit reporting, and cleanup. Paste never adds Enter.

## Safe testing

Use authorized destinations only. Documentation and automated/manual tests use
fictional configuration and a disposable loopback fixture. Never publish real
hostnames, account identifiers, keys, tokens, configuration, or remote output.

## Failure and cleanup

OpenSSH diagnostics remain visible and authoritative. Closing a session cleans
up the owned local client process and PTY without modifying user-owned OpenSSH
files. Failed optional inventory or shell integration leaves manual OpenSSH use
available.

See [Remote connections](guide/remote-connections.md) and
[SSH inventory](SSH-INVENTORY.md).

## Assurance evidence anchor compatibility

These headings preserve source-owned feature-matrix references after the
public documentation consolidation. They do not expand shipped behavior,
reintroduce private plans, or replace the current status stated above.

### F2/D5.0 planning dependency

The automation reviews documented by this source consume only the bounded,
capability-free F2/D5.0 profile and plan contract. They do not grant network,
process, PTY, authentication, or credential authority.

### M6 Review Only Implementation

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### Placement In The Existing Architecture

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.

### User Journeys

This compatibility anchor retains traceability to the current public
source, test, and evidence owner. Detailed future or commercial planning
remains private, and unavailable native evidence remains an explicit gate.
